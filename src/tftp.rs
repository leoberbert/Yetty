//! A small TFTP server (RFC 1350): serves and receives files in one folder.

use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RRQ: u16 = 1;
const WRQ: u16 = 2;
const DATA: u16 = 3;
const ACK: u16 = 4;
const ERR: u16 = 5;
const BLOCK: usize = 512;

pub struct TftpServer {
    stop: Arc<AtomicBool>,
    pub log: Arc<Mutex<Vec<String>>>,
    pub port: u16,
    pub root: PathBuf,
}

impl Drop for TftpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn say(log: &Arc<Mutex<Vec<String>>>, text: String) {
    let mut l = log.lock().unwrap();
    l.push(text);
    if l.len() > 200 {
        l.remove(0);
    }
}

/// The file inside `root` for a requested name; names that try to leave the folder are refused.
fn safe_path(root: &Path, name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    let mut p = root.to_path_buf();
    for part in name.split('/').filter(|s| !s.is_empty()) {
        if part == ".." || part == "." || part.contains(':') {
            return None;
        }
        p.push(part);
    }
    if p == root {
        None
    } else {
        Some(p)
    }
}

fn error_packet(code: u16, msg: &str) -> Vec<u8> {
    let mut v = vec![0, ERR as u8, (code >> 8) as u8, code as u8];
    v.extend_from_slice(msg.as_bytes());
    v.push(0);
    v
}

impl TftpServer {
    /// Start listening on `addr` (for example "0.0.0.0:69"; port 0 picks a free port).
    pub fn start(addr: &str, root: PathBuf) -> Result<TftpServer, String> {
        std::fs::create_dir_all(&root).map_err(|e| format!("Could not use the folder: {e}"))?;
        let sock = UdpSocket::bind(addr).map_err(|e| format!("Could not listen on {addr}: {e}"))?;
        sock.set_read_timeout(Some(Duration::from_millis(200))).map_err(|e| e.to_string())?;
        let port = sock.local_addr().map(|a| a.port()).unwrap_or(0);
        let stop = Arc::new(AtomicBool::new(false));
        let log = Arc::new(Mutex::new(vec![format!("TFTP server listening on port {port}, folder {}", root.display())]));
        {
            let (stop, log, root) = (stop.clone(), log.clone(), root.clone());
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                while !stop.load(Ordering::Relaxed) {
                    let Ok((n, peer)) = sock.recv_from(&mut buf) else { continue };
                    if n < 4 {
                        continue;
                    }
                    let op = u16::from_be_bytes([buf[0], buf[1]]);
                    if op != RRQ && op != WRQ {
                        continue;
                    }
                    let mut parts = buf[2..n].split(|&b| b == 0);
                    let name = String::from_utf8_lossy(parts.next().unwrap_or(&[])).into_owned();
                    let (log, root, stop) = (log.clone(), root.clone(), stop.clone());
                    std::thread::spawn(move || {
                        if op == RRQ {
                            serve_read(peer, &name, &root, &log, &stop)
                        } else {
                            serve_write(peer, &name, &root, &log, &stop)
                        }
                    });
                }
                say(&log, "TFTP server stopped".into());
            });
        }
        Ok(TftpServer { stop, log, port, root })
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn transfer_socket(peer: SocketAddr) -> Option<UdpSocket> {
    let bind = if peer.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let s = UdpSocket::bind(bind).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    s.connect(peer).ok()?;
    Some(s)
}

fn serve_read(peer: SocketAddr, name: &str, root: &Path, log: &Arc<Mutex<Vec<String>>>, stop: &AtomicBool) {
    let Some(sock) = transfer_socket(peer) else { return };
    let Some(path) = safe_path(root, name) else {
        let _ = sock.send(&error_packet(2, "Access violation"));
        say(log, format!("Refused {name} from {}", peer.ip()));
        return;
    };
    let Ok(data) = std::fs::read(&path) else {
        let _ = sock.send(&error_packet(1, "File not found"));
        say(log, format!("Not found: {name} (asked by {})", peer.ip()));
        return;
    };
    say(log, format!("Sending {name} to {}", peer.ip()));
    let blocks = data.len() / BLOCK + 1; // a final short (maybe empty) block ends the transfer
    for n in 1..=blocks {
        let start = (n - 1) * BLOCK;
        let end = (start + BLOCK).min(data.len());
        let mut pkt = vec![0, DATA as u8, (n >> 8) as u8, n as u8];
        pkt.extend_from_slice(&data[start.min(data.len())..end]);
        let mut acked = false;
        for _ in 0..5 {
            if stop.load(Ordering::Relaxed) || sock.send(&pkt).is_err() {
                return;
            }
            let mut r = [0u8; 64];
            if let Ok(len) = sock.recv(&mut r) {
                if len >= 4 && u16::from_be_bytes([r[0], r[1]]) == ACK && u16::from_be_bytes([r[2], r[3]]) as usize == (n & 0xffff) {
                    acked = true;
                    break;
                }
            }
        }
        if !acked {
            say(log, format!("Transfer of {name} to {} timed out", peer.ip()));
            return;
        }
    }
    say(log, format!("Sent {name} ({} bytes) to {}", data.len(), peer.ip()));
}

fn serve_write(peer: SocketAddr, name: &str, root: &Path, log: &Arc<Mutex<Vec<String>>>, stop: &AtomicBool) {
    let Some(sock) = transfer_socket(peer) else { return };
    let Some(path) = safe_path(root, name) else {
        let _ = sock.send(&error_packet(2, "Access violation"));
        say(log, format!("Refused {name} from {}", peer.ip()));
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    say(log, format!("Receiving {name} from {}", peer.ip()));
    let mut data: Vec<u8> = Vec::new();
    let mut expected: u16 = 1;
    let mut last_ack = vec![0, ACK as u8, 0, 0];
    if sock.send(&last_ack).is_err() {
        return;
    }
    let mut tries = 0;
    let mut buf = [0u8; 1024];
    while !stop.load(Ordering::Relaxed) {
        match sock.recv(&mut buf) {
            Ok(n) if n >= 4 && u16::from_be_bytes([buf[0], buf[1]]) == DATA => {
                let blk = u16::from_be_bytes([buf[2], buf[3]]);
                if blk == expected {
                    data.extend_from_slice(&buf[4..n]);
                    last_ack = vec![0, ACK as u8, buf[2], buf[3]];
                    let _ = sock.send(&last_ack);
                    expected = expected.wrapping_add(1);
                    tries = 0;
                    if n - 4 < BLOCK {
                        match std::fs::write(&path, &data) {
                            Ok(()) => say(log, format!("Received {name} ({} bytes) from {}", data.len(), peer.ip())),
                            Err(e) => say(log, format!("Could not save {name}: {e}")),
                        }
                        return;
                    }
                } else {
                    let _ = sock.send(&last_ack);
                }
            }
            Ok(_) => {}
            Err(_) => {
                tries += 1;
                if tries > 5 {
                    say(log, format!("Transfer of {name} from {} timed out", peer.ip()));
                    return;
                }
                let _ = sock.send(&last_ack);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> UdpSocket {
        let c = UdpSocket::bind("127.0.0.1:0").unwrap();
        c.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        c
    }

    fn request(op: u16, name: &str) -> Vec<u8> {
        let mut v = vec![0, op as u8];
        v.extend_from_slice(name.as_bytes());
        v.push(0);
        v.extend_from_slice(b"octet\0");
        v
    }

    #[test]
    fn read_and_write_files() {
        let root = std::env::temp_dir().join("yetty_tftp_test");
        let _ = std::fs::create_dir_all(&root);
        let content: Vec<u8> = (0..1300u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.join("hello.bin"), &content).unwrap();
        let server = TftpServer::start("127.0.0.1:0", root.clone()).unwrap();
        let target: SocketAddr = format!("127.0.0.1:{}", server.port).parse().unwrap();

        // download
        let c = client();
        c.send_to(&request(1, "hello.bin"), target).unwrap();
        let mut got = Vec::new();
        loop {
            let mut buf = [0u8; 600];
            let (n, from) = c.recv_from(&mut buf).unwrap();
            assert_eq!(u16::from_be_bytes([buf[0], buf[1]]), DATA);
            got.extend_from_slice(&buf[4..n]);
            c.send_to(&[0, 4, buf[2], buf[3]], from).unwrap();
            if n - 4 < BLOCK {
                break;
            }
        }
        assert_eq!(got, content);

        // a name that leaves the folder is refused
        let c2 = client();
        c2.send_to(&request(1, "../secret.txt"), target).unwrap();
        let mut buf = [0u8; 600];
        c2.recv_from(&mut buf).unwrap();
        assert_eq!(u16::from_be_bytes([buf[0], buf[1]]), ERR);

        // upload
        let c3 = client();
        c3.send_to(&request(2, "up.bin"), target).unwrap();
        let (_, from) = c3.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..4], &[0, 4, 0, 0]);
        let first = vec![7u8; 512];
        let mut p = vec![0, 3, 0, 1];
        p.extend_from_slice(&first);
        c3.send_to(&p, from).unwrap();
        c3.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..4], &[0, 4, 0, 1]);
        let mut p = vec![0, 3, 0, 2];
        p.extend_from_slice(b"tail");
        c3.send_to(&p, from).unwrap();
        c3.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..4], &[0, 4, 0, 2]);
        std::thread::sleep(Duration::from_millis(200));
        let saved = std::fs::read(root.join("up.bin")).unwrap();
        assert_eq!(saved.len(), 516);
        server.stop();
    }
}
