//! Saved sessions, global options and colour schemes (values from MECHANICS.md).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Ssh,
    Sftp,
    Telnet,
    /// Telnet over TLS (SChannel through native-tls).
    TelnetTls,
    Rlogin,
    Raw,
    Serial,
    LocalShell,
    /// Remote Desktop: starts the Windows Remote Desktop client (mstsc) in its own window.
    Rdp,
}

impl Protocol {
    /// Protocols offered in the Protocol drop-down, in the original's order (SFTP is the "SFTP session"
    /// check box of SSH2).
    pub const PICK: [Protocol; 8] = [
        Protocol::Ssh,
        Protocol::Telnet,
        Protocol::TelnetTls,
        Protocol::LocalShell,
        Protocol::Rdp,
        Protocol::Serial,
        Protocol::Rlogin,
        Protocol::Raw,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Protocol::Ssh | Protocol::Sftp => "SSH2",
            Protocol::Telnet => "Telnet",
            Protocol::TelnetTls => "Telnet/TLS",
            Protocol::Rlogin => "RLogin",
            Protocol::Raw => "Raw",
            Protocol::Serial => "Serial",
            Protocol::LocalShell => "Local Shell",
            Protocol::Rdp => "RDP",
        }
    }

    /// Well-known ports (estimated: the original keeps them in its protocol modules).
    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Ssh | Protocol::Sftp => 22,
            Protocol::Telnet | Protocol::Raw => 23,
            Protocol::TelnetTls => 992,
            Protocol::Rlogin => 513,
            Protocol::Rdp => 3389,
            Protocol::Serial | Protocol::LocalShell => 0,
        }
    }

    pub fn uses_host(self) -> bool {
        !matches!(self, Protocol::Serial | Protocol::LocalShell)
    }

    pub fn is_ssh(self) -> bool {
        matches!(self, Protocol::Ssh | Protocol::Sftp)
    }

    /// Telnet option negotiation runs on the connection (Telnet and Telnet/TLS).
    pub fn is_telnet(self) -> bool {
        matches!(self, Protocol::Telnet | Protocol::TelnetTls)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    None,
    Odd,
    Even,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    None,
    XonXoff,
    RtsCts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    VerticalBar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthMethod {
    pub name: String,
    pub enabled: bool,
}

pub const KEX_DEFAULT: &[&str] = &[
    "curve25519-sha256",
    "ecdh-sha2-nistp521",
    "ecdh-sha2-nistp384",
    "ecdh-sha2-nistp256",
    "diffie-hellman-group-exchange-sha256",
    "diffie-hellman-group16-sha512",
    "diffie-hellman-group14-sha256",
    "diffie-hellman-group14-sha1",
];
pub const CIPHERS_DEFAULT: &[&str] = &[
    "aes256-gcm@openssh.com",
    "aes128-gcm@openssh.com",
    "chacha20-poly1305@openssh.com",
    "aes256-ctr",
    "aes192-ctr",
    "aes128-ctr",
    "aes256-cbc",
    "aes192-cbc",
    "aes128-cbc",
];
pub const MACS_DEFAULT: &[&str] = &[
    "umac-128-etm@openssh.com",
    "umac-64-etm@openssh.com",
    "hmac-sha2-512-etm@openssh.com",
    "hmac-sha2-256-etm@openssh.com",
    "hmac-sha1-etm@openssh.com",
    "umac-128@openssh.com",
    "umac-64@openssh.com",
    "hmac-sha2-512",
    "hmac-sha2-256",
    "hmac-sha1",
];
pub const HOSTKEY_DEFAULT: &[&str] = &["ssh-ed25519", "ecdsa-sha2-nistp256", "ecdsa-sha2-nistp384", "ecdsa-sha2-nistp521", "rsa-sha2-512", "rsa-sha2-256"];

/// The label the original shows for an algorithm name.
pub fn alg_label(name: &str) -> String {
    let t: &[(&str, &str)] = &[
        ("aes256-gcm@openssh.com", "AES-256-GCM"),
        ("aes128-gcm@openssh.com", "AES-128-GCM"),
        ("chacha20-poly1305@openssh.com", "ChaCha20-Poly1305"),
        ("aes256-ctr", "AES-256-CTR"),
        ("aes192-ctr", "AES-192-CTR"),
        ("aes128-ctr", "AES-128-CTR"),
        ("aes256-cbc", "AES-256"),
        ("aes192-cbc", "AES-192"),
        ("aes128-cbc", "AES-128"),
        ("umac-128-etm@openssh.com", "UMAC-128-EtM"),
        ("umac-64-etm@openssh.com", "UMAC-64-EtM"),
        ("hmac-sha2-512-etm@openssh.com", "SHA2-512-EtM"),
        ("hmac-sha2-256-etm@openssh.com", "SHA2-256-EtM"),
        ("hmac-sha1-etm@openssh.com", "SHA1-EtM"),
        ("umac-128@openssh.com", "UMAC-128"),
        ("umac-64@openssh.com", "UMAC-64"),
        ("hmac-sha2-512", "SHA2-512"),
        ("hmac-sha2-256", "SHA2-256"),
        ("hmac-sha1", "SHA1"),
    ];
    t.iter().find(|x| x.0 == name).map(|x| x.1.to_string()).unwrap_or_else(|| name.to_string())
}
pub fn alg_list(names: &[&str]) -> Vec<AuthMethod> {
    names.iter().map(|n| AuthMethod { name: n.to_string(), enabled: true }).collect()
}

/// `Some("a,b,c")` when the list differs from the default (reordered or with unchecked items).
pub fn alg_option(list: &[AuthMethod], default: &[&str]) -> Option<String> {
    let on: Vec<&str> = list.iter().filter(|a| a.enabled).map(|a| a.name.as_str()).collect();
    if on.is_empty() || on == default {
        None
    } else {
        Some(on.join(","))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogonStep {
    pub expect: String,
    pub send: String,
    pub hide: bool,
    pub trailing_cr: bool,
    #[serde(default)]
    pub credentials: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForwardKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forward {
    pub name: String,
    pub kind: ForwardKind,
    pub port: u16,
    pub host: String,
    pub dest_port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MappedKey {
    pub key: String,
    #[serde(default)]
    pub func: String,
    pub send: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub name: String,
    pub description: String,
    pub folder: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub firewall: String,
    pub username: String,
    pub use_credentials: bool,
    pub credentials: String,
    pub auth: Vec<AuthMethod>,
    // logon actions
    pub automate_logon: bool,
    pub send_initial_cr: bool,
    pub logon_steps: Vec<LogonStep>,
    pub logon_script: bool,
    pub logon_script_path: String,
    pub logon_script_args: String,
    pub remote_command_on: bool,
    pub remote_command: String,
    pub display_logon_prompts: bool,
    // ssh
    pub sftp_start_dir: String,
    pub compression: bool,
    pub keepalive_secs: u32,
    pub agent_forward: bool,
    pub x11: bool,
    pub x11_trusted: bool,
    pub x11_display: String,
    pub known_hosts_file: String,
    pub identity_file: String,
    pub pubkey_use_global: bool,
    pub pubkey_agent: bool,
    pub identities_only: bool,
    pub key_passphrase: String,
    pub saved_password: String,
    pub gssapi_delegate: bool,
    pub gssapi_method: String,
    pub gssapi_delegation: String,
    pub gssapi_spn: String,
    pub kex: Vec<AuthMethod>,
    pub ciphers: Vec<AuthMethod>,
    pub macs: Vec<AuthMethod>,
    pub hostkey_algos: Vec<AuthMethod>,
    pub strict_host_key: String,
    pub ssh_verbose: bool,
    pub forwards: Vec<Forward>,
    // serial
    pub com_port: String,
    pub baud: u32,
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
    pub flow: FlowControl,
    // local shell
    pub shell_command: String,
    // terminal
    pub emulation: String,
    pub rows: u16,
    pub cols: u16,
    pub scrollback: usize,
    pub resize_retain: bool,
    pub scale_font: bool,
    pub xterm256: bool,
    pub use_color_scheme: bool,
    pub cua: bool,
    pub close_on_disconnect: bool,
    pub clear_on_disconnect: bool,
    pub auto_reconnect: bool,
    pub scroll_bottom_output: bool,
    pub scroll_bottom_keypress: bool,
    pub send_wheel_remote: bool,
    pub anti_idle_string_on: bool,
    pub anti_idle_string: String,
    pub anti_idle_secs: u32,
    pub anti_idle_noop_on: bool,
    pub anti_idle_noop_secs: u32,
    pub word_delims_on: bool,
    pub word_delims: String,
    pub window_on_top: bool,
    pub ansi_color: bool,
    pub truecolor: bool,
    pub app_cursor_keys: bool,
    pub app_keypad: bool,
    pub origin_mode: bool,
    pub insert_mode: bool,
    pub cursor_switching: bool,
    pub local_echo: bool,
    pub newline_mode: bool,
    pub meta_8bit: bool,
    pub delete_sends_del: bool,
    pub home_end_tilde: bool,
    pub answerback: String,
    pub line_wrap: bool,
    pub meta_key: bool,
    pub backspace_bs: bool,
    pub enter_sends: String,
    pub mapped_keys: Vec<MappedKey>,
    pub audible_bell: bool,
    pub visual_bell: bool,
    // appearance
    pub color_scheme: String,
    pub font_size: f32,
    pub cursor_style: CursorStyle,
    pub blinking_cursor: bool,
    pub encoding: String,
    pub margin: f32,
    pub show_scrollbar: bool,
    pub tab_title: String,
    pub keyword_highlighting: bool,
    pub highlight_style: String,
    pub highlight_color: [u8; 3],
    pub highlight_case: bool,
    pub highlight_line: bool,
    pub tab_color_on: bool,
    pub tab_color: [u8; 3],
    // log
    pub log_file: String,
    pub start_log_on_connect: bool,
    pub log_append: bool,
    pub log_timestamp: bool,
    pub log_prompt: bool,
    pub log_raw: bool,
    pub log_midnight: bool,
    pub log_custom_connect: String,
    pub log_custom_disconnect: String,
    pub log_custom_line: String,
    pub log_only_custom: bool,
    pub sftp_local_dir: String,
    pub compression_level: u8,
    // printing / transfer
    pub print_header: String,
    pub print_footer: String,
    pub upload_dir: String,
    pub xmodem_1k: bool,
    pub download_dir: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            folder: String::new(),
            protocol: Protocol::Ssh,
            host: String::new(),
            port: 22,
            firewall: "None".into(),
            username: String::new(),
            use_credentials: false,
            credentials: String::new(),
            auth: ["PublicKey", "Keyboard Interactive", "Password", "GSSAPI"]
                .iter()
                .map(|n| AuthMethod { name: n.to_string(), enabled: true })
                .collect(),
            automate_logon: false,
            send_initial_cr: true,
            logon_steps: Vec::new(),
            logon_script: false,
            logon_script_path: String::new(),
            logon_script_args: String::new(),
            remote_command_on: false,
            remote_command: String::new(),
            display_logon_prompts: false,
            sftp_start_dir: String::new(),
            compression: false,
            keepalive_secs: 30,
            agent_forward: false,
            x11: false,
            x11_trusted: false,
            x11_display: "localhost:0.0".into(),
            known_hosts_file: String::new(),
            identity_file: String::new(),
            pubkey_use_global: true,
            pubkey_agent: true,
            identities_only: false,
            key_passphrase: String::new(),
            saved_password: String::new(),
            gssapi_delegate: false,
            gssapi_method: "Auto Detect".into(),
            gssapi_delegation: "Full".into(),
            gssapi_spn: String::new(),
            kex: alg_list(KEX_DEFAULT),
            ciphers: alg_list(CIPHERS_DEFAULT),
            macs: alg_list(MACS_DEFAULT),
            hostkey_algos: alg_list(HOSTKEY_DEFAULT),
            strict_host_key: "ask".into(),
            ssh_verbose: false,
            forwards: Vec::new(),
            com_port: "COM1".into(),
            baud: 38400,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
            flow: FlowControl::XonXoff,
            shell_command: String::new(),
            emulation: "Xterm".into(),
            rows: 24,
            cols: 80,
            scrollback: 500,
            resize_retain: false,
            scale_font: false,
            xterm256: true,
            use_color_scheme: true,
            cua: true,
            close_on_disconnect: false,
            clear_on_disconnect: false,
            auto_reconnect: false,
            scroll_bottom_output: true,
            scroll_bottom_keypress: true,
            send_wheel_remote: false,
            anti_idle_string_on: false,
            anti_idle_string: String::new(),
            anti_idle_secs: 300,
            anti_idle_noop_on: false,
            anti_idle_noop_secs: 60,
            word_delims_on: false,
            word_delims: " ".into(),
            window_on_top: false,
            ansi_color: true,
            truecolor: false,
            app_cursor_keys: false,
            app_keypad: false,
            origin_mode: false,
            insert_mode: false,
            cursor_switching: true,
            local_echo: false,
            newline_mode: false,
            meta_8bit: false,
            delete_sends_del: false,
            home_end_tilde: true,
            answerback: String::new(),
            line_wrap: true,
            meta_key: false,
            backspace_bs: false,
            enter_sends: "CR".into(),
            mapped_keys: Vec::new(),
            audible_bell: true,
            visual_bell: true,
            color_scheme: SYSTEM_SCHEME.into(),
            font_size: 13.0,
            cursor_style: CursorStyle::Block,
            blinking_cursor: true,
            encoding: "UTF-8".into(),
            margin: 4.0,
            show_scrollbar: true,
            tab_title: String::new(),
            keyword_highlighting: true,
            highlight_style: "Color".into(),
            highlight_color: [0xff, 0xe0, 0x66],
            highlight_case: false,
            highlight_line: false,
            tab_color_on: false,
            tab_color: [0x00, 0x78, 0xd4],
            log_file: "%S_%Y-%M-%D_%h%m%s.log".into(),
            start_log_on_connect: false,
            log_append: true,
            log_timestamp: false,
            log_prompt: false,
            log_raw: false,
            log_midnight: false,
            log_custom_connect: String::new(),
            log_custom_disconnect: String::new(),
            log_custom_line: String::new(),
            log_only_custom: false,
            sftp_local_dir: std::env::var("USERPROFILE").map(|p| format!("{p}\\Documents")).unwrap_or_default(),
            compression_level: 5,
            print_header: String::new(),
            print_footer: String::new(),
            upload_dir: String::new(),
            xmodem_1k: false,
            download_dir: String::new(),
        }
    }
}

impl SessionConfig {
    pub fn display_name(&self) -> String {
        if !self.name.trim().is_empty() {
            return self.name.clone();
        }
        match self.protocol {
            Protocol::Serial => format!("Serial-{}", self.com_port),
            Protocol::LocalShell => "Local Shell".into(),
            _ => {
                if self.username.is_empty() {
                    self.host.clone()
                } else {
                    format!("{}@{}", self.username, self.host)
                }
            }
        }
    }

    pub fn summary(&self) -> String {
        match self.protocol {
            Protocol::Serial => format!(
                "Serial {} {} {}{}{}",
                self.com_port,
                self.baud,
                self.data_bits,
                match self.parity {
                    Parity::None => "N",
                    Parity::Odd => "O",
                    Parity::Even => "E",
                },
                self.stop_bits
            ),
            Protocol::LocalShell => "Local Shell".into(),
            Protocol::Sftp => format!("SFTP {}:{}", self.host, self.port),
            p => format!("{} {}:{}", p.label(), self.host, self.port),
        }
    }

    /// Command-line words that open this connection in a new window.
    pub fn cli_args(&self) -> Vec<String> {
        match self.protocol {
            Protocol::Serial => vec!["serial".into(), self.com_port.clone(), self.baud.to_string()],
            Protocol::LocalShell => vec!["local".into()],
            p => {
                let word = match p {
                    Protocol::Sftp => "sftp",
                    Protocol::Telnet => "telnet",
                    Protocol::TelnetTls => "telnets",
                    Protocol::Rlogin => "rlogin",
                    Protocol::Raw => "raw",
                    _ => "ssh",
                };
                let target = if self.username.is_empty() {
                    format!("{}:{}", self.host, self.port)
                } else {
                    format!("{}@{}:{}", self.username, self.host, self.port)
                };
                vec![word.into(), target]
            }
        }
    }
}

/// A command of the Command Manager (function "Send String").
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandDef {
    pub folder: String,
    pub label: String,
    /// Text sent to the session; `\r` means Enter.
    pub text: String,
    pub color: String,
    pub description: String,
    pub disable_tooltip: bool,
}

impl Default for CommandDef {
    fn default() -> Self {
        Self { folder: String::new(), label: String::new(), text: String::new(), color: "Green".into(), description: String::new(), disable_tooltip: false }
    }
}

pub const COMMAND_COLORS: [(&str, [u8; 3]); 8] = [
    ("Green", [0x16, 0xc6, 0x0c]),
    ("Red", [0xe8, 0x11, 0x23]),
    ("Blue", [0x00, 0x78, 0xd4]),
    ("Yellow", [0xff, 0xf1, 0x00]),
    ("Orange", [0xf7, 0x63, 0x0c]),
    ("Purple", [0x88, 0x17, 0x98]),
    ("Gray", [0x76, 0x76, 0x76]),
    ("Black", [0x20, 0x20, 0x20]),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ButtonDef {
    pub label: String,
    /// Text sent to the session; `\r` means Enter.
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub sessions: Vec<SessionConfig>,
    pub folders: Vec<String>,
    pub default_session: SessionConfig,
    pub default_serial: SessionConfig,
    pub default_local: SessionConfig,
    pub buttons: Vec<ButtonDef>,
    pub commands: Vec<CommandDef>,
    pub command_folders: Vec<String>,
    pub cmd_title_bar: bool,
    pub cmd_send_immediately: bool,
    pub keywords: Vec<String>,
    pub keyword_highlighting: bool,
    pub theme: String,
    pub show_menu_bar: bool,
    pub show_toolbar: bool,
    pub show_tabs: bool,
    pub show_command_window: bool,
    pub show_button_bar: bool,
    pub show_status_bar: bool,
    pub show_connect_bar: bool,
    pub show_hex_view: bool,
    pub paste_confirm: bool,
    /// Session Manager folders that are open (kept between runs, as the original does)
    pub tree_open: Vec<String>,
    /// Command Manager folders that are closed
    pub cmd_tree_closed: Vec<String>,
    pub ansi_palette: Vec<[u8; 3]>,
    pub tab_close_buttons: bool,
    pub paste_delay_on: bool,
    pub paste_delay_ms: u32,
    pub show_tab_sm: bool,
    pub show_command_manager: bool,
    pub show_tab_as: bool,
    pub show_descriptions: bool,
    pub quick_connect_at_start: bool,
    pub auto_save: bool,
    pub use_firewall: bool,
    pub copy_on_select: bool,
    pub recent: Vec<String>,
    pub recent_scripts: Vec<String>,
    pub snapshots: Vec<(String, Vec<SessionConfig>)>,
    pub global_identity: String,
    pub credentials: Vec<Credential>,
    /// Global Options values that have no older field (see globaldlg::GlobalOpts).
    pub global: crate::globaldlg::GlobalOpts,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Credential {
    pub name: String,
    pub user: String,
    pub pass: String,
}

impl Default for Settings {
    fn default() -> Self {
        let serial = SessionConfig { protocol: Protocol::Serial, port: 0, ..Default::default() };
        let local = SessionConfig { protocol: Protocol::LocalShell, port: 0, ..Default::default() };
        Self {
            sessions: Vec::new(),
            folders: Vec::new(),
            default_session: SessionConfig::default(),
            default_serial: serial,
            default_local: local,
            commands: Vec::new(),
            command_folders: Vec::new(),
            cmd_title_bar: true,
            cmd_send_immediately: false,
            buttons: vec![
                ButtonDef { label: "Clear".into(), text: "clear\\r".into() },
                ButtonDef { label: "Who am I".into(), text: "whoami\\r".into() },
            ],
            keywords: vec!["error".into(), "warning".into()],
            keyword_highlighting: true,
            theme: "VS Dark".into(),
            show_menu_bar: true,
            show_toolbar: true,
            show_tabs: true,
            show_command_window: false,
            show_button_bar: false,
            show_status_bar: true,
            show_connect_bar: true,
            show_hex_view: false,
            paste_confirm: true,
            tree_open: Vec::new(),
            cmd_tree_closed: Vec::new(),
            ansi_palette: Vec::new(),
            tab_close_buttons: true,
            paste_delay_on: false,
            paste_delay_ms: 5,
            show_tab_sm: true,
            show_command_manager: false,
            show_tab_as: true,
            show_descriptions: true,
            quick_connect_at_start: true,
            auto_save: true,
            use_firewall: true,
            copy_on_select: false,
            recent: Vec::new(),
            recent_scripts: Vec::new(),
            snapshots: Vec::new(),
            global_identity: String::new(),
            credentials: Vec::new(),
            global: crate::globaldlg::GlobalOpts::default(),
        }
    }
}

impl Settings {
    /// New-session defaults for a protocol (Default Session, Serial Defaults, Local Shell Defaults).
    pub fn defaults_for(&self, p: Protocol) -> SessionConfig {
        let mut c = match p {
            Protocol::Serial => self.default_serial.clone(),
            Protocol::LocalShell => self.default_local.clone(),
            _ => self.default_session.clone(),
        };
        c.protocol = p;
        c.port = p.default_port();
        c
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Yetty");
    // The program was called NexTerm before: its settings folder is moved over once.
    let old = base.join("NexTerm");
    if !dir.exists() && old.is_dir() {
        let _ = std::fs::rename(&old, &dir);
    }
    dir
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        let mut st: Settings = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // The old default font size (14) gave rows taller than the original's; 13 matches it.
        for c in st.sessions.iter_mut().chain([&mut st.default_session, &mut st.default_serial, &mut st.default_local]) {
            if (c.font_size - 14.0).abs() < 0.01 {
                c.font_size = 13.0;
            }
        }
        st
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(settings_path(), s);
        }
    }
}

/// Turn the `\r`, `\n`, `\t`, `\e` and `\\` escapes of a button text into characters.
pub fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('r') => out.push('\r'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('e') => out.push('\x1b'),
                Some('b') => out.push('\x08'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Local time and log file names (original log-name variables: %H %S %Y %M %D %h %m %s %t)

#[cfg(windows)]
mod sys {
    #[repr(C)]
    #[derive(Default)]
    pub struct SystemTime {
        pub year: u16,
        pub month: u16,
        pub dow: u16,
        pub day: u16,
        pub hour: u16,
        pub minute: u16,
        pub second: u16,
        pub millis: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetLocalTime(t: *mut SystemTime);
    }
    #[link(name = "user32")]
    extern "system" {
        pub fn GetKeyState(vk: i32) -> i16;
        pub fn MessageBeep(kind: u32) -> i32;
    }
}

pub struct Now {
    pub y: u32,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
    pub ms: u32,
}

pub fn now() -> Now {
    #[cfg(windows)]
    unsafe {
        let mut t = sys::SystemTime::default();
        sys::GetLocalTime(&mut t);
        return Now {
            y: t.year as u32,
            mo: t.month as u32,
            d: t.day as u32,
            h: t.hour as u32,
            mi: t.minute as u32,
            s: t.second as u32,
            ms: t.millis as u32,
        };
    }
    #[allow(unreachable_code)]
    {
        let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = d.as_secs();
        let days = (secs / 86400) as i64;
        let rem = secs % 86400;
        let z = days + 719468;
        let era = z.div_euclid(146097);
        let doe = z.rem_euclid(146097);
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let dd = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = if m <= 2 { y + 1 } else { y };
        Now { y: y as u32, mo: m, d: dd, h: (rem / 3600) as u32, mi: ((rem % 3600) / 60) as u32, s: (rem % 60) as u32, ms: d.subsec_millis() }
    }
}

/// Caps Lock / Num Lock state for the status bar.
pub fn lock_keys() -> (bool, bool) {
    #[cfg(windows)]
    unsafe {
        return (sys::GetKeyState(0x14) & 1 != 0, sys::GetKeyState(0x90) & 1 != 0);
    }
    #[allow(unreachable_code)]
    (false, false)
}

/// Private-key files found in the user's .ssh folder.
pub fn detected_keys() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(dir) = std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join(".ssh")) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !p.is_file() || name.ends_with(".pub") || ["known_hosts", "known_hosts.old", "config", "authorized_keys"].contains(&name.as_str()) {
                    continue;
                }
                if let Ok(head) = std::fs::read_to_string(&p).map(|t| t.chars().take(60).collect::<String>()) {
                    if head.contains("PRIVATE KEY") || head.contains("PuTTY-User-Key") {
                        out.push(p.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// The system's "Open file" dialog (called directly: it opens at once).
pub fn browse_file(title: &str, start_dir: Option<&str>) -> Option<String> {
    crate::nativedlg::open_file(title, start_dir)
}

#[allow(dead_code)]
fn browse_file_slow(title: &str, start_dir: Option<&str>) -> Option<String> {
    let dir = start_dir.map(|d| d.replace('\'', "''")).unwrap_or_default();
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Title = '{}'; if ('{}' -ne '') {{ $d.InitialDirectory = '{}' }}; $d.Filter = 'All files (*.*)|*.*'; if ($d.ShowDialog() -eq 'OK') {{ [Console]::Out.Write($d.FileName) }}",
        title.replace('\'', "''"),
        dir,
        dir
    );
    let out = std::process::Command::new("powershell.exe").args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Run a command line in a new console window (it stays open when the command ends).
pub fn open_console(cmdline: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd.exe").raw_arg(format!("/C start \"Yetty\" cmd.exe /K {cmdline}")).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = cmdline;
    }
}

fn powershell_pick(script: &str) -> Option<String> {
    let out = std::process::Command::new("powershell.exe").args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", script]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Folder chooser.
pub fn browse_folder(title: &str) -> Option<String> {
    crate::nativedlg::pick_folder(title)
}

#[allow(dead_code)]
fn browse_folder_slow(title: &str) -> Option<String> {
    powershell_pick(&format!(
        "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = '{}'; if ($d.ShowDialog() -eq 'OK') {{ [Console]::Out.Write($d.SelectedPath) }}",
        title.replace('\'', "''")
    ))
}

/// "Save as" chooser.
pub fn browse_save(title: &str) -> Option<String> {
    crate::nativedlg::save_file(title)
}

#[allow(dead_code)]
fn browse_save_slow(title: &str) -> Option<String> {
    powershell_pick(&format!(
        "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.SaveFileDialog; $d.Title = '{}'; $d.OverwritePrompt = $false; if ($d.ShowDialog() -eq 'OK') {{ [Console]::Out.Write($d.FileName) }}",
        title.replace('\'', "''")
    ))
}

pub fn ssh_dir() -> String {
    std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join(".ssh").to_string_lossy().into_owned()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Saved passwords are encrypted with the Windows user's key (DPAPI), as "(Password will be encrypted.)"

#[cfg(windows)]
mod dpapi {
    use std::ffi::c_void;
    #[repr(C)]
    pub struct Blob {
        pub cb: u32,
        pub pb: *mut u8,
    }
    #[link(name = "crypt32")]
    extern "system" {
        pub fn CryptProtectData(pin: *const Blob, desc: *const u16, entropy: *const Blob, reserved: *mut c_void, prompt: *mut c_void, flags: u32, out: *mut Blob) -> i32;
        pub fn CryptUnprotectData(pin: *const Blob, desc: *mut *mut u16, entropy: *const Blob, reserved: *mut c_void, prompt: *mut c_void, flags: u32, out: *mut Blob) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn LocalFree(p: *mut c_void) -> *mut c_void;
    }
}

const ENC_PREFIX: &str = "dpapi:";

pub fn is_protected(s: &str) -> bool {
    s.starts_with(ENC_PREFIX)
}

/// Encrypt a password for storing in the settings file.
pub fn protect(plain: &str) -> String {
    if plain.is_empty() || is_protected(plain) {
        return plain.to_string();
    }
    #[cfg(windows)]
    unsafe {
        let mut data = plain.as_bytes().to_vec();
        let input = dpapi::Blob { cb: data.len() as u32, pb: data.as_mut_ptr() };
        let mut out = dpapi::Blob { cb: 0, pb: std::ptr::null_mut() };
        if dpapi::CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut out) != 0 && !out.pb.is_null() {
            let bytes = std::slice::from_raw_parts(out.pb, out.cb as usize).to_vec();
            dpapi::LocalFree(out.pb as *mut _);
            return format!("{ENC_PREFIX}{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
    }
    plain.to_string()
}

/// The password in clear text (text that was never encrypted is returned as it is).
pub fn unprotect(stored: &str) -> String {
    let Some(hex) = stored.strip_prefix(ENC_PREFIX) else { return stored.to_string() };
    let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect();
    #[cfg(windows)]
    unsafe {
        let mut data = bytes;
        let input = dpapi::Blob { cb: data.len() as u32, pb: data.as_mut_ptr() };
        let mut out = dpapi::Blob { cb: 0, pb: std::ptr::null_mut() };
        if dpapi::CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut out) != 0 && !out.pb.is_null() {
            let plain = String::from_utf8_lossy(std::slice::from_raw_parts(out.pb, out.cb as usize)).into_owned();
            dpapi::LocalFree(out.pb as *mut _);
            return plain;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn password_roundtrip() {
        let enc = super::protect("s3cret é");
        #[cfg(windows)]
        assert!(super::is_protected(&enc) && !enc.contains("s3cret"));
        assert_eq!(super::unprotect(&enc), "s3cret é");
    }
}

/// Make the main window see-through (alpha 255 = opaque). Windows only.
pub fn set_window_opacity(alpha: u8) {
    #[cfg(windows)]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn GetActiveWindow() -> isize;
            fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
            fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
            fn SetLayeredWindowAttributes(hwnd: isize, key: u32, alpha: u8, flags: u32) -> i32;
        }
        let hwnd = GetActiveWindow();
        if hwnd != 0 {
            let ex = GetWindowLongPtrW(hwnd, -20);
            SetWindowLongPtrW(hwnd, -20, ex | 0x0008_0000); // WS_EX_LAYERED
            SetLayeredWindowAttributes(hwnd, 0, alpha, 2); // LWA_ALPHA
        }
    }
    #[cfg(not(windows))]
    {
        let _ = alpha;
    }
}

pub fn beep() {
    #[cfg(windows)]
    unsafe {
        sys::MessageBeep(0);
    }
}

pub fn expand_log_name(pattern: &str, host: &str, session: &str) -> String {
    let n = now();
    let mut out = String::new();
    let mut it = pattern.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('H') => out.push_str(host),
            Some('S') => out.push_str(session),
            Some('Y') => out.push_str(&format!("{:04}", n.y)),
            Some('M') => out.push_str(&format!("{:02}", n.mo)),
            Some('D') => out.push_str(&format!("{:02}", n.d)),
            Some('h') => out.push_str(&format!("{:02}", n.h)),
            Some('m') => out.push_str(&format!("{:02}", n.mi)),
            Some('s') => out.push_str(&format!("{:02}", n.s)),
            Some('t') => out.push_str(&format!("{:03}", n.ms)),
            Some('%') => out.push('%'),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out.chars().map(|c| if "<>:\"|?*".contains(c) { '_' } else { c }).collect()
}

// ---------------------------------------------------------------------------
// Colours

pub type Rgb = [u8; 3];

/// Default ANSI palette: what SecureCRT draws with its "Standard" palette under the System Color Scheme
/// (measured from the original's window on 2026-11-19, SGR 40-47 / 100-107 in a Local Shell and over SSH).
pub const ANSI_PALETTE: [Rgb; 16] = [
    [0x00, 0x00, 0x00],
    [0xff, 0x55, 0x55],
    [0x55, 0xff, 0x55],
    [0xff, 0xff, 0x55],
    [0x55, 0x55, 0xff],
    [0xff, 0x55, 0xff],
    [0x55, 0xff, 0xff],
    [0xbb, 0xbb, 0xbb],
    [0x55, 0x55, 0x55],
    [0xff, 0x55, 0x55],
    [0x55, 0xff, 0x55],
    [0xff, 0xff, 0x55],
    [0x55, 0x55, 0xff],
    [0xff, 0x55, 0xff],
    [0x55, 0xff, 0xff],
    [0xff, 0xff, 0xff],
];

const fn hex(v: u32) -> Rgb {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

pub const SYSTEM_SCHEME: &str = "System Color Scheme";

/// Built-in colour schemes (function 0x14023b640, tables from 0x140437ab0): (name, text, background).
/// "System Color Scheme" follows the system window colours (black, as the
/// original's dark window looks).
pub const SCHEMES: &[(&str, Rgb, Rgb)] = &[
    (SYSTEM_SCHEME, hex(0xffffff), hex(0x000000)),
    ("Birds of Paradise", hex(0xe0dbb7), hex(0x2a1f1d)),
    ("Black / Floral White", hex(0x000000), hex(0xfffbf0)),
    ("Chalkboard", hex(0xd9e6f2), hex(0x29262f)),
    ("Ciapre", hex(0xaea47a), hex(0x191c27)),
    ("Dark Pastel", hex(0xffffff), hex(0x000000)),
    ("Desert", hex(0xffffff), hex(0x333333)),
    ("Espresso", hex(0xffffff), hex(0x323232)),
    ("Floral White / Dark Cyan", hex(0xfffbf0), hex(0x008080)),
    ("Idletoes", hex(0xffffff), hex(0x323232)),
    ("Invisibone", hex(0xa0a0a0), hex(0x232323)),
    ("Monochrome", hex(0x000000), hex(0xffffff)),
    ("Novel", hex(0x3b2322), hex(0xdfdbc3)),
    ("Solarized Dark", hex(0x839496), hex(0x002b36)),
    ("Solarized Darcula", hex(0xd2d8d9), hex(0x3d3f41)),
    ("Solarized Light", hex(0x657b83), hex(0xfdf6e3)),
    ("Tomorrow", hex(0x4d4d4c), hex(0xfafafa)),
    ("Traditional", hex(0x00ff00), hex(0x000000)),
    ("White / Black", hex(0xffffff), hex(0x000000)),
    ("White / Blue", hex(0xffffff), hex(0x0000ff)),
    ("Yellow / Black", hex(0xffff00), hex(0x000000)),
    ("Zenburn", hex(0xdcdccc), hex(0x3f3f3f)),
];

pub fn scheme(name: &str) -> (Rgb, Rgb) {
    SCHEMES
        .iter()
        .find(|s| s.0 == name)
        .map(|s| (s.1, s.2))
        .unwrap_or((SCHEMES[0].1, SCHEMES[0].2))
}

/// xterm 256-colour palette entry.
pub fn xterm256(i: u8) -> Rgb {
    if i < 16 {
        return ansi_palette()[i as usize];
    }
    if i >= 232 {
        let v = 8 + (i - 232) * 10;
        return [v, v, v];
    }
    let i = i - 16;
    let conv = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
    [conv(i / 36), conv((i / 6) % 6), conv(i % 6)]
}

/// Nearest xterm-256 cube colour (used when Xterm-Truecolor is off).
pub fn nearest256(r: u8, g: u8, b: u8) -> Rgb {
    let q = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16 - 35) / 40).min(5) as u8
        }
    };
    xterm256(16 + 36 * q(r) + 6 * q(g) + q(b))
}

static PALETTE: std::sync::RwLock<Option<[Rgb; 16]>> = std::sync::RwLock::new(None);

/// The 16 ANSI colours in use (Global Options > ANSI Color); the original's defaults until changed.
pub fn ansi_palette() -> [Rgb; 16] {
    PALETTE.read().ok().and_then(|p| *p).unwrap_or(ANSI_PALETTE)
}

pub fn set_ansi_palette(p: [Rgb; 16]) {
    if let Ok(mut w) = PALETTE.write() {
        *w = if p == ANSI_PALETTE { None } else { Some(p) };
    }
}
