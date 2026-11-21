//! Zmodem file transfer (CRC-16 headers and data from our side, CRC-32 accepted) over the open connection.

use std::path::PathBuf;
use std::time::{Duration, Instant};

const ZPAD: u8 = b'*';
const ZDLE: u8 = 0x18;

const ZRQINIT: u8 = 0;
const ZRINIT: u8 = 1;
const ZACK: u8 = 3;
const ZFILE: u8 = 4;
const ZSKIP: u8 = 5;
const ZNAK: u8 = 6;
const ZABORT: u8 = 7;
const ZFIN: u8 = 8;
const ZRPOS: u8 = 9;
const ZDATA: u8 = 10;
const ZEOF: u8 = 11;

const ZCRCE: u8 = b'h';
const ZCRCG: u8 = b'i';
const ZCRCQ: u8 = b'j';
const ZCRCW: u8 = b'k';

fn crc16(data: &[u8]) -> u16 {
    crate::transfer::crc16(data)
}

fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xffff_ffff;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn push_escaped(out: &mut Vec<u8>, b: u8) {
    match b {
        ZDLE | 0x10 | 0x11 | 0x13 | 0x90 | 0x91 | 0x93 => {
            out.push(ZDLE);
            out.push(b ^ 0x40);
        }
        _ => out.push(b),
    }
}

fn hex_header(typ: u8, pos: u32) -> Vec<u8> {
    let mut raw = vec![typ];
    raw.extend_from_slice(&pos.to_le_bytes());
    let c = crc16(&raw);
    raw.push((c >> 8) as u8);
    raw.push(c as u8);
    let mut out = vec![ZPAD, ZPAD, ZDLE, b'B'];
    for b in raw {
        out.extend_from_slice(format!("{b:02x}").as_bytes());
    }
    out.push(0x0d);
    out.push(0x8a);
    if typ != ZACK && typ != ZFIN {
        out.push(0x11);
    }
    out
}

fn bin_header(typ: u8, pos: [u8; 4]) -> Vec<u8> {
    let mut raw = vec![typ];
    raw.extend_from_slice(&pos);
    let c = crc16(&raw);
    let mut out = vec![ZPAD, ZDLE, b'A'];
    for b in raw {
        push_escaped(&mut out, b);
    }
    push_escaped(&mut out, (c >> 8) as u8);
    push_escaped(&mut out, c as u8);
    out
}

fn subpacket(data: &[u8], end: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 8);
    for &b in data {
        push_escaped(&mut out, b);
    }
    out.push(ZDLE);
    out.push(end);
    let mut crc_in = data.to_vec();
    crc_in.push(end);
    let c = crc16(&crc_in);
    push_escaped(&mut out, (c >> 8) as u8);
    push_escaped(&mut out, c as u8);
    out
}

enum Esc {
    Byte(u8),
    End(u8),
}

/// Reads one (possibly ZDLE-escaped) byte at `i`; None when more input is needed. Flow-control bytes are skipped.
fn unescape(buf: &[u8], mut i: usize) -> Option<(Esc, usize)> {
    loop {
        let b = *buf.get(i)?;
        match b {
            0x11 | 0x13 | 0x91 | 0x93 => {
                i += 1;
            }
            ZDLE => {
                let n = *buf.get(i + 1)?;
                return Some(match n {
                    ZCRCE | ZCRCG | ZCRCQ | ZCRCW => (Esc::End(n), i + 2),
                    b'l' => (Esc::Byte(0x7f), i + 2),
                    b'm' => (Esc::Byte(0xff), i + 2),
                    _ => (Esc::Byte(n ^ 0x40), i + 2),
                });
            }
            _ => return Some((Esc::Byte(b), i + 1)),
        }
    }
}

#[derive(Debug)]
enum Item {
    Header { typ: u8, pos: u32, c32: bool },
    /// a data subpacket; `ok` is the CRC result
    Sub { data: Vec<u8>, end: u8, ok: bool },
}

fn hexval(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    // sender
    SWaitInit,
    SWaitPos,
    SData,
    SWaitEofAck,
    SWaitFin,
    // receiver
    RWaitFile,
    RWaitData,
    End,
}

pub struct Zmodem {
    pub send: bool,
    pub status: String,
    pub done: bool,
    pub failed: bool,
    pub bytes_done: usize,
    pub total: usize,
    pub name: String,
    folder: PathBuf,
    files: Vec<PathBuf>,
    cur: usize,
    data: Vec<u8>,
    pos: usize,
    state: State,
    rx: Vec<u8>,
    /// the next thing on the wire is a data subpacket (CRC type of the header before it)
    want_sub: Option<bool>,
    out_file: Vec<u8>,
    last: Instant,
    retries: u32,
    started: bool,
    sent_total: usize,
    rpos_sent: bool,
    batch_total: usize,
}

impl Zmodem {
    fn base(send: bool, folder: PathBuf) -> Zmodem {
        Zmodem {
            send,
            status: String::new(),
            done: false,
            failed: false,
            bytes_done: 0,
            total: 0,
            name: String::new(),
            folder,
            files: Vec::new(),
            cur: 0,
            data: Vec::new(),
            pos: 0,
            state: State::End,
            rx: Vec::new(),
            want_sub: None,
            out_file: Vec::new(),
            last: Instant::now(),
            retries: 0,
            started: false,
            sent_total: 0,
            rpos_sent: false,
            batch_total: 0,
        }
    }

    pub fn new_send(files: Vec<PathBuf>) -> Result<Zmodem, String> {
        if files.is_empty() {
            return Err("The upload list is empty.".into());
        }
        let mut z = Zmodem::base(true, PathBuf::new());
        for f in &files {
            let md = std::fs::metadata(f).map_err(|e| format!("Could not read {}: {e}", f.display()))?;
            z.batch_total += md.len() as usize;
        }
        z.files = files;
        z.state = State::SWaitInit;
        z.status = "Waiting for the receiver to start...".into();
        Ok(z)
    }

    pub fn new_receive(folder: PathBuf) -> Zmodem {
        let mut z = Zmodem::base(false, folder);
        z.state = State::RWaitFile;
        z.status = "Waiting for the sender...".into();
        z
    }

    fn fail(&mut self, why: &str) {
        self.done = true;
        self.failed = true;
        self.state = State::End;
        self.status = why.to_string();
    }

    fn finish(&mut self, msg: &str) {
        self.done = true;
        self.state = State::End;
        self.status = msg.to_string();
    }

    pub fn cancel(&mut self) -> Vec<u8> {
        if !self.done {
            self.fail("Cancelled");
        }
        let mut v = vec![ZDLE; 8];
        v.extend_from_slice(&[8; 8]);
        v
    }

    fn zrinit() -> Vec<u8> {
        // CANFDX | CANOVIO: the sender answers with CRC-16 frames
        hex_header(ZRINIT, u32::from_le_bytes([0, 0, 0, 0x03]))
    }

    fn load_current(&mut self) -> Result<Vec<u8>, String> {
        let f = self.files[self.cur].clone();
        self.data = std::fs::read(&f).map_err(|e| format!("Could not read {}: {e}", f.display()))?;
        self.name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        self.total = self.data.len();
        self.pos = 0;
        self.bytes_done = 0;
        let mut info = self.name.as_bytes().to_vec();
        info.push(0);
        info.extend_from_slice(format!("{} 0 100644 0 {} {}", self.data.len(), self.files.len() - self.cur, self.batch_total - self.sent_total).as_bytes());
        info.push(0);
        let mut out = bin_header(ZFILE, [0, 0, 0, 1]);
        out.extend(subpacket(&info, ZCRCW));
        Ok(out)
    }

    pub fn tick(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        match self.state {
            State::SWaitInit => {
                if !self.started || self.last.elapsed() > Duration::from_secs(4) {
                    if self.started {
                        self.retries += 1;
                        if self.retries > 12 {
                            self.fail("The receiver did not start");
                            return out;
                        }
                    }
                    self.started = true;
                    self.last = Instant::now();
                    out.extend(hex_header(ZRQINIT, 0));
                }
            }
            State::SData => self.send_chunks(&mut out),
            State::RWaitFile => {
                if !self.started {
                    self.started = true;
                    self.last = Instant::now();
                    out.extend(Zmodem::zrinit());
                } else if self.last.elapsed() > Duration::from_secs(4) {
                    self.last = Instant::now();
                    self.retries += 1;
                    if self.retries > 15 {
                        self.fail("The sender did not start");
                    } else {
                        out.extend(Zmodem::zrinit());
                    }
                }
            }
            State::RWaitData => {
                if self.last.elapsed() > Duration::from_secs(4) {
                    // nothing arrived: ask for the position again
                    self.retries += 1;
                    if self.retries > 5 {
                        self.fail("Timed out");
                    } else {
                        self.rpos_sent = false;
                        self.request_pos(&mut out);
                        self.last = Instant::now();
                    }
                }
            }
            State::End => {}
            _ => {
                if self.last.elapsed() > Duration::from_secs(15) {
                    self.fail("Timed out");
                }
            }
        }
        out
    }

    /// Up to 32 KiB of file data per call; the last chunk ends the frame and is followed by ZEOF.
    fn send_chunks(&mut self, out: &mut Vec<u8>) {
        for _ in 0..32 {
            let end = (self.pos + 1024).min(self.data.len());
            let last = end >= self.data.len();
            out.extend(subpacket(&self.data[self.pos..end], if last { ZCRCE } else { ZCRCG }));
            self.pos = end;
            self.bytes_done = self.pos;
            if last {
                out.extend(bin_header(ZEOF, (self.data.len() as u32).to_le_bytes()));
                self.state = State::SWaitEofAck;
                self.last = Instant::now();
                return;
            }
        }
        self.last = Instant::now();
    }

    fn start_data(&mut self, pos: usize, out: &mut Vec<u8>) {
        self.pos = pos.min(self.data.len());
        out.extend(bin_header(ZDATA, (self.pos as u32).to_le_bytes()));
        self.state = State::SData;
        self.status = "Sending...".into();
        if self.data.is_empty() {
            // empty file: no data, just the end of file
            out.extend(subpacket(&[], ZCRCE));
            out.extend(bin_header(ZEOF, [0; 4]));
            self.state = State::SWaitEofAck;
        }
    }

    fn next_item(&mut self) -> Option<Item> {
        if let Some(c32) = self.want_sub {
            // data subpacket
            let mut i = 0;
            let mut data = Vec::new();
            loop {
                let (e, n) = unescape(&self.rx, i)?;
                i = n;
                match e {
                    Esc::Byte(b) => data.push(b),
                    Esc::End(end) => {
                        let nc = if c32 { 4 } else { 2 };
                        let mut crc = Vec::new();
                        for _ in 0..nc {
                            let (e2, n2) = unescape(&self.rx, i)?;
                            i = n2;
                            match e2 {
                                Esc::Byte(b) => crc.push(b),
                                Esc::End(b) => crc.push(b),
                            }
                        }
                        let mut calc_in = data.clone();
                        calc_in.push(end);
                        let ok = if c32 {
                            crc32(&calc_in).to_le_bytes().to_vec() == crc
                        } else {
                            let c = crc16(&calc_in);
                            crc == vec![(c >> 8) as u8, c as u8]
                        };
                        self.rx.drain(..i);
                        // a frame that continues keeps expecting subpackets
                        if end == ZCRCE || end == ZCRCW || !ok {
                            self.want_sub = None;
                        }
                        return Some(Item::Sub { data, end, ok });
                    }
                }
            }
        }
        // header: find ZPAD ZDLE
        loop {
            let Some(p) = self.rx.iter().position(|&b| b == ZPAD) else {
                self.rx.clear();
                return None;
            };
            self.rx.drain(..p);
            let mut i = 0;
            while self.rx.get(i) == Some(&ZPAD) {
                i += 1;
            }
            if i >= self.rx.len() {
                return None;
            }
            if self.rx[i] != ZDLE {
                self.rx.drain(..i);
                continue;
            }
            let kind = *self.rx.get(i + 1)?;
            match kind {
                b'B' => {
                    // 14 hex digits
                    if self.rx.len() < i + 2 + 14 {
                        return None;
                    }
                    let h = &self.rx[i + 2..i + 16];
                    let mut raw = Vec::new();
                    let mut ok = true;
                    for pair in h.chunks(2) {
                        match (hexval(pair[0]), hexval(pair[1])) {
                            (Some(a), Some(b)) => raw.push(a << 4 | b),
                            _ => ok = false,
                        }
                    }
                    self.rx.drain(..i + 16);
                    // trailing CR LF XON
                    while matches!(self.rx.first(), Some(0x0d | 0x8d | 0x0a | 0x8a | 0x11)) {
                        self.rx.remove(0);
                    }
                    if !ok || raw.len() != 7 {
                        continue;
                    }
                    let c = crc16(&raw[..5]);
                    if raw[5] != (c >> 8) as u8 || raw[6] != c as u8 {
                        continue;
                    }
                    let pos = u32::from_le_bytes([raw[1], raw[2], raw[3], raw[4]]);
                    return Some(Item::Header { typ: raw[0], pos, c32: false });
                }
                b'A' | b'C' => {
                    let c32 = kind == b'C';
                    let need = if c32 { 9 } else { 7 };
                    let mut j = i + 2;
                    let mut raw = Vec::new();
                    while raw.len() < need {
                        let (e, n) = unescape(&self.rx, j)?;
                        j = n;
                        match e {
                            Esc::Byte(b) | Esc::End(b) => raw.push(b),
                        }
                    }
                    self.rx.drain(..j);
                    let ok = if c32 {
                        crc32(&raw[..5]).to_le_bytes()[..] == raw[5..9]
                    } else {
                        let c = crc16(&raw[..5]);
                        raw[5] == (c >> 8) as u8 && raw[6] == c as u8
                    };
                    if !ok {
                        continue;
                    }
                    let pos = u32::from_le_bytes([raw[1], raw[2], raw[3], raw[4]]);
                    return Some(Item::Header { typ: raw[0], pos, c32 });
                }
                _ => {
                    self.rx.drain(..i + 1);
                }
            }
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        self.last = Instant::now();
        self.rx.extend_from_slice(bytes);
        while !self.done {
            let Some(item) = self.next_item() else { break };
            if self.send {
                self.send_item(item, &mut out);
            } else {
                self.recv_item(item, &mut out);
            }
        }
        // keep the buffer bounded when line noise has no frames
        if self.rx.len() > 1 << 16 {
            self.rx.clear();
        }
        out
    }

    fn send_item(&mut self, item: Item, out: &mut Vec<u8>) {
        let Item::Header { typ, pos, .. } = item else { return };
        match (self.state, typ) {
            (State::SWaitInit, ZRINIT) | (State::SWaitEofAck, ZRINIT) => {
                if self.state == State::SWaitEofAck {
                    self.sent_total += self.data.len();
                    self.cur += 1;
                }
                if self.cur >= self.files.len() {
                    out.extend(hex_header(ZFIN, 0));
                    self.state = State::SWaitFin;
                    self.status = "Finishing...".into();
                } else {
                    match self.load_current() {
                        Ok(b) => {
                            out.extend(b);
                            self.state = State::SWaitPos;
                            self.status = "Sending...".into();
                        }
                        Err(e) => self.fail(&e),
                    }
                }
            }
            (State::SWaitPos, ZRPOS) | (State::SData, ZRPOS) | (State::SWaitEofAck, ZRPOS) => self.start_data(pos as usize, out),
            (State::SWaitPos, ZSKIP) | (State::SData, ZSKIP) | (State::SWaitEofAck, ZSKIP) => {
                self.sent_total += self.data.len();
                self.cur += 1;
                if self.cur >= self.files.len() {
                    out.extend(hex_header(ZFIN, 0));
                    self.state = State::SWaitFin;
                } else {
                    match self.load_current() {
                        Ok(b) => {
                            out.extend(b);
                            self.state = State::SWaitPos;
                        }
                        Err(e) => self.fail(&e),
                    }
                }
            }
            (State::SWaitFin, ZFIN) => {
                out.extend_from_slice(b"OO");
                self.bytes_done = self.total;
                self.finish("Transfer complete");
            }
            (_, ZABORT) => self.fail("The receiver aborted the transfer"),
            (State::SWaitPos, ZNAK) => {
                // ZFILE damaged: send it again
                if let Ok(b) = self.load_current() {
                    out.extend(b);
                }
            }
            _ => {}
        }
    }

    fn recv_item(&mut self, item: Item, out: &mut Vec<u8>) {
        match item {
            Item::Header { typ, pos, c32 } => match (self.state, typ) {
                (_, ZRQINIT) => out.extend(Zmodem::zrinit()),
                (State::RWaitFile, ZFILE) => self.want_sub = Some(c32),
                (State::RWaitData, ZFILE) => self.want_sub = Some(c32),
                (State::RWaitData, ZDATA) => {
                    if pos as usize == self.out_file.len() {
                        self.want_sub = Some(c32);
                    } else {
                        // out of step: ask for the position we have
                        self.request_pos(out);
                    }
                }
                (State::RWaitData, ZEOF) => {
                    if pos as usize != self.out_file.len() {
                        self.request_pos(out);
                        return;
                    }
                    if let Err(e) = self.save() {
                        self.fail(&e);
                        return;
                    }
                    self.state = State::RWaitFile;
                    out.extend(Zmodem::zrinit());
                }
                (State::RWaitFile, ZFIN) | (State::RWaitData, ZFIN) => {
                    out.extend(hex_header(ZFIN, 0));
                    self.finish("Transfer complete");
                }
                (_, ZABORT) => self.fail("The sender aborted the transfer"),
                _ => {}
            },
            Item::Sub { data, end, ok } => match self.state {
                State::RWaitFile => {
                    // the ZFILE info: name NUL size ...
                    if !ok {
                        out.extend(hex_header(ZNAK, 0));
                        return;
                    }
                    let nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                    let raw = String::from_utf8_lossy(&data[..nul]).into_owned();
                    let base = raw.rsplit(['/', '\\']).next().unwrap_or("").trim().to_string();
                    self.name = if base.is_empty() || base == "." || base == ".." { "received.bin".into() } else { base };
                    let rest = String::from_utf8_lossy(data.get(nul + 1..).unwrap_or(&[])).into_owned();
                    self.total = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    self.out_file.clear();
                    self.bytes_done = 0;
                    self.state = State::RWaitData;
                    self.status = "Receiving...".into();
                    out.extend(hex_header(ZRPOS, 0));
                }
                State::RWaitData => {
                    if !ok {
                        self.request_pos(out);
                        self.want_sub = None;
                        return;
                    }
                    self.rpos_sent = false;
                    self.retries = 0;
                    self.out_file.extend_from_slice(&data);
                    self.bytes_done = self.out_file.len();
                    self.total = self.total.max(self.bytes_done);
                    if end == ZCRCQ || end == ZCRCW {
                        out.extend(hex_header(ZACK, self.out_file.len() as u32));
                    }
                }
                _ => {}
            },
        }
    }

    /// Asks the sender to resume at the bytes already saved; once per error until data arrives again.
    fn request_pos(&mut self, out: &mut Vec<u8>) {
        if !self.rpos_sent {
            self.rpos_sent = true;
            out.extend(hex_header(ZRPOS, self.out_file.len() as u32));
        }
    }

    fn save(&mut self) -> Result<(), String> {
        std::fs::create_dir_all(&self.folder).map_err(|e| format!("Could not write the file: {e}"))?;
        std::fs::write(self.folder.join(&self.name), &self.out_file).map_err(|e| format!("Could not write the file: {e}"))?;
        Ok(())
    }

    pub fn is_start_sequence(data: &[u8]) -> bool {
        // "**" ZDLE "B00" is the sender's ZRQINIT
        data.windows(8).any(|w| w[0] == ZPAD && w[1] == ZPAD && w[2] == ZDLE && w[3] == b'B' && w[4] == b'0' && w[5] == b'0')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(files: Vec<PathBuf>, out_dir: PathBuf, damage: bool) {
        let mut tx = Zmodem::new_send(files).unwrap();
        let mut rx = Zmodem::new_receive(out_dir);
        let mut to_rx: Vec<u8> = Vec::new();
        let mut to_tx: Vec<u8> = Vec::new();
        let mut hit = false;
        for _ in 0..4000 {
            to_tx.extend(rx.tick());
            to_rx.extend(tx.tick());
            if !to_rx.is_empty() {
                let mut b = std::mem::take(&mut to_rx);
                if damage && !hit && b.len() > 3000 {
                    hit = true;
                    let l = b.len();
                    b[l / 3] ^= 0x04;
                }
                to_tx.extend(rx.feed(&b));
            }
            if !to_tx.is_empty() {
                let b = std::mem::take(&mut to_tx);
                to_rx.extend(tx.feed(&b));
            }
            if tx.done && rx.done {
                break;
            }
        }
        assert!(tx.done && !tx.failed, "sender: {} {:?} rx {:?} {} pos {}/{}", tx.status, tx.state, rx.state, rx.status, rx.out_file.len(), tx.data.len());
        assert!(rx.done && !rx.failed, "receiver: {}", rx.status);
    }

    fn sample(n: usize, mul: u32) -> Vec<u8> {
        // includes every byte value, so the escaping is exercised
        (0..n as u32).map(|i| (i * mul % 256) as u8).collect()
    }

    #[test]
    fn two_files_with_every_byte_value() {
        let dir = std::env::temp_dir().join("yetty_zmodem_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.bin"), dir.join("b.bin"));
        let (da, db) = (sample(9000, 7), sample(300, 13));
        std::fs::write(&a, &da).unwrap();
        std::fs::write(&b, &db).unwrap();
        let out = dir.join("out");
        run(vec![a, b], out.clone(), false);
        assert_eq!(std::fs::read(out.join("a.bin")).unwrap(), da);
        assert_eq!(std::fs::read(out.join("b.bin")).unwrap(), db);
    }

    #[test]
    fn crc_error_restarts_from_the_receiver_position() {
        let dir = std::env::temp_dir().join("yetty_zmodem_test2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.bin");
        let da = sample(20000, 11);
        std::fs::write(&a, &da).unwrap();
        let out = dir.join("out");
        run(vec![a], out.clone(), true);
        assert_eq!(std::fs::read(out.join("a.bin")).unwrap(), da);
    }
}
