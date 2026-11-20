//! One terminal tab: the screen emulator, its connection, its log, and how it is drawn.

use crate::config::{self, CursorStyle, Rgb, SessionConfig};
use crate::conn::{ConnEvent, Connection};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct TermCb {
    pub answerback: String,
    pub replies: Vec<u8>,
    pub title: Option<String>,
    pub bell: bool,
}

impl vt100::Callbacks for TermCb {
    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.bell = true;
    }
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(String::from_utf8_lossy(title).into_owned());
    }
    fn unhandled_control(&mut self, _: &mut vt100::Screen, b: u8) {
        if b == 0x05 && !self.answerback.is_empty() {
            self.replies.extend_from_slice(self.answerback.as_bytes());
        }
    }
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let first = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        match (i1, c) {
            (None, 'n') if first == 6 => {
                let (r, col) = screen.cursor_position();
                self.replies.extend_from_slice(format!("\x1b[{};{}R", r + 1, col + 1).as_bytes());
            }
            (None, 'n') if first == 5 => self.replies.extend_from_slice(b"\x1b[0n"),
            (None, 'c') => self.replies.extend_from_slice(b"\x1b[?62;22c"),
            (Some(b'>'), 'c') => self.replies.extend_from_slice(b"\x1b[>0;276;0c"),
            _ => {}
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Status {
    Connecting,
    Connected,
    Disconnected(String),
}

/// A cell position in "virtual lines": 0 is the top of the live screen, negatives are scrollback.
pub type VPos = (i64, u16);

/// What the ssh program is asking.
#[derive(Clone, Debug)]
pub enum SshPrompt {
    /// "Are you sure you want to continue connecting (yes/no/[fingerprint])?" with the text before it
    HostKey { detail: String },
    /// "user@host's password:"
    Password { label: String },
}

pub struct Session {
    pub id: u64,
    pub cfg: SessionConfig,
    pub title: String,
    pub parser: vt100::Parser<TermCb>,
    pub conn: Option<Connection>,
    pub status: Status,
    pub rows: u16,
    pub cols: u16,
    pub scroll_offset: usize,
    scroll_accum: f32,
    pub selection: Option<(VPos, VPos)>,
    pub find_hit: Option<(i64, u16, usize)>,
    pub log_path: Option<String>,
    log_file: Option<std::fs::File>,
    log_synced: Option<Instant>,
    pub log_paused: bool,
    log_strip: u8,
    pub connected_at: Option<Instant>,
    pub bytes_in: u64,
    pub bell_flash: Option<Instant>,
    pub recent: String,
    /// The password typed at a "password:" prompt, kept in memory only so a cloned tab need not ask again.
    pub typed_password: Option<String>,
    pw_waiting: bool,
    /// A question of the ssh program that the person answers in a window (host key, password).
    pub prompt_req: Option<SshPrompt>,
    hk_block: bool,
    /// Lines kept off the screen (ssh questions answered in windows), removed again from later redraws.
    hidden_lines: Vec<String>,
    prompt_done: Option<Instant>,
    sftp_pending: String,
    sftp_entries: Vec<crate::sftplist::Entry>,
    sftp_last: Option<Instant>,
    pw_buf: String,
    pub hex: std::collections::VecDeque<u8>,
    logon_idx: usize,
    logon_buf: String,
    password_sent: Option<Instant>,
    pub logon_script_due: bool,
    pub locked: Option<String>,
    /// The tab was renamed by the person (Rename): the title the host sets no longer replaces it,
    /// and Reset Name becomes available.
    pub renamed: bool,
    pub paste_quote: bool,
    /// A paste asked for by the mouse or a menu, not by the keyboard: it is pasted even when the
    /// Windows paste hotkey belongs to the remote.
    pub paste_requested: bool,
    /// Where the mouse pointer was when a key press hid it ("Hide mouse pointer on keypress").
    pointer_hidden_at: Option<Pos2>,
    /// The line selected by a triple click whose copy includes the newline.
    line_sel: Option<(VPos, VPos)>,
    /// Clipboard text waiting for the "Confirm Multi-line Paste" answer.
    pub pending_paste: Option<String>,
    paste_queue: std::collections::VecDeque<(String, u64)>,
    paste_next: Option<Instant>,
    pub recording: Option<Vec<String>>,
    rec_line: String,
    pub capture_file: Option<std::fs::File>,
    pub xfer: Option<crate::transfer::Transfer>,
    pub global_identity: String,
    pub log_raw: bool,
    pub log_prompt_due: bool,
    pub close_due: bool,
    reconnect_at: Option<Instant>,
    /// seconds before an automatic reconnect (Global Options > Terminal "Reconnect interval")
    pub reconnect_secs: u64,
    last_sent: Instant,
    last_noop: Instant,
    log_day: u32,
    log_line_start: bool,
    ctx: egui::Context,
}

impl Session {
    pub fn new_with_key(id: u64, cfg: SessionConfig, ctx: &egui::Context, global_identity: String) -> Session {
        Session::build(id, cfg, ctx, global_identity)
    }

    fn build(id: u64, cfg: SessionConfig, ctx: &egui::Context, global_identity: String) -> Session {
        let rows = cfg.rows.max(2);
        let cols = cfg.cols.max(10);
        let parser = vt100::Parser::new_with_callbacks(rows, cols, cfg.scrollback, TermCb::default());
        let mut s = Session {
            id,
            title: cfg.display_name(),
            cfg,
            parser,
            conn: None,
            status: Status::Disconnected(String::new()),
            rows,
            cols,
            scroll_offset: 0,
            scroll_accum: 0.0,
            selection: None,
            find_hit: None,
            log_path: None,
            log_file: None,
            log_synced: None,
            log_paused: false,
            log_strip: 0,
            connected_at: None,
            bytes_in: 0,
            bell_flash: None,
            recent: String::new(),
            typed_password: None,
            pw_waiting: false,
            prompt_req: None,
            hk_block: false,
            hidden_lines: Vec::new(),
            prompt_done: None,
            sftp_pending: String::new(),
            sftp_entries: Vec::new(),
            sftp_last: None,
            pw_buf: String::new(),
            hex: std::collections::VecDeque::new(),
            logon_idx: 0,
            logon_buf: String::new(),
            password_sent: None,
            logon_script_due: false,
            locked: None,
            renamed: false,
            paste_quote: false,
            paste_requested: false,
            pointer_hidden_at: None,
            line_sel: None,
            pending_paste: None,
            paste_queue: std::collections::VecDeque::new(),
            paste_next: None,
            recording: None,
            rec_line: String::new(),
            capture_file: None,
            xfer: None,
            global_identity,
            log_raw: false,
            log_prompt_due: false,
            close_due: false,
            reconnect_at: None,
            reconnect_secs: 3,
            last_sent: Instant::now(),
            last_noop: Instant::now(),
            log_day: 0,
            log_line_start: true,
            ctx: ctx.clone(),
        };
        s.connect();
        s
    }

    /// Feed what the connection delivered into the logon actions and the "wait for" buffer.
    fn note_text(&mut self, d: &[u8]) {
        let t = String::from_utf8_lossy(d).into_owned();
        let clean: String = t.chars().filter(|c| !c.is_control() || *c == '\n').collect();
        self.recent.push_str(&clean);
        if self.recent.len() > 8000 {
            let mut cut = self.recent.len() - 4000;
            while !self.recent.is_char_boundary(cut) {
                cut += 1;
            }
            self.recent.drain(..cut);
        }
        // Questions of the ssh program are asked in a window, as in the original.
        if self.prompt_req.is_none()
            && matches!(self.cfg.protocol, config::Protocol::Ssh | config::Protocol::Sftp)
            && self.prompt_done.map(|t| t.elapsed() > Duration::from_millis(600)).unwrap_or(true)
        {
            let tail: String = clean.trim_end().chars().rev().take(120).collect::<String>().chars().rev().collect();
            let lc = tail.to_lowercase();
            if lc.ends_with("(yes/no/[fingerprint])?") || lc.ends_with("(yes/no)?") {
                let start = self.recent.len().saturating_sub(700);
                let mut s = start;
                while !self.recent.is_char_boundary(s) {
                    s += 1;
                }
                self.prompt_req = Some(SshPrompt::HostKey { detail: self.recent[s..].trim().to_string() });
            } else if self.cfg.saved_password.is_empty()
                && lc.ends_with("password:")
                && !lc.contains("[sudo]")
                && self.connected_at.map(|t| t.elapsed() < Duration::from_secs(90)).unwrap_or(true)
            {
                let label = tail.rsplit('\n').next().unwrap_or("").trim().to_string();
                self.prompt_req = Some(SshPrompt::Password { label });
            }
        }
        // Without a saved password, remember what the person types at the next "password:" prompt.
        if self.cfg.saved_password.is_empty() && self.typed_password.is_none() {
            let t = clean.trim_end().to_lowercase();
            if t.ends_with("password:") {
                self.pw_waiting = true;
                self.pw_buf.clear();
            }
        }
        // Saved password: answer the "password:" prompt (at most once every few seconds).
        if !self.cfg.saved_password.is_empty() && self.cfg.protocol != config::Protocol::Raw {
            let tail: String = clean.trim_end().chars().rev().take(40).collect::<String>().chars().rev().collect();
            let t = tail.to_lowercase();
            let due = self.password_sent.map(|t| t.elapsed().as_secs() > 4).unwrap_or(true);
            if due && t.ends_with("password:") {
                self.password_sent = Some(Instant::now());
                let mut p = config::unprotect(&self.cfg.saved_password);
                p.push('\r');
                if let Some(c) = &self.conn {
                    c.send(p.as_bytes());
                }
            }
        }
        // Stored key passphrase: answer "Enter passphrase for key ...:".
        if !self.cfg.key_passphrase.is_empty() {
            let tail: String = clean.trim_end().chars().rev().take(80).collect::<String>().chars().rev().collect();
            let t = tail.to_lowercase();
            let due = self.password_sent.map(|t| t.elapsed().as_secs() > 4).unwrap_or(true);
            if due && t.contains("passphrase for") && t.ends_with(':') {
                self.password_sent = Some(Instant::now());
                let mut p = config::unprotect(&self.cfg.key_passphrase);
                p.push('\r');
                if let Some(c) = &self.conn {
                    c.send(p.as_bytes());
                }
            }
        }
        if self.cfg.automate_logon && self.logon_idx < self.cfg.logon_steps.len() {
            self.logon_buf.push_str(&clean);
            let st = self.cfg.logon_steps[self.logon_idx].clone();
            if st.expect.is_empty() || self.logon_buf.contains(&st.expect) {
                let mut out = config::unescape(&st.send);
                if st.trailing_cr {
                    out.push('\r');
                }
                self.logon_buf.clear();
                self.logon_idx += 1;
                if let Some(c) = &self.conn {
                    c.send(out.as_bytes());
                }
            }
        }
    }

    fn apply_modes(&mut self) {
        let mut m = String::new();
        m.push_str(if self.cfg.line_wrap { "\x1b[?7h" } else { "\x1b[?7l" });
        if self.cfg.app_cursor_keys {
            m.push_str("\x1b[?1h");
        }
        if self.cfg.app_keypad {
            m.push_str("\x1b=");
        }
        if self.cfg.origin_mode {
            m.push_str("\x1b[?6h");
        }
        if self.cfg.insert_mode {
            m.push_str("\x1b[4h");
        }
        self.parser.process(m.as_bytes());
    }

    pub fn connect(&mut self) {
        if let Some(c) = self.conn.take() {
            c.close();
        }
        let msg = format!("\x1b[0mConnecting to {} ...\r\n", self.cfg.summary());
        self.parser.process(msg.as_bytes());
        self.apply_modes();
        self.logon_idx = 0;
        self.logon_buf.clear();
        self.status = Status::Connecting;
        // "Use global public key setting": the key comes from the global option.
        let mut cfg = self.cfg.clone();
        if cfg.protocol.is_ssh() && cfg.pubkey_use_global && !self.global_identity.trim().is_empty() {
            cfg.identity_file = self.global_identity.clone();
        }
        self.xfer = None;
        self.conn = Some(Connection::open(&cfg, self.rows, self.cols, self.ctx.clone()));
        if self.cfg.start_log_on_connect && self.log_file.is_none() {
            if self.cfg.log_prompt {
                self.log_prompt_due = true;
            } else {
                let _ = self.start_log(None);
            }
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(c) = &self.conn {
            c.close();
        }
    }

    /// The session's settings for a clone or an SFTP tab: the password typed in this session (kept in memory
    /// only) is passed on in its encrypted form, so the new tab does not ask for it again.
    pub fn cfg_for_clone(&self) -> config::SessionConfig {
        let mut c = self.cfg.clone();
        if c.saved_password.is_empty() {
            if let Some(p) = &self.typed_password {
                c.saved_password = config::protect(p);
            }
        }
        c
    }

    pub fn is_connected(&self) -> bool {
        self.status == Status::Connected
    }

    pub fn send(&mut self, data: &[u8]) {
        if self.pw_waiting {
            for ch in String::from_utf8_lossy(data).chars() {
                match ch {
                    '\r' | '\n' => {
                        if !self.pw_buf.is_empty() {
                            self.typed_password = Some(std::mem::take(&mut self.pw_buf));
                        }
                        self.pw_waiting = false;
                        break;
                    }
                    '\x7f' | '\x08' => {
                        self.pw_buf.pop();
                    }
                    c if !c.is_control() => self.pw_buf.push(c),
                    _ => {}
                }
            }
        }
        if self.recording.is_some() {
            for ch in String::from_utf8_lossy(data).chars() {
                match ch {
                    '\r' => {
                        let line = std::mem::take(&mut self.rec_line);
                        if let Some(r) = self.recording.as_mut() {
                            r.push(format!("sendln {line}"));
                        }
                    }
                    '\x7f' | '\x08' => {
                        self.rec_line.pop();
                    }
                    c if !c.is_control() => self.rec_line.push(c),
                    _ => {}
                }
            }
        }
        if let Some(c) = &self.conn {
            if self.cfg.encoding == "ISO-8859-1" {
                let bytes: Vec<u8> = String::from_utf8_lossy(data).chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect();
                c.send(&bytes);
            } else {
                c.send(data);
            }
        }
        if self.cfg.local_echo {
            let echo = String::from_utf8_lossy(data).replace('\r', "\r\n");
            self.parser.process(echo.as_bytes());
        }
        self.last_sent = Instant::now();
        if self.cfg.scroll_bottom_keypress {
            self.scroll_offset = 0;
            self.parser.screen_mut().set_scrollback(0);
        }
    }

    pub fn send_str(&mut self, s: &str) {
        self.send(s.as_bytes());
    }

    pub fn send_break(&mut self) {
        if let Some(c) = &self.conn {
            c.send_break();
        }
    }

    /// Remove the host key question and the "user@host's password:" line from what is drawn (they are asked
    /// in windows instead), and ssh's "Warning: Permanently added ... known hosts." note (the original shows none).
    fn hide_ssh_questions(&mut self, d: Vec<u8>) -> Vec<u8> {
        let mut text = String::from_utf8_lossy(&d).into_owned();
        // The pseudo console redraws its whole screen after a resize, hidden lines included: what was hidden once
        // stays hidden (the line is left empty, so the rows of the redraw keep their places).
        for h in &self.hidden_lines {
            if text.contains(h.as_str()) {
                text = text.replace(h.as_str(), "");
            }
        }
        while let Some(i) = text.find("Warning: Permanently added") {
            let end = text[i..].find('\n').map(|k| i + k + 1).unwrap_or(text.len());
            let line = strip_escapes(&text[i..end]).trim().to_string();
            if !line.is_empty() && !self.hidden_lines.contains(&line) {
                self.hidden_lines.push(line);
            }
            text.replace_range(i..end, "");
        }
        let mut rest: &str = &text;
        let mut out = String::new();
        let mut cut = String::new();
        let cut_question = |s: &str, from: usize| -> usize {
            // the end of "...(yes/no/[fingerprint])?" starting the search at `from`
            match s[from..].find('?') {
                Some(k) => from + k + 1,
                None => s.len(),
            }
        };
        if self.hk_block {
            match rest.find("(yes/no") {
                Some(i) => {
                    let end = cut_question(rest, i);
                    cut.push_str(&rest[..end]);
                    rest = &rest[end..];
                    self.hk_block = false;
                }
                None => {
                    self.hide_lines_of(rest);
                    return Vec::new();
                }
            }
        }
        if let Some(i) = rest.find("The authenticity of host") {
            out.push_str(&rest[..i]);
            rest = &rest[i..];
            match rest.find("(yes/no") {
                Some(j) => {
                    let end = cut_question(rest, j);
                    cut.push_str(&rest[..end]);
                    rest = &rest[end..];
                }
                None => {
                    cut.push_str(rest);
                    self.hk_block = true;
                    rest = "";
                }
            }
        }
        self.hide_lines_of(&cut);
        out.push_str(rest);
        let recent_connect = self.connected_at.map(|t| t.elapsed() < Duration::from_secs(90)).unwrap_or(true);
        let tail_start = out.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let tl = out[tail_start..].trim_end().to_lowercase();
        if recent_connect && tl.ends_with("password:") && !tl.contains("[sudo]") {
            let line = out[tail_start..].to_string();
            self.hide_lines_of(&line);
            out.truncate(tail_start);
        }
        out.into_bytes()
    }

    /// Remember the lines of `s` as hidden; a question line is also remembered with the answer ssh echoes after it.
    fn hide_lines_of(&mut self, s: &str) {
        for l in strip_escapes(s).lines() {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            let mut add = vec![l.to_string()];
            if l.ends_with('?') {
                add.insert(0, format!("{l} yes"));
                add.insert(1, format!("{l} no"));
            }
            for a in add {
                if !self.hidden_lines.contains(&a) {
                    self.hidden_lines.push(a);
                }
            }
        }
    }

    /// The person answered the question shown in the window: `Some(text)` is sent (with Enter), `None` cancels.
    pub fn answer_prompt(&mut self, answer: Option<&str>) {
        let was = self.prompt_req.take();
        self.prompt_done = Some(Instant::now());
        match answer {
            Some(t) => {
                if matches!(was, Some(SshPrompt::Password { .. })) && !t.is_empty() {
                    self.typed_password = Some(t.to_string());
                }
                self.pw_waiting = false;
                let mut line = t.to_string();
                line.push('\r');
                if let Some(c) = &self.conn {
                    c.send(line.as_bytes());
                }
            }
            None => {
                if matches!(was, Some(SshPrompt::HostKey { .. })) {
                    if let Some(c) = &self.conn {
                        c.send(b"no\r");
                    }
                } else {
                    self.disconnect();
                }
            }
        }
    }

    /// SFTP: collect the lines of an `ls -l` listing and give them back sorted, in the original's layout.
    /// Everything that is not a listing line passes through unchanged.
    fn sftp_filter(&mut self, d: Vec<u8>) -> Vec<u8> {
        let now = config::now();
        self.sftp_pending.push_str(&String::from_utf8_lossy(&d));
        let mut out = String::new();
        while let Some(i) = self.sftp_pending.find('\n') {
            let raw: String = self.sftp_pending.drain(..=i).collect();
            let line = raw.trim_end_matches(['\r', '\n']);
            match crate::sftplist::parse(line, now.y, now.mo) {
                Some(e) => self.sftp_entries.push(e),
                None => {
                    self.flush_sftp_entries(&mut out);
                    out.push_str(&raw);
                }
            }
        }
        // a partial line: a prompt goes out at once, the start of a listing line waits for the rest
        if !self.sftp_pending.is_empty() && !crate::sftplist::could_be_listing(&self.sftp_pending) {
            self.flush_sftp_entries(&mut out);
            out.push_str(&std::mem::take(&mut self.sftp_pending));
        }
        self.sftp_last = Some(Instant::now());
        out.into_bytes()
    }

    fn flush_sftp_entries(&mut self, out: &mut String) {
        if self.sftp_entries.is_empty() {
            return;
        }
        for l in crate::sftplist::format(std::mem::take(&mut self.sftp_entries)) {
            out.push_str(&l);
            out.push_str("\r\n");
        }
    }

    /// Listing lines (or a cut line) that nothing followed for a moment are shown anyway.
    fn sftp_idle_flush(&mut self) {
        if self.sftp_entries.is_empty() && self.sftp_pending.is_empty() {
            return;
        }
        if self.sftp_last.map(|t| t.elapsed() < Duration::from_millis(250)).unwrap_or(false) {
            return;
        }
        let mut out = String::new();
        self.flush_sftp_entries(&mut out);
        out.push_str(&std::mem::take(&mut self.sftp_pending));
        if !out.is_empty() {
            self.write_log(out.as_bytes());
            self.parser.process(out.as_bytes());
        }
    }

    /// Send pasted text (carriage returns between lines). With a delay, one line goes out every
    /// `delay_ms` milliseconds; a program that asked for bracketed paste gets the text in one piece.
    pub fn paste_text(&mut self, p: &str, delay_ms: Option<u64>) {
        if self.parser.screen().bracketed_paste() {
            self.send_str(&format!("\x1b[200~{p}\x1b[201~"));
        } else if let Some(ms) = delay_ms.filter(|ms| *ms > 0 && p.contains('\r')) {

            let mut rest = p;
            while let Some(i) = rest.find('\r') {
                self.paste_queue.push_back((rest[..=i].to_string(), ms));
                rest = &rest[i + 1..];
            }
            if !rest.is_empty() {
                self.paste_queue.push_back((rest.to_string(), ms));
            }
            self.paste_next = None;
        } else {
            self.send_str(p);
        }
    }

    fn pump_paste_queue(&mut self) {
        if self.paste_queue.is_empty() {
            return;
        }
        if self.paste_next.map(|t| Instant::now() >= t).unwrap_or(true) {
            if let Some((line, wait)) = self.paste_queue.pop_front() {
                self.send_str(&line);
                self.paste_next = Some(Instant::now() + Duration::from_millis(wait));
            }
        }
    }

    /// Send pieces of text one after the other, waiting the given milliseconds after each
    /// (a `\p` in a command string pauses one second).
    pub fn send_chunks(&mut self, chunks: Vec<(String, u64)>) {
        for c in chunks {
            self.paste_queue.push_back(c);
        }
    }

    /// Pull everything the connection delivered since the last frame.
    pub fn pump(&mut self) {
        self.pump_paste_queue();
        if self.cfg.protocol == config::Protocol::Sftp {
            self.sftp_idle_flush();
        }
        if self.log_file.is_some() && self.log_synced.map(|t| t.elapsed() >= Duration::from_millis(500)).unwrap_or(false) {
            self.sync_log(false);
        }
        if self.log_file.is_some() && self.cfg.log_midnight && config::now().d != self.log_day {
            let raw = self.log_raw;
            self.stop_log();
            self.log_raw = raw;
            let _ = self.start_log(None);
        }
        if let Some(t) = self.reconnect_at {
            if Instant::now() >= t {
                self.reconnect_at = None;
                self.connect();
            }
        }
        if self.is_connected() {
            // anti-idle: send a string, or a protocol no-op, when nothing was sent for a while
            if self.cfg.anti_idle_string_on && !self.cfg.anti_idle_string.is_empty() && self.last_sent.elapsed().as_secs() >= self.cfg.anti_idle_secs.max(1) as u64 {
                let s = config::unescape(&self.cfg.anti_idle_string);
                self.send_str(&s);
            }
            if self.cfg.anti_idle_noop_on && self.cfg.protocol.is_telnet() && self.last_noop.elapsed().as_secs() >= self.cfg.anti_idle_noop_secs.max(1) as u64 {
                self.last_noop = Instant::now();
                if let Some(c) = &self.conn {
                    c.send_raw(&[255, 241]);
                }
            }
        }
        if self.parser.callbacks().answerback != self.cfg.answerback {
            self.parser.callbacks_mut().answerback = self.cfg.answerback.clone();
        }
        let mut events = Vec::new();
        if let Some(c) = &self.conn {
            while let Ok(ev) = c.rx.try_recv() {
                events.push(ev);
            }
        }
        for ev in events {
            match ev {
                ConnEvent::Connected => {
                    let p = self.cfg.log_custom_connect.clone();
                    self.log_custom(&p);
                    self.status = Status::Connected;
                    self.connected_at = Some(Instant::now());
                    if self.cfg.automate_logon && self.cfg.send_initial_cr {
                        self.send(b"\r");
                    }
                    if self.cfg.logon_script && !self.cfg.logon_script_path.trim().is_empty() {
                        self.logon_script_due = true;
                    }
                }
                ConnEvent::Data(d) => {
                    // A file transfer in progress takes the raw bytes instead of the screen.
                    if let Some(x) = self.xfer.as_mut() {
                        if !x.done {
                            let out = x.feed(&d);
                            if !out.is_empty() {
                                if let Some(c) = &self.conn {
                                    c.send(&out);
                                }
                            }
                            continue;
                        }
                    }
                    // The ssh program's questions (host key, password) are asked in windows, so their text is kept
                    // off the screen; the logon logic still sees the original text.
                    let ssh_like = matches!(self.cfg.protocol, config::Protocol::Ssh | config::Protocol::Sftp);
                    let d = if ssh_like {
                        self.note_text(&d);
                        self.hide_ssh_questions(d)
                    } else {
                        d
                    };
                    let d = if self.cfg.protocol == config::Protocol::Sftp { self.sftp_filter(d) } else { d };
                    if d.is_empty() {
                        continue;
                    }
                    self.bytes_in += d.len() as u64;
                    self.hex.extend(d.iter().copied());
                    while self.hex.len() > 4096 {
                        self.hex.pop_front();
                    }
                    self.write_log(&d);
                    let d = if self.cfg.encoding == "ISO-8859-1" {
                        d.iter().map(|&b| b as char).collect::<String>().into_bytes()
                    } else {
                        d
                    };
                    let d = if self.cfg.newline_mode {
                        let mut v = Vec::with_capacity(d.len() + 8);
                        let mut prev = 0u8;
                        for &b in &d {
                            if b == b'\n' && prev != b'\r' {
                                v.push(b'\r');
                            }
                            v.push(b);
                            prev = b;
                        }
                        v
                    } else {
                        d
                    };
                    if !ssh_like {
                        self.note_text(&d);
                    }
                    let keep = if self.cfg.scroll_bottom_output { 0 } else { self.scroll_offset };
                    if self.cfg.scroll_bottom_output {
                        self.scroll_offset = 0;
                    }
                    self.parser.screen_mut().set_scrollback(0);
                    self.parser.process(&d);
                    if keep > 0 {
                        self.parser.screen_mut().set_scrollback(keep);
                        self.scroll_offset = self.parser.screen().scrollback();
                    }
                }
                ConnEvent::Closed(why) => {
                    let p = self.cfg.log_custom_disconnect.clone();
                    self.log_custom(&p);
                    if !matches!(self.status, Status::Disconnected(_)) {
                        let msg = format!("\r\n\x1b[0m[{why}]\r\n");
                        self.parser.process(msg.as_bytes());
                        let had_link = self.connected_at.is_some();
                        self.status = Status::Disconnected(why);
                        if self.cfg.clear_on_disconnect {
                            self.clear_all();
                        }
                        if self.cfg.close_on_disconnect {
                            self.close_due = true;
                        } else if self.cfg.auto_reconnect && had_link {
                            self.reconnect_at = Some(Instant::now() + std::time::Duration::from_secs(self.reconnect_secs));
                        }
                    }
                    self.connected_at = None;
                }
            }
        }
        if let Some(x) = self.xfer.as_mut() {
            let out = x.tick();
            if !out.is_empty() {
                if let Some(c) = &self.conn {
                    c.send(&out);
                }
            }
        }
        let cb = self.parser.callbacks_mut();
        let replies = std::mem::take(&mut cb.replies);
        let title = cb.title.take();
        let bell = std::mem::take(&mut cb.bell);
        if !replies.is_empty() {
            if let Some(c) = &self.conn {
                c.send(&replies);
            }
        }
        if let Some(t) = title {
            // The pseudo console reports the program's path as a title; keep the session name then.
            let looks_like_path = t.to_lowercase().ends_with(".exe");
            if !t.trim().is_empty() && !looks_like_path && !self.renamed {
                self.title = t;
            }
        }
        if bell {
            if self.cfg.visual_bell {
                self.bell_flash = Some(Instant::now());
            }
            if self.cfg.audible_bell {
                config::beep();
            }
        }
    }

    /// The text of the whole live screen (used by Print).
    pub fn screen_text(&mut self) -> String {
        let keep = self.scroll_offset;
        self.parser.screen_mut().set_scrollback(0);
        let t = self.parser.screen().contents();
        self.parser.screen_mut().set_scrollback(keep);
        t
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if rows == self.rows && cols == self.cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.screen_mut().set_size(rows, cols);
        if let Some(c) = &self.conn {
            c.resize(rows, cols);
        }
    }

    // ----- scrollback ------------------------------------------------------

    pub fn scrollback_len(&mut self) -> usize {
        let keep = self.scroll_offset;
        self.parser.screen_mut().set_scrollback(usize::MAX);
        let n = self.parser.screen().scrollback();
        self.parser.screen_mut().set_scrollback(keep);
        n
    }

    pub fn scroll_to(&mut self, offset: usize) {
        self.parser.screen_mut().set_scrollback(offset);
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Text of one virtual line (0 = top of the live screen, negative = scrollback).
    pub fn line_text(&mut self, v: i64) -> String {
        let keep = self.scroll_offset;
        let off = if v < 0 { (-v) as usize } else { 0 };
        self.parser.screen_mut().set_scrollback(off);
        let real_off = self.parser.screen().scrollback() as i64;
        let row = v + real_off;
        let mut s = String::new();
        if row >= 0 && row < self.rows as i64 {
            let screen = self.parser.screen();
            for c in 0..self.cols {
                if let Some(cell) = screen.cell(row as u16, c) {
                    if cell.is_wide_continuation() {
                        continue;
                    }
                    let t = cell.contents();
                    if t.is_empty() {
                        s.push(' ');
                    } else {
                        s.push_str(t);
                    }
                }
            }
        }
        self.parser.screen_mut().set_scrollback(keep);
        s
    }

    pub fn selected_text(&mut self) -> Option<String> {
        let (a, b) = self.selection?;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let mut out = String::new();
        for v in a.0..=b.0 {
            let line: Vec<char> = self.line_text(v).chars().collect();
            let start = if v == a.0 { a.1 as usize } else { 0 };
            let end = if v == b.0 { (b.1 as usize + 1).min(line.len()) } else { line.len() };
            let piece: String = line.get(start.min(line.len())..end.max(start.min(line.len()))).unwrap_or(&[]).iter().collect();
            out.push_str(piece.trim_end());
            if v != b.0 {
                out.push_str("\r\n");
            }
        }
        // "Triple-click selection includes newline"
        if self.line_sel.is_some() && self.line_sel == self.selection {
            out.push_str("\r\n");
        }
        Some(out)
    }

    pub fn select_all(&mut self) {
        let n = self.scrollback_len() as i64;
        self.selection = Some(((-n, 0), (self.rows as i64 - 1, self.cols - 1)));
    }

    /// Search forward (or backward) from the last hit; returns whether something was found.
    pub fn find(&mut self, query: &str, backward: bool, case_sensitive: bool) -> bool {
        if query.is_empty() {
            return false;
        }
        let q = if case_sensitive { query.to_string() } else { query.to_lowercase() };
        let n = self.scrollback_len() as i64;
        let first = -n;
        let last = self.rows as i64 - 1;
        let lines: Vec<String> = (first..=last)
            .map(|v| {
                let t = self.line_text(v);
                if case_sensitive { t } else { t.to_lowercase() }
            })
            .collect();
        let (cur_line, cur_col) = match self.find_hit {
            Some((l, c, _)) => (l, c as i64),
            None => (if backward { last + 1 } else { first - 1 }, 0),
        };
        let total = lines.len() as i64;
        for step in 0..=total {
            let v = if backward { cur_line - step } else { cur_line + step };
            if v < first || v > last {
                continue;
            }
            let line = &lines[(v - first) as usize];
            let chars: Vec<char> = line.chars().collect();
            let qc: Vec<char> = q.chars().collect();
            let mut positions = Vec::new();
            if chars.len() >= qc.len() {
                for i in 0..=(chars.len() - qc.len()) {
                    if chars[i..i + qc.len()] == qc[..] {
                        positions.push(i as i64);
                    }
                }
            }
            let pick = if backward {
                positions.into_iter().filter(|&p| step > 0 || p < cur_col).last()
            } else {
                positions.into_iter().find(|&p| step > 0 || p > cur_col)
            };
            if let Some(p) = pick {
                self.find_hit = Some((v, p as u16, qc.len()));
                self.selection = Some(((v, p as u16), (v, (p as usize + qc.len() - 1) as u16)));
                // make the line visible
                let off = if v < 0 { (-v) as usize } else { 0 };
                self.scroll_to(off);
                return true;
            }
        }
        false
    }

    pub fn clear_screen(&mut self) {
        self.parser.process(b"\x1b[2J\x1b[H");
    }

    pub fn clear_scrollback(&mut self) {
        let state = self.parser.screen().state_formatted();
        let cb = std::mem::take(self.parser.callbacks_mut());
        self.parser = vt100::Parser::new_with_callbacks(self.rows, self.cols, self.cfg.scrollback, cb);
        self.parser.process(&state);
        self.scroll_offset = 0;
        self.selection = None;
        self.find_hit = None;
    }

    pub fn clear_all(&mut self) {
        let cb = std::mem::take(self.parser.callbacks_mut());
        self.parser = vt100::Parser::new_with_callbacks(self.rows, self.cols, self.cfg.scrollback, cb);
        self.scroll_offset = 0;
        self.selection = None;
        self.find_hit = None;
    }

    pub fn reset(&mut self) {
        self.parser.process(b"\x1bc");
    }

    // ----- logging -------------------------------------------------------------

    /// The log file name from the session's pattern (variables %H %S %Y %M %D %h %m %s %t).
    pub fn default_log_path(&self) -> String {
        let session: String = self
            .cfg
            .display_name()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' || c == '@' { c } else { '_' })
            .collect();
        let pattern = if self.cfg.log_file.trim().is_empty() { "%S_%Y-%M-%D_%h%m%s.log" } else { self.cfg.log_file.trim() };
        let name = config::expand_log_name(pattern, &self.cfg.host, &session);
        let p = std::path::PathBuf::from(&name);
        if p.is_absolute() {
            name
        } else {
            config::data_dir().join("logs").join(name).to_string_lossy().into_owned()
        }
    }

    pub fn start_log(&mut self, path: Option<String>) -> Result<String, String> {
        let path = path.filter(|p| !p.trim().is_empty()).unwrap_or_else(|| self.default_log_path());
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut oo = std::fs::OpenOptions::new();
        oo.create(true);
        if self.cfg.log_append {
            oo.append(true);
        } else {
            oo.write(true).truncate(true);
        }
        let f = oo.open(&path).map_err(|e| format!("Could not create the log file: {e}"))?;
        self.log_file = Some(f);
        self.log_path = Some(path.clone());
        self.log_paused = false;
        self.log_line_start = true;
        self.log_raw = self.log_raw || self.cfg.log_raw;
        self.log_day = config::now().d;
        Ok(path)
    }

    fn expand_custom(&self, pattern: &str) -> String {
        let session: String = self.cfg.display_name();
        config::expand_log_name(pattern, &self.cfg.host, &session)
    }

    /// Write one custom line (upon connect / upon disconnect) to the log.
    fn log_custom(&mut self, pattern: &str) {
        if pattern.is_empty() || self.log_paused {
            return;
        }
        let text = format!("{}\r\n", self.expand_custom(pattern));
        if let Some(f) = self.log_file.as_mut() {
            let _ = f.write_all(text.as_bytes());
        }
    }

    /// Start capturing the received text to a file (Transfer > Receive ASCII).
    pub fn start_capture(&mut self, path: &str) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let f = std::fs::File::create(path).map_err(|e| format!("Could not create the file: {e}"))?;
        self.capture_file = Some(f);
        Ok(())
    }

    pub fn stop_log(&mut self) {
        self.log_raw = false;
        self.log_file = None;
        self.log_path = None;
    }

    pub fn is_logging(&self) -> bool {
        self.log_file.is_some()
    }

    fn write_log(&mut self, data: &[u8]) {
        if self.log_file.is_none() && self.capture_file.is_none() {
            return;
        }
        // Strip escape sequences so the log holds the plain text.
        let mut out = Vec::with_capacity(data.len());
        let raw = self.log_raw && self.capture_file.is_none();
        for &b in data {
            if raw {
                out.push(b);
                continue;
            }
            match self.log_strip {
                0 => match b {
                    0x1b => self.log_strip = 1,
                    b'\n' | b'\r' | b'\t' => out.push(b),
                    0x20..=0x7e | 0x80..=0xff => out.push(b),
                    _ => {}
                },
                1 => {
                    self.log_strip = match b {
                        b'[' => 2,
                        b']' | b'P' | b'_' | b'^' => 3,
                        b'(' | b')' | b'*' | b'+' | b'#' => 5,
                        _ => 0,
                    }
                }
                2 => {
                    if (0x40..=0x7e).contains(&b) {
                        self.log_strip = 0
                    }
                }
                3 => {
                    if b == 0x07 {
                        self.log_strip = 0
                    } else if b == 0x1b {
                        self.log_strip = 4
                    }
                }
                4 => self.log_strip = if b == b'\\' { 0 } else { 3 },
                _ => self.log_strip = 0,
            }
        }
        if let Some(f) = self.capture_file.as_mut() {
            let _ = f.write_all(&out);
        }
        if self.log_paused || self.log_file.is_none() {
            return;
        }
        let custom_line = !self.cfg.log_custom_line.is_empty();
        let stamped: Vec<u8> = if self.cfg.log_only_custom && (custom_line || !self.cfg.log_custom_connect.is_empty() || !self.cfg.log_custom_disconnect.is_empty()) {
            // only the custom text, once per received line
            let mut v = Vec::new();
            if custom_line {
                let line = self.expand_custom(&self.cfg.log_custom_line.clone());
                for &b in &out {
                    if b == b'\n' {
                        v.extend_from_slice(line.as_bytes());
                        v.extend_from_slice(b"\r\n");
                    }
                }
            }
            v
        } else if self.cfg.log_timestamp || custom_line {
            let mut v = Vec::with_capacity(out.len() + 32);
            for &b in &out {
                if self.log_line_start && b != b'\n' && b != b'\r' {
                    if self.cfg.log_timestamp {
                        let n = config::now();
                        v.extend_from_slice(format!("[{:02}:{:02}:{:02}.{:03}] ", n.h, n.mi, n.s, n.ms).as_bytes());
                    }
                    if custom_line {
                        v.extend_from_slice(self.expand_custom(&self.cfg.log_custom_line.clone()).as_bytes());
                        v.push(b' ');
                    }
                    self.log_line_start = false;
                }
                v.push(b);
                if b == b'\n' {
                    self.log_line_start = true;
                }
            }
            v
        } else {
            out
        };
        if let Some(f) = self.log_file.as_mut() {
            let _ = f.write_all(&stamped);
        }
        self.sync_log(false);
    }

    /// Windows keeps the size of an open file stale until it is flushed, so a log looked at while it is
    /// written (a file manager, `tail -f`) seemed empty; flush it at most every 300 ms.
    fn sync_log(&mut self, force: bool) {
        let due = force || self.log_synced.map(|t| t.elapsed() >= Duration::from_millis(300)).unwrap_or(true);
        if due {
            if let Some(f) = self.log_file.as_mut() {
                let _ = f.sync_data();
                self.log_synced = Some(Instant::now());
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// ---------------------------------------------------------------------------
// Drawing and input

fn c32(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn resolve(color: vt100::Color, default: Rgb, bold: bool, ansi: bool, truecolor: bool) -> Rgb {
    if !ansi {
        return default;
    }
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(i) => {
            let i = if bold && i < 8 { i + 8 } else { i };
            config::xterm256(i)
        }
        vt100::Color::Rgb(r, g, b) => {
            if truecolor {
                [r, g, b]
            } else {
                config::nearest256(r, g, b)
            }
        }
    }
}

pub struct ViewOptions<'a> {
    pub keywords: &'a [String],
    pub highlight: bool,
    pub copy_on_select: bool,
    pub font_size: f32,
    /// false while a dialog is open: the keys belong to the dialog
    pub accept_input: bool,
    /// ask before pasting text with `paste_prompt_lines` or more newlines
    pub confirm_paste: bool,
    /// "Prompt on multi-line paste for [n] or more newlines"
    pub paste_prompt_lines: u32,
    /// pause between pasted lines, in milliseconds (None = send at once)
    pub paste_delay: Option<u64>,
    /// "Paste on [middle|right] button": Some(0) the middle button pastes, Some(1) the right button
    /// pastes (the caller then shows no context menu on a right click), None = off
    pub paste_button: Option<u8>,
    /// hide the mouse pointer over the terminal after a key press, until the pointer moves
    pub hide_pointer: bool,
    /// CTRL+click on an http(s) URL opens it
    pub url_ctrl_click: bool,
    /// a triple click selects the line; its copy ends with the newline when this is true
    pub triple_click_newline: bool,
    /// "Quotation character(s)" of Paste as Quotation
    pub quote_chars: &'a str,
    /// "Insert pasted text between" the quotation characters (false: they only go before it)
    pub quote_between: bool,
    /// the web browser program that opens URLs ("" = the system's default browser)
    pub browser: &'a str,
}

/// Open a URL in `browser`, or in the system's default browser when it is empty.
pub fn open_url(url: &str, browser: &str) {
    if !browser.trim().is_empty() {
        let _ = std::process::Command::new(browser.trim()).arg(url).spawn();
        return;
    }
    #[cfg(windows)]
    {
        // url.dll takes the URL as it is (cmd's "start" would split it at '&')
        let _ = std::process::Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", url]).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// The http:// or https:// URL in `line` that covers character `col`.
fn url_at(line: &[char], col: usize) -> Option<String> {
    let stop = |c: char| c.is_whitespace() || "<>\"'`{}|\\^".contains(c);
    if col >= line.len() || stop(line[col]) {
        return None;
    }
    let mut a = col;
    while a > 0 && !stop(line[a - 1]) {
        a -= 1;
    }
    let mut b = col;
    while b + 1 < line.len() && !stop(line[b + 1]) {
        b += 1;
    }
    let lower: Vec<char> = line[a..=b].iter().map(|c| c.to_ascii_lowercase()).collect();
    let rel = col - a;
    let starts_at = |i: usize, p: &str| p.chars().enumerate().all(|(k, pc)| lower.get(i + k) == Some(&pc));
    // the last URL start at or before the clicked character
    let start = (0..=rel).rev().find(|&i| starts_at(i, "http://") || starts_at(i, "https://"))?;
    let url: String = line[a + start..=b].iter().collect();
    let url = url.trim_end_matches(|c: char| ".,;:!?)]".contains(c)).to_string();
    if start + url.chars().count() <= rel || url.split_once("://").is_none_or(|(_, rest)| rest.is_empty()) {
        return None;
    }
    Some(url)
}

const SCROLLBAR_W: f32 = 14.0;

/// Draw the session's screen in `rect`, handle its keyboard and mouse input.
pub fn show(ui: &mut egui::Ui, s: &mut Session, opts: &ViewOptions, rect: Rect) -> egui::Response {
    let pad = s.cfg.margin.clamp(0.0, 40.0);
    let sbw = if s.cfg.show_scrollbar { SCROLLBAR_W } else { 0.0 };
    let mut font = FontId::monospace(opts.font_size);
    let (mut cw, mut ch) = ui.ctx().fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
    if s.cfg.scale_font {
        // "Scale the font": the logical rows x columns stay, the font grows or shrinks with the window
        let aw = (rect.width() - 2.0 * pad - sbw).max(50.0);
        let ah = (rect.height() - 2.0 * pad).max(50.0);
        let k = (aw / (s.cfg.cols.max(10) as f32 * cw.max(1.0))).min(ah / (s.cfg.rows.max(2) as f32 * ch.max(1.0)));
        font = FontId::monospace((opts.font_size * k).clamp(6.0, 60.0));
        let m = ui.ctx().fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        cw = m.0;
        ch = m.1;
    }
    let cw = cw.max(1.0);
    // The original's rows are as tall as the font is big (13 px rows for a 13 px font); the font's own
    // line height (with its line gap) would make them about 20% taller.
    let ch = font.size.round().max(ch.min(font.size.round()).max(1.0));
    let text_rect = Rect::from_min_max(
        rect.min + Vec2::splat(pad),
        Pos2::new(rect.max.x - sbw - pad, rect.max.y - pad),
    );
    let (cols, rows) = if s.cfg.resize_retain || s.cfg.scale_font {
        (s.cfg.cols.max(10), s.cfg.rows.max(2))
    } else {
        (
            ((text_rect.width() / cw).floor() as i32).clamp(10, 1000) as u16,
            ((text_rect.height() / ch).floor() as i32).clamp(2, 500) as u16,
        )
    };
    s.resize(rows, cols);

    let id = egui::Id::new(("terminal", s.id));
    let resp = ui.interact(Rect::from_min_max(rect.min, Pos2::new(rect.max.x - sbw, rect.max.y)), id, Sense::click_and_drag());
    if resp.clicked() || resp.drag_started() || resp.secondary_clicked() || resp.middle_clicked() {
        resp.request_focus();
    }
    // "Paste on [middle|right] button"
    let paste_click = match opts.paste_button {
        Some(0) => resp.middle_clicked(),
        Some(1) => resp.secondary_clicked(),
        _ => false,
    };
    if paste_click {
        s.paste_requested = true;
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }
    // a pointer hidden by a key press comes back when it moves
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    if s.pointer_hidden_at.is_some() && (!opts.hide_pointer || pointer_pos != s.pointer_hidden_at) {
        s.pointer_hidden_at = None;
    }
    let focused = resp.has_focus();
    if focused {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
            )
        });
    }

    let (fg_def, bg_def) = if s.cfg.use_color_scheme { config::scheme(&s.cfg.color_scheme) } else { ([0xc0, 0xc0, 0xc0], [0, 0, 0]) };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, c32(bg_def));

    // ----- mouse: wheel, selection --------------------------------------
    let sb_len = s.scrollback_len();
    if resp.hovered() || ui.rect_contains_pointer(rect) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0.0 && s.cfg.send_wheel_remote && s.parser.screen().alternate_screen() {
            // full-screen programs get the wheel as arrow keys
            let n = (dy / ch).round() as i32;
            for _ in 0..n.abs().min(10) {
                s.send(if n > 0 { b"\x1b[A" } else { b"\x1b[B" });
            }
        } else if dy != 0.0 {
            s.scroll_accum += dy / ch;
            let lines = s.scroll_accum.trunc();
            if lines != 0.0 {
                s.scroll_accum -= lines;
                let new = (s.scroll_offset as i64 + lines as i64).clamp(0, sb_len as i64) as usize;
                s.scroll_to(new);
            }
        }
    }
    let to_cell = |p: Pos2, off: usize| -> VPos {
        let c = ((p.x - text_rect.min.x) / cw).floor().clamp(0.0, (cols - 1) as f32) as u16;
        let r = ((p.y - text_rect.min.y) / ch).floor().clamp(0.0, (rows - 1) as f32) as i64;
        (r - off as i64, c)
    };
    // The drag only "starts" after the pointer has moved a few pixels, so the anchor is where the
    // button went down, not where the drag was noticed (that lost the first characters of a quick drag).
    if resp.drag_started_by(egui::PointerButton::Primary) {
        let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| resp.interact_pointer_pos());
        if let Some(p) = origin {
            let c = to_cell(p, s.scroll_offset);
            s.selection = Some((c, c));
            s.find_hit = None;
        }
    }
    if resp.dragged_by(egui::PointerButton::Primary) || resp.drag_stopped_by(egui::PointerButton::Primary) {
        let now = ui.input(|i| i.pointer.latest_pos()).or_else(|| resp.interact_pointer_pos());
        if let (Some(p), Some((a, _))) = (now, s.selection) {
            // auto-scroll when dragging beyond the edges
            if p.y < text_rect.min.y && s.scroll_offset < sb_len {
                s.scroll_to(s.scroll_offset + 1);
            } else if p.y > text_rect.max.y && s.scroll_offset > 0 {
                s.scroll_to(s.scroll_offset - 1);
            }
            s.selection = Some((a, to_cell(p, s.scroll_offset)));
        }
    }
    if resp.drag_stopped() && opts.copy_on_select {
        if let Some(t) = s.selected_text() {
            ui.ctx().copy_text(t);
        }
    }
    // ctrl held over a URL: a hand pointer, and a click opens it ("Open URL with CTRL+click")
    let ctrl_down = ui.input(|i| i.modifiers.ctrl);
    let url_under = if opts.url_ctrl_click && ctrl_down && resp.hovered() {
        pointer_pos.and_then(|p| {
            let (v, c) = to_cell(p, s.scroll_offset);
            let line: Vec<char> = s.line_text(v).chars().collect();
            url_at(&line, c as usize)
        })
    } else {
        None
    };
    if resp.triple_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let (v, _) = to_cell(p, s.scroll_offset);
            let line = ((v, 0), (v, cols - 1));
            s.selection = Some(line);
            s.line_sel = if opts.triple_click_newline { Some(line) } else { None };
            s.find_hit = None;
            if opts.copy_on_select {
                if let Some(t) = s.selected_text() {
                    ui.ctx().copy_text(t);
                }
            }
        }
    } else if resp.double_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let (v, c) = to_cell(p, s.scroll_offset);
            let line: Vec<char> = s.line_text(v).chars().collect();
            let delims = s.cfg.word_delims.clone();
            let custom = s.cfg.word_delims_on;
            let is_word = |ch: char| if custom { !ch.is_whitespace() && !delims.contains(ch) } else { ch.is_alphanumeric() || "-_./@:~".contains(ch) };
            let c = c as usize;
            if c < line.len() && is_word(line[c]) {
                let mut a = c;
                while a > 0 && is_word(line[a - 1]) {
                    a -= 1;
                }
                let mut b = c;
                while b + 1 < line.len() && is_word(line[b + 1]) {
                    b += 1;
                }
                s.selection = Some(((v, a as u16), (v, b as u16)));
                if opts.copy_on_select {
                    if let Some(t) = s.selected_text() {
                        ui.ctx().copy_text(t);
                    }
                }
            }
        }
    } else if resp.clicked() {
        s.selection = None;
        if let Some(url) = &url_under {
            open_url(url, opts.browser);
        }
    }

    // ----- keyboard ---------------------------------------------------------
    if focused && opts.accept_input {
        let events = ui.input(|i| i.events.clone());
        // "Hide mouse pointer on keypress": until the pointer moves
        if opts.hide_pointer && events.iter().any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { pressed: true, .. })) {
            if let Some(p) = pointer_pos.filter(|p| rect.contains(*p)) {
                s.pointer_hidden_at = Some(p);
            }
        }
        let app_cursor = if s.cfg.cursor_switching { s.parser.screen().application_cursor() } else { s.cfg.app_cursor_keys };
        let insert_down = ui.input(|i| i.key_down(egui::Key::Insert));
        for ev in events {
            match ev {
                egui::Event::Text(t) => s.send_str(&t),
                egui::Event::Copy => {
                    if !s.cfg.cua && !insert_down {
                        s.send(&[0x03]);
                    } else if s.selection.is_some() {
                        if let Some(t) = s.selected_text() {
                            ui.ctx().copy_text(t);
                        }
                    } else {
                        s.send(&[0x03]);
                    }
                }
                egui::Event::Cut => s.send(&[0x18]),
                egui::Event::Paste(p) => {
                    // a paste asked for by the mouse or a menu is always a paste; Ctrl+V is one only with the Windows hotkeys
                    let requested = std::mem::take(&mut s.paste_requested) || s.paste_quote;
                    if !s.cfg.cua && !insert_down && !requested {
                        s.send(&[0x16]);
                        continue;
                    }
                    let mut p = p.replace("\r\n", "\r").replace('\n', "\r");
                    if s.paste_quote {
                        // "Paste as Quotation": the quotation character(s) before the text, and after it too
                        // with "Insert pasted text between"
                        s.paste_quote = false;
                        p = if opts.quote_between { format!("{q}{p}{q}", q = opts.quote_chars) } else { format!("{}{p}", opts.quote_chars) };
                    }
                    let newlines = p.matches('\r').count() as u32;
                    if opts.confirm_paste && newlines >= opts.paste_prompt_lines.max(1) && s.pending_paste.is_none() {
                        s.pending_paste = Some(p); // the "Confirm Multi-line Paste" window decides
                    } else {
                        s.paste_text(&p, opts.paste_delay);
                    }
                }
                egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => {
                    // AltGr (Ctrl+Alt): type the character the keyboard layout gives with it.
                    if modifiers.ctrl && modifiers.alt {
                        let ch = physical_key.and_then(|k| altgr_char(k, modifiers.shift)).or_else(|| altgr_char(key, modifiers.shift));
                        if let Some(c) = ch {
                            s.send_str(&c.to_string());
                            continue;
                        }
                    }
                    if modifiers.shift && matches!(key, egui::Key::PageUp | egui::Key::PageDown) {
                        let page = rows as i64 - 1;
                        let delta = if key == egui::Key::PageUp { page } else { -page };
                        let new = (s.scroll_offset as i64 + delta).clamp(0, sb_len as i64) as usize;
                        s.scroll_to(new);
                        continue;
                    }
                    // Mapped keys (Keymap): "F5", "Ctrl+F5", "Shift+Home"...
                    if !s.cfg.mapped_keys.is_empty() {
                        let mut combo = String::new();
                        if modifiers.ctrl {
                            combo.push_str("Ctrl+");
                        }
                        if modifiers.alt {
                            combo.push_str("Alt+");
                        }
                        if modifiers.shift {
                            combo.push_str("Shift+");
                        }
                        combo.push_str(key.name());
                        let hit = s.cfg.mapped_keys.iter().find(|m| m.key.trim().eq_ignore_ascii_case(&combo)).map(|m| (m.func.clone(), m.send.clone()));
                        if let Some((func, arg)) = hit {
                            match func.as_str() {
                                "Send Hex String" => {
                                    let bytes: Vec<u8> = arg.split_whitespace().filter_map(|h| u8::from_str_radix(h.trim_start_matches("0x"), 16).ok()).collect();
                                    s.send(&bytes);
                                }
                                "Copy" => {
                                    if let Some(t) = s.selected_text() {
                                        ui.ctx().copy_text(t);
                                    }
                                }
                                "Paste" => ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                                "Clear Screen" => s.clear_screen(),
                                "Send Break" => s.send_break(),
                                "Disconnect" => s.disconnect(),
                                _ => s.send_str(&config::unescape(&arg)),
                            }
                            continue;
                        }
                    }
                    if modifiers.alt && !modifiers.ctrl {
                        // Alt as Meta: ESC followed by the character.
                        if s.cfg.meta_key {
                            let name = key.name();
                            let mut chars = name.chars();
                            if let (Some(c), None) = (chars.next(), chars.next()) {
                                let c = if modifiers.shift { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() };
                                if s.cfg.meta_8bit && c.is_ascii() {
                                    s.send(&[(c as u8) | 0x80]);
                                } else {
                                    s.send_str(&format!("\x1b{c}"));
                                }
                            }
                        }
                        continue; // other Alt shortcuts belong to the menus
                    }
                    if let Some(bytes) = key_bytes(key, modifiers, app_cursor, s.cfg.backspace_bs, &s.cfg.enter_sends, s.cfg.delete_sends_del, s.cfg.home_end_tilde) {
                        s.send(&bytes);
                    }
                }
                _ => {}
            }
        }
    }

    // ----- screen -------------------------------------------------------------
    let blink_on = !s.cfg.blinking_cursor || !focused || (ui.input(|i| i.time) * 1000.0 / 530.0) as i64 % 2 == 0;
    if s.cfg.blinking_cursor && focused {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(265));
    }
    let off = s.scroll_offset as i64;
    let sel = s.selection.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
    let in_sel = |v: i64, c: u16| -> bool {
        match sel {
            Some((a, b)) => (v, c) >= a && (v, c) <= b,
            None => false,
        }
    };
    let kw: Vec<String> = if opts.highlight && s.cfg.keyword_highlighting {
        opts.keywords.iter().filter(|k| !k.trim().is_empty()).map(|k| if s.cfg.highlight_case { k.clone() } else { k.to_lowercase() }).collect()
    } else {
        Vec::new()
    };
    let kw_case = s.cfg.highlight_case;
    let kw_line = s.cfg.highlight_line;
    let sel_bg = Color32::from_rgb(0x33, 0x66, 0xcc);
    let kw_color = c32(s.cfg.highlight_color);
    let kw_style = s.cfg.highlight_style.clone();
    let truecolor = s.cfg.truecolor;
    let screen = s.parser.screen();
    for r in 0..rows {
        let v = r as i64 - off;
        let y = text_rect.min.y + r as f32 * ch;
        // keyword columns for this row
        let mut kw_cols = vec![false; cols as usize];
        if !kw.is_empty() {
            let mut line = String::new();
            let mut map = Vec::new();
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    let t = cell.contents();
                    let t = if t.is_empty() { " " } else { t };
                    let t = if kw_case { t.to_string() } else { t.to_lowercase() };
                    for ch in t.chars() {
                        line.push(ch);
                        map.push(c);
                    }
                }
            }
            let chars: Vec<char> = line.chars().collect();
            for k in &kw {
                let kc: Vec<char> = k.chars().collect();
                if kc.is_empty() || chars.len() < kc.len() {
                    continue;
                }
                for i in 0..=(chars.len() - kc.len()) {
                    if chars[i..i + kc.len()] == kc[..] {
                        let (a, b) = if kw_line { (0, chars.len()) } else { (i, i + kc.len()) };
                        for j in a..b {
                            if let Some(&c) = map.get(j) {
                                kw_cols[c as usize] = true;
                            }
                        }
                    }
                }
            }
        }
        let mut c = 0u16;
        while c < cols {
            let Some(cell) = screen.cell(r, c) else { break };
            if cell.is_wide_continuation() {
                c += 1;
                continue;
            }
            let width = if cell.is_wide() { 2 } else { 1 };
            let mut fg = resolve(cell.fgcolor(), fg_def, cell.bold(), s.cfg.ansi_color, truecolor);
            let mut bg = resolve(cell.bgcolor(), bg_def, false, s.cfg.ansi_color, truecolor);
            let mut force_bold = false;
            if cell.inverse() {
                std::mem::swap(&mut fg, &mut bg);
            }
            let mut fgc = c32(fg);
            let mut bgc = c32(bg);
            if cell.dim() {
                fgc = fgc.gamma_multiply(0.6);
            }
            if kw_cols[c as usize] {
                match kw_style.as_str() {
                    "Bold" => {
                        fgc = kw_color;
                        force_bold = true;
                    }
                    "Reverse" => std::mem::swap(&mut fgc, &mut bgc),
                    _ => {
                        bgc = kw_color;
                        fgc = Color32::BLACK;
                    }
                }
            }
            if in_sel(v, c) {
                bgc = sel_bg;
                fgc = Color32::WHITE;
            }
            let x = text_rect.min.x + c as f32 * cw;
            let cell_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cw * width as f32, ch));
            if bgc != c32(bg_def) {
                painter.rect_filled(cell_rect, 0.0, bgc);
            }
            let t = cell.contents();
            if !t.is_empty() && t != " " {
                painter.text(Pos2::new(x, y), egui::Align2::LEFT_TOP, t, font.clone(), fgc);
                if cell.bold() || force_bold {
                    painter.text(Pos2::new(x + 0.6, y), egui::Align2::LEFT_TOP, t, font.clone(), fgc);
                }
            }
            if cell.underline() {
                painter.line_segment(
                    [Pos2::new(x, y + ch - 1.5), Pos2::new(x + cw * width as f32, y + ch - 1.5)],
                    Stroke::new(1.0, fgc),
                );
            }
            c += width;
        }
    }

    // cursor
    if off == 0 && !screen.hide_cursor() {
        let (cr, cc) = screen.cursor_position();
        if cr < rows && cc < cols {
            let x = text_rect.min.x + cc as f32 * cw;
            let y = text_rect.min.y + cr as f32 * ch;
            let cur_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cw, ch));
            let cur_color = c32(fg_def);
            if !focused {
                painter.rect_stroke(cur_rect.shrink(0.5), 0.0, Stroke::new(1.0, cur_color), egui::StrokeKind::Inside);
            } else if blink_on {
                match s.cfg.cursor_style {
                    CursorStyle::Block => {
                        painter.rect_filled(cur_rect, 0.0, cur_color);
                        if let Some(cell) = screen.cell(cr, cc) {
                            let t = cell.contents();
                            if !t.is_empty() {
                                painter.text(Pos2::new(x, y), egui::Align2::LEFT_TOP, t, font.clone(), c32(bg_def));
                            }
                        }
                    }
                    CursorStyle::Underline => {
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(x, y + ch - 2.5), cur_rect.max),
                            0.0,
                            cur_color,
                        );
                    }
                    CursorStyle::VerticalBar => {
                        painter.rect_filled(
                            Rect::from_min_max(cur_rect.min, Pos2::new(x + 2.0, cur_rect.max.y)),
                            0.0,
                            cur_color,
                        );
                    }
                }
            }
        }
    }

    // bell flash
    if let Some(t) = s.bell_flash {
        let el = t.elapsed().as_secs_f32();
        if el < 0.15 {
            painter.rect_filled(rect, 0.0, Color32::from_white_alpha(40));
            ui.ctx().request_repaint();
        } else {
            s.bell_flash = None;
        }
    }

    let pointer_icon = if s.pointer_hidden_at.is_some() {
        egui::CursorIcon::None
    } else if url_under.is_some() {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Text
    };
    // ----- scrollbar ------------------------------------------------------------
    if !s.cfg.show_scrollbar {
        if resp.hovered() {
            ui.ctx().set_cursor_icon(pointer_icon);
        }
        return resp;
    }
    let track = Rect::from_min_max(Pos2::new(rect.max.x - SCROLLBAR_W, rect.min.y), rect.max);
    let sb_resp = ui.interact(track, id.with("scrollbar"), Sense::click_and_drag());
    let dark_bg = (bg_def[0] as u32 + bg_def[1] as u32 + bg_def[2] as u32) < 384;
    let track_col = if dark_bg { Color32::from_gray(45) } else { Color32::from_gray(230) };
    let thumb_col = if dark_bg { Color32::from_gray(110) } else { Color32::from_gray(160) };
    painter.rect_filled(track, 0.0, track_col);
    let total = (sb_len + rows as usize) as f32;
    let thumb_h = (track.height() * rows as f32 / total).max(20.0).min(track.height());
    let movable = track.height() - thumb_h;
    let frac_from_bottom = if sb_len == 0 { 0.0 } else { s.scroll_offset as f32 / sb_len as f32 };
    let thumb_top = track.min.y + movable * (1.0 - frac_from_bottom);
    let thumb = Rect::from_min_size(Pos2::new(track.min.x + 3.0, thumb_top), Vec2::new(SCROLLBAR_W - 6.0, thumb_h));
    painter.rect_filled(thumb, 4.0, thumb_col);
    if (sb_resp.dragged() || sb_resp.clicked()) && sb_len > 0 && movable > 0.0 {
        if let Some(p) = sb_resp.interact_pointer_pos() {
            let f = ((p.y - track.min.y - thumb_h / 2.0) / movable).clamp(0.0, 1.0);
            let new = ((1.0 - f) * sb_len as f32).round() as usize;
            s.scroll_to(new);
        }
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(pointer_icon);
    }
    resp
}

fn key_bytes(key: egui::Key, m: egui::Modifiers, app_cursor: bool, backspace_bs: bool, enter: &str, delete_del: bool, home_end_tilde: bool) -> Option<Vec<u8>> {
    use egui::Key;
    let modcode = 1 + (m.shift as u8) + 2 * (m.alt as u8) + 4 * (m.ctrl as u8);
    let csi_mod = |final_: char| -> Vec<u8> {
        if modcode > 1 {
            format!("\x1b[1;{modcode}{final_}").into_bytes()
        } else if app_cursor {
            format!("\x1bO{final_}").into_bytes()
        } else {
            format!("\x1b[{final_}").into_bytes()
        }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if modcode > 1 {
            format!("\x1b[{n};{modcode}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };
    let bytes = match key {
        Key::Enter => match enter {
            "CRLF" => b"\r\n".to_vec(),
            "LF" => b"\n".to_vec(),
            _ => b"\r".to_vec(),
        },
        Key::Tab => {
            if m.shift {
                b"\x1b[Z".to_vec()
            } else {
                b"\t".to_vec()
            }
        }
        Key::Backspace => {
            if m.ctrl || backspace_bs {
                vec![0x08]
            } else {
                vec![0x7f]
            }
        }
        Key::Escape => vec![0x1b],
        Key::ArrowUp => csi_mod('A'),
        Key::ArrowDown => csi_mod('B'),
        Key::ArrowRight => csi_mod('C'),
        Key::ArrowLeft => csi_mod('D'),
        Key::Home if home_end_tilde && !app_cursor => tilde(1),
        Key::Home => csi_mod('H'),
        Key::End if home_end_tilde && !app_cursor => tilde(4),
        Key::End => csi_mod('F'),
        Key::Insert => tilde(2),
        Key::Delete => {
            if delete_del {
                vec![0x08]
            } else {
                tilde(3)
            }
        }
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        Key::F3 => b"\x1bOR".to_vec(),
        Key::F4 => b"\x1bOS".to_vec(),
        Key::F5 => tilde(15),
        Key::F6 => tilde(17),
        Key::F7 => tilde(18),
        Key::F8 => tilde(19),
        Key::F9 => tilde(20),
        Key::F10 => tilde(21),
        Key::F11 => tilde(23),
        Key::F12 => tilde(24),
        Key::Space if m.ctrl => vec![0],
        Key::OpenBracket if m.ctrl => vec![0x1b],
        Key::Backslash if m.ctrl => vec![0x1c],
        Key::CloseBracket if m.ctrl => vec![0x1d],
        _ if m.ctrl && !m.alt => {
            let name = key.name();
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => vec![(c.to_ascii_uppercase() as u8) & 0x1f],
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(bytes)
}

/// AltGr reaches the program as Ctrl+Alt, and the window library then drops the typed character
/// (so "/" on a Brazilian keyboard, "@" on a German one...). Ask Windows which character that
/// key gives with AltGr held, so it can still be sent to the host.
#[cfg(windows)]
fn altgr_char(key: egui::Key, shift: bool) -> Option<char> {
    use egui::Key as K;
    #[link(name = "user32")]
    extern "system" {
        fn ToUnicode(vk: u32, scan: u32, state: *const u8, buf: *mut u16, len: i32, flags: u32) -> i32;
    }
    let name = key.name();
    let mut it = name.chars();
    let vk: u32 = match (it.next(), it.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => c.to_ascii_uppercase() as u32,
        _ => match key {
            K::Slash => 0xBF,
            K::Backslash => 0xDC,
            K::Semicolon => 0xBA,
            K::Comma => 0xBC,
            K::Period => 0xBE,
            K::Minus => 0xBD,
            K::Equals => 0xBB,
            K::OpenBracket => 0xDB,
            K::CloseBracket => 0xDD,
            K::Quote => 0xDE,
            K::Backtick => 0xC0,
            _ => return None,
        },
    };
    let mut state = [0u8; 256];
    state[0x11] = 0x80; // Ctrl
    state[0x12] = 0x80; // Alt
    state[0xA2] = 0x80;
    state[0xA5] = 0x80; // right Alt = AltGr
    if shift {
        state[0x10] = 0x80;
    }
    let mut buf = [0u16; 8];
    let n = unsafe { ToUnicode(vk, 0, state.as_ptr(), buf.as_mut_ptr(), buf.len() as i32, 4) };
    if n == 1 {
        let c = char::from_u32(buf[0] as u32)?;
        if !c.is_control() {
            return Some(c);
        }
    }
    None
}

#[cfg(not(windows))]
fn altgr_char(_key: egui::Key, _shift: bool) -> Option<char> {
    None
}

#[cfg(test)]
mod altscreen_tests {
    /// Running a full-screen program (less) and leaving it must bring the shell screen back untouched,
    /// with the new prompt on its own line.
    #[test]
    fn leaving_less_restores_screen() {
        let mut p = vt100::Parser::new(10, 40, 100);
        p.process(b"ubuntu@zeus:~$ less /etc/services\r\n");
        p.process(b"\x1b[?1049h\x1b[H\x1b[2J# services\r\nline 2\r\n:");
        p.process(b"\x1b[?1049l\r");
        p.process(b"ubuntu@zeus:~$ ");
        let rows: Vec<String> = p.screen().rows(0, 40).collect();
        assert_eq!(rows[0].trim_end(), "ubuntu@zeus:~$ less /etc/services");
        assert_eq!(rows[1].trim_end(), "ubuntu@zeus:~$");
    }
}

#[cfg(test)]
mod home_end_tests {
    use super::key_bytes;
    use egui::{Key, Modifiers};

    #[test]
    fn home_end_follow_the_option() {
        let n = Modifiers::NONE;
        assert_eq!(key_bytes(Key::Home, n, false, false, "CR", false, true).unwrap(), b"\x1b[1~");
        assert_eq!(key_bytes(Key::End, n, false, false, "CR", false, true).unwrap(), b"\x1b[4~");
        assert_eq!(key_bytes(Key::Home, n, false, false, "CR", false, false).unwrap(), b"\x1b[H");
        assert_eq!(key_bytes(Key::End, n, false, false, "CR", false, false).unwrap(), b"\x1b[F");
        // application cursor mode keeps ESC O H / ESC O F
        assert_eq!(key_bytes(Key::Home, n, true, false, "CR", false, true).unwrap(), b"\x1bOH");
    }
}

/// The text of `s` without escape sequences (CSI, OSC, other ESC pairs) and control characters other than line ends.
fn strip_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\x1b' => match it.next() {
                Some('[') => {
                    // parameters and intermediates up to the final byte @..~
                    for x in it.by_ref() {
                        if ('@'..='~').contains(&x) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: up to BEL or ESC \
                    while let Some(x) = it.next() {
                        if x == '\x07' {
                            break;
                        }
                        if x == '\x1b' && it.peek() == Some(&'\\') {
                            it.next();
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\n' => out.push('\n'),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}
