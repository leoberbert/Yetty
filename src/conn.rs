//! Connections: SSH/SFTP/local shell through a pseudo console, Telnet/Telnet-TLS/RLogin/raw TCP, serial.

use crate::config::{FlowControl, ForwardKind, Parity, Protocol, SessionConfig};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub enum ConnEvent {
    Connected,
    Data(Vec<u8>),
    Closed(String),
}

enum Ctl {
    None,
    Pty {
        master: Box<dyn MasterPty + Send>,
        child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    },
    Tcp(TcpStream),
    Serial(Box<dyn serialport::SerialPort>),
}

struct Shared {
    writer: Option<Box<dyn Write + Send>>,
    ctl: Ctl,
    closed: bool,
    telnet: bool,
    naws: bool,
    size: (u16, u16),
}

pub struct Connection {
    shared: Arc<Mutex<Shared>>,
    pub rx: Receiver<ConnEvent>,
    protocol: Protocol,
}

/// The terminal type name sent to the host for the chosen emulation.
pub fn term_name(emulation: &str) -> &'static str {
    // the names of the original's list (ANSI, Linux, SCOANSI, TN3270, VShell, TVI910, TVI925, VT100, VT102, VT220,
    // VT320, WYSE50, WYSE60, Xterm, Dumb) and the spellings earlier versions saved ("SCO ANSI", "Wyse50")
    match emulation.to_ascii_uppercase().replace(' ', "").as_str() {
        "VT100" => "vt100",
        "VT102" => "vt102",
        "VT220" => "vt220",
        "VT320" => "vt320",
        "ANSI" => "ansi",
        "LINUX" => "linux",
        "SCOANSI" => "ansi",
        "WYSE50" => "wyse50",
        "WYSE60" => "wyse60",
        "TVI910" => "tvi910",
        "TVI925" => "tvi925",
        "DUMB" => "dumb",
        _ => "xterm-256color",
    }
}

pub fn find_program(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    let sys = std::path::PathBuf::from(
        std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()),
    )
    .join("System32")
    .join("OpenSSH")
    .join(name);
    if sys.is_file() {
        return Some(sys.to_string_lossy().into_owned());
    }
    None
}

impl Connection {
    pub fn open(cfg: &SessionConfig, rows: u16, cols: u16, ctx: egui::Context) -> Connection {
        let (tx, rx) = channel();
        let shared = Arc::new(Mutex::new(Shared {
            writer: None,
            ctl: Ctl::None,
            closed: false,
            telnet: cfg.protocol.is_telnet(),
            naws: false,
            size: (rows, cols),
        }));
        let conn = Connection {
            shared: shared.clone(),
            rx,
            protocol: cfg.protocol,
        };
        let cfg = cfg.clone();
        std::thread::spawn(move || {
            let res = match cfg.protocol {
                Protocol::Ssh | Protocol::Sftp | Protocol::LocalShell => {
                    open_pty(&cfg, rows, cols, &shared, &tx, &ctx)
                }
                Protocol::Telnet | Protocol::TelnetTls | Protocol::Rlogin | Protocol::Raw => {
                    open_tcp(&cfg, &shared, &tx, &ctx)
                }
                Protocol::Serial => open_serial(&cfg, &shared, &tx, &ctx),
                Protocol::Rdp => Err("Remote Desktop opens in its own window.".to_string()),
            };
            if let Err(e) = res {
                let _ = tx.send(ConnEvent::Closed(e));
                ctx.request_repaint();
            }
        });
        conn
    }

    /// Send what the person typed.
    pub fn send(&self, data: &[u8]) {
        let mut sh = self.shared.lock().unwrap();
        if sh.closed {
            return;
        }
        let telnet = sh.telnet;
        if let Some(w) = sh.writer.as_mut() {
            if telnet {
                let mut out = Vec::with_capacity(data.len() + 4);
                for &b in data {
                    match b {
                        0xff => out.extend_from_slice(&[0xff, 0xff]),
                        b'\r' => out.extend_from_slice(b"\r\0"),
                        _ => out.push(b),
                    }
                }
                let _ = w.write_all(&out);
            } else {
                let _ = w.write_all(data);
            }
            let _ = w.flush();
        }
    }

    /// Send bytes as they are (no Telnet escaping): protocol commands.
    pub fn send_raw(&self, data: &[u8]) {
        let mut sh = self.shared.lock().unwrap();
        if sh.closed {
            return;
        }
        if let Some(w) = sh.writer.as_mut() {
            let _ = w.write_all(data);
            let _ = w.flush();
        }
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let mut sh = self.shared.lock().unwrap();
        if sh.size == (rows, cols) {
            return;
        }
        sh.size = (rows, cols);
        match &sh.ctl {
            Ctl::Pty { master, .. } => {
                let _ = master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
            Ctl::Tcp(_) if sh.telnet && sh.naws => {
                let msg = naws_message(rows, cols);
                if let Some(w) = sh.writer.as_mut() {
                    let _ = w.write_all(&msg);
                }
            }
            _ => {}
        }
    }

    pub fn send_break(&self) {
        let mut sh = self.shared.lock().unwrap();
        let telnet = sh.telnet;
        let sh = &mut *sh;
        match &mut sh.ctl {
            Ctl::Serial(port) => {
                let _ = port.set_break();
                std::thread::sleep(Duration::from_millis(250));
                let _ = port.clear_break();
            }
            Ctl::Tcp(_) if telnet => {
                if let Some(w) = sh.writer.as_mut() {
                    let _ = w.write_all(&[255, 243]);
                }
            }
            Ctl::Pty { .. } if self.protocol == Protocol::Ssh => {
                if let Some(w) = sh.writer.as_mut() {
                    let _ = w.write_all(b"\r~B");
                }
            }
            _ => {}
        }
    }

    pub fn close(&self) {
        let mut sh = self.shared.lock().unwrap();
        if sh.closed {
            return;
        }
        sh.closed = true;
        sh.writer = None;
        match std::mem::replace(&mut sh.ctl, Ctl::None) {
            Ctl::Pty { child, master } => {
                let _ = child.lock().unwrap().kill();
                drop(master);
            }
            Ctl::Tcp(s) => {
                let _ = s.shutdown(Shutdown::Both);
            }
            Ctl::Serial(p) => drop(p),
            Ctl::None => {}
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}

fn naws_message(rows: u16, cols: u16) -> Vec<u8> {
    let mut m = vec![255, 250, 31];
    for v in [cols, rows] {
        for b in v.to_be_bytes() {
            m.push(b);
            if b == 255 {
                m.push(255);
            }
        }
    }
    m.extend_from_slice(&[255, 240]);
    m
}

fn reader_loop(
    mut reader: Box<dyn Read + Send>,
    shared: Arc<Mutex<Shared>>,
    tx: Sender<ConnEvent>,
    ctx: egui::Context,
    mut filter: Option<Telnet>,
) {
    let mut buf = vec![0u8; 16384];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let data = match filter.as_mut() {
                    Some(t) => t.filter(&buf[..n], &shared),
                    None => buf[..n].to_vec(),
                };
                if !data.is_empty() {
                    if tx.send(ConnEvent::Data(data)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::Interrupted =>
            {
                if shared.lock().unwrap().closed {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let was_closed = shared.lock().unwrap().closed;
    let _ = tx.send(ConnEvent::Closed(if was_closed {
        "Disconnected".into()
    } else {
        "The connection was closed by the remote host".into()
    }));
    ctx.request_repaint();
}

fn open_pty(
    cfg: &SessionConfig,
    rows: u16,
    cols: u16,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ConnEvent>,
    ctx: &egui::Context,
) -> Result<(), String> {
    let mut cmd = match cfg.protocol {
        Protocol::Ssh | Protocol::Sftp => {
            let exe = if cfg.protocol == Protocol::Ssh {
                "ssh.exe"
            } else {
                "sftp.exe"
            };
            // The Windows OpenSSH sftp prints permissions as "-rw-******" (it cannot format the group and other
            // bits); the sftp that comes with Git for Windows prints them in full, as SecureCRT does, so it is
            // preferred for SFTP sessions when it is installed.
            let git_sftp = if cfg.protocol == Protocol::Sftp {
                ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
                    .iter()
                    .filter_map(|v| std::env::var(v).ok())
                    .flat_map(|base| {
                        let b = std::path::PathBuf::from(base);
                        [
                            b.join("Git").join("usr").join("bin").join("sftp.exe"),
                            b.join("Programs")
                                .join("Git")
                                .join("usr")
                                .join("bin")
                                .join("sftp.exe"),
                        ]
                    })
                    .find(|p| p.is_file())
                    .map(|p| p.to_string_lossy().into_owned())
            } else {
                None
            };
            let path = git_sftp.or_else(|| find_program(exe)).ok_or_else(|| {
                format!("The OpenSSH client ({exe}) was not found. Turn it on in Settings > Apps > Optional features.")
            })?;
            if cfg.host.trim().is_empty() {
                return Err("Enter the hostname.".into());
            }
            let mut c = CommandBuilder::new(path);
            let ssh = cfg.protocol == Protocol::Ssh;
            c.arg(if ssh { "-p" } else { "-P" });
            c.arg(cfg.port.to_string());
            let alive = if cfg.anti_idle_noop_on {
                cfg.anti_idle_noop_secs
            } else {
                cfg.keepalive_secs
            };
            if alive > 0 {
                c.arg("-o");
                c.arg(format!("ServerAliveInterval={alive}"));
            }
            if cfg.compression {
                c.arg("-C");
            }
            if cfg.ssh_verbose || (ssh && !cfg.identity_file.trim().is_empty()) {
                c.arg("-v");
            }
            if ssh && cfg.remote_command_on && !cfg.remote_command.trim().is_empty() {
                c.arg("-t"); // a terminal for commands such as passwd
            }
            if !cfg.identity_file.trim().is_empty() {
                c.arg("-i");
                c.arg(cfg.identity_file.trim());
            }
            if cfg.identities_only {
                c.arg("-o");
                c.arg("IdentitiesOnly=yes");
            }
            if !cfg.pubkey_agent {
                c.arg("-o");
                c.arg("IdentityAgent=none");
            }
            if cfg.auth.iter().any(|a| a.name == "GSSAPI" && a.enabled) {
                c.arg("-o");
                c.arg("GSSAPIAuthentication=yes");
                if cfg.gssapi_delegation == "Full" {
                    c.arg("-o");
                    c.arg("GSSAPIDelegateCredentials=yes");
                }
                if !cfg.gssapi_spn.trim().is_empty() {
                    c.arg("-o");
                    c.arg(format!("GSSAPIServerIdentity={}", cfg.gssapi_spn.trim()));
                }
            }
            for (opt, list, def) in [
                ("KexAlgorithms", &cfg.kex, crate::config::KEX_DEFAULT),
                ("Ciphers", &cfg.ciphers, crate::config::CIPHERS_DEFAULT),
                ("MACs", &cfg.macs, crate::config::MACS_DEFAULT),
                (
                    "HostKeyAlgorithms",
                    &cfg.hostkey_algos,
                    crate::config::HOSTKEY_DEFAULT,
                ),
            ] {
                if let Some(v) = crate::config::alg_option(list, def) {
                    c.arg("-o");
                    c.arg(format!("{opt}={v}"));
                }
            }
            if !cfg.known_hosts_file.trim().is_empty() {
                c.arg("-o");
                c.arg(format!(
                    "UserKnownHostsFile={}",
                    cfg.known_hosts_file.trim()
                ));
            }
            match cfg.strict_host_key.as_str() {
                "accept-new" => {
                    c.arg("-o");
                    c.arg("StrictHostKeyChecking=accept-new");
                }
                "no" => {
                    c.arg("-o");
                    c.arg("StrictHostKeyChecking=no");
                }
                _ => {}
            }
            // Authentication methods, in the order of the list (unchecked ones are left out).
            let methods: Vec<&str> = cfg
                .auth
                .iter()
                .filter(|a| a.enabled)
                .filter_map(|a| match a.name.as_str() {
                    "PublicKey" => Some("publickey"),
                    "Keyboard Interactive" => Some("keyboard-interactive"),
                    "Password" => Some("password"),
                    "GSSAPI" => Some("gssapi-with-mic"),
                    _ => None,
                })
                .collect();
            let default_order = [
                "publickey",
                "keyboard-interactive",
                "password",
                "gssapi-with-mic",
            ];
            if !methods.is_empty() && methods != default_order {
                c.arg("-o");
                c.arg(format!("PreferredAuthentications={}", methods.join(",")));
            }
            if ssh {
                if cfg.agent_forward {
                    c.arg("-A");
                }
                if cfg.x11 {
                    c.arg(if cfg.x11_trusted { "-Y" } else { "-X" });
                    if !cfg.x11_display.trim().is_empty() {
                        c.env("DISPLAY", cfg.x11_display.trim());
                    }
                }
                for f in &cfg.forwards {
                    match f.kind {
                        ForwardKind::Local => {
                            c.arg("-L");
                            c.arg(format!("{}:{}:{}", f.port, f.host, f.dest_port));
                        }
                        ForwardKind::Remote => {
                            c.arg("-R");
                            c.arg(format!("{}:{}:{}", f.port, f.host, f.dest_port));
                        }
                        ForwardKind::Dynamic => {
                            c.arg("-D");
                            c.arg(f.port.to_string());
                        }
                    }
                }
            }
            let mut target = if cfg.username.trim().is_empty() {
                cfg.host.trim().to_string()
            } else {
                format!("{}@{}", cfg.username.trim(), cfg.host.trim())
            };
            if !ssh && !cfg.sftp_start_dir.trim().is_empty() {
                target = format!("{}:{}", target, cfg.sftp_start_dir.trim());
            }
            c.arg(target);
            if !ssh
                && !cfg.sftp_local_dir.trim().is_empty()
                && std::path::Path::new(cfg.sftp_local_dir.trim()).is_dir()
            {
                c.cwd(cfg.sftp_local_dir.trim());
            }
            if ssh && cfg.remote_command_on && !cfg.remote_command.trim().is_empty() {
                c.arg(cfg.remote_command.trim());
            }
            c
        }
        _ => {
            let shell = if cfg.shell_command.trim().is_empty() {
                std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
            } else {
                cfg.shell_command.trim().to_string()
            };
            let mut parts = shell.split_whitespace();
            let mut c = CommandBuilder::new(parts.next().unwrap_or("cmd.exe"));
            for p in parts {
                c.arg(p);
            }
            if let Some(home) = std::env::var_os("USERPROFILE") {
                c.cwd(home);
            }
            c
        }
    };
    cmd.env(
        "TERM",
        if cfg.emulation == "Xterm" && !cfg.xterm256 {
            "xterm"
        } else {
            term_name(&cfg.emulation)
        },
    );
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Could not open the terminal: {e}"))?;
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Could not start the program: {e}"))?;
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let child = Arc::new(Mutex::new(child));
    {
        let mut sh = shared.lock().unwrap();
        if sh.closed {
            let _ = child.lock().unwrap().kill();
            return Ok(());
        }
        sh.writer = Some(writer);
        sh.ctl = Ctl::Pty {
            master: pair.master,
            child: child.clone(),
        };
        let (r, c) = sh.size;
        if (r, c) != (rows, cols) {
            if let Ctl::Pty { master, .. } = &sh.ctl {
                let _ = master.resize(PtySize {
                    rows: r,
                    cols: c,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        }
    }
    let _ = tx.send(ConnEvent::Connected);
    ctx.request_repaint();
    // Watch for the program ending (the pseudo console does not always end the reader).
    {
        let shared = shared.clone();
        let tx = tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(300));
            if shared.lock().unwrap().closed {
                return;
            }
            let done = child.lock().unwrap().try_wait().ok().flatten();
            if let Some(status) = done {
                std::thread::sleep(Duration::from_millis(200));
                let mut sh = shared.lock().unwrap();
                if !sh.closed {
                    sh.closed = true;
                    sh.writer = None;
                    sh.ctl = Ctl::None;
                    drop(sh);
                    let _ = tx.send(ConnEvent::Closed(format!(
                        "The program ended (code {})",
                        status.exit_code()
                    )));
                    ctx.request_repaint();
                }
                return;
            }
        });
    }
    reader_loop(reader, shared.clone(), tx.clone(), ctx.clone(), None);
    Ok(())
}

fn tcp_connect(cfg: &SessionConfig) -> Result<TcpStream, String> {
    let host = cfg.host.trim();
    if host.is_empty() {
        return Err("Enter the hostname.".into());
    }
    let addrs: Vec<_> = (host, cfg.port)
        .to_socket_addrs()
        .map_err(|e| format!("Could not find {host}: {e}"))?
        .collect();
    let mut last = String::from("no address");
    for a in addrs {
        match TcpStream::connect_timeout(&a, Duration::from_secs(10)) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                return Ok(s);
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("Failed to connect to {host}:{}: {last}", cfg.port))
}

fn telnet_filter(cfg: &SessionConfig) -> Option<Telnet> {
    cfg.protocol.is_telnet().then(|| Telnet {
        ttype: term_name(&cfg.emulation).to_uppercase(),
        ..Default::default()
    })
}

fn open_tcp(
    cfg: &SessionConfig,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ConnEvent>,
    ctx: &egui::Context,
) -> Result<(), String> {
    let stream = tcp_connect(cfg)?;
    if cfg.protocol == Protocol::TelnetTls {
        return open_tls(cfg, stream, shared, tx, ctx);
    }
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let reader = stream.try_clone().map_err(|e| e.to_string())?;
    if cfg.protocol == Protocol::Rlogin {
        let local = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
        let remote = if cfg.username.is_empty() {
            local.clone()
        } else {
            cfg.username.clone()
        };
        let hello = format!("\0{local}\0{remote}\0{}/38400\0", term_name(&cfg.emulation));
        let _ = writer.write_all(hello.as_bytes());
    }
    {
        let mut sh = shared.lock().unwrap();
        if sh.closed {
            let _ = stream.shutdown(Shutdown::Both);
            return Ok(());
        }
        sh.writer = Some(Box::new(writer));
        sh.ctl = Ctl::Tcp(stream);
    }
    let _ = tx.send(ConnEvent::Connected);
    ctx.request_repaint();
    reader_loop(
        Box::new(reader),
        shared.clone(),
        tx.clone(),
        ctx.clone(),
        telnet_filter(cfg),
    );
    Ok(())
}

type Tls = Arc<Mutex<native_tls::TlsStream<TcpStream>>>;

/// Telnet/TLS: TLS handshake (SChannel: certificate checked against the Windows store and the hostname),
/// then Telnet over the decrypted stream. The TLS stream is shared by the reader thread and the writer; its
/// socket is non-blocking so neither holds the lock while waiting.
fn open_tls(
    cfg: &SessionConfig,
    stream: TcpStream,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ConnEvent>,
    ctx: &egui::Context,
) -> Result<(), String> {
    let host = cfg.host.trim();
    let failed = |why: String| format!("TLS negotiation with {host}:{} failed: {why}", cfg.port);
    // the socket kept for shutdown (close) and the Telnet NAWS/BREAK checks
    let ctl = stream.try_clone().map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(20)));
    let connector = native_tls::TlsConnector::new().map_err(|e| failed(e.to_string()))?;
    let tls = connector.connect(host, stream).map_err(|e| {
        failed(match e {
            native_tls::HandshakeError::Failure(e) => e.to_string(),
            native_tls::HandshakeError::WouldBlock(_) => "the handshake did not complete".into(),
        })
    })?;
    let sock = tls.get_ref();
    let _ = sock.set_read_timeout(None);
    let _ = sock.set_write_timeout(None);
    sock.set_nonblocking(true)
        .map_err(|e| failed(e.to_string()))?;
    let tls: Tls = Arc::new(Mutex::new(tls));
    {
        let mut sh = shared.lock().unwrap();
        if sh.closed {
            let _ = ctl.shutdown(Shutdown::Both);
            return Ok(());
        }
        sh.writer = Some(Box::new(TlsWriter(tls.clone())));
        sh.ctl = Ctl::Tcp(ctl);
    }
    let _ = tx.send(ConnEvent::Connected);
    ctx.request_repaint();
    let reader = TlsReader {
        tls,
        shared: shared.clone(),
    };
    reader_loop(
        Box::new(reader),
        shared.clone(),
        tx.clone(),
        ctx.clone(),
        telnet_filter(cfg),
    );
    Ok(())
}

/// Retries an operation on the non-blocking TLS socket until it does not report WouldBlock. The lock is
/// taken inside `op` and released between attempts, so the reader thread keeps running.
fn retry_tls<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    loop {
        match op() {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2))
            }
            r => return r,
        }
    }
}

struct TlsWriter(Tls);

impl Write for TlsWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // schannel keeps a record it could not send and finishes it when write is called again with the same data
        retry_tls(|| self.0.lock().unwrap().write(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        retry_tls(|| self.0.lock().unwrap().flush())
    }
}

/// Reads decrypted data; waits (without holding the lock) while nothing has arrived, so `reader_loop`
/// never sees WouldBlock.
struct TlsReader {
    tls: Tls,
    shared: Arc<Mutex<Shared>>,
}

impl Read for TlsReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let r = self.tls.lock().unwrap().read(buf);
            match r {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if self.shared.lock().unwrap().closed {
                        return Ok(0);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                r => return r,
            }
        }
    }
}

fn open_serial(
    cfg: &SessionConfig,
    shared: &Arc<Mutex<Shared>>,
    tx: &Sender<ConnEvent>,
    ctx: &egui::Context,
) -> Result<(), String> {
    use serialport::{DataBits, StopBits};
    let data_bits = match cfg.data_bits {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        _ => DataBits::Eight,
    };
    let parity = match cfg.parity {
        Parity::None => serialport::Parity::None,
        Parity::Odd => serialport::Parity::Odd,
        Parity::Even => serialport::Parity::Even,
    };
    let stop = if cfg.stop_bits == 2 {
        StopBits::Two
    } else {
        StopBits::One
    };
    let flow = match cfg.flow {
        FlowControl::None => serialport::FlowControl::None,
        FlowControl::XonXoff => serialport::FlowControl::Software,
        FlowControl::RtsCts => serialport::FlowControl::Hardware,
    };
    let port = serialport::new(cfg.com_port.trim(), cfg.baud)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop)
        .flow_control(flow)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| format!("Could not open {}: {e}", cfg.com_port))?;
    let reader = port.try_clone().map_err(|e| e.to_string())?;
    let writer = port.try_clone().map_err(|e| e.to_string())?;
    {
        let mut sh = shared.lock().unwrap();
        if sh.closed {
            return Ok(());
        }
        sh.writer = Some(Box::new(writer));
        sh.ctl = Ctl::Serial(port);
    }
    let _ = tx.send(ConnEvent::Connected);
    ctx.request_repaint();
    reader_loop(
        Box::new(reader),
        shared.clone(),
        tx.clone(),
        ctx.clone(),
        None,
    );
    Ok(())
}

pub fn serial_ports() -> Vec<String> {
    serialport::available_ports()
        .map(|v| v.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Telnet option negotiation (RFC 854/855, TTYPE 1091, NAWS 1073)

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;
const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3;
const OPT_TTYPE: u8 = 24;
const OPT_NAWS: u8 = 31;

struct Telnet {
    state: u8, // 0 data, 1 iac, 2 will, 3 wont, 4 do, 5 dont, 6 sb, 7 sb-iac
    sb: Vec<u8>,
    we_will: [bool; 256],
    they_will: [bool; 256],
    ttype: String,
}

impl Default for Telnet {
    fn default() -> Self {
        Telnet {
            state: 0,
            sb: Vec::new(),
            we_will: [false; 256],
            they_will: [false; 256],
            ttype: "XTERM".into(),
        }
    }
}

impl Telnet {
    fn filter(&mut self, input: &[u8], shared: &Arc<Mutex<Shared>>) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len());
        let mut reply: Vec<u8> = Vec::new();
        for &b in input {
            match self.state {
                0 => {
                    if b == IAC {
                        self.state = 1
                    } else {
                        out.push(b)
                    }
                }
                1 => {
                    self.state = match b {
                        IAC => {
                            out.push(IAC);
                            0
                        }
                        WILL => 2,
                        WONT => 3,
                        DO => 4,
                        DONT => 5,
                        SB => {
                            self.sb.clear();
                            6
                        }
                        _ => 0,
                    }
                }
                2 => {
                    // server WILL x
                    let ok = matches!(b, OPT_ECHO | OPT_SGA);
                    if self.they_will[b as usize] != ok || !ok {
                        reply.extend_from_slice(&[IAC, if ok { DO } else { DONT }, b]);
                    }
                    self.they_will[b as usize] = ok;
                    self.state = 0;
                }
                3 => {
                    if self.they_will[b as usize] {
                        reply.extend_from_slice(&[IAC, DONT, b]);
                    }
                    self.they_will[b as usize] = false;
                    self.state = 0;
                }
                4 => {
                    // server DO x
                    let ok = matches!(b, OPT_TTYPE | OPT_NAWS | OPT_SGA);
                    if !self.we_will[b as usize] || !ok {
                        reply.extend_from_slice(&[IAC, if ok { WILL } else { WONT }, b]);
                    }
                    self.we_will[b as usize] = ok;
                    if b == OPT_NAWS {
                        let mut sh = shared.lock().unwrap();
                        sh.naws = true;
                        let (r, c) = sh.size;
                        drop(sh);
                        reply.extend_from_slice(&naws_message(r, c));
                    }
                    self.state = 0;
                }
                5 => {
                    if self.we_will[b as usize] {
                        reply.extend_from_slice(&[IAC, WONT, b]);
                    }
                    self.we_will[b as usize] = false;
                    if b == OPT_NAWS {
                        shared.lock().unwrap().naws = false;
                    }
                    self.state = 0;
                }
                6 => {
                    if b == IAC {
                        self.state = 7
                    } else {
                        self.sb.push(b)
                    }
                }
                _ => {
                    if b == SE {
                        if self.sb.first() == Some(&OPT_TTYPE) && self.sb.get(1) == Some(&1) {
                            reply.extend_from_slice(&[IAC, SB, OPT_TTYPE, 0]);
                            reply.extend_from_slice(self.ttype.as_bytes());
                            reply.extend_from_slice(&[IAC, SE]);
                        }
                        self.state = 0;
                    } else {
                        self.sb.push(b);
                        self.state = 6;
                    }
                }
            }
        }
        if !reply.is_empty() {
            let mut sh = shared.lock().unwrap();
            if let Some(w) = sh.writer.as_mut() {
                let _ = w.write_all(&reply);
                let _ = w.flush();
            }
        }
        out
    }
}
