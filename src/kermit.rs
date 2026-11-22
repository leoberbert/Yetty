//! Kermit file transfer (classic packets, type 1 checksum, control and 8-bit prefixing) over the open connection.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MARK: u8 = 0x01;
const MAXL: usize = 90;

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    /// sender: S sent, waiting for its ACK
    SInit,
    SFile,
    SData,
    SEof,
    SBreak,
    /// receiver: waiting for S / F / D / Z / B
    Receiving,
    End,
}

pub struct Kermit {
    pub send: bool,
    pub status: String,
    pub done: bool,
    pub failed: bool,
    pub bytes_done: usize,
    pub total: usize,
    pub name: String,
    path: PathBuf,
    data: Vec<u8>,
    pos: usize,
    seq: u8,
    state: State,
    rx: Vec<u8>,
    last_pkt: Vec<u8>,
    last: Instant,
    retries: u32,
    qbin: bool,
    maxl: usize,
    first: bool,
    out_file: Vec<u8>,
}

fn tochar(n: u8) -> u8 {
    n.wrapping_add(32)
}
fn unchar(c: u8) -> u8 {
    c.wrapping_sub(32)
}
fn ctl(c: u8) -> u8 {
    c ^ 64
}

fn check1(b: &[u8]) -> u8 {
    let s: u32 = b.iter().map(|&x| x as u32).sum();
    tochar(((s + ((s & 192) / 64)) & 63) as u8)
}

fn packet(seq: u8, typ: u8, data: &[u8]) -> Vec<u8> {
    let mut b = vec![tochar((data.len() + 3) as u8), tochar(seq % 64), typ];
    b.extend_from_slice(data);
    let c = check1(&b);
    b.insert(0, MARK);
    b.push(c);
    b.push(b'\r');
    b
}

fn encode_byte(c: u8, qbin: bool, out: &mut Vec<u8>) {
    let hi = c >= 128;
    let c7 = c & 127;
    if hi && qbin {
        out.push(b'&');
    }
    if c7 < 32 || c7 == 127 {
        out.push(b'#');
        out.push(ctl(c7));
    } else if c7 == b'#' || (qbin && c7 == b'&') {
        out.push(b'#');
        out.push(c7);
    } else {
        out.push(c7);
    }
}

fn decode(data: &[u8], qbin: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let mut hi = false;
        if qbin && data[i] == b'&' && i + 1 < data.len() {
            hi = true;
            i += 1;
        }
        let mut c = data[i];
        if c == b'#' && i + 1 < data.len() {
            i += 1;
            let n = data[i];
            c = if matches!(n & 127, b'#' | b'&' | b'~') { n } else { ctl(n) };
        }
        out.push(if hi { c | 128 } else { c });
        i += 1;
    }
    out
}

fn our_params() -> Vec<u8> {
    vec![tochar(MAXL as u8), tochar(5), tochar(0), ctl(0), tochar(13), b'#', b'&', b'1', b' ', tochar(0)]
}

struct Pkt {
    seq: u8,
    typ: u8,
    data: Vec<u8>,
}

impl Kermit {
    fn base(send: bool, path: PathBuf) -> Kermit {
        Kermit {
            send,
            status: String::new(),
            done: false,
            failed: false,
            bytes_done: 0,
            total: 0,
            name: String::new(),
            path,
            data: Vec::new(),
            pos: 0,
            seq: 0,
            state: State::End,
            rx: Vec::new(),
            last_pkt: Vec::new(),
            last: Instant::now(),
            retries: 0,
            qbin: false,
            maxl: 80,
            first: true,
            out_file: Vec::new(),
        }
    }

    pub fn new_send(path: &Path) -> Result<Kermit, String> {
        let data = std::fs::read(path).map_err(|e| format!("Could not read the file: {e}"))?;
        let mut k = Kermit::base(true, path.to_path_buf());
        k.name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        k.total = data.len();
        k.data = data;
        k.state = State::SInit;
        k.status = "Waiting for the receiver to start...".into();
        Ok(k)
    }

    /// `folder` receives the incoming files.
    pub fn new_receive(folder: PathBuf) -> Kermit {
        let mut k = Kermit::base(false, folder);
        k.state = State::Receiving;
        k.seq = 0;
        k.status = "Waiting for the sender...".into();
        k
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

    fn emit(&mut self, out: &mut Vec<u8>, p: Vec<u8>) {
        self.last_pkt = p.clone();
        out.extend_from_slice(&p);
        self.last = Instant::now();
        self.retries = 0;
    }

    pub fn cancel(&mut self) -> Vec<u8> {
        if !self.done {
            self.fail("Cancelled");
        }
        packet(self.seq, b'E', b"Cancelled")
    }

    pub fn tick(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        if self.send && self.first {
            self.first = false;
            let p = packet(0, b'S', &our_params());
            self.emit(&mut out, p);
            return out;
        }
        if self.last.elapsed() > Duration::from_secs(5) {
            self.last = Instant::now();
            self.retries += 1;
            let limit = if self.state == State::Receiving && self.last_pkt.is_empty() { 24 } else { 10 };
            if self.retries > limit {
                self.fail(if self.send { "The receiver did not respond" } else { "The sender did not respond" });
            } else if !self.last_pkt.is_empty() {
                out.extend_from_slice(&self.last_pkt.clone());
            } else if !self.send {
                // still waiting for the first packet: ask for it
                out.extend_from_slice(&packet(0, b'N', &[]));
            }
        }
        out
    }

    fn parse(&mut self) -> Vec<Result<Pkt, u8>> {
        // Ok(packet) or Err(seq of a damaged packet when it could be read)
        let mut res = Vec::new();
        loop {
            let Some(start) = self.rx.iter().position(|&b| b == MARK) else {
                self.rx.clear();
                break;
            };
            self.rx.drain(..start);
            if self.rx.len() < 2 {
                break;
            }
            let len = unchar(self.rx[1]) as usize;
            if len < 3 {
                self.rx.remove(0);
                continue;
            }
            if self.rx.len() < len + 2 {
                break;
            }
            let body: Vec<u8> = self.rx[1..len + 2].to_vec();
            self.rx.drain(..len + 2);
            let (head, chk) = body.split_at(body.len() - 1);
            let seq = unchar(head[1]);
            if check1(head) != chk[0] {
                res.push(Err(seq));
                continue;
            }
            res.push(Ok(Pkt { seq, typ: head[2], data: head[3..].to_vec() }));
        }
        res
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if self.done {
            return out;
        }
        self.rx.extend_from_slice(bytes);
        for r in self.parse() {
            if self.done {
                break;
            }
            if self.send {
                self.send_packet(r, &mut out);
            } else {
                self.recv_packet(r, &mut out);
            }
        }
        out
    }

    fn next_data(&mut self) -> Vec<u8> {
        let cap = self.maxl.saturating_sub(3 + 4).max(8);
        let mut d = Vec::new();
        while self.pos < self.data.len() && d.len() + 4 <= cap {
            encode_byte(self.data[self.pos], self.qbin, &mut d);
            self.pos += 1;
        }
        d
    }

    fn send_packet(&mut self, r: Result<Pkt, u8>, out: &mut Vec<u8>) {
        let p = match r {
            Ok(p) => p,
            Err(_) => {
                // damaged answer: send the packet again
                if !self.last_pkt.is_empty() {
                    out.extend_from_slice(&self.last_pkt.clone());
                }
                return;
            }
        };
        match p.typ {
            b'E' => {
                let msg = String::from_utf8_lossy(&p.data).into_owned();
                self.fail(&format!("The receiver reported an error: {msg}"));
                return;
            }
            b'N' => {
                if !self.last_pkt.is_empty() && p.seq == self.seq % 64 {
                    self.retries += 1;
                    if self.retries > 10 {
                        self.fail("Too many errors");
                    } else {
                        out.extend_from_slice(&self.last_pkt.clone());
                    }
                }
                return;
            }
            b'Y' => {}
            _ => return,
        }
        if p.seq != self.seq % 64 {
            return; // ACK of an earlier packet (duplicate)
        }
        match self.state {
            State::SInit => {
                if p.data.len() > 0 {
                    let m = unchar(p.data[0]) as usize;
                    self.maxl = m.clamp(20, MAXL);
                }
                self.qbin = p.data.get(6).map(|&c| c == b'&' || c == b'Y').unwrap_or(false);
                self.seq = (self.seq + 1) % 64;
                let mut name = Vec::new();
                for &b in self.name.as_bytes() {
                    encode_byte(b, false, &mut name);
                }
                let pk = packet(self.seq, b'F', &name);
                self.emit(out, pk);
                self.state = State::SFile;
                self.status = "Sending...".into();
            }
            State::SFile | State::SData => {
                self.bytes_done = self.pos;
                self.seq = (self.seq + 1) % 64;
                if self.pos >= self.data.len() {
                    let pk = packet(self.seq, b'Z', &[]);
                    self.emit(out, pk);
                    self.state = State::SEof;
                } else {
                    let d = self.next_data();
                    let pk = packet(self.seq, b'D', &d);
                    self.emit(out, pk);
                    self.state = State::SData;
                }
            }
            State::SEof => {
                self.seq = (self.seq + 1) % 64;
                let pk = packet(self.seq, b'B', &[]);
                self.emit(out, pk);
                self.state = State::SBreak;
            }
            State::SBreak => {
                self.bytes_done = self.total;
                self.finish("Transfer complete");
            }
            _ => {}
        }
    }

    fn recv_packet(&mut self, r: Result<Pkt, u8>, out: &mut Vec<u8>) {
        let p = match r {
            Ok(p) => p,
            Err(seq) => {
                let pk = packet(seq, b'N', &[]);
                out.extend_from_slice(&pk);
                return;
            }
        };
        if p.typ == b'E' {
            let msg = String::from_utf8_lossy(&p.data).into_owned();
            self.fail(&format!("The sender reported an error: {msg}"));
            return;
        }
        // an S packet restarts the sequence
        if p.typ != b'S' && p.seq != self.seq % 64 {
            if p.seq == (self.seq + 63) % 64 && !self.last_pkt.is_empty() {
                out.extend_from_slice(&self.last_pkt.clone()); // duplicate: repeat the ACK
            } else {
                out.extend_from_slice(&packet(self.seq, b'N', &[]));
            }
            return;
        }
        match p.typ {
            b'S' => {
                self.seq = p.seq;
                let asks_8bit = p.data.get(6).map(|&c| c == b'&' || c == b'Y').unwrap_or(false);
                self.qbin = asks_8bit;
                if let Some(&m) = p.data.first() {
                    self.maxl = (unchar(m) as usize).clamp(20, MAXL);
                }
                let mut params = our_params();
                params[6] = if asks_8bit { b'&' } else { b'N' };
                let pk = packet(self.seq, b'Y', &params);
                self.emit(out, pk);
                self.seq = (self.seq + 1) % 64;
            }
            b'F' => {
                let raw = decode(&p.data, false);
                let name = String::from_utf8_lossy(&raw).into_owned();
                let base = name.rsplit(['/', '\\']).next().unwrap_or("").trim().to_string();
                self.name = if base.is_empty() || base == "." || base == ".." { "received.bin".into() } else { base };
                self.out_file.clear();
                self.bytes_done = 0;
                self.status = "Receiving...".into();
                let pk = packet(p.seq, b'Y', &[]);
                self.emit(out, pk);
                self.seq = (self.seq + 1) % 64;
            }
            b'D' => {
                let d = decode(&p.data, self.qbin);
                self.out_file.extend_from_slice(&d);
                self.bytes_done = self.out_file.len();
                self.total = self.total.max(self.bytes_done);
                let pk = packet(p.seq, b'Y', &[]);
                self.emit(out, pk);
                self.seq = (self.seq + 1) % 64;
            }
            b'Z' => {
                let discard = p.data.first() == Some(&b'D');
                if !discard {
                    if let Err(e) = std::fs::create_dir_all(&self.path).and_then(|_| std::fs::write(self.path.join(&self.name), &self.out_file)) {
                        out.extend_from_slice(&packet(p.seq, b'E', b"Could not write the file"));
                        self.fail(&format!("Could not write the file: {e}"));
                        return;
                    }
                }
                self.out_file.clear();
                let pk = packet(p.seq, b'Y', &[]);
                self.emit(out, pk);
                self.seq = (self.seq + 1) % 64;
            }
            b'B' => {
                out.extend_from_slice(&packet(p.seq, b'Y', &[]));
                self.finish("Transfer complete");
            }
            _ => out.extend_from_slice(&packet(p.seq, b'N', &[])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kermit_roundtrip_binary() {
        let dir = std::env::temp_dir().join("yetty_kermit_test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("source.bin");
        // large enough for the 8-bit packet counter to pass 255 (sequence numbers wrap at 64)
        let data: Vec<u8> = (0..60_000u32).map(|i| (i * 13 % 256) as u8).collect();
        std::fs::write(&src, &data).unwrap();
        let out_dir = dir.join("out");
        let mut tx = Kermit::new_send(&src).unwrap();
        let mut rx = Kermit::new_receive(out_dir.clone());
        let mut to_rx: Vec<u8> = tx.tick();
        let mut to_tx: Vec<u8> = Vec::new();
        for _ in 0..50_000 {
            if !to_rx.is_empty() {
                let b = std::mem::take(&mut to_rx);
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
        assert!(tx.done && !tx.failed, "sender: {}", tx.status);
        assert!(rx.done && !rx.failed, "receiver: {}", rx.status);
        assert_eq!(std::fs::read(out_dir.join("source.bin")).unwrap(), data);
    }

    #[test]
    fn damaged_packet_is_repeated() {
        let dir = std::env::temp_dir().join("yetty_kermit_test2");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("s.txt");
        let data = b"hello kermit\r\nsecond line # & ~ end".to_vec();
        std::fs::write(&src, &data).unwrap();
        let out_dir = dir.join("out");
        let mut tx = Kermit::new_send(&src).unwrap();
        let mut rx = Kermit::new_receive(out_dir.clone());
        let mut to_rx = tx.tick();
        let mut n = 0;
        let mut to_tx: Vec<u8> = Vec::new();
        for _ in 0..200 {
            if !to_rx.is_empty() {
                n += 1;
                let mut b = std::mem::take(&mut to_rx);
                if n == 4 {
                    let l = b.len();
                    b[l / 2] ^= 0x01; // flip a bit in one packet
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
        assert!(tx.done && !tx.failed, "sender: {}", tx.status);
        assert_eq!(std::fs::read(out_dir.join("s.txt")).unwrap(), data);
    }
}

#[cfg(test)]
mod gk {
    use super::*;

    #[test]
    fn answers_the_s_packet_of_g_kermit() {
        let pkt: Vec<u8> = vec![
            0x01, 0x39, 0x20, 0x53, 0x7e, 0x27, 0x20, 0x40, 0x2d, 0x23, 0x59, 0x33, 0x7e, 0x2a, 0x21, 0x4a, 0x2a, 0x30, 0x2b, 0x2b, 0x2b, 0x4a, 0x22, 0x55, 0x31, 0x41, 0x50, 0x0d,
        ];
        let mut rx = Kermit::new_receive(std::env::temp_dir().join("yetty_gk"));
        let out = rx.feed(&pkt);
        println!("reply: {:?}", String::from_utf8_lossy(&out));
        assert!(out.len() > 4 && out[3] == b'Y', "no ACK: {out:?}");
    }
}
