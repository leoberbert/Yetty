//! XMODEM (checksum, CRC, 1K) and YMODEM file transfers over the open connection.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const CRC_REQ: u8 = b'C';

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Proto {
    Xmodem,
    Xmodem1k,
    Ymodem,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dir {
    Send,
    Receive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    SWaitStart,
    SWaitAck0,
    SWaitC1,
    SData,
    SWaitEot,
    SWaitC2,
    SWaitAckFinal,
    RStart,
    RData,
    End,
}

pub struct Transfer {
    pub proto: Proto,
    pub dir: Dir,
    pub status: String,
    pub done: bool,
    pub failed: bool,
    pub bytes_done: usize,
    pub total: usize,
    pub name: String,
    path: PathBuf,
    data: Vec<u8>,
    pos: usize,
    blk: u8,
    use_crc: bool,
    phase: Phase,
    rx: Vec<u8>,
    retries: u32,
    last: Instant,
    ysize: Option<usize>,
    eot_seen: bool,
    c_sent: u32,
    last_out: Vec<u8>,
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

impl Transfer {
    fn base(proto: Proto, dir: Dir, path: PathBuf) -> Transfer {
        Transfer {
            proto,
            dir,
            status: String::new(),
            done: false,
            failed: false,
            bytes_done: 0,
            total: 0,
            name: String::new(),
            path,
            data: Vec::new(),
            pos: 0,
            blk: 1,
            use_crc: true,
            phase: Phase::End,
            rx: Vec::new(),
            retries: 0,
            last: Instant::now(),
            ysize: None,
            eot_seen: false,
            c_sent: 0,
            last_out: Vec::new(),
        }
    }

    pub fn new_send(proto: Proto, path: &Path) -> Result<Transfer, String> {
        let data = std::fs::read(path).map_err(|e| format!("Could not read the file: {e}"))?;
        if data.is_empty() {
            return Err("The file is empty.".into());
        }
        let mut t = Transfer::base(proto, Dir::Send, path.to_path_buf());
        t.name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        t.total = data.len();
        t.data = data;
        t.phase = Phase::SWaitStart;
        t.status = "Waiting for the receiver to start...".into();
        Ok(t)
    }

    /// For XMODEM `path` is the file to write; for YMODEM it is the folder that receives the files.
    pub fn new_receive(proto: Proto, path: PathBuf) -> Transfer {
        let mut t = Transfer::base(proto, Dir::Receive, path);
        t.phase = Phase::RStart;
        t.blk = if proto == Proto::Ymodem { 0 } else { 1 };
        t.status = "Waiting for the sender...".into();
        t.name = t.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        t
    }

    pub fn label(&self) -> &'static str {
        match (self.proto, self.dir) {
            (Proto::Xmodem, Dir::Send) => "Send Xmodem",
            (Proto::Xmodem, Dir::Receive) => "Receive Xmodem",
            (Proto::Xmodem1k, Dir::Send) => "Send Xmodem-1K",
            (Proto::Xmodem1k, Dir::Receive) => "Receive Xmodem",
            (Proto::Ymodem, Dir::Send) => "Send Ymodem",
            (Proto::Ymodem, Dir::Receive) => "Receive Ymodem",
        }
    }

    fn block_size(&self) -> usize {
        if self.proto == Proto::Xmodem {
            128
        } else {
            1024
        }
    }

    fn block_bytes(&self, num: u8, payload: &[u8], size: usize, pad: u8) -> Vec<u8> {
        let mut b = vec![if size == 1024 { STX } else { SOH }, num, !num];
        let mut p = payload.to_vec();
        p.resize(size, pad);
        if self.use_crc {
            let c = crc16(&p);
            b.extend_from_slice(&p);
            b.push((c >> 8) as u8);
            b.push(c as u8);
        } else {
            let sum = p.iter().fold(0u8, |a, &x| a.wrapping_add(x));
            b.extend_from_slice(&p);
            b.push(sum);
        }
        b
    }

    fn data_block(&self) -> Vec<u8> {
        let size = self.block_size();
        let end = (self.pos + size).min(self.data.len());
        self.block_bytes(self.blk, &self.data[self.pos..end], size, SUB)
    }

    fn header_block(&self) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(self.name.as_bytes());
        p.push(0);
        p.extend_from_slice(self.data.len().to_string().as_bytes());
        let size = if p.len() > 120 { 1024 } else { 128 };
        self.block_bytes(0, &p, size, 0)
    }

    fn fail(&mut self, why: &str) {
        self.done = true;
        self.failed = true;
        self.phase = Phase::End;
        self.status = why.to_string();
    }

    fn finish(&mut self, msg: &str) {
        self.done = true;
        self.phase = Phase::End;
        self.status = msg.to_string();
    }

    fn emit(&mut self, out: &mut Vec<u8>, bytes: Vec<u8>) {
        self.last_out = bytes.clone();
        out.extend_from_slice(&bytes);
        self.last = Instant::now();
    }

    fn resend(&mut self, out: &mut Vec<u8>) {
        self.retries += 1;
        if self.retries > 10 {
            self.fail("Too many errors");
            return;
        }
        out.extend_from_slice(&self.last_out.clone());
        self.last = Instant::now();
    }

    /// Cancel: tell the other side.
    pub fn cancel(&mut self) -> Vec<u8> {
        if !self.done {
            self.fail("Cancelled");
        }
        vec![CAN, CAN, CAN, CAN, CAN]
    }

    pub fn tick(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        match self.phase {
            Phase::RStart => {
                if self.c_sent == 0 || self.last.elapsed() > Duration::from_secs(3) {
                    if self.c_sent >= 8 && self.proto != Proto::Ymodem {
                        self.use_crc = false;
                        self.emit(&mut out, vec![NAK]);
                    } else {
                        self.use_crc = true;
                        self.emit(&mut out, vec![CRC_REQ]);
                    }
                    self.c_sent += 1;
                    if self.c_sent > 20 {
                        self.fail("The sender did not start");
                    }
                }
            }
            Phase::RData => {
                if self.last.elapsed() > Duration::from_secs(10) {
                    self.rx.clear();
                    self.retries += 1;
                    if self.retries > 10 {
                        self.fail("Timed out");
                    } else {
                        self.emit(&mut out, vec![NAK]);
                    }
                }
            }
            Phase::End => {}
            _ => {
                if self.last.elapsed() > Duration::from_secs(10) {
                    if matches!(self.phase, Phase::SWaitStart | Phase::SWaitC1 | Phase::SWaitC2) {
                        self.retries += 1;
                        self.last = Instant::now();
                        if self.retries > 6 {
                            self.fail("The receiver did not respond");
                        }
                    } else {
                        self.resend(&mut out);
                    }
                }
            }
        }
        out
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        self.last = Instant::now();
        match self.dir {
            Dir::Send => {
                for &b in bytes {
                    if self.done {
                        break;
                    }
                    self.send_byte(b, &mut out);
                }
            }
            Dir::Receive => {
                self.rx.extend_from_slice(bytes);
                self.receive(&mut out);
            }
        }
        out
    }

    fn send_next_data(&mut self, out: &mut Vec<u8>) {
        let b = self.data_block();
        self.emit(out, b);
        self.phase = Phase::SData;
        self.status = format!("Sending {} ...", self.name);
    }

    fn send_byte(&mut self, b: u8, out: &mut Vec<u8>) {
        if b == CAN {
            self.fail("Cancelled by the other side");
            return;
        }
        match (self.phase, b) {
            (Phase::SWaitStart, CRC_REQ) => {
                self.use_crc = true;
                if self.proto == Proto::Ymodem {
                    let h = self.header_block();
                    self.emit(out, h);
                    self.phase = Phase::SWaitAck0;
                } else {
                    self.send_next_data(out);
                }
            }
            (Phase::SWaitStart, NAK) if self.proto != Proto::Ymodem => {
                self.use_crc = false;
                self.send_next_data(out);
            }
            (Phase::SWaitAck0, ACK) => self.phase = Phase::SWaitC1,
            (Phase::SWaitAck0, NAK) => self.resend(out),
            (Phase::SWaitC1, CRC_REQ) => {
                self.blk = 1;
                self.send_next_data(out);
            }
            (Phase::SData, ACK) => {
                let size = self.block_size();
                self.pos = (self.pos + size).min(self.data.len());
                self.bytes_done = self.pos;
                self.blk = self.blk.wrapping_add(1);
                self.retries = 0;
                if self.pos >= self.data.len() {
                    self.emit(out, vec![EOT]);
                    self.phase = Phase::SWaitEot;
                } else {
                    self.send_next_data(out);
                }
            }
            (Phase::SData, NAK) => self.resend(out),
            (Phase::SWaitEot, ACK) => {
                if self.proto == Proto::Ymodem {
                    self.phase = Phase::SWaitC2;
                } else {
                    self.finish("Transfer complete");
                }
            }
            (Phase::SWaitEot, NAK) => self.resend(out),
            (Phase::SWaitC2, CRC_REQ) => {
                self.use_crc = true;
                let b = self.block_bytes(0, &[], 128, 0);
                self.emit(out, b);
                self.phase = Phase::SWaitAckFinal;
            }
            (Phase::SWaitAckFinal, ACK) => self.finish("Transfer complete"),
            (Phase::SWaitAckFinal, NAK) => self.resend(out),
            _ => {}
        }
    }

    fn receive(&mut self, out: &mut Vec<u8>) {
        loop {
            if self.rx.is_empty() || self.done {
                break;
            }
            match self.rx[0] {
                SOH | STX => {
                    let size = if self.rx[0] == SOH { 128 } else { 1024 };
                    let need = 3 + size + if self.use_crc { 2 } else { 1 };
                    if self.rx.len() < need {
                        break;
                    }
                    let blk: Vec<u8> = self.rx.drain(..need).collect();
                    self.handle_block(&blk, size, out);
                }
                EOT => {
                    self.rx.remove(0);
                    self.handle_eot(out);
                }
                CAN => {
                    self.fail("Cancelled by the other side");
                }
                _ => {
                    self.rx.remove(0);
                }
            }
        }
    }

    fn handle_block(&mut self, blk: &[u8], size: usize, out: &mut Vec<u8>) {
        let payload = &blk[3..3 + size];
        let valid_hdr = blk[1] == !blk[2];
        let valid_sum = if self.use_crc {
            let c = crc16(payload);
            blk[3 + size] == (c >> 8) as u8 && blk[4 + size] == c as u8
        } else {
            blk[3 + size] == payload.iter().fold(0u8, |a, &x| a.wrapping_add(x))
        };
        if !valid_hdr || !valid_sum {
            self.retries += 1;
            if self.retries > 10 {
                self.fail("Too many errors");
            } else {
                self.emit(out, vec![NAK]);
            }
            return;
        }
        let expected = self.blk;
        if blk[1] == expected.wrapping_sub(1) && !(self.phase == Phase::RStart && self.proto == Proto::Ymodem) {
            self.emit(out, vec![ACK]); // duplicate block
            return;
        }
        if blk[1] != expected {
            self.fail("Blocks out of sequence");
            return;
        }
        self.retries = 0;
        if self.proto == Proto::Ymodem && expected == 0 {
            // file header block
            let name_end = payload.iter().position(|&b| b == 0).unwrap_or(0);
            if name_end == 0 {
                self.emit(out, vec![ACK]);
                self.finish("Transfer complete");
                return;
            }
            let raw = String::from_utf8_lossy(&payload[..name_end]).into_owned();
            self.name = Path::new(&raw).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(raw);
            let rest = &payload[name_end + 1..];
            let size_txt: String = rest.iter().take_while(|&&b| b != 0 && b != b' ').map(|&b| b as char).collect();
            self.ysize = size_txt.parse().ok();
            self.total = self.ysize.unwrap_or(0);
            self.data.clear();
            self.emit(out, vec![ACK]);
            self.last_out = vec![ACK];
            out.push(CRC_REQ);
            self.blk = 1;
            self.phase = Phase::RData;
            self.status = format!("Receiving {} ...", self.name);
            return;
        }
        self.phase = Phase::RData;
        self.data.extend_from_slice(payload);
        self.bytes_done = self.data.len();
        self.blk = self.blk.wrapping_add(1);
        self.status = format!("Receiving {} ...", if self.name.is_empty() { "file" } else { &self.name });
        self.emit(out, vec![ACK]);
    }

    fn write_out(&mut self) -> Result<(), String> {
        let mut data = std::mem::take(&mut self.data);
        match (self.proto, self.ysize) {
            (Proto::Ymodem, Some(n)) if n <= data.len() => data.truncate(n),
            (Proto::Ymodem, _) => {}
            _ => {
                while data.last() == Some(&SUB) {
                    data.pop();
                }
            }
        }
        let target = if self.proto == Proto::Ymodem { self.path.join(&self.name) } else { self.path.clone() };
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&target, &data).map_err(|e| format!("Could not save the file: {e}"))?;
        self.bytes_done = data.len();
        Ok(())
    }

    fn handle_eot(&mut self, out: &mut Vec<u8>) {
        if self.proto == Proto::Ymodem {
            if !self.eot_seen {
                self.eot_seen = true;
                self.emit(out, vec![NAK]);
            } else {
                match self.write_out() {
                    Ok(()) => {
                        self.eot_seen = false;
                        self.blk = 0;
                        self.phase = Phase::RStart;
                        self.c_sent = 1;
                        self.ysize = None;
                        out.push(ACK);
                        out.push(CRC_REQ);
                        self.last = Instant::now();
                        self.status = "File received, waiting for the next one...".into();
                    }
                    Err(e) => self.fail(&e),
                }
            }
        } else {
            out.push(ACK);
            match self.write_out() {
                Ok(()) => self.finish("Transfer complete"),
                Err(e) => self.fail(&e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_pair(proto: Proto) {
        let dir = std::env::temp_dir().join(format!("yetty_xfer_test_{:?}", proto));
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("source.bin");
        let data: Vec<u8> = (0..3000u32).map(|i| (i * 7 % 251) as u8).collect();
        std::fs::write(&src, &data).unwrap();
        let dst = if proto == Proto::Ymodem { dir.join("out") } else { dir.join("received.bin") };
        let mut tx = Transfer::new_send(proto, &src).unwrap();
        let mut rx = Transfer::new_receive(if proto == Proto::Xmodem1k { Proto::Xmodem } else { proto }, dst.clone());
        let mut to_tx: Vec<u8> = Vec::new();
        let mut to_rx: Vec<u8> = Vec::new();
        for _ in 0..2000 {
            to_tx.extend(rx.tick());
            to_rx.extend(tx.tick());
            if !to_tx.is_empty() {
                let b = std::mem::take(&mut to_tx);
                to_rx.extend(tx.feed(&b));
            }
            if !to_rx.is_empty() {
                let b = std::mem::take(&mut to_rx);
                to_tx.extend(rx.feed(&b));
            }
            if tx.done && rx.done {
                break;
            }
        }
        assert!(tx.done && !tx.failed, "sender: {}", tx.status);
        if proto == Proto::Ymodem {
            // the receiver waits for the next file header after the batch ends: finish it
            assert!(rx.done || rx.status.contains("next"), "receiver: {}", rx.status);
            let got = std::fs::read(dst.join("source.bin")).unwrap();
            assert_eq!(got, data);
        } else {
            assert!(rx.done && !rx.failed, "receiver: {}", rx.status);
            let mut got = std::fs::read(&dst).unwrap();
            // XMODEM pads the last block, the receiver strips trailing SUB bytes
            got.truncate(data.len());
            assert_eq!(got, data);
        }
    }

    #[test]
    fn xmodem_roundtrip() {
        run_pair(Proto::Xmodem);
    }

    #[test]
    fn xmodem1k_roundtrip() {
        run_pair(Proto::Xmodem1k);
    }

    #[test]
    fn ymodem_roundtrip() {
        run_pair(Proto::Ymodem);
    }
}
