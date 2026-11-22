//! Main window: menus, toolbar, side panels, tabs/tiles, command window, button bar, status bar.

use crate::config::{self, ButtonDef, Protocol, SessionConfig, Settings};
use crate::dialogs::{self, EditTarget, OptOut, OptionsDlg, QuickDlg, QuickOut};
use crate::globaldlg;
use crate::keymapdlg;
use crate::term::{self, Session, Status, ViewOptions};
use egui::{Color32, Pos2, RichText, Sense};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
enum Side {
    SessionManager,
    ActiveSessions,
    CommandManager,
}

#[derive(Clone, Copy, PartialEq)]
enum Layout {
    Tabs,
    TileV,
    TileH,
    Cascade,
}

#[derive(Default)]
struct UnlockDlg {
    pass: String,
    err: String,
}

#[derive(Default)]
struct PassDlg {
    old: String,
    new: String,
    confirm: String,
    err: String,
}

#[derive(Clone, PartialEq)]
enum Prompt {
    SendAscii,
    SendBinary,
    ReceiveAscii,
    LogFile,
    SaveScript,
    ExportSettings,
    ImportSettings,
    ExportSessions,
    NewFolder,
    CmdFolder,
    RenameCommand(usize),
    DuplicateSession(usize),
    RenameCmdFolder(String),
    RenameSession(usize),
    RenameFolder(String),
    SnapshotName,
    RenameTab(usize),
    /// Tab menu > Save Session...: store the tab's session in the Session Manager under this name.
    SaveSession(u64),
    LockPassword,
    XferSend(crate::transfer::Proto),
    XferRecv(crate::transfer::Proto),
    ConvertKey,
    NewButtonBar,
    TftpFolder,
    CredPassword(usize),
}

struct PromptDlg {
    kind: Prompt,
    title: &'static str,
    label: &'static str,
    text: String,
}

#[derive(Default)]
struct FindState {
    open: bool,
    query: String,
    case_sensitive: bool,
    message: String,
}

struct ButtonEditor {
    index: Option<usize>,
    def: ButtonDef,
}

struct ScriptRun {
    sid: u64,
    name: String,
    lines: Vec<String>,
    pc: usize,
    sleep_until: Option<Instant>,
    waiting: Option<(String, Instant)>,
}

/// What is dragged in the session tree.
#[derive(Clone)]
pub enum DragItem {
    Session(usize),
    Folder(String),
}

#[derive(Clone)]
enum TreeAct {
    Connect(usize),
    /// Connect in New Tab Group: the new tab starts a tab group of its own
    ConnectGroup(usize),
    ConnectSftp(usize),
    NewWindow(usize),
    Cut,
    Copy,
    CopyPath,
    Duplicate,
    Paste,
    Find,
    FindNext,
    Delete,
    Rename,
    Shortcut,
    ExpandAll,
    CollapseAll,
    Refresh,
    Sort(bool),
    ToggleDescriptions,
    Export,
    Properties,
    NewFolder,
    Move(DragItem, String),
}

struct Node {
    name: String,
    path: String,
    children: Vec<Node>,
    sessions: Vec<usize>,
}

impl Node {
    fn child(&mut self, name: &str) -> &mut Node {
        if let Some(i) = self.children.iter().position(|c| c.name == name) {
            return &mut self.children[i];
        }
        let path = if self.path.is_empty() { name.to_string() } else { format!("{}/{}", self.path, name) };
        self.children.push(Node { name: name.to_string(), path, children: Vec::new(), sessions: Vec::new() });
        self.children.last_mut().unwrap()
    }
}

struct TreeCtx<'a> {
    sessions: &'a [SessionConfig],
    selected: Option<usize>,
    selected_folder: Option<String>,
    sel_default: Option<usize>,
    act: Option<TreeAct>,
    expand: Option<bool>,
    show_desc: bool,
    has_clip: bool,
    filtering: bool,
    /// the folders that are open (the root is open unless "<root-closed>" is listed); kept in the settings
    open_list: Vec<String>,
    open_changed: bool,
}

fn tree_set_open(list: &mut Vec<String>, path: &str, is_root: bool, open: bool) {
    let key = if is_root { "<root-closed>" } else { path };
    list.retain(|x| x != key);
    if open != is_root {
        // a folder is listed when open; the root when closed
        list.push(key.to_string());
    }
}

fn build_tree(sessions: &[SessionConfig], folders: &[String], filter: &str, order: u8) -> Node {
    let mut root = Node { name: "Sessions".into(), path: String::new(), children: Vec::new(), sessions: Vec::new() };
    let f = filter.to_lowercase();
    if f.is_empty() {
        for p in folders {
            let mut n = &mut root;
            for part in p.split('/').filter(|s| !s.is_empty()) {
                n = n.child(part);
            }
        }
    }
    for (i, s) in sessions.iter().enumerate() {
        if !f.is_empty() && !(s.display_name().to_lowercase().contains(&f) || s.folder.to_lowercase().contains(&f) || s.host.to_lowercase().contains(&f)) {
            continue;
        }
        let mut n = &mut root;
        for part in s.folder.split('/').filter(|s| !s.is_empty()) {
            n = n.child(part);
        }
        n.sessions.push(i);
    }
    if order != 2 {
        sort_tree(&mut root, sessions, order == 1);
    }
    root
}

/// Natural order: runs of digits compare by value ("host2" before "host10"), the rest without regard to case.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                let mut nb = String::new();
                while let Some(c) = ai.next_if(|c| c.is_ascii_digit()) {
                    na.push(c);
                }
                while let Some(c) = bi.next_if(|c| c.is_ascii_digit()) {
                    nb.push(c);
                }
                let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Global Options "Session list sort order": folders, then sessions, each by name (Alphabetically: without
/// regard to case; Logically: with numbers by value).
fn sort_tree(n: &mut Node, sessions: &[SessionConfig], natural: bool) {
    if natural {
        n.children.sort_by(|a, b| natural_cmp(&a.name, &b.name));
        n.sessions.sort_by(|&a, &b| natural_cmp(&sessions[a].display_name(), &sessions[b].display_name()));
    } else {
        n.children.sort_by_key(|c| c.name.to_lowercase());
        n.sessions.sort_by_key(|&i| sessions[i].display_name().to_lowercase());
    }
    for c in &mut n.children {
        sort_tree(c, sessions, natural);
    }
}

fn draw_node(ui: &mut egui::Ui, node: &Node, c: &mut TreeCtx, is_root: bool) {
    let id = egui::Id::new(("tree", &node.path));
    let title = {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let mut job = egui::text::LayoutJob::default();
        job.append("\u{1F4C1}", 0.0, egui::text::TextFormat { font_id: egui::FontId::new(14.0, egui::FontFamily::Name("symbols".into())), color: Color32::from_rgb(0xff, 0xc8, 0x3d), ..Default::default() });
        job.append(" ", 0.0, egui::text::TextFormat { font_id: font.clone(), color: Color32::PLACEHOLDER, ..Default::default() });
        job.append(&node.name, 0.0, egui::text::TextFormat { font_id: font, color: Color32::PLACEHOLDER, ..Default::default() });
        job
    };
    let want = if c.filtering {
        true
    } else if is_root {
        !c.open_list.iter().any(|x| x == "<root-closed>")
    } else {
        c.open_list.iter().any(|x| x == &node.path)
    };
    let mut header = egui::CollapsingHeader::new(title).id_salt(id).open(Some(want));
    if let Some(e) = c.expand {
        let o = e || is_root;
        tree_set_open(&mut c.open_list, &node.path, is_root, o);
        c.open_changed = true;
        header = header.open(Some(o));
    }
    let r = header.show(ui, |ui| {
        for ch in &node.children {
            draw_node(ui, ch, c, false);
        }
        for &i in &node.sessions {
            let s = &c.sessions[i];
            let sel = c.selected == Some(i);
            let label = {
                let font = egui::TextStyle::Body.resolve(ui.style());
                let mut job = egui::text::LayoutJob::default();
                job.append("\u{E756} ", 0.0, egui::text::TextFormat { font_id: font.clone(), color: Color32::PLACEHOLDER, ..Default::default() });
                job.append(&s.display_name(), 0.0, egui::text::TextFormat { font_id: font, color: Color32::PLACEHOLDER, ..Default::default() });
                job
            };
            let r = ui.selectable_label(sel, label).interact(egui::Sense::click_and_drag());
            if r.dragged() {
                egui::DragAndDrop::set_payload(ui.ctx(), DragItem::Session(i));
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            if r.dnd_hover_payload::<DragItem>().is_some() {
                ui.painter().rect_stroke(r.rect, 2.0, egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)), egui::StrokeKind::Inside);
            }
            if let Some(p) = r.dnd_release_payload::<DragItem>() {
                // dropped on a session: it goes into that session's folder
                c.act = Some(TreeAct::Move((*p).clone(), s.folder.clone()));
            }
            if r.clicked() || r.secondary_clicked() {
                c.selected = Some(i);
                c.selected_folder = None;
                c.sel_default = None;
            }
            if r.double_clicked() {
                c.act = Some(TreeAct::Connect(i));
            }
            let r = if c.show_desc {
                let tip = if s.description.trim().is_empty() { format!("Hostname: {}", s.host) } else { format!("{}\nHostname: {}", s.description, s.host) };
                r.on_hover_text(tip)
            } else {
                r
            };
            r.context_menu(|ui| session_menu(ui, c, i));
        }
    });
    if !c.filtering && r.header_response.clicked() {
        tree_set_open(&mut c.open_list, &node.path, is_root, !want);
        c.open_changed = true;
    }
    let hr = r.header_response.interact(egui::Sense::click_and_drag());
    if !is_root && hr.dragged() {
        egui::DragAndDrop::set_payload(ui.ctx(), DragItem::Folder(node.path.clone()));
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if hr.dnd_hover_payload::<DragItem>().is_some() {
        ui.painter().rect_stroke(hr.rect, 2.0, egui::Stroke::new(1.5, Color32::from_rgb(0x00, 0x78, 0xd4)), egui::StrokeKind::Inside);
    }
    if let Some(p) = hr.dnd_release_payload::<DragItem>() {
        c.act = Some(TreeAct::Move((*p).clone(), node.path.clone()));
    }
    if hr.clicked() || hr.secondary_clicked() {
        c.selected_folder = Some(node.path.clone());
        c.selected = None;
    }
    hr.context_menu(|ui| {
        if ui.button("New Folder...").clicked() {
            c.selected_folder = Some(node.path.clone());
            c.act = Some(TreeAct::NewFolder);
            ui.close();
        }
        if ui.add_enabled(c.has_clip, egui::Button::new("Paste")).clicked() {
            c.selected_folder = Some(node.path.clone());
            c.act = Some(TreeAct::Paste);
            ui.close();
        }
        if !is_root && ui.button("Rename").clicked() {
            c.selected_folder = Some(node.path.clone());
            c.act = Some(TreeAct::Rename);
            ui.close();
        }
        if !is_root && ui.button("Delete").clicked() {
            c.selected_folder = Some(node.path.clone());
            c.act = Some(TreeAct::Delete);
            ui.close();
        }
        ui.separator();
        if ui.button("Expand All Folders").clicked() {
            c.act = Some(TreeAct::ExpandAll);
            ui.close();
        }
        if ui.button("Collapse All Folders").clicked() {
            c.act = Some(TreeAct::CollapseAll);
            ui.close();
        }
    });
}

/// The original's menu icons (the same pictures as its toolbar), by menu item text.
fn menu_icon(text: &str) -> &'static str {
    match text {
        "Connect..." => "\u{E839}",
        "Quick Connect..." => "\u{E945}",
        "Connect in Tab/Tile..." | "Connect In Tab/Tile..." => "\u{E8A7}",
        "Connect Local Shell" => "\u{E756}",
        "Open Scratchpad" => "\u{E70B}",
        "Reconnect" => "\u{E8EE}",
        "Disconnect" => "\u{E8CD}",
        "Copy" => "\u{E8C8}",
        "Paste" => "\u{E77F}",
        "Find..." => "\u{E721}",
        "Print Screen" | "Print Selection" => "\u{E749}",
        "Session Options..." => "\u{E713}",
        "Keymap Editor..." => "\u{E765}",
        "Public-Key Assistant..." => "\u{E8D7}",
        _ => "",
    }
}

/// A menu item: icon column (blank when the item has no picture), text and shortcut, as in the original's menus.
fn mi(ui: &mut egui::Ui, text: &str, sc: &str, enabled: bool) -> bool {
    mi_with_icon(ui, menu_icon(text.trim_start_matches(['✓', ' '])), text, sc, enabled)
}

/// A menu item with an empty icon column: the original's tab menu has no pictures.
fn mi_plain(ui: &mut egui::Ui, text: &str, enabled: bool) -> bool {
    mi_with_icon(ui, "", text, "", enabled)
}

fn mi_with_icon(ui: &mut egui::Ui, icon: &str, text: &str, sc: &str, enabled: bool) -> bool {
    use egui::AtomExt;
    let icon = RichText::new(icon).size(12.5).atom_size(egui::vec2(18.0, 14.0));
    let (job, mnemonic) = mnemonic_job(ui, text, enabled, true);
    let mut b = match job {
        Some(job) => egui::Button::new((icon, egui::WidgetText::from(job))),
        None => egui::Button::new((icon, text)),
    };
    if !sc.is_empty() {
        b = b.shortcut_text(sc);
    }
    let mut hit = ui.add_enabled(enabled, b).clicked();
    // typing the underlined letter while the menu is open chooses the item
    if enabled && !hit {
        if let Some(letter) = mnemonic {
            if mnemonic_pressed(ui, letter) {
                ui.close();
                hit = true;
            }
        }
    }
    hit
}

/// The caption as a text with its mnemonic letter underlined (None when the caption has no mnemonic); the letter too.
fn mnemonic_job(ui: &egui::Ui, text: &str, enabled: bool, show: bool) -> (Option<egui::text::LayoutJob>, Option<char>) {
    let rest = text.trim_start_matches(['✓', ' ']);
    let lead = &text[..text.len() - rest.len()];
    let Some((idx, letter)) = crate::mnemonics::lookup(rest) else { return (None, None) };
    if !show || !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("mnemonics_visible"))).unwrap_or(false) {
        return (None, Some(letter));
    }
    let color = if enabled { ui.visuals().text_color() } else { ui.visuals().widgets.noninteractive.fg_stroke.color };
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let plain = egui::text::TextFormat { font_id: font_id.clone(), color: Color32::PLACEHOLDER, ..Default::default() };
    let under = egui::text::TextFormat { font_id, color: Color32::PLACEHOLDER, underline: egui::Stroke::new(1.0, color), ..Default::default() };
    let chars: Vec<char> = rest.chars().collect();
    let mut job = egui::text::LayoutJob::default();
    let before: String = lead.chars().chain(chars[..idx.min(chars.len())].iter().copied()).collect();
    job.append(&before, 0.0, plain.clone());
    if let Some(c) = chars.get(idx) {
        job.append(&c.to_string(), 0.0, under);
        let after: String = chars[idx + 1..].iter().collect();
        job.append(&after, 0.0, plain);
    }
    (Some(job), Some(letter))
}

/// A menu bar title with its mnemonic letter underlined while `show`.
fn bar_title(ctx: &egui::Context, text: &str, show: bool) -> egui::WidgetText {
    let Some((idx, _)) = crate::mnemonics::lookup(text) else { return text.into() };
    if !show {
        return text.into();
    }
    let font_id = egui::TextStyle::Button.resolve(&ctx.global_style());
    let plain = egui::text::TextFormat { font_id: font_id.clone(), color: Color32::PLACEHOLDER, ..Default::default() };
    let under = egui::text::TextFormat { font_id, color: Color32::PLACEHOLDER, underline: egui::Stroke::new(1.0, ctx.global_style().visuals.text_color()), ..Default::default() };
    let chars: Vec<char> = text.chars().collect();
    let mut job = egui::text::LayoutJob::default();
    job.append(&chars[..idx].iter().collect::<String>(), 0.0, plain.clone());
    job.append(&chars[idx].to_string(), 0.0, under);
    job.append(&chars[idx + 1..].iter().collect::<String>(), 0.0, plain);
    job.into()
}

/// A submenu inside a menu, with the same empty icon column as the items so its text lines up with theirs.
fn sub_menu(ui: &mut egui::Ui, text: &str, add: impl FnOnce(&mut egui::Ui)) {
    use egui::AtomExt;
    use egui::containers::menu::{MenuState, SubMenu};
    let icon = RichText::new("").size(12.5).atom_size(egui::vec2(18.0, 14.0));
    let (job, mnemonic) = mnemonic_job(ui, text, true, true);
    let label: egui::WidgetText = match job {
        Some(job) => job.into(),
        None => text.into(),
    };
    let r = ui.menu_button((icon, label), add);
    // typing the underlined letter opens the submenu
    if let Some(letter) = mnemonic {
        if mnemonic_pressed(ui, letter) {
            let id = SubMenu::id_from_widget_id(r.response.id);
            MenuState::mark_shown(ui.ctx(), id);
            MenuState::from_ui(ui, |state, _| state.open_item = Some(id));
            ui.request_repaint();
        }
    }
}

/// True when the mnemonic letter was typed for this menu: not while one of its submenus is open (the letter then
/// belongs to the submenu). The key press is consumed.
fn mnemonic_pressed(ui: &egui::Ui, letter: char) -> bool {
    if egui::containers::menu::MenuState::from_ui(ui, |state, _| state.open_item.is_some()) {
        return false;
    }
    let Some(key) = egui::Key::from_name(&letter.to_ascii_uppercase().to_string()).or_else(|| egui::Key::from_name(&letter.to_string())) else { return false };
    let letter_s = letter.to_string();
    ui.input_mut(|i| {
        let p = i.consume_key(egui::Modifiers::NONE, key);
        if p {
            i.events.retain(|e| !matches!(e, egui::Event::Text(t) if t.eq_ignore_ascii_case(&letter_s)));
        }
        p
    })
}

/// A checkbox menu item with its mnemonic; the letter toggles it like a click. True when it changed.
fn mcheck(ui: &mut egui::Ui, value: &mut bool, text: &str, enabled: bool) -> bool {
    let (job, mnemonic) = mnemonic_job(ui, text, enabled, true);
    let label: egui::WidgetText = match job {
        Some(job) => job.into(),
        None => text.into(),
    };
    let mut hit = ui.add_enabled(enabled, egui::Checkbox::new(value, label)).clicked();
    if enabled && !hit {
        if let Some(letter) = mnemonic {
            if mnemonic_pressed(ui, letter) {
                *value = !*value;
                ui.close();
                hit = true;
            }
        }
    }
    hit
}

/// A radio menu item with its mnemonic; true when chosen (by click or by the letter).
fn mradio(ui: &mut egui::Ui, selected: bool, text: &str) -> bool {
    let (job, mnemonic) = mnemonic_job(ui, text, true, true);
    let label: egui::WidgetText = match job {
        Some(job) => job.into(),
        None => text.into(),
    };
    let mut hit = ui.add(egui::RadioButton::new(selected, label)).clicked();
    if !hit {
        if let Some(letter) = mnemonic {
            if mnemonic_pressed(ui, letter) {
                ui.close();
                hit = true;
            }
        }
    }
    hit
}

#[derive(Clone)]
enum CmdAct {
    Send(usize),
    Add,
    Edit(usize),
    Cut,
    Copy,
    Paste,
    Delete,
    NewFolder,
    Rename,
    ExpandAll,
    CollapseAll,
    Target(u8),
}

/// Start another Yetty window that connects like this session (Clone in New Window, Send to New Window).
fn spawn_window_for(cfg: &SessionConfig) {
    let Ok(exe) = std::env::current_exe() else { return };
    let mut args: Vec<String> = Vec::new();
    match cfg.protocol {
        Protocol::LocalShell => args.push("local".into()),
        Protocol::Serial => {
            args.push("serial".into());
            args.push(cfg.com_port.clone());
            args.push(cfg.baud.to_string());
        }
        p => {
            let word = match p {
                Protocol::Sftp => "sftp",
                Protocol::Telnet => "telnet",
                Protocol::TelnetTls => "telnets",
                Protocol::Rlogin => "rlogin",
                Protocol::Raw => "raw",
                _ => "ssh",
            };
            args.push(word.into());
            let user = if cfg.username.trim().is_empty() { String::new() } else { format!("{}@", cfg.username.trim()) };
            args.push(format!("{user}{}:{}", cfg.host.trim(), cfg.port));
        }
    }
    let _ = std::process::Command::new(exe).args(args).spawn();
}

fn cmd_icon_color(name: &str) -> Color32 {
    config::COMMAND_COLORS.iter().find(|c| c.0 == name).map(|c| Color32::from_rgb(c.1[0], c.1[1], c.1[2])).unwrap_or(Color32::from_rgb(0x16, 0xc6, 0x0c))
}

/// One folder of the Command Manager tree (folders first, then its commands).
fn cmd_tree_node(ui: &mut egui::Ui, st: &Settings, path: &str, filter: &str, sel: &mut Option<usize>, sel_folder: &mut Option<String>, act: &mut Option<CmdAct>, force: Option<bool>, can_paste: bool, target: u8, closed_change: &mut Vec<(String, bool)>) {
    let parent_of = |f: &str| f.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
    let mut folders: Vec<String> = Vec::new();
    let add_with_parents = |f: &str, list: &mut Vec<String>| {
        let mut cur = String::new();
        for part in f.split('/').filter(|s| !s.is_empty()) {
            if !cur.is_empty() {
                cur.push('/');
            }
            cur.push_str(part);
            if !list.contains(&cur) {
                list.push(cur.clone());
            }
        }
    };
    for f in &st.command_folders {
        add_with_parents(f, &mut folders);
    }
    for c in &st.commands {
        add_with_parents(&c.folder, &mut folders);
    }
    folders.sort_by_key(|f| f.to_lowercase());
    let matches = |c: &config::CommandDef| filter.is_empty() || c.label.to_lowercase().contains(filter) || c.folder.to_lowercase().contains(filter);
    let is_root = path.is_empty();
    let name = if is_root { "Commands".to_string() } else { path.rsplit('/').next().unwrap_or("").to_string() };
    let id = ui.make_persistent_id(("cmd_folder", path));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
    // open or closed as the person left it (kept in the settings)
    let key = if is_root { "<root>".to_string() } else { path.to_string() };
    let mut wanted = !st.cmd_tree_closed.contains(&key);
    if !filter.is_empty() {
        wanted = true;
    }
    if let Some(v) = force {
        wanted = v;
        closed_change.push((key.clone(), !v));
    }
    state.set_open(wanted);
    let header = state.show_header(ui, |ui| {
        let selected = sel_folder.as_deref() == Some(path) && sel.is_none();
        let r = ui.selectable_label(selected, format!("📁 {name}"));
        if r.clicked() || r.secondary_clicked() {
            *sel_folder = Some(path.to_string());
            *sel = None;
        }
        r.context_menu(|ui| {
            *sel_folder = Some(path.to_string());
            *sel = None;
            cmd_menu(ui, None, act, can_paste, target, is_root);
        });
    });
    if filter.is_empty() && force.is_none() && header.is_open() != wanted {
        closed_change.push((key.clone(), !header.is_open()));
    }
    header.body(|ui| {
        for f in folders.iter().filter(|f| parent_of(f) == path) {
            cmd_tree_node(ui, st, f, filter, sel, sel_folder, act, force, can_paste, target, closed_change);
        }
        for (i, c) in st.commands.iter().enumerate() {
            if c.folder.trim_matches('/') != path || !matches(c) {
                continue;
            }
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 5.0, cmd_icon_color(&c.color));
                let selected = *sel == Some(i);
                let mut r = ui.selectable_label(selected, &c.label);
                if !c.disable_tooltip {
                    let tip = if c.description.trim().is_empty() { c.text.clone() } else { c.description.clone() };
                    r = r.on_hover_text(tip);
                }
                if r.clicked() || r.secondary_clicked() {
                    *sel = Some(i);
                    *sel_folder = None;
                }
                if r.double_clicked() {
                    *act = Some(CmdAct::Send(i));
                }
                r.context_menu(|ui| {
                    *sel = Some(i);
                    *sel_folder = None;
                    cmd_menu(ui, Some(i), act, can_paste, target, false);
                });
            });
        }
    });
}

/// The right-click menu of the Command Manager (a command when `cmd` is given, else a folder).
fn cmd_menu(ui: &mut egui::Ui, cmd: Option<usize>, act: &mut Option<CmdAct>, can_paste: bool, target: u8, is_root: bool) {
    ui.set_min_width(250.0);
    let on_cmd = cmd.is_some();
    if mi(ui, "Send Command", "Enter", on_cmd) {
        *act = Some(CmdAct::Send(cmd.unwrap_or(0)));
        ui.close();
    }
    sub_menu(ui, "Send Command to", |ui| {
        ui.set_min_width(200.0);
        for (n, label) in [(0u8, "Active Session"), (1, "All Sessions"), (2, "Visible Sessions")] {
            if mi(ui, &format!("{}{label}", if target == n { "✓ " } else { "   " }), "", true) {
                *act = Some(CmdAct::Target(n));
                ui.close();
            }
        }
    });
    ui.separator();
    if mi(ui, "Cut", "Ctrl+X", on_cmd) {
        *act = Some(CmdAct::Cut);
        ui.close();
    }
    if mi(ui, "Copy", "Ctrl+C", on_cmd) {
        *act = Some(CmdAct::Copy);
        ui.close();
    }
    if mi(ui, "Paste", "Ctrl+V", can_paste) {
        *act = Some(CmdAct::Paste);
        ui.close();
    }
    ui.separator();
    if mi(ui, "Delete", "Del", on_cmd || !is_root) {
        *act = Some(CmdAct::Delete);
        ui.close();
    }
    if mi(ui, "Rename", "F2", on_cmd || !is_root) {
        *act = Some(CmdAct::Rename);
        ui.close();
    }
    ui.separator();
    if mi(ui, "Expand All Folders", "", true) {
        *act = Some(CmdAct::ExpandAll);
        ui.close();
    }
    if mi(ui, "Collapse All Folders", "", true) {
        *act = Some(CmdAct::CollapseAll);
        ui.close();
    }
    ui.separator();
    if mi(ui, "Properties", "Alt+Enter", on_cmd) {
        *act = Some(CmdAct::Edit(cmd.unwrap_or(0)));
        ui.close();
    }
}

fn session_menu(ui: &mut egui::Ui, c: &mut TreeCtx, i: usize) {
    ui.set_min_width(280.0);
    let mut act = None;
    if mi(ui, "Connect Terminal", "Ctrl+Alt+T", true) {
        act = Some(TreeAct::Connect(i));
    }
    if mi(ui, "Connect SFTP Session", "Ctrl+Alt+S", c.sessions[i].protocol.is_ssh()) {
        act = Some(TreeAct::ConnectSftp(i));
    }
    if mi(ui, "Connect in New Window", "Ctrl+Alt+W", true) {
        act = Some(TreeAct::NewWindow(i));
    }
    if mi(ui, "Connect in New Tab Group", "Ctrl+Alt+G", true) {
        act = Some(TreeAct::ConnectGroup(i));
    }
    ui.separator();
    if mi(ui, "Cut", "Ctrl+X", true) {
        act = Some(TreeAct::Cut);
    }
    if mi(ui, "Copy", "Ctrl+C", true) {
        act = Some(TreeAct::Copy);
    }
    if mi(ui, "Copy as Path", "Ctrl+Shift+C", true) {
        act = Some(TreeAct::CopyPath);
    }
    if mi(ui, "Duplicate...", "Ctrl+D", true) {
        act = Some(TreeAct::Duplicate);
    }
    if mi(ui, "Paste", "Ctrl+V", c.has_clip) {
        act = Some(TreeAct::Paste);
    }
    ui.separator();
    if mi(ui, "Find...", "Ctrl+F", true) {
        act = Some(TreeAct::Find);
    }
    if mi(ui, "Find Next", "F3", true) {
        act = Some(TreeAct::FindNext);
    }
    ui.separator();
    if mi(ui, "Delete", "Del", true) {
        act = Some(TreeAct::Delete);
    }
    if mi(ui, "Rename", "F2", true) {
        act = Some(TreeAct::Rename);
    }
    if mi(ui, "Create Desktop Shortcut", "Ctrl+Alt+E", true) {
        act = Some(TreeAct::Shortcut);
    }
    ui.separator();
    if mi(ui, "Expand All Folders", "", true) {
        act = Some(TreeAct::ExpandAll);
    }
    if mi(ui, "Collapse All Folders", "", true) {
        act = Some(TreeAct::CollapseAll);
    }
    ui.separator();
    if mi(ui, "Refresh", "Ctrl+F5", true) {
        act = Some(TreeAct::Refresh);
    }
    ui.separator();
    sub_menu(ui, "Arrange Tree", |ui| {
        if ui.button("Sort by name (A-Z)").clicked() {
            act = Some(TreeAct::Sort(true));
        }
        if ui.button("Sort by name (Z-A)").clicked() {
            act = Some(TreeAct::Sort(false));
        }
    });
    let mut d = c.show_desc;
    if ui.checkbox(&mut d, "Show Descriptions").clicked() {
        act = Some(TreeAct::ToggleDescriptions);
    }
    ui.separator();
    if mi(ui, "Export Selected Session(s)...", "", true) {
        act = Some(TreeAct::Export);
    }
    ui.separator();
    if mi(ui, "Properties", "Alt+Enter", true) {
        act = Some(TreeAct::Properties);
    }
    if act.is_some() {
        c.selected = Some(i);
        c.act = act;
        ui.close();
    }
}

/// Characters the built-in fonts lack (check marks, arrows, Chinese / Japanese / Korean) come from the
/// fonts Windows ships; they are added behind the built-in ones, so the look of normal text is unchanged.
fn install_fallback_fonts(ctx: &egui::Context) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [
        ("fb_mdl2", "segmdl2.ttf"),
        ("fb_symbols", "seguisym.ttf"),
        ("fb_cjk_sc", "msyh.ttc"),
        ("fb_cjk_jp", "msgothic.ttc"),
        ("fb_korean", "malgun.ttf"),
        ("fb_cjk_tc", "msjh.ttc"),
    ];
    let mut fonts = egui::FontDefinitions::default();
    // The original draws the session window in Consolas and its menus in Segoe UI.
    for (name, file, fam) in [
        ("win_consolas", "consola.ttf", egui::FontFamily::Monospace),
        ("win_segoeui", "segoeui.ttf", egui::FontFamily::Proportional),
    ] {
        let path = std::path::Path::new(&windir).join("Fonts").join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            fonts.font_data.insert(name.to_string(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            if let Some(list) = fonts.families.get_mut(&fam) {
                list.insert(0, name.to_string());
            }
        }
    }
    let mut added = Vec::new();
    for (name, file) in candidates {
        let path = std::path::Path::new(&windir).join("Fonts").join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            fonts.font_data.insert(name.to_string(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            added.push(name.to_string());
        }
    }
    // a family for the folder picture: Segoe UI Symbol first (a filled folder), then the usual fonts
    let mut sym: Vec<String> = added.iter().filter(|n| n.as_str() == "fb_symbols").cloned().collect();
    sym.extend(fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default());
    fonts.families.insert(egui::FontFamily::Name("symbols".into()), sym);
    for fam in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
        if let Some(list) = fonts.families.get_mut(&fam) {
            list.extend(added.iter().cloned());
        }
    }
    ctx.set_fonts(fonts);
}

fn side_tab(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let dark = ui.visuals().dark_mode;
    let color = match (dark, selected) {
        (true, true) => Color32::WHITE,
        (true, false) => Color32::from_gray(205),
        (false, _) => Color32::from_gray(20),
    };
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let (len, h) = (galley.size().x, galley.size().y);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, len + 20.0), Sense::click());
    if selected || resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, if dark { Color32::from_rgb(0x3a, 0x3a, 0x3a) } else { Color32::from_rgb(0xc8, 0xc8, 0xc8) });
    }
    let pos = Pos2::new(rect.center().x + h / 2.0, rect.min.y + 10.0);
    ui.painter().add(egui::epaint::TextShape::new(pos, galley, color).with_angle(std::f32::consts::FRAC_PI_2));
    resp
}

fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str, enabled: bool) -> bool {
    let color = if ui.visuals().dark_mode { Color32::WHITE } else { Color32::from_gray(30) };
    let b = egui::Button::new(RichText::new(glyph).size(15.0).color(color)).frame(false).min_size(egui::vec2(24.0, 24.0));
    ui.add_enabled(enabled, b).on_hover_text(tip).clicked()
}

fn tb_sep(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 20.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), egui::Stroke::new(1.0, Color32::from_gray(0x48)));
}

fn palette(theme: &str) -> (bool, Color32, Color32) {
    // (dark, panel, central/background)
    match theme {
        "Carbon" => (true, Color32::from_rgb(0x1b, 0x1b, 0x1b), Color32::from_rgb(0x16, 0x16, 0x16)),
        "VS Blue" => (true, Color32::from_rgb(0x29, 0x39, 0x55), Color32::from_rgb(0x1f, 0x2b, 0x40)),
        "System" | "Native" | "VS White" | "Office White" => (false, Color32::from_rgb(0xf0, 0xf0, 0xf0), Color32::from_rgb(0xab, 0xab, 0xab)),
        _ => (true, Color32::from_rgb(0x1f, 0x1f, 0x1f), Color32::from_rgb(0x1f, 0x1f, 0x1f)),
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: &str) {
    let (dark, panel, _) = palette(theme);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = panel;
    v.window_fill = panel;
    v.extreme_bg_color = if dark { Color32::from_rgb(0x2b, 0x2b, 0x2b) } else { Color32::WHITE };
    v.faint_bg_color = if dark { Color32::from_rgb(0x28, 0x28, 0x28) } else { Color32::from_rgb(0xe8, 0xe8, 0xe8) };
    v.selection.bg_fill = Color32::from_rgb(0x00, 0x63, 0xb1);
    v.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
    v.window_corner_radius = egui::CornerRadius::same(2);
    v.menu_corner_radius = egui::CornerRadius::same(2);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(2);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
    v.widgets.active.corner_radius = egui::CornerRadius::same(2);
    if dark {
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2d, 0x2d, 0x2d);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0x2d, 0x2d, 0x2d);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x3c, 0x3c, 0x3c);
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_gray(70));
    }
    if dark {
        // pop-up menus: the original's grey with a thin lighter border and round corners
        v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(0xd6);
        v.widgets.inactive.fg_stroke.color = Color32::from_gray(0xd6);
        v.window_fill = Color32::from_gray(0x2e);
        v.window_stroke = egui::Stroke::new(1.0, Color32::from_gray(0x67));
        v.menu_corner_radius = egui::CornerRadius::same(6);
    }
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Body, FontId::proportional(12.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(12.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(11.0));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(12.0));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.0));
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
        s.spacing.menu_margin = egui::Margin::symmetric(2, 3);
        // Labels are not text: the pointer must stay an arrow over them (no text cursor).
        s.interaction.selectable_labels = false;
    });
}

pub struct YettyApp {
    settings: Settings,
    sessions: Vec<Session>,
    active: usize,
    next_id: u64,
    layout: Layout,
    side: Option<Side>,
    pinned: bool,
    side_rect: egui::Rect,
    selected: Option<usize>,
    selected_folder: Option<String>,
    sel_default: Option<usize>,
    filter: String,
    clip: Option<(Vec<usize>, bool)>,
    expand: Option<bool>,
    connect_bar: String,
    keyword_bar: String,
    command_line: String,
    /// Send Commands to: 0 Active Session, 1 All Sessions, 2 Visible Sessions, 3 Selected Tab Group(s).
    cmd_target: u8,
    /// The tab group the Command Window sends to while `cmd_target` is 3.
    cmd_group: usize,
    pane_max: Option<u64>,
    pane_min: Vec<u64>,
    /// Tab groups (Send to New Tab Group): session id -> group; sessions missing from the map are in group 0.
    group_of: std::collections::HashMap<u64, usize>,
    n_groups: usize,
    /// The session each tab group shows (by id), one entry per group.
    group_current: Vec<u64>,
    /// New Session Wizard (Global Options > General > "New session uses wizard").
    wizard: Option<crate::wizard::WizardState>,
    /// Some session was connected in the previous frame (for "Exit when last session is disconnected").
    was_connected: bool,
    global_state: globaldlg::GlobalState,
    keymap_state: keymapdlg::KeymapState,
    open_after: Option<u64>,
    cmd_expand: Option<bool>,
    cm_hover: bool,
    cmd_sel: Option<usize>,
    cmd_sel_folder: Option<String>,
    cmd_filter: String,
    cmd_clip: Option<(config::CommandDef, Option<usize>)>,
    cmd_edit: Option<(Option<usize>, config::CommandDef)>,
    quick: Option<QuickDlg>,
    options: Option<OptionsDlg>,
    prompt: Option<PromptDlg>,
    global_open: bool,
    keymap_open: bool,
    keyassist_open: bool,
    snapshots_open: bool,
    scratch: Option<String>,
    about_open: bool,
    find: FindState,
    button_editor: Option<ButtonEditor>,
    scripts: Vec<ScriptRun>,
    rec_target: Option<u64>,
    last_snapshot: Option<String>,
    status_msg: String,
    zoom: f32,
    last_title: String,
    screenshot: Option<String>,
    frames: u32,
    shot_requested: bool,
    focus_terminal: bool,
    dirty: bool,
    always_on_top: bool,
    auto_print_on: bool,
    error_msg: Option<String>,
    pending_log: bool,
    pending_raw: bool,
    pending_trace: bool,
    rec_wait: bool,
    tftp: Option<crate::tftp::TftpServer>,
    tftp_open: bool,
    zm_open: bool,
    zm_list: Vec<String>,
    unlock: Option<UnlockDlg>,
    /// Sessions the start-up asked for, opened once the configuration passphrase is typed.
    deferred: Vec<SessionConfig>,
    deferred_connect: Option<String>,
    pass_dlg: Option<PassDlg>,
    transparent: bool,
    cred_tmp: String,
    cred_tmp_for: Option<usize>,
    trace_open: bool,
    creds_open: bool,
    cred_sel: Option<usize>,
    sm_hover: bool,
    confirm: Option<(String, ConfirmAct)>,
    exit_ok: bool,
}

/// What a Yes / No question does when it is answered Yes.
#[derive(Clone)]
enum ConfirmAct {
    DeleteTree,
    CloseTab(u64),
    CloseOthers(u64),
    CloseRight(u64),
    CloseGroup(usize),
    CloseAll,
    Disconnect(u64),
    DisconnectAll,
    DeleteCommand,
    Exit,
}

impl YettyApp {
    fn ask(&mut self, text: impl Into<String>, act: ConfirmAct) {
        self.confirm = Some((text.into(), act));
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.sessions.iter().position(|s| s.id == id)
    }

    /// The tab group of session `i` (0 when it was never sent to another group).
    fn group_index(&self, i: usize) -> usize {
        let g = self.sessions.get(i).and_then(|s| self.group_of.get(&s.id).copied()).unwrap_or(0);
        g.min(self.n_groups.saturating_sub(1))
    }

    /// Session indices of tab group `g`, in tab order.
    fn group_members(&self, g: usize) -> Vec<usize> {
        (0..self.sessions.len()).filter(|&i| self.group_index(i) == g).collect()
    }

    /// Keeps the tab groups consistent after sessions open, close or move: empty groups go away (the
    /// rest are renumbered in order) and every group shows one of its own sessions, the active session's
    /// group showing the active session.
    fn sync_groups(&mut self) {
        let live: std::collections::HashSet<u64> = self.sessions.iter().map(|s| s.id).collect();
        self.group_of.retain(|id, _| live.contains(id));
        let n = self.n_groups.max(1);
        let mut used = vec![false; n];
        for s in &self.sessions {
            used[self.group_of.get(&s.id).copied().unwrap_or(0).min(n - 1)] = true;
        }
        let mut renumber = vec![0usize; n];
        let mut k = 0;
        for g in 0..n {
            renumber[g] = k;
            if used[g] {
                k += 1;
            }
        }
        let k = k.max(1);
        for g in self.group_of.values_mut() {
            *g = renumber[(*g).min(n - 1)].min(k - 1);
        }
        let mut current = vec![u64::MAX; k];
        for (g, id) in self.group_current.iter().enumerate() {
            if g < n && used[g] {
                current[renumber[g]] = *id;
            }
        }
        self.n_groups = k;
        if let Some(id) = self.sessions.get(self.active).map(|s| s.id) {
            current[self.group_index(self.active)] = id;
        }
        for (g, cur) in current.iter_mut().enumerate() {
            let ok = self.index_of(*cur).map(|i| self.group_index(i) == g).unwrap_or(false);
            if !ok {
                *cur = self.group_members(g).first().map(|&i| self.sessions[i].id).unwrap_or(u64::MAX);
            }
        }
        self.group_current = current;
        if self.cmd_group >= k {
            self.cmd_group = 0;
        }
        if self.cmd_target == 3 && k == 1 {
            self.cmd_target = 0;
        }
    }

    fn connected_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.is_connected()).count()
    }

    /// Close one tab; a connected session is asked about first.
    fn request_close(&mut self, i: usize) {
        let Some(s) = self.sessions.get(i) else { return };
        let (id, connected, name) = (s.id, s.is_connected(), s.title.clone());
        if connected {
            self.ask(format!("You are currently connected to \"{name}\".\nAre you sure you want to disconnect and close it?"), ConfirmAct::CloseTab(id));
        } else {
            self.close_tab(i);
        }
    }

    fn request_disconnect(&mut self, i: usize) {
        if let Some(s) = self.sessions.get(i) {
            if s.is_connected() {
                let (id, name) = (s.id, s.title.clone());
                // Global Options > General > Dialogs > "Show confirm disconnect dialog"
                if self.settings.global.confirm_disconnect {
                    self.ask(format!("Are you sure you want to disconnect from \"{name}\"?"), ConfirmAct::Disconnect(id));
                } else {
                    self.sessions[i].disconnect();
                }
            }
        }
    }

    fn run_confirmed(&mut self, act: ConfirmAct, ctx: &egui::Context) {
        match act {
            ConfirmAct::DeleteTree => self.tree_action_now(TreeAct::Delete, ctx),
            ConfirmAct::CloseTab(id) => {
                if let Some(i) = self.index_of(id) {
                    self.close_tab(i);
                }
            }
            // the tab menu acts on the tab bar it was opened from: other / right-hand tabs of the same group
            ConfirmAct::CloseOthers(id) => {
                if let Some(i) = self.index_of(id) {
                    let g = self.group_index(i);
                    let gone: Vec<u64> = self.group_members(g).into_iter().filter(|&k| k != i).map(|k| self.sessions[k].id).collect();
                    self.close_ids(&gone, id);
                }
            }
            ConfirmAct::CloseRight(id) => {
                if let Some(i) = self.index_of(id) {
                    let g = self.group_index(i);
                    let gone: Vec<u64> = self.group_members(g).into_iter().filter(|&k| k > i).map(|k| self.sessions[k].id).collect();
                    self.close_ids(&gone, id);
                }
            }
            ConfirmAct::CloseGroup(g) => {
                let gone: Vec<u64> = self.group_members(g).into_iter().map(|k| self.sessions[k].id).collect();
                let keep = self.sessions.iter().map(|s| s.id).find(|x| !gone.contains(x)).unwrap_or(u64::MAX);
                self.close_ids(&gone, keep);
            }
            ConfirmAct::CloseAll => {
                self.sessions.clear();
                self.active = 0;
            }
            ConfirmAct::Disconnect(id) => {
                if let Some(i) = self.index_of(id) {
                    self.sessions[i].disconnect();
                }
            }
            ConfirmAct::DeleteCommand => {
                if let Some(f) = self.cmd_sel_folder.clone().filter(|f| !f.is_empty()) {
                    let inside = |p: &str| p == f || p.starts_with(&format!("{f}/"));
                    self.settings.commands.retain(|c| !inside(c.folder.trim_matches('/')));
                    self.settings.command_folders.retain(|x| !inside(x));
                    self.cmd_sel_folder = None;
                    self.cmd_sel = None;
                    self.dirty = true;
                } else if let Some(i) = self.cmd_sel {
                    if i < self.settings.commands.len() {
                        self.settings.commands.remove(i);
                        self.cmd_sel = None;
                        self.dirty = true;
                    }
                }
            }
            ConfirmAct::DisconnectAll => {
                for s in &mut self.sessions {
                    s.disconnect();
                }
            }
            ConfirmAct::Exit => {
                self.exit_ok = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn request_exit(&mut self, ctx: &egui::Context) {
        let n = self.connected_count();
        if n > 0 {
            let txt = if n == 1 {
                "You are currently connected to 1 session.\nAre you sure you want to disconnect and exit?".to_string()
            } else {
                format!("You are currently connected to {n} sessions.\nAre you sure you want to disconnect and exit?")
            };
            self.ask(txt, ConfirmAct::Exit);
        } else {
            self.exit_ok = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<String>, connect: Option<String>) -> Self {
        let mut settings = if screenshot.is_some() { Settings::default() } else { Settings::load() };
        // Global Options > General "Save expanded folder state for session list" off: folders start closed
        if !settings.global.save_tree_state {
            settings.tree_open.clear();
        }
        install_fallback_fonts(&cc.egui_ctx);
        if settings.ansi_palette.len() == 16 {
            let mut p = [[0u8; 3]; 16];
            for (i, c) in settings.ansi_palette.iter().enumerate() {
                p[i] = *c;
            }
            config::set_ansi_palette(p);
        }
        apply_theme(&cc.egui_ctx, &settings.theme);
        // Ctrl+= / Ctrl+- belong to the session zoom (or to the terminal when that is turned off), never to egui's
        // whole-window zoom
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let startup_quick = settings.quick_connect_at_start && connect.is_none();
        let defaults = settings.defaults_for(Protocol::Ssh);
        // Global Options > Terminal > Advanced > "Initially send commands to"
        let initial_target = settings.global.initial_send_target.min(3);
        let mut app = YettyApp {
            settings,
            sessions: Vec::new(),
            active: 0,
            next_id: 1,
            layout: Layout::Tabs,
            side: None,
            pinned: false,
            side_rect: egui::Rect::NOTHING,
            selected: None,
            selected_folder: None,
            sel_default: None,
            filter: String::new(),
            clip: None,
            expand: None,
            connect_bar: String::new(),
            keyword_bar: String::new(),
            command_line: String::new(),
            cmd_target: initial_target,
            cmd_group: 0,
            pane_max: None,
            pane_min: Vec::new(),
            group_of: std::collections::HashMap::new(),
            n_groups: 1,
            group_current: Vec::new(),
            wizard: None,
            was_connected: false,
            global_state: globaldlg::GlobalState::default(),
            keymap_state: keymapdlg::KeymapState::default(),
            open_after: None,
            cmd_expand: None,
            cm_hover: false,
            cmd_sel: None,
            cmd_sel_folder: None,
            cmd_filter: String::new(),
            cmd_clip: None,
            cmd_edit: None,
            quick: None,
            options: None,
            prompt: None,
            global_open: false,
            keymap_open: false,
            keyassist_open: false,
            snapshots_open: false,
            scratch: None,
            about_open: false,
            find: FindState::default(),
            button_editor: None,
            scripts: Vec::new(),
            rec_target: None,
            last_snapshot: None,
            status_msg: "Ready".into(),
            zoom: 1.0,
            last_title: String::new(),
            screenshot,
            frames: 0,
            shot_requested: false,
            focus_terminal: false,
            dirty: false,
            always_on_top: false,
            auto_print_on: false,
            error_msg: None,
            confirm: None,
            exit_ok: false,
            pending_log: false,
            pending_raw: false,
            pending_trace: false,
            rec_wait: false,
            tftp: None,
            tftp_open: false,
            zm_open: false,
            zm_list: Vec::new(),
            unlock: None,
            deferred: Vec::new(),
            deferred_connect: None,
            pass_dlg: None,
            transparent: false,
            cred_tmp: String::new(),
            cred_tmp_for: None,
            trace_open: false,
            creds_open: false,
            cred_sel: None,
            sm_hover: false,
        };
        if startup_quick {
            app.quick = Some(QuickDlg::new(defaults));
        }
        // Test hook for the screenshot check: YETTY_SHOT_SCENE=manager | options:<page>, YETTY_SHOT_THEME=<theme>
        if app.screenshot.is_some() {
            if let Ok(t) = std::env::var("YETTY_SHOT_THEME") {
                if !t.is_empty() {
                    app.settings.theme = t.clone();
                    apply_theme(&cc.egui_ctx, &t);
                }
            }
            if let Ok(scene) = std::env::var("YETTY_SHOT_SCENE") {
                let mut sample = app.settings.defaults_for(Protocol::Ssh);
                sample.host = "192.0.2.10".into();
                sample.description = "Sample server".into();
                let mut in_folder = sample.clone();
                in_folder.host = "138.2.244.44".into();
                in_folder.folder = "Leo".into();
                app.settings.sessions.push(in_folder);
                app.settings.sessions.push(sample);
                app.selected = Some(0);
                app.quick = None;
                if let Some(m) = scene.strip_prefix("quick:") {
                    let mut q = QuickDlg::new(app.settings.defaults_for(Protocol::Ssh));
                    if !m.is_empty() {
                        q.auth_open = Some(m.to_string());
                    }
                    app.quick = Some(q);
                    app.side = Some(Side::SessionManager);
                } else if scene == "manager" {
                    app.side = Some(Side::SessionManager);
                    app.settings.tree_open.push("Leo".into()); // a folder left open last time stays open
                } else if scene == "glyphs" {
                    let mut cfg = app.settings.defaults_for(Protocol::LocalShell);
                    cfg.name = "Glyph test".into();
                    app.open_session(cfg, &cc.egui_ctx);
                    if let Some(s) = app.sessions.last_mut() {
                        s.parser.process("check \u{2713} \u{2714} arrows \u{2192} \u{2190}\r\ncjk \u{6f22}\u{5b57} \u{65e5}\u{672c}\u{8a9e} \u{d55c}\u{ae00}\r\npath /etc/a?b @ \\ |\r\n-rwxrwxr-x 1 ubuntu ubuntu      519 nov  7  2025  \x1b[1;32mteste2.sh\x1b[0m\r\ndrwxrwxr-x 1 ubuntu ubuntu       36 dez 11  2025  \x1b[1;34mcatalogo\x1b[0m\r\n-rw-rw-r-- 1 ubuntu ubuntu 28948275 jan  5  2026  \x1b[1;31mk6-v1.5.0.tar.gz\x1b[0m\r\n".as_bytes());
                    }
                } else if scene.starts_with("cmd") {
                    // cmdwin = Command Window; cmdman = Command Manager; cmddlg = Command Properties
                    app.settings.show_command_window = scene == "cmdwin";
                    if scene != "cmdwin" {
                        app.settings.show_command_manager = true;
                        app.side = Some(Side::CommandManager);
                        app.settings.commands.push(config::CommandDef { label: "List files".into(), text: "ls -lrt\\r".into(), ..Default::default() });
                        app.settings.command_folders.push("Servers".into());
                    }
                    if scene == "cmddlg" {
                        app.cmd_edit = Some((None, config::CommandDef::default()));
                    }
                } else if let Some(pg) = scene.strip_prefix("global") {
                    app.global_open = true;
                    if let Some(p) = pg.strip_prefix(':') {
                        app.global_state.page = p.to_string();
                    }
                } else if scene == "keymap" {
                    app.keymap_open = true;
                    app.settings.default_session.mapped_keys.push(config::MappedKey { key: "Ctrl+F5".into(), func: "Send String".into(), send: "ls\\r".into() });
                    app.keymap_state.ctrl = true;
                    app.keymap_state.selected = egui::Key::ALL.iter().copied().find(|k| k.name() == "F5");
                } else if scene == "hostkey" || scene == "sshpw" {
                    let mut cfg = app.settings.defaults_for(Protocol::LocalShell);
                    cfg.name = "Prompt test".into();
                    cfg.host = "127.0.0.1".into();
                    cfg.username = "qa".into();
                    app.open_session(cfg, &cc.egui_ctx);
                    if let Some(s) = app.sessions.last_mut() {
                        s.prompt_req = Some(if scene == "hostkey" {
                            term::SshPrompt::HostKey { detail: "The authenticity of host '127.0.0.1 (127.0.0.1)' can't be established.\nED25519 key fingerprint is SHA256:abcdEFGH1234.\nThis key is not known by any other names.\nAre you sure you want to continue connecting (yes/no/[fingerprint])?".into() }
                        } else {
                            term::SshPrompt::Password
                        });
                    }
                } else if scene == "tile" {
                    for n in ["Left pane", "Right pane"] {
                        let mut cfg = app.settings.defaults_for(Protocol::LocalShell);
                        cfg.name = n.into();
                        app.open_session(cfg, &cc.egui_ctx);
                    }
                    app.layout = Layout::TileV;
                    app.active = 0;
                } else if scene == "groups" {
                    // three tabs; the last one sent to a second tab group
                    for n in ["Left one", "Left two", "Right"] {
                        let mut cfg = app.settings.defaults_for(Protocol::LocalShell);
                        cfg.name = n.into();
                        app.open_session(cfg, &cc.egui_ctx);
                    }
                    let id = app.sessions[2].id;
                    app.group_of.insert(id, 1);
                    app.n_groups = 2;
                    app.group_current = vec![app.sessions[0].id, id];
                    app.active = 2;
                    app.settings.show_command_window = true;
                    app.cmd_target = 3;
                    app.cmd_group = 1;
                } else if let Some(page) = scene.strip_prefix("wizard") {
                    let mut w = crate::wizard::WizardState::new(&app.settings);
                    if let Some(p) = page.strip_prefix(':').and_then(|p| p.parse().ok()) {
                        w.page = p;
                        w.cfg.host = "example.host".into();
                        w.cfg.name = "example.host".into();
                    }
                    app.wizard = Some(w);
                } else if scene == "multipaste" {
                    let mut cfg = app.settings.defaults_for(Protocol::LocalShell);
                    cfg.name = "Paste test".into();
                    app.open_session(cfg, &cc.egui_ctx);
                    if let Some(s) = app.sessions.last_mut() {
                        s.pending_paste = Some("curl https://example.com/a/b/c\recho one\recho two\r".into());
                    }
                } else if scene == "confirm" {
                    app.side = Some(Side::SessionManager);
                    app.selected = Some(0);
                    app.ask("Are you sure you want to delete the selected item(s)?", ConfirmAct::DeleteTree);
                } else if let Some(page) = scene.strip_prefix("options:") {
                    app.side = Some(Side::SessionManager);
                    app.open_options(EditTarget::Saved(0), page);
                }
            }
        }
        let locked = !app.settings.config_passphrase_check.is_empty();
        if locked {
            app.unlock = Some(UnlockDlg::default());
        }
        if app.screenshot.is_none() && connect.is_none() {
            // Global Options > General > "Show Session Manager or Connect dialog on startup"
            if app.settings.global.show_sm_on_startup {
                app.side = Some(Side::SessionManager);
            }
            // Global Options > General > Auto Session
            let start: Vec<SessionConfig> = match app.settings.global.auto_session {
                1 => vec![app.settings.defaults_for(Protocol::LocalShell)],
                2 => app.settings.global.last_sessions.clone(),
                3 => app.settings.snapshots.iter().find(|s| s.0 == app.settings.global.auto_snapshot).map(|s| s.1.clone()).unwrap_or_default(),
                _ => Vec::new(),
            };
            for cfg in start {
                if locked {
                    app.deferred.push(cfg);
                } else {
                    app.open_session(cfg, &cc.egui_ctx);
                }
            }
            app.dirty = false;
        }
        if let Some(c) = connect {
            if locked {
                app.deferred_connect = Some(c);
            } else {
                app.run_connect(c, &cc.egui_ctx);
                app.dirty = false;
            }
        }
        app
    }

    // ----- sessions ------------------------------------------------------------

    fn run_connect(&mut self, c: String, ctx: &egui::Context) {
        if let Some(name) = c.strip_prefix("session:") {
            if let Some(cfg) = self.settings.sessions.iter().find(|s| s.display_name() == name).cloned() {
                self.open_session(cfg, ctx);
            }
        } else {
            self.connect_bar = c;
            self.connect_from_bar(ctx);
        }
    }

    /// The start-up window asking for the configuration passphrase; true while it is still open.
    fn unlock_ui(&mut self, ctx: &egui::Context) -> bool {
        let Some(mut d) = self.unlock.take() else { return false };
        let (mut ok, mut cancel, mut open) = (false, false, true);
        dialogs::dialog(ctx, "unlock_cfg", "Enter Configuration Passphrase", 380.0, &mut open, false, |ui| {
            ui.add_space(4.0);
            ui.label("Enter the passphrase that protects the saved passwords:");
            ui.add_space(4.0);
            let r = ui.add(egui::TextEdit::singleline(&mut d.pass).password(true).desired_width(f32::INFINITY));
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
            if !d.err.is_empty() {
                ui.colored_label(Color32::from_rgb(0xe0, 0x60, 0x60), &d.err);
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    cancel = true;
                }
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                    ok = true;
                }
            });
        });
        if cancel || !open {
            self.exit_ok = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.unlock = Some(d);
            return true;
        }
        if ok {
            if config::passphrase_matches(&self.settings.config_passphrase_check, &d.pass) {
                config::set_config_passphrase(Some(&d.pass));
                for cfg in std::mem::take(&mut self.deferred) {
                    self.open_session(cfg, ctx);
                }
                if let Some(c) = self.deferred_connect.take() {
                    self.run_connect(c, ctx);
                }
                self.dirty = false;
                return false;
            }
            d.err = "The passphrase is not correct.".into();
            d.pass.clear();
        }
        self.unlock = Some(d);
        true
    }

    /// Tools > Change Configuration Passphrase: the saved passwords are encrypted again with the new passphrase.
    fn pass_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.pass_dlg.take() else { return };
        let has_cur = !self.settings.config_passphrase_check.is_empty();
        let (mut ok, mut cancel, mut open) = (false, false, true);
        dialogs::dialog(ctx, "change_cfg_pass", "Change Configuration Passphrase", 400.0, &mut open, true, |ui| {
            ui.add_space(4.0);
            ui.label("The passphrase protects the saved passwords and key passphrases in the settings. It is asked for each time Yetty starts.");
            ui.add_space(4.0);
            egui::Grid::new("pass_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                if has_cur {
                    ui.label("Current passphrase:");
                    ui.add(egui::TextEdit::singleline(&mut d.old).password(true).desired_width(200.0));
                    ui.end_row();
                }
                ui.label("New passphrase:");
                ui.add(egui::TextEdit::singleline(&mut d.new).password(true).desired_width(200.0));
                ui.end_row();
                ui.label("Confirm passphrase:");
                ui.add(egui::TextEdit::singleline(&mut d.confirm).password(true).desired_width(200.0));
                ui.end_row();
            });
            ui.weak("Leave the new passphrase empty to remove the passphrase.");
            if !d.err.is_empty() {
                ui.colored_label(Color32::from_rgb(0xe0, 0x60, 0x60), &d.err);
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    cancel = true;
                }
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                    ok = true;
                }
            });
        });
        if cancel || !open {
            return;
        }
        if ok {
            if has_cur && !config::passphrase_matches(&self.settings.config_passphrase_check, &d.old) {
                d.err = "The current passphrase is not correct.".into();
            } else if d.new != d.confirm {
                d.err = "The two passphrases are not the same.".into();
            } else {
                let old = has_cur.then(|| d.old.clone());
                let new = if d.new.is_empty() { None } else { Some(d.new.clone()) };
                self.settings.rekey(old.as_deref(), new.as_deref());
                for s in &mut self.sessions {
                    s.cfg.rekey_secrets(old.as_deref(), new.as_deref());
                }
                config::set_config_passphrase(new.as_deref());
                self.settings.save();
                self.status_msg = if new.is_some() { "Configuration passphrase changed".into() } else { "Configuration passphrase removed".into() };
                return;
            }
        }
        self.pass_dlg = Some(d);
    }

    fn open_session(&mut self, cfg: SessionConfig, ctx: &egui::Context) {
        let mut cfg = cfg;
        if cfg.protocol == Protocol::Rdp {
            // Remote Desktop: the Windows client runs in its own window.
            let port = if cfg.port == 0 { 3389 } else { cfg.port };
            let mut c = std::process::Command::new("mstsc.exe");
            c.arg(format!("/v:{}:{}", cfg.host.trim(), port));
            self.status_msg = match c.spawn() {
                Ok(_) => format!("Remote Desktop to {} started", cfg.host.trim()),
                Err(e) => format!("Could not start Remote Desktop: {e}"),
            };
            return;
        }
        // Window > Initial position: the Yetty window moves there when the first session opens
        if cfg.initial_pos_on && self.sessions.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(cfg.initial_x as f32, cfg.initial_y as f32)));
        }
        if cfg.protocol.uses_host() && cfg.port == 0 {
            cfg.port = cfg.protocol.default_port();
        }
        // A saved credential supplies the user name and password.
        if cfg.use_credentials {
            if let Some(c) = self.settings.credentials.iter().find(|c| c.name == cfg.credentials) {
                cfg.username = c.user.clone();
                if !c.pass.is_empty() {
                    cfg.saved_password = c.pass.clone();
                }
            }
        }
        // Logon steps with credentials: \s becomes the user name and \w the password.
        let creds = self.settings.credentials.clone();
        for step in cfg.logon_steps.iter_mut() {
            if let Some(c) = creds.iter().find(|c| !step.credentials.is_empty() && c.name == step.credentials) {
                step.send = step.send.replace("\\s", &c.user).replace("\\w", &config::unprotect(&c.pass));
            }
        }
        // Auto Hide: the Session Manager slips away as soon as a session is opened from it.
        if !self.pinned && self.side.is_some() {
            self.side = None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let name = cfg.display_name();
        let global = self.settings.global_identity.clone();
        // Log Session / Raw Log Session / Trace Options turned on before this session existed
        if self.pending_log || self.pending_raw {
            cfg.start_log_on_connect = true;
            cfg.log_prompt = false;
            cfg.log_raw = cfg.log_raw || self.pending_raw;
        }
        if self.pending_trace {
            cfg.ssh_verbose = true;
        }
        if cfg.window_on_top && !self.always_on_top {
            self.always_on_top = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        }
        // Global Options > Terminal > "Auto reconnect" (every session) and its interval
        let mut cfg = cfg;
        cfg.auto_reconnect |= self.settings.global.auto_reconnect;
        let mut s = Session::new_with_key(id, cfg, ctx, global);
        s.reconnect_secs = u64::from(self.settings.global.reconnect_secs.max(1));
        if !s.cfg.tab_title.trim().is_empty() {
            s.title = s.cfg.tab_title.clone();
        }
        if self.rec_wait {
            self.rec_wait = false;
            s.recording = Some(Vec::new());
            self.rec_target = Some(s.id);
        }
        let new_pos = match self.open_after.take().and_then(|oid| self.sessions.iter().position(|x| x.id == oid)) {
            Some(pos) => pos + 1, // an SFTP tab opens right after the session it came from
            None => self.sessions.len(),
        };
        // a new tab opens in the tab group of the session that is active now
        let group = self.group_index(self.active);
        let sid = s.id;
        self.sessions.insert(new_pos, s);
        if group > 0 {
            self.group_of.insert(sid, group);
        }
        self.active = new_pos;
        self.focus_terminal = true;
        self.settings.recent.retain(|r| r != &name);
        self.settings.recent.insert(0, name);
        // Global Options > General > "Recently used session list"
        self.settings.recent.truncate(self.settings.global.recent_list_size.clamp(1, 16) as usize);
        self.dirty = true;
    }

    /// Tools > Create Public Key... (also Global Options > SSH2 > Create Identity File...): ssh-keygen in a tab.
    fn tool_create_public_key(&mut self, ctx: &egui::Context) {
        let mut cfg = self.settings.defaults_for(Protocol::LocalShell);
        cfg.name = "Create Public Key".into();
        cfg.shell_command = "ssh-keygen.exe -t ed25519".into();
        self.open_session(cfg, ctx);
    }

    /// Tools > Manage Agent Keys... (also Global Options > SSH2 > Agent Keys): the agent's keys in a tab.
    fn tool_manage_agent_keys(&mut self, ctx: &egui::Context) {
        let mut cfg = self.settings.defaults_for(Protocol::LocalShell);
        cfg.name = "Agent Keys".into();
        cfg.shell_command = "cmd.exe /k ssh-add -l".into();
        self.open_session(cfg, ctx);
    }

    fn open_new_window(&mut self, cfg: &SessionConfig, saved_name: Option<&str>) {
        if let Ok(exe) = std::env::current_exe() {
            let mut cmd = std::process::Command::new(exe);
            match saved_name {
                Some(n) => {
                    cmd.arg("--session").arg(n);
                }
                None => {
                    cmd.args(cfg.cli_args());
                }
            }
            let _ = cmd.spawn();
        }
    }

    fn local_shell(&mut self, ctx: &egui::Context) {
        let mut cfg = self.settings.defaults_for(Protocol::LocalShell);
        cfg.name = "Local Shell".into();
        self.open_session(cfg, ctx);
    }

    /// Session Manager > New Session: the wizard, or the Session Options of a new session when the wizard
    /// is turned off.
    fn new_session(&mut self) {
        if self.settings.global.new_session_wizard {
            self.wizard = Some(crate::wizard::WizardState::new(&self.settings));
        } else {
            self.open_options(EditTarget::New, "connection");
        }
    }

    fn quick_connect(&mut self) {
        let cfg = self.settings.defaults_for(Protocol::Ssh);
        self.quick = Some(QuickDlg::new(cfg));
    }

    fn close_tab(&mut self, i: usize) {
        if i >= self.sessions.len() {
            return;
        }
        let g = self.group_index(i);
        let was_active = i == self.active;
        let active_id = self.sessions.get(self.active).map(|s| s.id);
        self.sessions.remove(i);
        if was_active {
            // the next tab of the same group takes over (the previous one when it was the last)
            let members = self.group_members(g);
            self.active = members.iter().copied().find(|&k| k >= i).or(members.last().copied()).unwrap_or(i.min(self.sessions.len().saturating_sub(1)));
        } else {
            self.active = active_id.and_then(|id| self.index_of(id)).unwrap_or(0);
        }
    }

    /// Closes the sessions with these ids; `keep` stays (or becomes) the active one.
    fn close_ids(&mut self, gone: &[u64], keep: u64) {
        let active_id = self.sessions.get(self.active).map(|s| s.id).filter(|id| !gone.contains(id));
        self.sessions.retain(|s| !gone.contains(&s.id));
        self.active = active_id.or(Some(keep)).and_then(|id| self.index_of(id)).unwrap_or(0);
    }

    fn clone_tab(&mut self, i: usize, ctx: &egui::Context) {
        if let Some(s) = self.sessions.get(i) {
            let cfg = s.cfg_for_clone();
            self.open_session(cfg, ctx);
        }
    }

    /// `ssh user@host:port`, `telnet host 23`, `serial COM3 9600`, `local`, or a bare host.
    fn connect_from_bar(&mut self, ctx: &egui::Context) {
        let text = self.connect_bar.trim().to_string();
        if text.is_empty() {
            return;
        }
        let mut parts: Vec<&str> = text.split_whitespace().collect();
        let first = parts[0].to_lowercase();
        let proto = match first.as_str() {
            "ssh" | "ssh2" => Some(Protocol::Ssh),
            "sftp" => Some(Protocol::Sftp),
            "telnet" => Some(Protocol::Telnet),
            "telnets" | "telnet/tls" => Some(Protocol::TelnetTls),
            "rlogin" => Some(Protocol::Rlogin),
            "raw" | "tcp" => Some(Protocol::Raw),
            "serial" | "com" => Some(Protocol::Serial),
            "local" | "shell" | "cmd" => Some(Protocol::LocalShell),
            _ => None,
        };
        let p = proto.unwrap_or(Protocol::Ssh);
        if proto.is_some() {
            parts.remove(0);
        }
        let mut cfg = self.settings.defaults_for(p);
        cfg.name.clear();
        match p {
            Protocol::Serial => {
                if let Some(x) = parts.first() {
                    cfg.com_port = x.to_uppercase();
                }
                if let Some(b) = parts.get(1).and_then(|b| b.parse().ok()) {
                    cfg.baud = b;
                }
            }
            Protocol::LocalShell => cfg.shell_command = parts.join(" "),
            _ => {
                let Some(target) = parts.first() else {
                    self.status_msg = "Enter a hostname in the Enter host box.".into();
                    return;
                };
                let mut target = target.to_string();
                if let Some((u, h)) = target.clone().split_once('@') {
                    cfg.username = u.to_string();
                    target = h.to_string();
                }
                if let Some((h, pt)) = target.clone().rsplit_once(':') {
                    if let Ok(pt) = pt.parse() {
                        cfg.port = pt;
                        target = h.to_string();
                    }
                }
                if let Some(pt) = parts.get(1).and_then(|x| x.parse().ok()) {
                    cfg.port = pt;
                }
                cfg.host = target;
            }
        }
        self.connect_bar.clear();
        self.open_session(cfg, ctx);
    }

    /// "name (copy)", "name (copy 2)"... the first that is not used in the folder.
    fn unique_session_name(&self, folder: &str, base: &str) -> String {
        let taken = |n: &str| self.settings.sessions.iter().any(|s| s.folder == folder && s.display_name().eq_ignore_ascii_case(n));
        let first = format!("{base} (copy)");
        if !taken(&first) {
            return first;
        }
        (2..)
            .map(|k| format!("{base} (copy {k})"))
            .find(|n| !taken(n))
            .unwrap_or(first)
    }

    /// `base` when no session of the folder uses it, otherwise "base (1)", "base (2)"...
    fn free_session_name(&self, folder: &str, base: &str) -> String {
        let taken = |n: &str| self.settings.sessions.iter().any(|s| s.folder == folder && s.display_name().eq_ignore_ascii_case(n));
        if !taken(base) {
            return base.to_string();
        }
        (1..).map(|k| format!("{base} ({k})")).find(|n| !taken(n)).unwrap_or_else(|| base.to_string())
    }

    /// Files and folders dragged from Windows onto an SFTP session are uploaded to the remote folder
    /// it is in (`put`, and `put -r` for folders), as the original does.
    fn handle_file_drops(&mut self, ctx: &egui::Context) {
        let sftp_active = self.sessions.get(self.active).map(|s| s.cfg.protocol == Protocol::Sftp && s.is_connected()).unwrap_or(false);
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if hovering && sftp_active && !self.any_dialog_open() {
            let screen = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop_hint")));
            painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0x00, 0x78, 0xd4, 40));
            painter.text(screen.center(), egui::Align2::CENTER_CENTER, "Drop to upload to the SFTP session", egui::FontId::proportional(18.0), Color32::WHITE);
        }
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if dropped.is_empty() {
            return;
        }
        if !sftp_active {
            self.status_msg = "Drop files on a connected SFTP session to upload them".into();
            return;
        }
        let mut n = 0;
        let mut cmds = String::new();
        for p in &dropped {
            let shown = p.to_string_lossy().replace('\\', "/").replace('"', "");
            if p.is_dir() {
                cmds.push_str(&format!("put -r \"{shown}\"\r"));
            } else {
                cmds.push_str(&format!("put \"{shown}\"\r"));
            }
            n += 1;
        }
        if let Some(s) = self.sessions.get_mut(self.active) {
            s.send_str(&cmds);
        }
        self.status_msg = format!("Uploading {n} item(s)...");
        self.focus_terminal = true;
    }

    /// Send a command string with its escapes (\r \n \t \e \b \\) and pauses (\p = one second).
    fn send_command_string(&mut self, raw: &str) {
        let mut chunks: Vec<(String, u64)> = Vec::new();
        let mut cur = String::new();
        let mut it = raw.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\\' {
                match it.peek() {
                    Some('p') => {
                        it.next();
                        chunks.push((config::unescape(&cur), 1000));
                        cur.clear();
                    }
                    Some(&o) => {
                        it.next();
                        cur.push('\\');
                        cur.push(o);
                    }
                    None => cur.push('\\'),
                }
            } else {
                cur.push(c);
            }
        }
        chunks.push((config::unescape(&cur), 0));
        for i in self.command_targets() {
            self.sessions[i].send_chunks(chunks.clone());
        }
    }

    /// The sessions the Command Window, Command Manager and button bar send to (Send Commands to ...):
    /// 0 the active session, 1 all sessions, 2 the visible ones (the session shown in each tab group, or every
    /// unfolded tile), 3 the sessions of the selected tab group.
    fn command_targets(&self) -> Vec<usize> {
        let n = self.sessions.len();
        match self.cmd_target {
            1 => (0..n).collect(),
            2 if self.layout == Layout::Tabs => self.group_current.iter().filter_map(|id| self.index_of(*id)).collect(),
            2 => (0..n).filter(|&i| !self.pane_min.contains(&self.sessions[i].id) || self.pane_max == Some(self.sessions[i].id)).collect(),
            3 => (0..n).filter(|&i| self.group_index(i) == self.cmd_group).collect(),
            _ if self.active < n => vec![self.active],
            _ => Vec::new(),
        }
    }

    fn send_to_targets(&mut self, text: &str) {
        for i in self.command_targets() {
            self.sessions[i].send_str(text);
        }
    }

    fn prompt(&mut self, kind: Prompt, title: &'static str, label: &'static str, text: String) {
        self.prompt = Some(PromptDlg { kind, title, label, text });
    }

    fn open_options(&mut self, target: EditTarget, page: &str) {
        let (cfg, title) = match target {
            EditTarget::Saved(i) => match self.settings.sessions.get(i) {
                Some(c) => (c.clone(), format!("Session Options - {}", c.display_name())),
                None => return,
            },
            EditTarget::New => {
                let mut c = self.settings.defaults_for(Protocol::Ssh);
                c.name = "New Session".into();
                (c, "Session Options - New Session".to_string())
            }
            EditTarget::Active => match self.sessions.get(self.active) {
                Some(s) => (s.cfg.clone(), format!("Session Options - {}", s.cfg.display_name())),
                None => return,
            },
            EditTarget::DefaultSession => (self.settings.default_session.clone(), "Default Session".to_string()),
            EditTarget::DefaultSerial => (self.settings.default_serial.clone(), "Serial Defaults".to_string()),
            EditTarget::DefaultLocal => (self.settings.default_local.clone(), "Local Shell Defaults".to_string()),
        };
        self.options = Some(OptionsDlg::new(target, cfg, title, page));
    }

    fn apply_options(&mut self, d: OptionsDlg) {
        match d.target {
            EditTarget::Saved(i) => {
                if i < self.settings.sessions.len() {
                    self.settings.sessions[i] = d.cfg;
                }
            }
            EditTarget::New => {
                self.settings.sessions.push(d.cfg);
                self.selected = Some(self.settings.sessions.len() - 1);
            }
            EditTarget::Active => {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    // The edit is also written to the saved session it came from.
                    let (old_name, old_host) = (s.cfg.display_name(), s.cfg.host.clone());
                    let mut cfg = d.cfg;
                    if let Some(saved) = self.settings.sessions.iter_mut().find(|x| x.display_name() == old_name && x.host == old_host) {
                        let (n, f) = (saved.name.clone(), saved.folder.clone());
                        *saved = cfg.clone();
                        saved.name = n;
                        saved.folder = f;
                    }
                    cfg.name = s.cfg.name.clone();
                    cfg.folder = s.cfg.folder.clone();
                    s.title = if cfg.tab_title.trim().is_empty() { cfg.display_name() } else { cfg.tab_title.clone() };
                    s.cfg = cfg;
                }
            }
            EditTarget::DefaultSession => self.settings.default_session = d.cfg,
            EditTarget::DefaultSerial => self.settings.default_serial = d.cfg,
            EditTarget::DefaultLocal => self.settings.default_local = d.cfg,
        }
        self.dirty = true;
    }

    // ----- shortcuts ------------------------------------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let alt = |k| KeyboardShortcut::new(Modifiers::ALT, k);
        let alt_shift = |k| KeyboardShortcut::new(Modifiers::ALT | Modifiers::SHIFT, k);
        let ctrl = |k| KeyboardShortcut::new(Modifiers::CTRL, k);
        let ctrl_alt = |k| KeyboardShortcut::new(Modifiers::CTRL | Modifiers::ALT, k);
        if ctx.input_mut(|i| i.consume_shortcut(&alt_shift(Key::L))) {
            self.local_shell(ctx);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::Q)) || i.consume_shortcut(&alt(Key::B)) || i.consume_shortcut(&alt(Key::C))) {
            self.quick_connect();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::M))) {
            self.side = if self.side == Some(Side::SessionManager) { None } else { Some(Side::SessionManager) };
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::Z))) {
            self.side = if self.side == Some(Side::ActiveSessions) { None } else { Some(Side::ActiveSessions) };
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::J))) {
            self.settings.show_command_manager = true;
            self.side = Some(Side::CommandManager);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F4)) && !self.sessions.is_empty() {
            self.request_close(self.active);
        }
        // Ctrl+Tab / Ctrl+Shift+Tab: next / previous session tab (before the terminal can see the key)
        if self.sessions.len() > 1 {
            let n = self.sessions.len();
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab)) {
                self.active = (self.active + n - 1) % n;
                self.focus_terminal = true;
            } else if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Tab)) {
                self.active = (self.active + 1) % n;
                self.focus_terminal = true;
            }
        }
        let cs = |k| KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, k);
        if self.settings.show_command_window {
            if ctx.input_mut(|i| i.consume_shortcut(&cs(Key::I))) {
                self.settings.cmd_send_immediately = !self.settings.cmd_send_immediately;
            }
            if ctx.input_mut(|i| i.consume_shortcut(&cs(Key::W))) {
                self.settings.cmd_title_bar = !self.settings.cmd_title_bar;
            }
            if ctx.input_mut(|i| i.consume_shortcut(&cs(Key::T))) {
                self.cmd_target = 0;
            }
            if ctx.input_mut(|i| i.consume_shortcut(&cs(Key::S))) {
                self.cmd_target = 1;
            }
            if ctx.input_mut(|i| i.consume_shortcut(&cs(Key::B))) {
                self.cmd_target = 2;
            }
            if self.n_groups > 1 && ctx.input_mut(|i| i.consume_shortcut(&cs(Key::G))) {
                self.cmd_target = 3;
                self.cmd_group = self.group_index(self.active);
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::Y))) {
            self.settings.show_command_manager = true;
            self.side = Some(Side::CommandManager);
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("cmd_filter")));
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::G))) {
            self.settings.show_command_window = !self.settings.show_command_window;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::R))) {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("connect_bar")));
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::U))) {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("keyword_bar")));
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::Enter))) {
            // Alt+Enter: Properties in the Session Manager, Full Screen elsewhere (Alt+Return in the original)
            match self.selected {
                Some(i) if self.side == Some(Side::SessionManager) && self.sm_hover => self.open_options(EditTarget::Saved(i), "connection"),
                _ => {
                    let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                }
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::A))) {
            for s in &mut self.sessions {
                if !s.is_connected() {
                    s.connect();
                }
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::P))) {
            if let Some(s) = self.sessions.get(self.active).filter(|s| s.cfg.protocol == Protocol::Ssh) {
                let mut cfg = s.cfg_for_clone();
                self.open_after = Some(s.id);
                cfg.protocol = Protocol::Sftp;
                cfg.name = format!("SFTP-{}", cfg.host.trim());
                self.open_session(cfg, ctx);
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::A))) {
            if let Some(s) = self.sessions.get_mut(self.active) {
                s.select_all();
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&ctrl_alt(Key::U)) || i.consume_key(Modifiers::NONE, Key::F10)) {
            self.settings.show_menu_bar = !self.settings.show_menu_bar;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&ctrl_alt(Key::T))) {
            if let Some(cfg) = self.selected.and_then(|i| self.settings.sessions.get(i)).cloned() {
                self.open_session(cfg, ctx);
            }
        }
        // Global Options > Terminal > Advanced: "Disable session zooming with keyboard shortcuts / mouse scrollwheel"
        if !self.settings.global.no_zoom_keys {
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::Equals)) || i.consume_shortcut(&ctrl(Key::Plus))) {
                self.zoom = (self.zoom + 0.1).min(3.0);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::Minus))) {
                self.zoom = (self.zoom - 0.1).max(0.5);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::Num0))) {
                self.zoom = 1.0;
            }
        }
        if !self.settings.global.no_zoom_wheel && !self.sessions.is_empty() {
            // Ctrl + mouse wheel (egui reports it as a zoom factor instead of a scroll)
            let z = ctx.input(|i| i.zoom_delta());
            if (z - 1.0).abs() > f32::EPSILON {
                self.zoom = (self.zoom * z).clamp(0.5, 3.0);
            }
        }
        // "ALT+N switches to the Nth tabbed or tiled session"
        if self.settings.global.alt_n_switch {
            let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
            for (n, k) in keys.into_iter().enumerate() {
                if n < self.sessions.len() && ctx.input_mut(|i| i.consume_shortcut(&alt(k))) {
                    self.active = n;
                    self.focus_terminal = true;
                }
            }
        }
        // Session Manager keys (only when the pointer is over it and no text box has the focus)
        if self.side == Some(Side::SessionManager) && self.sm_hover && !ctx.text_edit_focused() {
            let (copy, cut, paste) = ctx.input(|i| {
                (
                    i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
                )
            });
            if copy {
                self.tree_action(TreeAct::Copy, ctx);
            }
            if cut {
                self.tree_action(TreeAct::Cut, ctx);
            }
            if paste {
                self.tree_action(TreeAct::Paste, ctx);
            }
            if ctx.input(|i| i.key_pressed(Key::F2)) {
                self.tree_action(TreeAct::Rename, ctx);
            }
            if ctx.input(|i| i.key_pressed(Key::Delete)) {
                self.tree_action(TreeAct::Delete, ctx);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::D))) {
                self.tree_action(TreeAct::Duplicate, ctx);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::F))) {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("sm_filter")));
            }
        }
        // Command Manager keys (pointer over it, no text box in use)
        if self.side == Some(Side::CommandManager) && self.cm_hover && !ctx.text_edit_focused() {
            let (copy, cut, paste) = ctx.input(|i| {
                (
                    i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
                )
            });
            if copy {
                self.command_action(CmdAct::Copy);
            }
            if cut {
                self.command_action(CmdAct::Cut);
            }
            if paste {
                self.command_action(CmdAct::Paste);
            }
            if ctx.input(|i| i.key_pressed(Key::F2)) {
                self.command_action(CmdAct::Rename);
            }
            if ctx.input(|i| i.key_pressed(Key::Delete)) {
                self.command_action(CmdAct::Delete);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                if let Some(i) = self.cmd_sel {
                    self.command_action(CmdAct::Send(i));
                }
            }
            if ctx.input_mut(|i| i.consume_shortcut(&alt(Key::Enter))) {
                if let Some(i) = self.cmd_sel {
                    self.command_action(CmdAct::Edit(i));
                }
            }
        }
        let nums = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        for (i, k) in nums.iter().enumerate() {
            if ctx.input_mut(|inp| inp.consume_shortcut(&alt(*k))) && i < self.sessions.len() {
                self.active = i;
                self.focus_terminal = true;
            }
        }
    }

    // ----- small helpers on the active session -----------------------------------------

    fn copy_selection(&mut self, ctx: &egui::Context) {
        if let Some(s) = self.sessions.get_mut(self.active) {
            if let Some(t) = s.selected_text() {
                ctx.copy_text(t);
            }
        }
    }

    fn start_zmodem_upload(&mut self) {
        let files: Vec<std::path::PathBuf> = self.zm_list.iter().map(std::path::PathBuf::from).collect();
        match crate::transfer::Transfer::new_zmodem_send(files) {
            Ok(t) => {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    let cmd = s.cfg.zmodem_upload_cmd.trim().to_string();
                    if !cmd.is_empty() {
                        s.send(format!("{cmd}\r").as_bytes());
                    }
                    s.xfer = Some(t);
                    self.zm_list.clear();
                }
            }
            Err(e) => self.status_msg = e,
        }
    }

    fn download_dir(&self) -> std::path::PathBuf {
        let d = self.sessions.get(self.active).map(|s| s.cfg.download_dir.clone()).unwrap_or_default();
        if d.trim().is_empty() {
            config::data_dir().join("downloads")
        } else {
            std::path::PathBuf::from(d)
        }
    }

    fn toggle_auto_print(&mut self) {
        let dir = config::data_dir().join("prints");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("auto_print.txt");
        if self.auto_print_on {
            self.auto_print_on = false;
            for s in &mut self.sessions {
                s.capture_file = None;
            }
            let _ = std::process::Command::new("notepad.exe").arg("/p").arg(&path).spawn();
            self.status_msg = "Auto print finished: sent to the default printer".into();
        } else if let Some(s) = self.sessions.get_mut(self.active) {
            match s.start_capture(&path.to_string_lossy()) {
                Ok(()) => {
                    self.auto_print_on = true;
                    self.status_msg = "Auto print on".into();
                }
                Err(e) => self.status_msg = e,
            }
        }
    }

    fn print_text(&mut self, selection: bool) {
        let Some(s) = self.sessions.get_mut(self.active) else { return };
        let body = if selection { s.selected_text().unwrap_or_default() } else { s.screen_text() };
        let mut text = String::new();
        if !s.cfg.print_header.is_empty() {
            text.push_str(&s.cfg.print_header);
            text.push_str("\r\n\r\n");
        }
        text.push_str(&body.replace('\n', "\r\n"));
        if !s.cfg.print_footer.is_empty() {
            text.push_str("\r\n\r\n");
            text.push_str(&s.cfg.print_footer);
        }
        let dir = config::data_dir().join("prints");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("print.txt");
        if std::fs::write(&path, text).is_ok() {
            let _ = std::process::Command::new("notepad.exe").arg("/p").arg(&path).spawn();
            self.status_msg = "Sent to the default printer".into();
        }
    }

    fn start_script(&mut self, sid: u64, path: &str) {
        let path = path.trim().trim_matches('"').to_string();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let lines: Vec<String> = text.lines().map(|l| l.trim().to_string()).collect();
                let name = std::path::Path::new(&path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(path.clone());
                self.scripts.push(ScriptRun { sid, name, lines, pc: 0, sleep_until: None, waiting: None });
                self.settings.recent_scripts.retain(|r| r != &path);
                self.settings.recent_scripts.insert(0, path);
                self.settings.recent_scripts.truncate(6);
                self.status_msg = "Script running".into();
                self.dirty = true;
            }
            Err(e) => self.status_msg = format!("Could not read the script: {e}"),
        }
    }

    fn run_scripts(&mut self, ctx: &egui::Context) {
        if self.scripts.is_empty() {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        let mut finished = Vec::new();
        let mut connects: Vec<String> = Vec::new();
        let active_id = self.sessions.get(self.active).map(|s| s.id);
        for (n, run) in self.scripts.iter_mut().enumerate() {
            // A script started without a session works on the active session once there is one.
            let target = if run.sid == 0 { active_id } else { Some(run.sid) };
            let mut sess = target.and_then(|id| self.sessions.iter_mut().find(|s| s.id == id));
            if sess.is_none() && run.sid != 0 {
                finished.push(n);
                continue;
            }
            if let Some(t) = run.sleep_until {
                if Instant::now() < t {
                    continue;
                }
                run.sleep_until = None;
            }
            if let Some((text, started)) = &run.waiting {
                let found = sess.as_ref().map(|s| s.recent.contains(text.as_str())).unwrap_or(false);
                if found || started.elapsed() > Duration::from_secs(120) {
                    run.waiting = None;
                    run.pc += 1;
                } else {
                    continue;
                }
            }
            let mut guard = 0;
            while run.pc < run.lines.len() && guard < 200 {
                guard += 1;
                let line = run.lines[run.pc].clone();
                let (cmd, arg) = match line.split_once(char::is_whitespace) {
                    Some((c, a)) => (c.to_lowercase(), a.trim().to_string()),
                    None => (line.to_lowercase(), String::new()),
                };
                match cmd.as_str() {
                    "" => run.pc += 1,
                    c if c.starts_with('#') || c.starts_with("//") || c == "rem" => run.pc += 1,
                    "connect" => {
                        connects.push(arg);
                        run.pc += 1;
                        break; // the new session takes a moment to appear
                    }
                    "sleep" => {
                        let secs: f32 = arg.parse().unwrap_or(1.0);
                        run.sleep_until = Some(Instant::now() + Duration::from_millis((secs * 1000.0) as u64));
                        run.pc += 1;
                        break;
                    }
                    _ => {
                        let Some(s) = sess.as_mut() else { break }; // needs a session: wait for one
                        match cmd.as_str() {
                            "send" => {
                                s.send_str(&config::unescape(&arg));
                                run.pc += 1;
                            }
                            "sendln" => {
                                s.send_str(&format!("{}\r", config::unescape(&arg)));
                                run.pc += 1;
                            }
                            "waitfor" => {
                                s.recent.clear();
                                run.waiting = Some((arg, Instant::now()));
                                break;
                            }
                            _ => run.pc += 1,
                        }
                    }
                }
            }
            if run.pc >= run.lines.len() && run.waiting.is_none() && run.sleep_until.is_none() {
                finished.push(n);
            }
        }
        for n in finished.into_iter().rev() {
            let r = self.scripts.remove(n);
            self.status_msg = format!("Script {} finished", r.name);
        }
        for c in connects {
            self.connect_bar = c;
            self.connect_from_bar(ctx);
        }
    }

    // ----- menus --------------------------------------------------------------------------

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        use crate::transfer::Proto;
        let ctx = ui.ctx().clone();
        let has = !self.sessions.is_empty();
        let cur = self.sessions.get(self.active);
        let connected = cur.map(|s| s.is_connected()).unwrap_or(false);
        let logging = cur.map(|s| s.is_logging()).unwrap_or(false);
        let raw_logging = cur.map(|s| s.is_logging() && s.log_raw).unwrap_or(false);
        let paused = cur.map(|s| s.log_paused).unwrap_or(false);
        let is_ssh = cur.map(|s| s.cfg.protocol.is_ssh()).unwrap_or(false);
        let capturing = cur.map(|s| s.capture_file.is_some()).unwrap_or(false);
        let recording = cur.map(|s| s.recording.is_some()).unwrap_or(false);
        let tracing = cur.map(|s| s.cfg.ssh_verbose).unwrap_or(self.pending_trace);
        let any_down = self.sessions.iter().any(|s| !s.is_connected());
        let hs_on = cur.map(|s| s.cfg.show_hscrollbar).unwrap_or(false);
        let has_sel = cur.map(|s| s.selection.is_some()).unwrap_or(false);
        let tftp_on = self.tftp.is_some();
        let locked = cur.map(|s| s.locked.is_some()).unwrap_or(false);
        let x1k = cur.map(|s| s.cfg.xmodem_1k).unwrap_or(false);
        let vs_on = cur.map(|s| s.cfg.show_scrollbar).unwrap_or(true);
        let mut menus: Vec<egui::Response> = Vec::new();
        // Alt+letter opens a menu (the original's mnemonics) unless Alt is Meta for the session.
        let alt_open: Option<usize> = if cur.map(|s| s.cfg.meta_key).unwrap_or(false) {
            None
        } else {
            const BAR_KEYS: [egui::Key; 9] = [egui::Key::F, egui::Key::E, egui::Key::V, egui::Key::O, egui::Key::T, egui::Key::S, egui::Key::L, egui::Key::W, egui::Key::H];
            ctx.input_mut(|i| BAR_KEYS.iter().position(|k| i.consume_key(egui::Modifiers::ALT, *k)))
        };
        // the underlined letters show after Alt was pressed (while it is down, or while the menu it opened is open),
        // not for a menu opened with the pointer
        let vis_id = egui::Id::new("mnemonics_visible");
        let mut vis = ctx.data(|d| d.get_temp::<bool>(vis_id)).unwrap_or(false);
        if alt_open.is_some() || ctx.input(|i| i.modifiers.alt) {
            vis = true;
        } else if !egui::Popup::is_any_open(&ctx) {
            vis = false;
        }
        ctx.data_mut(|d| d.insert_temp(vis_id, vis));
        let bar_ul = vis;
        egui::MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().button_padding = egui::vec2(6.0, 1.0);
            menus.push(ui.menu_button(bar_title(&ctx, "File", bar_ul), |ui| {
                ui.set_min_width(260.0);
                if mi(ui, "Connect...", "Alt+C", true) {
                    self.quick_connect();
                }
                if mi(ui, "Quick Connect...", "Alt+Q", true) {
                    self.quick_connect();
                }
                if mi(ui, "Connect In Tab/Tile...", "Alt+B", true) {
                    self.quick_connect();
                }
                if mi(ui, "Connect Local Shell", "Alt+Shift+L", true) {
                    self.local_shell(&ctx);
                }
                ui.separator();
                sub_menu(ui, "Open Snapshot", |ui| {
                    if self.settings.snapshots.is_empty() {
                        ui.add_enabled(false, egui::Button::new("(none)"));
                    }
                    let names: Vec<String> = self.settings.snapshots.iter().map(|s| s.0.clone()).collect();
                    for (i, n) in names.iter().enumerate() {
                        if ui.button(n).clicked() {
                            let cfgs = self.settings.snapshots[i].1.clone();
                            for c in cfgs {
                                self.open_session(c, &ctx);
                            }
                            self.last_snapshot = Some(n.clone());
                        }
                    }
                });
                sub_menu(ui, "Save Snapshot", |ui| {
                    if mi(ui, "Update", "", self.last_snapshot.is_some() && has) {
                        if let Some(n) = self.last_snapshot.clone() {
                            let cfgs: Vec<SessionConfig> = self.sessions.iter().map(|s| s.cfg.clone()).collect();
                            if let Some(e) = self.settings.snapshots.iter_mut().find(|e| e.0 == n) {
                                e.1 = cfgs;
                            }
                            self.dirty = true;
                        }
                    }
                    if mi(ui, "Save As...", "", has) {
                        self.prompt(Prompt::SnapshotName, "Save Snapshot", "Snapshot name:", String::new());
                    }
                });
                if mi(ui, "Manage Snapshot...", "", true) {
                    self.snapshots_open = true;
                }
                ui.separator();
                if mi(ui, "Reconnect", "", has && !connected) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.connect();
                    }
                }
                if mi(ui, "Reconnect All", "Alt+A", any_down) {
                    for s in &mut self.sessions {
                        if !s.is_connected() {
                            s.connect();
                        }
                    }
                }
                if mi(ui, "Disconnect", "", connected) {
                    self.request_disconnect(self.active);
                }
                if mi(ui, "Disconnect All", "", self.sessions.iter().any(|s| s.is_connected())) {
                    self.ask("Are you sure you want to disconnect all sessions?", ConfirmAct::DisconnectAll);
                }
                ui.separator();
                if mi(ui, "Clone Session", "", has) {
                    self.clone_tab(self.active, &ctx);
                }
                if mi(ui, "Connect SFTP Session", "Alt+P", is_ssh) {
                    if let Some(s) = self.sessions.get(self.active) {
                        let mut cfg = s.cfg_for_clone();
                        self.open_after = Some(s.id);
                        cfg.protocol = Protocol::Sftp;
                        cfg.name = format!("SFTP-{}", cfg.host.trim());
                        self.open_session(cfg, &ctx);
                    }
                }
                ui.separator();
                if mi(ui, if locked { "Unlock Session" } else { "Lock Session..." }, "", has) {
                    if locked {
                        if let Some(s) = self.sessions.get_mut(self.active) {
                            s.locked = None;
                        }
                    } else {
                        self.prompt(Prompt::LockPassword, "Lock Session", "Password to unlock:", String::new());
                    }
                }
                ui.separator();
                sub_menu(ui, "Print", |ui| {
                    let auto = self.auto_print_on;
                    if mi(ui, if auto { "✔ Auto Print" } else { "Auto Print" }, "", has) {
                        self.toggle_auto_print();
                    }
                    if mi(ui, "Screen", "", has) {
                        self.print_text(false);
                    }
                    if mi(ui, "Selection", "", has) {
                        self.print_text(true);
                    }
                    mi(ui, "Eject page", "", false);
                    mi(ui, "Cancel", "", false);
                });
                if mi(ui, "Page Setup...", "", has) {
                    self.open_options(EditTarget::Active, "printing");
                }
                ui.separator();
                // These three can be turned on before any session exists: they apply to the next one.
                let mut log_on = if has { logging } else { self.pending_log };
                if ui.checkbox(&mut log_on, "Log Session").clicked() {
                    if has {
                        if let Some(s) = self.sessions.get_mut(self.active) {
                            if s.is_logging() {
                                s.stop_log();
                                self.status_msg = "Logging stopped".into();
                            } else {
                                let p = s.default_log_path();
                                self.prompt(Prompt::LogFile, "Log Session", "Log file:", p);
                            }
                        }
                    } else {
                        self.pending_log = log_on;
                    }
                }
                if mi(ui, if paused { "Resume Logging" } else { "Pause Logging" }, "", logging) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.log_paused = !s.log_paused;
                    }
                }
                let mut raw_on = if has { raw_logging } else { self.pending_raw };
                if ui.checkbox(&mut raw_on, "Raw Log Session").clicked() {
                    if has {
                        if let Some(s) = self.sessions.get_mut(self.active) {
                            if s.is_logging() && s.log_raw {
                                s.stop_log();
                            } else {
                                if s.is_logging() {
                                    s.stop_log();
                                }
                                let p = s.default_log_path().replace(".log", ".raw.log");
                                s.log_raw = true;
                                self.status_msg = match s.start_log(Some(p)) {
                                    Ok(p) => format!("Raw logging to {p}"),
                                    Err(e) => e,
                                };
                            }
                        }
                    } else {
                        self.pending_raw = raw_on;
                    }
                }
                let mut trace_on = tracing;
                if ui.checkbox(&mut trace_on, "Trace Options").clicked() {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.cfg.ssh_verbose = trace_on;
                        self.status_msg = "SSH debug messages apply the next time this tab connects".into();
                    } else {
                        self.pending_trace = trace_on;
                    }
                }
                ui.separator();
                if self.settings.recent.is_empty() {
                    mi(ui, "Recent Session", "", false);
                } else {
                    sub_menu(ui, "Recent Session", |ui| {
                        let recent = self.settings.recent.clone();
                        for r in recent {
                            if ui.button(&r).clicked() {
                                if let Some(cfg) = self.settings.sessions.iter().find(|s| s.display_name() == r).cloned() {
                                    self.open_session(cfg, &ctx);
                                }
                            }
                        }
                    });
                }
                ui.separator();
                if mi(ui, "Open Scratchpad", "", true) {
                    if self.scratch.is_none() {
                        self.scratch = Some(String::new());
                    }
                }
                if mi(ui, "Open File...", "", true) {
                    ui.close();
                    if let Some(p) = config::browse_file("Open File", None) {
                        match std::fs::read_to_string(&p) {
                            Ok(t) => self.scratch = Some(t),
                            Err(e) => self.error_msg = Some(format!("Could not read the file:\n{e}")),
                        }
                    }
                }
                ui.separator();
                if mi(ui, "Exit", "", true) {
                    self.request_exit(&ctx);
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Edit", bar_ul), |ui| {
                ui.set_min_width(300.0);
                if mi(ui, "Copy", "Ctrl+Ins", true) {
                    self.copy_selection(&ctx);
                }
                if mi(ui, "Paste", "Shift+Ins", true) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    self.focus_terminal = true;
                }
                if mi(ui, "Copy and Paste", "", has_sel) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        if let Some(t) = s.selected_text() {
                            s.send_str(&t);
                        }
                    }
                }
                if mi(ui, "Paste as Quotation", "", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.paste_quote = true;
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    self.focus_terminal = true;
                }
                if mi(ui, "Select All", "Ctrl+Shift+A", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.select_all();
                    }
                }
                if mi(ui, "Find...", "", has) {
                    self.find.open = true;
                }
                ui.separator();
                if mi(ui, "Print Selection", "", true) {
                    self.print_text(true);
                }
                ui.separator();
                if mi(ui, "Go to Command Window", "Alt+G", true) {
                    self.settings.show_command_window = true;
                    ctx.memory_mut(|m| m.request_focus(egui::Id::new("command_line")));
                }
                if mi(ui, "Go to Session Manager", "Alt+M", true) {
                    self.side = Some(Side::SessionManager);
                }
                if mi(ui, "Go to Command Manager", "Alt+J", true) {
                    self.settings.show_command_manager = true;
                    self.side = Some(Side::CommandManager);
                }
                if mi(ui, "Go to Active Sessions", "Alt+Z", true) {
                    self.side = Some(Side::ActiveSessions);
                }
                ui.separator();
                if mi(ui, "Clear Scrollback", "", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.clear_scrollback();
                    }
                }
                if mi(ui, "Clear Screen", "", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.clear_screen();
                    }
                }
                if mi(ui, "Clear Screen and Scrollback", "", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.clear_all();
                    }
                }
                ui.separator();
                if mi(ui, "Send Break", "", connected) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.send_break();
                    }
                }
                if mi(ui, "Reset", "", true) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.reset();
                    }
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "View", bar_ul), |ui| {
                ui.set_min_width(250.0);
                let st = &mut self.settings;
                mcheck(ui, &mut st.show_menu_bar, "Menu Bar", true);
                mcheck(ui, &mut st.show_toolbar, "Toolbar", true);
                mcheck(ui, &mut st.show_command_window, "Command Window", true);
                mcheck(ui, &mut st.show_connect_bar, "Connect Bar", true);
                mcheck(ui, &mut st.show_status_bar, "Status Bar", true);
                mcheck(ui, &mut st.show_button_bar, "Button Bar", true);
                mcheck(ui, &mut st.show_tabs, "Session Tabs", true);
                mcheck(ui, &mut st.show_tab_sm, "Session Manager", true);
                mcheck(ui, &mut st.show_command_manager, "Command Manager", true);
                mcheck(ui, &mut st.show_tab_as, "Active Sessions", true);
                ui.separator();
                mcheck(ui, &mut st.show_hex_view, "Hex View", true);
                ui.separator();
                sub_menu(ui, "Themes", |ui| {
                    for key in ["System", "Native", "Carbon", "VS Blue", "VS Dark", "VS White", "Office White"] {
                        if mradio(ui, self.settings.theme == key, key) {
                            self.settings.theme = key.to_string();
                            apply_theme(&ctx, key);
                            self.dirty = true;
                        }
                    }
                });
                sub_menu(ui, "Zoom", |ui| {
                    if mi(ui, "Zoom in", "Ctrl+=", true) {
                        self.zoom = (self.zoom + 0.1).min(3.0);
                    }
                    if mi(ui, "Zoom out", "Ctrl+-", true) {
                        self.zoom = (self.zoom - 0.1).max(0.5);
                    }
                    ui.separator();
                    if mi(ui, "100%", "Ctrl+0", true) {
                        self.zoom = 1.0;
                    }
                });
                ui.separator();
                let mut tr = self.transparent;
                if mcheck(ui, &mut tr, "Window Transparency", true) {
                    self.transparent = tr;
                    config::set_window_opacity(if tr { 200 } else { 255 });
                }
                ui.separator();
                let mut hs = hs_on;
                if mcheck(ui, &mut hs, "Horizontal Scroll Bar", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.cfg.show_hscrollbar = hs;
                    }
                }
                let mut vs = vs_on;
                if mcheck(ui, &mut vs, "Vertical Scroll Bar", has) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.cfg.show_scrollbar = vs;
                    }
                }
                ui.separator();
                let mut top = self.always_on_top;
                if mcheck(ui, &mut top, "Always on Top", true) {
                    self.always_on_top = top;
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal }));
                }
                let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if mi(ui, if full { "✓ Full Screen" } else { "Full Screen" }, "Alt+Enter", true) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Options", bar_ul), |ui| {
                ui.set_min_width(260.0);
                if mi(ui, "Session Options...", "", has) {
                    self.open_options(EditTarget::Active, "connection");
                }
                sub_menu(ui, "Configure", |ui| {
                    if mi(ui, "Default Session...", "", true) {
                        self.open_options(EditTarget::DefaultSession, "connection");
                    }
                    if mi(ui, "Local Shell Defaults...", "", true) {
                        self.open_options(EditTarget::DefaultLocal, "connection");
                    }
                    if mi(ui, "Serial Defaults...", "", true) {
                        self.open_options(EditTarget::DefaultSerial, "connection");
                    }
                    mi(ui, "RDP Defaults...", "", false);
                });
                if mi(ui, "Global Options...", "", true) {
                    self.global_open = true;
                }
                ui.separator();
                let mut kh = self.settings.keyword_highlighting;
                if mcheck(ui, &mut kh, "Keyword Highlighting", has) {
                    self.settings.keyword_highlighting = kh;
                    self.dirty = true;
                }
                if mi(ui, "Keyword Highlighting Properties...", "", has) {
                    self.open_options(EditTarget::Active, "keywords");
                }
                mcheck(ui, &mut self.settings.use_firewall, "Use Firewall", true);
                ui.separator();
                mcheck(ui, &mut self.settings.auto_save, "Auto Save Options", true);
                if mi(ui, "Save Settings Now", "", true) {
                    self.settings.save();
                    self.status_msg = "Settings saved".into();
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Transfer", bar_ul), |ui| {
                ui.set_min_width(240.0);
                if mi(ui, "Send ASCII...", "", has) {
                    self.prompt(Prompt::SendAscii, "Send ASCII", "Text file to send:", String::new());
                }
                if mi(ui, if capturing { "✓ Receive ASCII..." } else { "Receive ASCII..." }, "", has) {
                    if capturing {
                        if let Some(s) = self.sessions.get_mut(self.active) {
                            s.capture_file = None;
                        }
                        self.status_msg = "Stopped receiving".into();
                    } else {
                        let base = self.download_dir().to_string_lossy().into_owned();
                        self.prompt(Prompt::ReceiveAscii, "Receive ASCII", "Save received text to:", format!("{base}\\received.txt"));
                    }
                }
                ui.separator();
                if mi(ui, "Send Binary...", "", has) {
                    self.prompt(Prompt::SendBinary, "Send Binary", "File to send:", String::new());
                }
                ui.separator();
                if mi(ui, "Send Kermit...", "", has) {
                    self.prompt(Prompt::XferSend(Proto::Kermit), "Send Kermit", "File to send:", String::new());
                }
                if mi(ui, "Receive Kermit...", "", has) {
                    let p = self.download_dir().to_string_lossy().into_owned();
                    self.prompt(Prompt::XferRecv(Proto::Kermit), "Receive Kermit", "Save received files in this folder:", p);
                }
                ui.separator();
                if mi(ui, "Send Xmodem...", "", has) {
                    self.prompt(Prompt::XferSend(if x1k { Proto::Xmodem1k } else { Proto::Xmodem }), "Send Xmodem", "File to send:", String::new());
                }
                if mi(ui, "Receive Xmodem...", "", has) {
                    let p = self.download_dir().join("xmodem.bin").to_string_lossy().into_owned();
                    self.prompt(Prompt::XferRecv(Proto::Xmodem), "Receive Xmodem", "Save the received file as:", p);
                }
                ui.separator();
                if mi(ui, "Send Ymodem...", "", has) {
                    self.prompt(Prompt::XferSend(Proto::Ymodem), "Send Ymodem", "File to send:", String::new());
                }
                if mi(ui, "Receive Ymodem...", "", has) {
                    let p = self.download_dir().to_string_lossy().into_owned();
                    self.prompt(Prompt::XferRecv(Proto::Ymodem), "Receive Ymodem", "Save received files in this folder:", p);
                }
                ui.separator();
                if mi(ui, "Zmodem Upload List...", "", true) {
                    self.zm_open = true;
                }
                if mi(ui, "Start Zmodem Upload", "", has && !self.zm_list.is_empty()) {
                    self.start_zmodem_upload();
                }
                ui.separator();
                // The TFTP server does not need a session: it can be started at any time.
                if mi(ui, if tftp_on { "Stop TFTP Server" } else { "Start TFTP Server" }, "", true) {
                    if tftp_on {
                        self.tftp = None;
                        self.status_msg = "TFTP server stopped".into();
                    } else {
                        let p = config::data_dir().join("tftp").to_string_lossy().into_owned();
                        self.prompt(Prompt::TftpFolder, "Start TFTP Server", "Folder to serve (files are sent from and saved to it):", p);
                    }
                }
                if tftp_on && mi(ui, "TFTP Server Activity...", "", true) {
                    self.tftp_open = true;
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Script", bar_ul), |ui| {
                ui.set_min_width(240.0);
                if mi(ui, "Run...", "", true) {
                    // The original opens the Windows file dialog straight away.
                    ui.close();
                    if let Some(p) = crate::nativedlg::open_script("Select Script to Run") {
                        if let Some(s) = self.sessions.get(self.active) {
                            let id = s.id;
                            self.start_script(id, &p);
                        } else {
                            self.start_script(0, &p);
                        }
                    }
                }
                if mi(ui, "Edit...", "", true) {
                    ui.close();
                    if let Some(p) = crate::nativedlg::open_script("Select Script to Edit") {
                        let _ = std::process::Command::new("notepad.exe").arg(&p).spawn();
                    }
                }
                if mi(ui, "Cancel", "", !self.scripts.is_empty()) {
                    self.scripts.clear();
                    self.status_msg = "Script cancelled".into();
                }
                ui.separator();
                if mi(ui, "Start Recording Script", "", !recording && !self.rec_wait) {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.recording = Some(Vec::new());
                        self.rec_target = Some(s.id);
                        self.status_msg = "Recording script".into();
                    } else {
                        self.rec_wait = true;
                        self.status_msg = "Recording starts when a session is opened".into();
                    }
                }
                if mi(ui, "Stop Recording Script...", "", recording) {
                    let p = config::data_dir().join("scripts").join("recorded.txt").to_string_lossy().into_owned();
                    self.prompt(Prompt::SaveScript, "Save Recorded Script", "Save script to:", p);
                }
                if mi(ui, "Cancel Recording Script", "", recording || self.rec_wait) {
                    self.rec_wait = false;
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.recording = None;
                    }
                }
                ui.separator();
                if self.settings.recent_scripts.is_empty() {
                    mi(ui, "Recent Script", "", false);
                } else {
                    sub_menu(ui, "Recent Script", |ui| {
                        let list = self.settings.recent_scripts.clone();
                        for p in list {
                            if ui.button(&p).clicked() {
                                let id = self.sessions.get(self.active).map(|s| s.id).unwrap_or(0);
                                self.start_script(id, &p);
                            }
                        }
                    });
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Tools", bar_ul), |ui| {
                ui.set_min_width(320.0);
                if mi(ui, "Keymap Editor...", "", true) {
                    self.keymap_open = true;
                }
                if mi(ui, "Create Public Key...", "", true) {
                    self.tool_create_public_key(&ctx);
                }
                if mi(ui, "Convert Private Key to OpenSSH Format...", "", true) {
                    self.prompt(Prompt::ConvertKey, "Convert Private Key", "Private key file:", crate::config::ssh_dir());
                }
                if mi(ui, "Export Public Key from Certificate...", "", true) {
                    ui.close();
                    if let Some(p) = config::browse_file("Select the certificate (PEM, DER, PFX or P12)", None) {
                        match crate::certkey::public_key_from_certificate(&p) {
                            Ok(line) => {
                                if let Some(dest) = config::browse_save("Export Public Key") {
                                    self.status_msg = match std::fs::write(&dest, format!("{line}\n")) {
                                        Ok(()) => format!("Public key exported to {dest}"),
                                        Err(e) => format!("Could not write {dest}: {e}"),
                                    };
                                }
                            }
                            Err(e) => self.error_msg = Some(e),
                        }
                    }
                }
                if mi(ui, "Public-Key Assistant...", "", has) {
                    self.keyassist_open = true;
                }
                ui.separator();
                if mi(ui, "Manage Agent Keys...", "", true) {
                    self.tool_manage_agent_keys(&ctx);
                }
                if mi(ui, "Manage Saved Credentials...", "", true) {
                    self.creds_open = true;
                }
                if self.settings.credentials.is_empty() {
                    mi(ui, "Change Credentials Password", "", false);
                } else {
                    sub_menu(ui, "Change Credentials Password", |ui| {
                        let names: Vec<String> = self.settings.credentials.iter().map(|c| c.name.clone()).collect();
                        for (i, n) in names.iter().enumerate() {
                            if ui.button(n).clicked() {
                                self.prompt(Prompt::CredPassword(i), "Change Credentials Password", "New password:", String::new());
                            }
                        }
                    });
                }
                ui.separator();
                if mi(ui, "Change Configuration Passphrase...", "", true) {
                    self.pass_dlg = Some(PassDlg::default());
                }
                ui.separator();
                if mi(ui, "Export Settings...", "", true) {
                    let p = config::data_dir().join("settings-export.json").to_string_lossy().into_owned();
                    self.prompt(Prompt::ExportSettings, "Export Settings", "Export to:", p);
                }
                if mi(ui, "Import Settings from XML File...", "", true) {
                    self.prompt(Prompt::ImportSettings, "Import Settings", "Settings file:", String::new());
                }
                if mi(ui, "Import Settings from Text File...", "", true) {
                    self.prompt(Prompt::ImportSettings, "Import Settings", "Settings file:", String::new());
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Window", bar_ul), |ui| {
                ui.set_min_width(210.0);
                if mradio(ui, self.layout == Layout::Tabs, "Tabs") {
                    self.layout = Layout::Tabs;
                }
                if mradio(ui, self.layout == Layout::TileV, "Tile Vertically") {
                    self.layout = Layout::TileV;
                }
                if mradio(ui, self.layout == Layout::TileH, "Tile Horizontally") {
                    self.layout = Layout::TileH;
                }
                if mradio(ui, self.layout == Layout::Cascade, "Cascade") {
                    self.layout = Layout::Cascade;
                }
                ui.separator();
                if mi(ui, "Single Tab Group", "", self.n_groups > 1) {
                    // every tab goes back to the one tab bar, keeping its order
                    self.group_of.clear();
                    self.n_groups = 1;
                }
                if has {
                    ui.separator();
                }
                for i in 0..self.sessions.len() {
                    let label = format!("{} {}", i + 1, self.sessions[i].title);
                    if ui.radio(i == self.active, label).clicked() {
                        self.active = i;
                        self.focus_terminal = true;
                    }
                }
            }).response);
            menus.push(ui.menu_button(bar_title(&ctx, "Help", bar_ul), |ui| {
                ui.set_min_width(200.0);
                if mi_plain(ui, "About Yetty...", true) {
                    self.about_open = true;
                }
            }).response);
        });
        if let Some(i) = alt_open {
            for (j, r) in menus.iter().enumerate() {
                let id = egui::Popup::default_response_id(r);
                if j == i {
                    egui::Popup::open_id(&ctx, id);
                } else {
                    egui::Popup::close_id(&ctx, id);
                }
            }
            ctx.request_repaint();
        }
        // Like the original: while a menu is open, moving the pointer over another menu opens that one.
        let any_open = menus.iter().any(|r| egui::Popup::is_id_open(&ctx, egui::Popup::default_response_id(r)));
        if any_open {
            if let Some(p) = ctx.pointer_hover_pos() {
                for r in &menus {
                    let id = egui::Popup::default_response_id(r);
                    if r.rect.contains(p) && !egui::Popup::is_id_open(&ctx, id) {
                        egui::Popup::open_id(&ctx, id);
                        ctx.request_repaint();
                        break;
                    }
                }
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let has = !self.sessions.is_empty();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "\u{E839}", "Connect... (Alt+C)", true) {
                self.quick_connect();
            }
            if icon_button(ui, "\u{E945}", "Quick Connect (Alt+Q)", true) {
                self.quick_connect();
            }
            if icon_button(ui, "\u{E756}", "Connect Local Shell (Alt+Shift+L)", true) {
                self.local_shell(&ctx);
            }
            if icon_button(ui, "\u{E8EE}", "Reconnect", has) {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    s.connect();
                }
            }
            if icon_button(ui, "\u{E8CD}", "Disconnect", has) {
                self.request_disconnect(self.active);
            }
            if self.settings.show_connect_bar {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.connect_bar)
                        .id(egui::Id::new("connect_bar"))
                        .hint_text("Enter host <Alt+R>")
                        .desired_width(145.0),
                );
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.connect_from_bar(&ctx);
                }
            }
            tb_sep(ui);
            if icon_button(ui, "\u{E8C8}", "Copy", has) {
                self.copy_selection(&ctx);
            }
            if icon_button(ui, "\u{E77F}", "Paste", has) {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                self.focus_terminal = true;
            }
            if icon_button(ui, "\u{E721}", "Find", has) {
                self.find.open = true;
            }
            let r = ui.add(egui::TextEdit::singleline(&mut self.keyword_bar).id(egui::Id::new("keyword_bar")).hint_text("Keyword <Alt+U>").desired_width(120.0));
            let go = icon_button(ui, "\u{E721}", "Find keyword", has) || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go && !self.keyword_bar.trim().is_empty() {
                let q = self.keyword_bar.clone();
                if let Some(s) = self.sessions.get_mut(self.active) {
                    let found = s.find(&q, false, false);
                    self.status_msg = if found { "Ready".into() } else { format!("\"{q}\" not found") };
                }
            }
            tb_sep(ui);
            if icon_button(ui, "\u{E749}", "Print Screen", has) {
                self.print_text(false);
            }
            tb_sep(ui);
            if icon_button(ui, "\u{E713}", "Session Options", has) {
                self.open_options(EditTarget::Active, "connection");
            }
            if icon_button(ui, "\u{E765}", "Keymap Editor", true) {
                self.keymap_open = true;
            }
            if icon_button(ui, "\u{E8D7}", "Public-Key Assistant", true) {
                self.keyassist_open = true;
            }
            tb_sep(ui);
            if icon_button(ui, "\u{E897}", "About Yetty", true) {
                self.about_open = true;
            }
        });
    }

    // ----- side panels --------------------------------------------------------------------

    fn side_stripe(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let sm = self.side == Some(Side::SessionManager);
        if self.settings.show_tab_sm && side_tab(ui, "Session Manager", sm).clicked() {
            self.side = if sm { None } else { Some(Side::SessionManager) };
        }
        let cm = self.side == Some(Side::CommandManager);
        if self.settings.show_command_manager && side_tab(ui, "Command Manager", cm).clicked() {
            self.side = if cm { None } else { Some(Side::CommandManager) };
        }
        let asn = self.side == Some(Side::ActiveSessions);
        if self.settings.show_tab_as && side_tab(ui, "Active Sessions", asn).clicked() {
            self.side = if asn { None } else { Some(Side::ActiveSessions) };
        }
    }

    fn side_header(&mut self, ui: &mut egui::Ui, title: &str) {
        // the blue line on top of a docked panel in the original
        let top = ui.max_rect();
        ui.painter().hline(top.x_range().expand(4.0), top.min.y - 3.0, egui::Stroke::new(2.0, Color32::from_rgb(0x00, 0x78, 0xd4)));
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, "\u{E711}", "Close", true) {
                    self.side = None;
                }
                if icon_button(ui, if self.pinned { "\u{E840}" } else { "\u{E718}" }, "Auto Hide", true) {
                    self.pinned = !self.pinned;
                }
                // dotted grip between the title and the buttons
                let w = ui.available_width().max(0.0);
                let (r, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), Sense::hover());
                let mut x = r.min.x + 4.0;
                while x < r.max.x {
                    ui.painter().circle_filled(Pos2::new(x, r.center().y), 0.5, Color32::from_gray(110));
                    x += 3.0;
                }
            });
        });
    }

    fn session_manager(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.sm_hover = ui.rect_contains_pointer(ui.max_rect());
        self.side_header(ui, "Session Manager");
        let sel_ok = self.selected.map(|i| i < self.settings.sessions.len()).unwrap_or(false);
        let mut tact: Option<TreeAct> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "\u{E71B}", "Connect", sel_ok) {
                if let Some(i) = self.selected {
                    tact = Some(TreeAct::Connect(i));
                }
            }
            if icon_button(ui, "\u{E8F4}", "New Folder", true) {
                tact = Some(TreeAct::NewFolder);
            }
            if icon_button(ui, "\u{E710}", "New Session", true) {
                self.new_session();
            }
            if icon_button(ui, "\u{E8C6}", "Cut", sel_ok) {
                tact = Some(TreeAct::Cut);
            }
            if icon_button(ui, "\u{E8C8}", "Copy", sel_ok) {
                tact = Some(TreeAct::Copy);
            }
            if icon_button(ui, "\u{E77F}", "Paste", self.clip.is_some()) {
                tact = Some(TreeAct::Paste);
            }
            if icon_button(ui, "\u{E711}", "Delete", sel_ok) {
                tact = Some(TreeAct::Delete);
            }
            if icon_button(ui, "\u{E713}", "Properties", sel_ok) {
                tact = Some(TreeAct::Properties);
            }
            if icon_button(ui, "\u{E721}", "Find", true) {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("sm_filter")));
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .id(egui::Id::new("sm_filter"))
                .hint_text("Filter by folder/session name <Alt+I>")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(2.0);
        let tree = build_tree(&self.settings.sessions, &self.settings.folders, &self.filter, self.settings.global.sort_order);
        let mut tc = TreeCtx {
            sessions: &self.settings.sessions,
            selected: self.selected,
            selected_folder: self.selected_folder.clone(),
            sel_default: self.sel_default,
            act: tact,
            expand: self.expand.take(),
            show_desc: self.settings.show_descriptions,
            has_clip: self.clip.is_some(),
            filtering: !self.filter.is_empty(),
            open_list: self.settings.tree_open.clone(),
            open_changed: false,
        };
        egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(70))).inner_margin(2).show(ui, |ui| {
            ui.set_min_height(ui.available_height() - 4.0);
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                draw_node(ui, &tree, &mut tc, true);
            });
        });
        let act = tc.act.take();
        let (open_list, open_changed) = (std::mem::take(&mut tc.open_list), tc.open_changed);
        self.selected = tc.selected;
        self.sel_default = tc.sel_default;
        self.selected_folder = tc.selected_folder.clone();
        if open_changed {
            self.settings.tree_open = open_list;
            self.dirty = true;
        }
        if let Some(a) = act {
            self.tree_action(a, &ctx);
        }
    }

    fn selected_folder_path(&self) -> String {
        if let Some(f) = &self.selected_folder {
            return f.clone();
        }
        self.selected.and_then(|i| self.settings.sessions.get(i)).map(|s| s.folder.clone()).unwrap_or_default()
    }

    fn tree_action(&mut self, a: TreeAct, ctx: &egui::Context) {
        if matches!(a, TreeAct::Delete) {
            let folder = self.selected_folder.as_ref().map(|f| !f.is_empty()).unwrap_or(false);
            let session = self.selected.map(|i| i < self.settings.sessions.len()).unwrap_or(false);
            if folder || session {
                self.ask("Are you sure you want to delete the selected item(s)?", ConfirmAct::DeleteTree);
            }
            return;
        }
        self.tree_action_now(a, ctx);
    }

    fn tree_action_now(&mut self, a: TreeAct, ctx: &egui::Context) {
        match a {
            TreeAct::Connect(i) => {
                if let Some(cfg) = self.settings.sessions.get(i).cloned() {
                    self.open_session(cfg, ctx);
                }
            }
            TreeAct::ConnectGroup(i) => {
                if let Some(cfg) = self.settings.sessions.get(i).cloned() {
                    let before = self.sessions.len();
                    self.open_session(cfg, ctx);
                    // RDP opens its own window and adds no tab
                    if self.sessions.len() > before && before > 0 {
                        let id = self.sessions[self.active].id;
                        self.group_of.insert(id, self.n_groups);
                        self.n_groups += 1;
                        self.group_current.push(id);
                    }
                }
            }
            TreeAct::ConnectSftp(i) => {
                if let Some(mut cfg) = self.settings.sessions.get(i).cloned() {
                    cfg.protocol = Protocol::Sftp;
                    cfg.name = format!("SFTP-{}", cfg.host.trim());
                    self.open_session(cfg, ctx);
                }
            }
            TreeAct::NewWindow(i) => {
                if let Some(cfg) = self.settings.sessions.get(i).cloned() {
                    let name = cfg.display_name();
                    self.open_new_window(&cfg, Some(&name));
                }
            }
            TreeAct::Cut | TreeAct::Copy => {
                if let Some(i) = self.selected {
                    self.clip = Some((vec![i], matches!(a, TreeAct::Cut)));
                }
            }
            TreeAct::CopyPath => {
                if let Some(s) = self.selected.and_then(|i| self.settings.sessions.get(i)) {
                    let p = if s.folder.is_empty() { format!("Sessions/{}", s.display_name()) } else { format!("Sessions/{}/{}", s.folder, s.display_name()) };
                    ctx.copy_text(p);
                }
            }
            TreeAct::Duplicate => {
                if let Some(i) = self.selected {
                    if let Some(c) = self.settings.sessions.get(i) {
                        let suggestion = self.unique_session_name(&c.folder, &c.display_name());
                        self.prompt(Prompt::DuplicateSession(i), "Duplicate Session", "Name of the new session:", suggestion);
                    }
                }
            }
            TreeAct::Paste => {
                if let Some((idx, cut)) = self.clip.clone() {
                    let folder = self.selected_folder_path();
                    for i in idx {
                        if let Some(mut c) = self.settings.sessions.get(i).cloned() {
                            c.folder = folder.clone();
                            if cut {
                                self.settings.sessions[i] = c;
                            } else {
                                c.name = self.unique_session_name(&c.folder, &c.display_name());
                                self.settings.sessions.push(c);
                            }
                        }
                    }
                    if cut {
                        self.clip = None;
                    }
                    self.dirty = true;
                }
            }
            TreeAct::Find => ctx.memory_mut(|m| m.request_focus(egui::Id::new("sm_filter"))),
            TreeAct::FindNext => {}
            TreeAct::Delete => {
                if let Some(f) = self.selected_folder.clone().filter(|f| !f.is_empty()) {
                    self.settings.sessions.retain(|s| s.folder != f && !s.folder.starts_with(&format!("{f}/")));
                    self.settings.folders.retain(|x| x != &f && !x.starts_with(&format!("{f}/")));
                    self.selected_folder = None;
                    self.dirty = true;
                } else if let Some(i) = self.selected {
                    if i < self.settings.sessions.len() {
                        self.settings.sessions.remove(i);
                        self.selected = None;
                        self.dirty = true;
                    }
                }
            }
            TreeAct::Rename => {
                if let Some(f) = self.selected_folder.clone().filter(|f| !f.is_empty()) {
                    let last = f.rsplit('/').next().unwrap_or("").to_string();
                    self.prompt(Prompt::RenameFolder(f), "Rename Folder", "New name:", last);
                } else if let Some(i) = self.selected {
                    if let Some(s) = self.settings.sessions.get(i) {
                        let n = s.display_name();
                        self.prompt(Prompt::RenameSession(i), "Rename Session", "New name:", n);
                    }
                }
            }
            TreeAct::Shortcut => {
                if let Some(s) = self.selected.and_then(|i| self.settings.sessions.get(i)) {
                    if let Ok(exe) = std::env::current_exe() {
                        let name: String = s.display_name().chars().map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
                        let script = format!(
                            "$d=[Environment]::GetFolderPath('Desktop');$w=New-Object -ComObject WScript.Shell;$l=$w.CreateShortcut(\"$d\\{}.lnk\");$l.TargetPath='{}';$l.Arguments='--session \"{}\"';$l.Save()",
                            name.replace('\'', "''"),
                            exe.display().to_string().replace('\'', "''"),
                            s.display_name().replace('\'', "''")
                        );
                        let _ = std::process::Command::new("powershell.exe").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]).spawn();
                        self.status_msg = "Desktop shortcut created".into();
                    }
                }
            }
            TreeAct::ExpandAll => self.expand = Some(true),
            TreeAct::CollapseAll => self.expand = Some(false),
            TreeAct::Refresh => {
                self.settings.save();
                self.settings = Settings::load();
            }
            TreeAct::Sort(asc) => {
                self.settings.sessions.sort_by_key(|s| s.display_name().to_lowercase());
                if !asc {
                    self.settings.sessions.reverse();
                }
                self.selected = None;
                self.dirty = true;
            }
            TreeAct::ToggleDescriptions => self.settings.show_descriptions = !self.settings.show_descriptions,
            TreeAct::Export => {
                let p = config::data_dir().join("sessions-export.json").to_string_lossy().into_owned();
                self.prompt(Prompt::ExportSessions, "Export Session(s)", "Export to:", p);
            }
            TreeAct::Move(item, target) => match item {
                DragItem::Session(i) => {
                    if let Some(s) = self.settings.sessions.get_mut(i) {
                        if s.folder.trim_matches('/') != target.trim_matches('/') {
                            s.folder = target.clone();
                            self.dirty = true;
                            self.status_msg = if target.is_empty() { "Session moved to Sessions".into() } else { format!("Session moved to {target}") };
                        }
                    }
                }
                DragItem::Folder(from) => {
                    let last = from.rsplit('/').next().unwrap_or("").to_string();
                    let new = if target.is_empty() { last.clone() } else { format!("{target}/{last}") };
                    if new == from {
                        // dropped where it already is
                    } else if target == from || target.starts_with(&format!("{from}/")) {
                        self.error_msg = Some("A folder cannot be moved into itself.".into());
                    } else if self.settings.folders.iter().any(|f| f.eq_ignore_ascii_case(&new)) || self.settings.sessions.iter().any(|s| s.folder.eq_ignore_ascii_case(&new)) {
                        self.error_msg = Some(format!("A folder named \"{last}\" already exists there."));
                    } else {
                        for s in &mut self.settings.sessions {
                            if s.folder == from {
                                s.folder = new.clone();
                            } else if let Some(rest) = s.folder.strip_prefix(&format!("{from}/")) {
                                s.folder = format!("{new}/{rest}");
                            }
                        }
                        for f in &mut self.settings.folders {
                            if *f == from {
                                *f = new.clone();
                            } else if let Some(rest) = f.strip_prefix(&format!("{from}/")) {
                                *f = format!("{new}/{rest}");
                            }
                        }
                        if !self.settings.folders.contains(&new) {
                            self.settings.folders.push(new.clone());
                        }
                        self.selected_folder = Some(new);
                        self.dirty = true;
                    }
                }
            },
            TreeAct::Properties => {
                if let Some(n) = self.sel_default.filter(|_| self.selected.is_none()) {
                    self.open_options(if n == 0 { EditTarget::DefaultLocal } else { EditTarget::DefaultSerial }, "connection");
                } else if let Some(i) = self.selected {
                    self.open_options(EditTarget::Saved(i), "connection");
                }
            }
            TreeAct::NewFolder => {
                let base = self.selected_folder_path();
                let hint = if base.is_empty() { String::new() } else { format!("{base}/") };
                self.prompt(Prompt::NewFolder, "New Folder", "Folder name:", hint);
            }
        }
    }

    fn active_sessions_panel(&mut self, ui: &mut egui::Ui) {
        self.side_header(ui, "Active Sessions");
        let mut pick = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, s) in self.sessions.iter().enumerate() {
                let dot = match s.status {
                    Status::Connected => Color32::from_rgb(0x2e, 0xa0, 0x43),
                    Status::Connecting => Color32::from_rgb(0xe0, 0xa0, 0x00),
                    Status::Disconnected(_) => Color32::from_rgb(0xc8, 0x32, 0x32),
                };
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 14.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, dot);
                    if ui.selectable_label(i == self.active, &s.title).clicked() {
                        pick = Some(i);
                    }
                });
            }
            if self.sessions.is_empty() {
                ui.weak("No active sessions.");
            }
        });
        if let Some(i) = pick {
            self.active = i;
            self.focus_terminal = true;
        }
    }

    fn command_manager(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.cm_hover = ui.rect_contains_pointer(ui.max_rect());
        self.side_header(ui, "Command Manager");
        let sel_ok = self.cmd_sel.map(|i| i < self.settings.commands.len()).unwrap_or(false);
        let has_folder = self.cmd_sel_folder.as_ref().map(|f| !f.is_empty()).unwrap_or(false);
        let mut act: Option<CmdAct> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "➤", "Send Command", sel_ok) {
                act = Some(CmdAct::Send(self.cmd_sel.unwrap_or(0)));
            }
            if icon_button(ui, "➕", "Add Command", true) {
                act = Some(CmdAct::Add);
            }
            if icon_button(ui, "✂", "Cut", sel_ok) {
                act = Some(CmdAct::Cut);
            }
            if icon_button(ui, "📑", "Copy", sel_ok) {
                act = Some(CmdAct::Copy);
            }
            if icon_button(ui, "📋", "Paste", self.cmd_clip.is_some()) {
                act = Some(CmdAct::Paste);
            }
            if icon_button(ui, "✖", "Delete", sel_ok || has_folder) {
                act = Some(CmdAct::Delete);
            }
            if icon_button(ui, "⚙", "Properties", sel_ok) {
                act = Some(CmdAct::Edit(self.cmd_sel.unwrap_or(0)));
            }
            if icon_button(ui, "📁", "New Folder", true) {
                act = Some(CmdAct::NewFolder);
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.cmd_filter)
                .id(egui::Id::new("cmd_filter"))
                .hint_text("Filter by folder/command name <Alt+Y>")
                .desired_width(f32::INFINITY),
        );
        let filter = self.cmd_filter.trim().to_lowercase();
        let mut sel = self.cmd_sel;
        let mut sel_folder = self.cmd_sel_folder.clone();
        let mut closed_change: Vec<(String, bool)> = Vec::new();
        egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(70))).inner_margin(2).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("cmd_tree").auto_shrink([false, false]).show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                cmd_tree_node(ui, &self.settings, "", &filter, &mut sel, &mut sel_folder, &mut act, self.cmd_expand.take(), self.cmd_clip.is_some(), self.cmd_target, &mut closed_change);
            });
        });
        self.cmd_sel = sel;
        self.cmd_sel_folder = sel_folder;
        for (k, closed) in closed_change {
            self.settings.cmd_tree_closed.retain(|x| x != &k);
            if closed {
                self.settings.cmd_tree_closed.push(k);
            }
            self.dirty = true;
        }
        if let Some(a) = act {
            self.command_action(a);
        }
        let _ = ctx;
    }

    fn command_action(&mut self, a: CmdAct) {
        let folder_now = self
            .cmd_sel_folder
            .clone()
            .filter(|f| !f.is_empty())
            .or_else(|| self.cmd_sel.and_then(|i| self.settings.commands.get(i)).map(|c| c.folder.clone()))
            .unwrap_or_default();
        match a {
            CmdAct::Send(i) => {
                if let Some(c) = self.settings.commands.get(i) {
                    let t = c.text.clone();
                    self.send_command_string(&t);
                    self.focus_terminal = true;
                }
            }
            CmdAct::Target(n) => self.cmd_target = n,
            CmdAct::ExpandAll => self.cmd_expand = Some(true),
            CmdAct::CollapseAll => self.cmd_expand = Some(false),
            CmdAct::Rename => {
                if let Some(f) = self.cmd_sel_folder.clone().filter(|f| !f.is_empty()) {
                    let last = f.rsplit('/').next().unwrap_or("").to_string();
                    self.prompt(Prompt::RenameCmdFolder(f), "Rename Folder", "New name:", last);
                } else if let Some(i) = self.cmd_sel {
                    if let Some(c) = self.settings.commands.get(i) {
                        let l = c.label.clone();
                        self.prompt(Prompt::RenameCommand(i), "Rename Command", "New name:", l);
                    }
                }
            }
            CmdAct::Add => {
                self.cmd_edit = Some((None, config::CommandDef { folder: folder_now, ..Default::default() }));
            }
            CmdAct::Edit(i) => {
                if let Some(c) = self.settings.commands.get(i) {
                    self.cmd_edit = Some((Some(i), c.clone()));
                }
            }
            CmdAct::Cut => {
                if let Some(i) = self.cmd_sel {
                    if let Some(c) = self.settings.commands.get(i) {
                        self.cmd_clip = Some((c.clone(), Some(i)));
                    }
                }
            }
            CmdAct::Copy => {
                if let Some(i) = self.cmd_sel {
                    if let Some(c) = self.settings.commands.get(i) {
                        self.cmd_clip = Some((c.clone(), None));
                    }
                }
            }
            CmdAct::Paste => {
                if let Some((mut c, cut)) = self.cmd_clip.take() {
                    c.folder = folder_now;
                    match cut {
                        Some(i) if i < self.settings.commands.len() => {
                            self.settings.commands[i] = c;
                        }
                        _ => {
                            self.settings.commands.push(c.clone());
                            self.cmd_clip = Some((c, None)); // a copy can be pasted again
                        }
                    }
                    self.dirty = true;
                }
            }
            CmdAct::Delete => {
                self.ask("Are you sure you want to delete the selected item(s)?", ConfirmAct::DeleteCommand);
            }
            CmdAct::NewFolder => {
                let hint = if folder_now.is_empty() { String::new() } else { format!("{folder_now}/") };
                self.prompt(Prompt::CmdFolder, "New Folder", "Folder name:", hint);
            }
        }
    }

    // ----- bars ---------------------------------------------------------------------------

    /// The tab bar; with tab groups, `group` is the group whose tabs this bar shows.
    fn tabs(&mut self, ui: &mut egui::Ui, group: Option<usize>) {
        let ctx = ui.ctx().clone();
        #[derive(Clone, Copy, PartialEq)]
        enum TabAct {
            Close,
            CloseOthers,
            CloseRight,
            CloseAll,
            CloseGroup,
            Clone,
            Sftp,
            Lock,
            Reconnect,
            Disconnect,
            Rename,
            Properties,
            ResetName,
            CloseDisconnected,
            CloneWindow,
            SendWindow,
            SendGroup,
            Font,
            SaveSession,
            CommandsToGroup,
        }
        let members: Vec<usize> = match group {
            Some(g) => self.group_members(g),
            None => (0..self.sessions.len()).collect(),
        };
        let shown = group.and_then(|g| self.group_current.get(g).copied());
        let mut select = None;
        let mut act: Option<(usize, TabAct)> = None;
        let mut tab_rects: Vec<egui::Rect> = Vec::new();
        // positions in `members` of the tab being dragged / dropped
        let mut drag_tab: Option<usize> = None;
        let mut drop_tab: Option<usize> = None;
        egui::ScrollArea::horizontal().id_salt(("tabs", group.unwrap_or(0))).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                for (pos, &i) in members.iter().enumerate() {
                    let s = &self.sessions[i];
                    let (glyph, dot) = match s.status {
                        Status::Connected => ("\u{2714}", Color32::from_rgb(0x1f, 0xc8, 0x1f)),
                        Status::Connecting => ("\u{2714}", Color32::from_rgb(0xe0, 0xa0, 0x00)),
                        Status::Disconnected(_) => ("\u{2716}", Color32::from_rgb(0xe0, 0x3c, 0x3c)),
                    };
                    // in a tab group the shown tab is drawn selected; the blue frame marks the active session
                    let selected = match shown {
                        Some(id) => s.id == id,
                        None => i == self.active,
                    };
                    let focused = i == self.active;
                    let title = if s.title.chars().count() > 30 { format!("{}…", s.title.chars().take(29).collect::<String>()) } else { s.title.clone() };
                    let text_color = if selected { ui.visuals().strong_text_color() } else { ui.visuals().text_color() };
                    let galley = ui.painter().layout_no_wrap(title, egui::FontId::proportional(12.5), text_color);
                    let width = 8.0 + 10.0 + 4.0 + galley.size().x + 8.0 + 16.0 + 6.0;
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::click_and_drag());
                    tab_rects.push(rect);
                    if resp.dragged() {
                        drag_tab = Some(pos);
                    }
                    if resp.drag_stopped() {
                        drop_tab = Some(pos);
                    }
                    let fill = if selected { Color32::from_gray(0x3d) } else { Color32::from_gray(0x2e) };
                    ui.painter().rect_filled(rect, 0.0, fill);
                    if selected {
                        let top = if focused { Color32::from_rgb(0x00, 0x78, 0xd4) } else { Color32::from_gray(0x70) };
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 3.0)), 0.0, top);
                    }
                    {
                        // a bold check mark (a cross when disconnected), drawn as lines so it stays heavy at any size
                        let c = Pos2::new(rect.min.x + 13.0, rect.center().y);
                        let st = egui::Stroke::new(2.2, dot);
                        if glyph == "\u{2714}" {
                            ui.painter().line_segment([Pos2::new(c.x - 5.0, c.y + 0.5), Pos2::new(c.x - 1.5, c.y + 4.0)], st);
                            ui.painter().line_segment([Pos2::new(c.x - 1.5, c.y + 4.0), Pos2::new(c.x + 5.0, c.y - 4.0)], st);
                        } else {
                            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y - 4.0), Pos2::new(c.x + 4.0, c.y + 4.0)], st);
                            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y + 4.0), Pos2::new(c.x + 4.0, c.y - 4.0)], st);
                        }
                    }
                    if s.cfg.tab_color_on {
                        let tc = s.cfg.tab_color;
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 3.0)), 0.0, Color32::from_rgb(tc[0], tc[1], tc[2]));
                    }
                    ui.painter().galley(Pos2::new(rect.min.x + 22.0, rect.center().y - galley.size().y / 2.0), galley, text_color);
                    // The close button sits on top of the tab and takes its own clicks.
                    // like the original, only the selected tab (or the one under the pointer) shows its close button
                    let show_x = selected || ui.rect_contains_pointer(rect);
                    let x_rect = egui::Rect::from_center_size(Pos2::new(rect.max.x - 14.0, rect.center().y), egui::vec2(16.0, 16.0));
                    let x_resp = if show_x { Some(ui.interact(x_rect, egui::Id::new(("tab_close", self.sessions[i].id)), Sense::click()).on_hover_text("Close tab")) } else { None };
                    if let Some(x_resp) = &x_resp {
                        if x_resp.hovered() {
                            ui.painter().rect_filled(x_rect, 2.0, Color32::from_rgb(0xc4, 0x2b, 0x1c));
                        }
                        let xc = if x_resp.hovered() { Color32::WHITE } else { ui.visuals().text_color() };
                        let c = x_rect.center();
                        let d = 3.5;
                        ui.painter().line_segment([Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)], egui::Stroke::new(1.3, xc));
                        ui.painter().line_segment([Pos2::new(c.x - d, c.y + d), Pos2::new(c.x + d, c.y - d)], egui::Stroke::new(1.3, xc));
                    }
                    if x_resp.as_ref().map(|r| r.clicked()).unwrap_or(false) {
                        act = Some((i, TabAct::Close));
                    } else if resp.clicked() {
                        select = Some(i);
                    }
                    if resp.middle_clicked() {
                        act = Some((i, TabAct::Close));
                    }
                    let connected = self.sessions[i].is_connected();
                    let is_ssh = self.sessions[i].cfg.protocol.is_ssh();
                    let many = self.sessions.len() > 1;
                    let many_here = members.len() > 1;
                    let last = pos + 1 >= members.len();
                    let locked = self.sessions[i].locked.is_some();
                    let renamed = self.sessions[i].renamed;
                    // the original's tab menu, in its order (SecureCRT 9, right-click on a tab)
                    resp.context_menu(|ui| {
                        ui.set_min_width(230.0);
                        if mi(ui, "Rename", "", true) {
                            act = Some((i, TabAct::Rename));
                        }
                        if mi(ui, "Reset Name", "", renamed) {
                            act = Some((i, TabAct::ResetName));
                        }
                        if mi_plain(ui, "Reconnect", !connected) {
                            act = Some((i, TabAct::Reconnect));
                        }
                        if mi_plain(ui, "Disconnect", connected) {
                            act = Some((i, TabAct::Disconnect));
                        }
                        ui.separator();
                        if mi(ui, "Close", "Ctrl+F4", true) {
                            act = Some((i, TabAct::Close));
                        }
                        if mi(ui, "Close Disconnected Tabs", "", true) {
                            act = Some((i, TabAct::CloseDisconnected));
                        }
                        if mi(ui, "Close Other Tabs", "", many_here) {
                            act = Some((i, TabAct::CloseOthers));
                        }
                        if mi(ui, "Close Tabs to the Right", "", !last) {
                            act = Some((i, TabAct::CloseRight));
                        }
                        if mi(ui, "Close Tab Group", "", true) {
                            act = Some((i, TabAct::CloseGroup));
                        }
                        if mi(ui, "Close All Tabs", "", true) {
                            act = Some((i, TabAct::CloseAll));
                        }
                        ui.separator();
                        if mi(ui, if locked { "Unlock Session" } else { "Lock..." }, "", true) {
                            act = Some((i, TabAct::Lock));
                        }
                        ui.separator();
                        if mi(ui, "Clone Session", "", true) {
                            act = Some((i, TabAct::Clone));
                        }
                        if mi(ui, "Clone in New Window", "", true) {
                            act = Some((i, TabAct::CloneWindow));
                        }
                        if mi(ui, "Send to New Window", "", many) {
                            act = Some((i, TabAct::SendWindow));
                        }
                        if mi(ui, "Send to New Tab Group", "", many_here) {
                            act = Some((i, TabAct::SendGroup));
                        }
                        if mi(ui, "Connect SFTP Session", "", is_ssh && connected) {
                            act = Some((i, TabAct::Sftp));
                        }
                        // SecureFX is a separate VanDyke product; the original greys this out when it is not installed
                        mi(ui, "Open SecureFX", "", false);
                        ui.separator();
                        if mi_plain(ui, "Session Options...", true) {
                            act = Some((i, TabAct::Properties));
                        }
                        if mi(ui, "Font...", "", true) {
                            act = Some((i, TabAct::Font));
                        }
                        if mi(ui, "Save Session...", "", true) {
                            act = Some((i, TabAct::SaveSession));
                        }
                        if mi(ui, "Send Commands to This Group", "", true) {
                            act = Some((i, TabAct::CommandsToGroup));
                        }
                    });
                }
            });
        });
        // the blue line under the tab bar
        let bar = ui.max_rect();
        ui.painter().hline(bar.x_range(), bar.bottom() - 0.5, egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)));
        // Dragging a tab: a blue marker shows where it will land; releasing moves it there.
        let pointer = ui.ctx().input(|i| i.pointer.latest_pos());
        let slot_for = |p: Pos2| -> usize {
            let mut slot = tab_rects.len();
            for (k, r) in tab_rects.iter().enumerate() {
                if p.x < r.center().x {
                    slot = k;
                    break;
                }
            }
            slot
        };
        if let (Some(from), Some(p)) = (drag_tab, pointer) {
            let slot = slot_for(p);
            if let Some(r) = tab_rects.get(slot).or(tab_rects.last()) {
                let x = if slot >= tab_rects.len() { r.max.x } else { r.min.x };
                ui.painter().vline(x, r.y_range(), egui::Stroke::new(2.0, Color32::from_rgb(0x00, 0x78, 0xd4)));
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let _ = from;
        }
        if let (Some(from), Some(p)) = (drop_tab, pointer) {
            let mut slot = slot_for(p);
            if slot > from {
                slot -= 1;
            }
            if slot != from && from < members.len() {
                // reorder this bar's tabs among the places they hold in the session list; the active
                // session stays the active one, wherever it ends up
                let active_id = self.sessions.get(self.active).map(|x| x.id);
                let mut order = members.clone();
                let moved = order.remove(from);
                order.insert(slot.min(order.len()), moved);
                let mut perm: Vec<usize> = (0..self.sessions.len()).collect();
                for (place, &src) in members.iter().zip(order.iter()) {
                    perm[*place] = src;
                }
                let mut old: Vec<Option<Session>> = std::mem::take(&mut self.sessions).into_iter().map(Some).collect();
                self.sessions = perm.iter().filter_map(|&k| old[k].take()).collect();
                self.active = active_id.and_then(|id| self.index_of(id)).unwrap_or(0);
                self.focus_terminal = true;
            }
        }
        if let Some(i) = select {
            self.active = i;
            self.focus_terminal = true;
        }
        if let Some((i, a)) = act {
            match a {
                TabAct::Close => self.request_close(i),
                TabAct::CloseOthers => {
                    let id = self.sessions[i].id;
                    let n = members.iter().filter(|&&k| k != i && self.sessions[k].is_connected()).count();
                    if n > 0 {
                        self.ask("Some of the other sessions are connected.\nAre you sure you want to disconnect and close them?", ConfirmAct::CloseOthers(id));
                    } else {
                        self.run_confirmed(ConfirmAct::CloseOthers(id), &ctx);
                    }
                }
                TabAct::CloseRight => {
                    let id = self.sessions[i].id;
                    let n = members.iter().filter(|&&k| k > i && self.sessions[k].is_connected()).count();
                    if n > 0 {
                        self.ask("Some of the sessions to the right are connected.\nAre you sure you want to disconnect and close them?", ConfirmAct::CloseRight(id));
                    } else {
                        self.run_confirmed(ConfirmAct::CloseRight(id), &ctx);
                    }
                }
                TabAct::CloseGroup => {
                    let g = self.group_index(i);
                    if members.iter().any(|&k| self.sessions[k].is_connected()) {
                        self.ask("Some sessions in this tab group are connected.\nAre you sure you want to disconnect and close the tab group?", ConfirmAct::CloseGroup(g));
                    } else {
                        self.run_confirmed(ConfirmAct::CloseGroup(g), &ctx);
                    }
                }
                TabAct::SendGroup => {
                    // the tab leaves its bar and starts a new tab group to the right of the others
                    let id = self.sessions[i].id;
                    self.group_of.insert(id, self.n_groups);
                    self.n_groups += 1;
                    self.group_current.push(id);
                    self.active = i;
                    self.focus_terminal = true;
                }
                TabAct::SaveSession => {
                    let name = self.sessions[i].cfg.display_name();
                    self.prompt(Prompt::SaveSession(self.sessions[i].id), "Save Session", "Session name:", name);
                }
                TabAct::CommandsToGroup => {
                    self.cmd_target = 3;
                    self.cmd_group = self.group_index(i);
                    self.settings.show_command_window = true;
                }
                TabAct::CloseAll => {
                    if self.connected_count() > 0 {
                        self.ask("Some sessions are connected.\nAre you sure you want to disconnect and close all tabs?", ConfirmAct::CloseAll);
                    } else {
                        self.run_confirmed(ConfirmAct::CloseAll, &ctx);
                    }
                }
                TabAct::Clone => self.clone_tab(i, &ctx),
                TabAct::Sftp => {
                    let mut cfg = self.sessions[i].cfg_for_clone();
                    self.open_after = Some(self.sessions[i].id);
                    cfg.protocol = Protocol::Sftp;
                    cfg.name = format!("SFTP-{}", cfg.host.trim());
                    self.open_session(cfg, &ctx);
                }
                TabAct::Lock => {
                    self.active = i;
                    if self.sessions[i].locked.is_some() {
                        self.sessions[i].locked = None;
                    } else {
                        self.prompt(Prompt::LockPassword, "Lock Session", "Password to unlock:", String::new());
                    }
                }
                TabAct::Reconnect => self.sessions[i].connect(),
                TabAct::Disconnect => self.request_disconnect(i),
                TabAct::Rename => {
                    let t = self.sessions[i].title.clone();
                    self.prompt(Prompt::RenameTab(i), "Rename", "Tab name:", t);
                }
                TabAct::CloneWindow | TabAct::SendWindow => {
                    let cfg = self.sessions[i].cfg_for_clone();
                    spawn_window_for(&cfg);
                    if a == TabAct::SendWindow {
                        self.close_tab(i);
                    }
                }
                TabAct::Font => {
                    self.active = i;
                    self.open_options(EditTarget::Active, "appearance");
                }
                TabAct::ResetName => {
                    self.sessions[i].title = self.sessions[i].cfg.display_name();
                    self.sessions[i].renamed = false;
                }
                TabAct::CloseDisconnected => {
                    let active_id = self.sessions.get(self.active).map(|x| x.id);
                    self.sessions.retain(|s| s.is_connected() || matches!(s.status, Status::Connecting));
                    self.active = self.sessions.iter().position(|x| Some(x.id) == active_id).unwrap_or(0);
                }
                TabAct::Properties => {
                    self.active = i;
                    self.open_options(EditTarget::Active, "connection");
                }
            }
        }
    }

    /// The buttons of the bar the active session shows (its "Button bar" option; the default bar otherwise).
    fn bar_mut(&mut self) -> &mut Vec<ButtonDef> {
        let name = self.sessions.get(self.active).map(|s| s.cfg.button_bar.clone()).unwrap_or_default();
        match self.settings.button_bars.iter().position(|b| b.0 == name) {
            Some(i) if !name.is_empty() => &mut self.settings.button_bars[i].1,
            _ => &mut self.settings.buttons,
        }
    }

    fn button_bar(&mut self, ui: &mut egui::Ui) {
        let mut edit = None;
        let mut remove = None;
        let mut send = None;
        let mut add = false;
        let mut new_bar = false;
        let mut del_bar = false;
        let bar_name = self.sessions.get(self.active).map(|s| s.cfg.button_bar.clone()).unwrap_or_default();
        let buttons = self.bar_mut().clone();
        ui.horizontal_wrapped(|ui| {
            for (i, b) in buttons.iter().enumerate() {
                let r = ui.button(&b.label).on_hover_text(format!("Sends: {}", b.text));
                if r.clicked() {
                    send = Some(config::unescape(&b.text));
                }
                r.context_menu(|ui| {
                    if ui.button("Edit...").clicked() {
                        edit = Some(i);
                    }
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
            }
            let plus = ui.button("➕").on_hover_text("New button");
            if plus.clicked() {
                add = true;
            }
            plus.context_menu(|ui| {
                if ui.button("New Button Bar...").clicked() {
                    new_bar = true;
                    ui.close();
                }
                if ui.add_enabled(!bar_name.is_empty(), egui::Button::new("Delete This Button Bar")).clicked() {
                    del_bar = true;
                    ui.close();
                }
            });
        });
        if new_bar {
            self.prompt(Prompt::NewButtonBar, "New Button Bar", "Name of the button bar:", String::new());
        }
        if del_bar {
            self.settings.button_bars.retain(|b| b.0 != bar_name);
            for s in &mut self.sessions {
                if s.cfg.button_bar == bar_name {
                    s.cfg.button_bar.clear();
                }
            }
            self.dirty = true;
        }
        if add {
            self.button_editor = Some(ButtonEditor { index: None, def: ButtonDef { label: "New".into(), text: "\\r".into() } });
        }
        if let Some(t) = send {
            self.send_to_targets(&t);
            self.focus_terminal = true;
        }
        if let Some(i) = edit {
            self.button_editor = Some(ButtonEditor { index: Some(i), def: buttons[i].clone() });
        }
        if let Some(i) = remove {
            self.bar_mut().remove(i);
            self.dirty = true;
        }
    }

    fn command_window(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let focus_blue = Color32::from_rgb(0x00, 0x78, 0xd4);
        let full = ui.max_rect();
        // the line on top is blue only while the box has the keyboard focus, grey otherwise (as in the original)
        let box_focus = ui.ctx().memory(|m| m.has_focus(egui::Id::new("command_line")));
        ui.painter().rect_filled(egui::Rect::from_min_max(Pos2::new(full.min.x - 6.0, full.min.y), Pos2::new(full.max.x + 2.0, full.min.y + 5.0)), 0.0, Color32::from_gray(0x3d));
        if box_focus {
            ui.painter().hline(full.x_range(), full.min.y + 1.0, egui::Stroke::new(2.0, focus_blue));
        }
        ui.add_space(5.0);
        if self.settings.cmd_title_bar {
            ui.horizontal(|ui| {
                let title = match self.cmd_target {
                    1 => "Send commands to all sessions",
                    2 => "Send commands to visible sessions",
                    3 => "Send commands to selected tab group(s)",
                    _ => "Send commands to active session",
                };
                ui.label(RichText::new(title).size(11.5));
                // dotted grip
                let w = (ui.available_width() - 20.0).max(0.0);
                let (r, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), Sense::hover());
                let mut x = r.min.x + 4.0;
                while x < r.max.x {
                    ui.painter().circle_filled(Pos2::new(x, r.center().y), 0.5, Color32::from_gray(110));
                    x += 3.0;
                }
                if ui.add(egui::Button::new(RichText::new("✖").size(10.0)).frame(false)).clicked() {
                    self.settings.show_command_window = false;
                }
            });
        }
        let id = egui::Id::new("command_line");
        let has_focus = ctx.memory(|m| m.has_focus(id));
        // Enter sends the line (Ctrl+Enter instead with "Swap ENTER and CTRL+ENTER in Command window")
        let send_mods = if self.settings.global.swap_enter { egui::Modifiers::CTRL } else { egui::Modifiers::NONE };
        let enter = has_focus && ctx.input_mut(|i| i.consume_key(send_mods, egui::Key::Enter));
        let mut menu_act: Option<&'static str> = None;
        let before = self.command_line.clone();
        let rect_resp = egui::Frame::new().fill(Color32::from_gray(0x38)).stroke(egui::Stroke::new(1.0, Color32::from_gray(0x9a))).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("cmd_scroll").max_height(84.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.command_line)
                        .id(id)
                        .font(egui::TextStyle::Monospace)
                        .frame(egui::Frame::NONE)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                )
            })
        });
        let r = rect_resp.inner.inner;
        let has_sel = {
            let st = egui::TextEdit::load_state(&ctx, id);
            st.and_then(|s| s.cursor.char_range()).map(|c| c.primary.index.0 != c.secondary.index.0).unwrap_or(false)
        };
        let empty = self.command_line.is_empty();
        let immediate = self.settings.cmd_send_immediately;
        // the box keeps its own undo history; the menu entry follows it
        let undo_now = |text: &str| {
            let cur = egui::TextEdit::load_state(&ctx, id).and_then(|s| s.cursor.char_range()).unwrap_or_default();
            (cur, text.to_string())
        };
        let can_undo = egui::TextEdit::load_state(&ctx, id).map(|s| s.undoer().has_undo(&undo_now(&self.command_line))).unwrap_or(false);
        r.context_menu(|ui| {
            ui.set_min_width(330.0);
            if mi(ui, "Undo", "Ctrl+Z", can_undo) {
                menu_act = Some("undo");
            }
            ui.separator();
            if mi(ui, "Cut", "Ctrl+X", has_sel) {
                menu_act = Some("cut");
            }
            if mi(ui, "Copy", "Ctrl+C", has_sel) {
                menu_act = Some("copy");
            }
            if mi(ui, "Paste", "Ctrl+V", true) {
                menu_act = Some("paste");
            }
            if mi(ui, "Delete", "Del", has_sel) {
                menu_act = Some("delete");
            }
            ui.separator();
            if mi(ui, "Select All", "Ctrl+A", !empty) {
                menu_act = Some("all");
            }
            ui.separator();
            if mi(ui, &format!("{}Send Characters Immediately", if immediate { "✓ " } else { "   " }), "Ctrl+Shift+I", true) {
                menu_act = Some("immediate");
            }
            if mi(ui, &format!("{}Show Command Window Title Bar", if self.settings.cmd_title_bar { "✓ " } else { "   " }), "Ctrl+Shift+W", true) {
                menu_act = Some("titlebar");
            }
            sub_menu(ui, "Send Commands to", |ui| {
                ui.set_min_width(260.0);
                let t = self.cmd_target;
                if mi(ui, &format!("{}Active Session", if t == 0 { "✓ " } else { "   " }), "Ctrl+Shift+T", true) {
                    menu_act = Some("t_active");
                }
                if mi(ui, &format!("{}All Sessions", if t == 1 { "✓ " } else { "   " }), "Ctrl+Shift+S", true) {
                    menu_act = Some("t_all");
                }
                if mi(ui, &format!("{}Selected Tab Group(s)", if t == 3 { "✓ " } else { "   " }), "Ctrl+Shift+G", self.n_groups > 1) {
                    menu_act = Some("t_group");
                }
                if mi(ui, &format!("{}Visible Sessions", if t == 2 { "✓ " } else { "   " }), "Ctrl+Shift+B", true) {
                    menu_act = Some("t_visible");
                }
            });
            ui.separator();
            if mi(ui, "Properties", "Alt+Enter", true) {
                menu_act = Some("props");
            }
        });
        // selection helpers (character positions)
        let sel_range = || {
            let st = egui::TextEdit::load_state(&ctx, id);
            st.and_then(|s| s.cursor.char_range()).map(|c| {
                let (a, b) = (c.primary.index.0, c.secondary.index.0);
                (a.min(b), a.max(b))
            })
        };
        match menu_act {
            Some("copy") | Some("cut") | Some("delete") => {
                if let Some((a, b)) = sel_range() {
                    let chars: Vec<char> = self.command_line.chars().collect();
                    let piece: String = chars[a.min(chars.len())..b.min(chars.len())].iter().collect();
                    if menu_act != Some("delete") {
                        ctx.copy_text(piece);
                    }
                    if menu_act != Some("copy") {
                        let rest: String = chars[..a.min(chars.len())].iter().chain(chars[b.min(chars.len())..].iter()).collect();
                        self.command_line = rest;
                    }
                }
            }
            Some("undo") => {
                if let Some(mut st) = egui::TextEdit::load_state(&ctx, id) {
                    let mut u = st.undoer();
                    if let Some((range, text)) = u.undo(&undo_now(&self.command_line)).cloned() {
                        self.command_line = text;
                        st.cursor.set_char_range(Some(range));
                        st.set_undoer(u);
                        st.store(&ctx, id);
                        ctx.memory_mut(|m| m.request_focus(id));
                    }
                }
            }
            Some("paste") => {
                ctx.memory_mut(|m| m.request_focus(id));
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            Some("all") => {
                let n = self.command_line.chars().count();
                let mut st = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(n))));
                st.store(&ctx, id);
                ctx.memory_mut(|m| m.request_focus(id));
            }
            Some("immediate") => self.settings.cmd_send_immediately = !self.settings.cmd_send_immediately,
            Some("titlebar") => self.settings.cmd_title_bar = !self.settings.cmd_title_bar,
            Some("t_active") => self.cmd_target = 0,
            Some("t_all") => self.cmd_target = 1,
            Some("t_visible") => self.cmd_target = 2,
            Some("t_group") => {
                self.cmd_target = 3;
                self.cmd_group = self.group_index(self.active);
            }
            Some("props") => {
                // the Command window's options live on Terminal > Advanced
                self.global_state.page = "termadvanced".into();
                self.global_open = true;
            }
            _ => {}
        }
        if immediate {
            // every typed character goes out at once; Enter sends the line break
            if self.command_line != before && !self.command_line.is_empty() {
                let t = self.command_line.replace('\n', "\r");
                self.send_to_targets(&t);
                self.command_line.clear();
            }
            if enter {
                self.send_to_targets("\r");
            }
        } else if enter {
            let line = format!("{}\r", self.command_line.replace('\n', "\r"));
            self.send_to_targets(&line);
            self.command_line.clear();
        }
    }

    fn hex_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Hex View").strong());
        let bytes: Vec<u8> = self.sessions.get(self.active).map(|s| s.hex.iter().rev().take(512).rev().copied().collect()).unwrap_or_default();
        egui::ScrollArea::vertical().stick_to_bottom(true).max_height(120.0).auto_shrink([false, false]).show(ui, |ui| {
            for (n, chunk) in bytes.chunks(16).enumerate() {
                let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02X}")).collect();
                let asc: String = chunk.iter().map(|&b| if (32..127).contains(&b) { b as char } else { '.' }).collect();
                ui.monospace(format!("{:04X}  {:<47}  {}", n * 16, hex.join(" "), asc));
            }
        });
    }

    /// The tab group bar under the sessions: the group list as a drop-down, like the original's "Default" button.
    fn group_bar(&mut self, ui: &mut egui::Ui) {
        let can_split = self.sessions.len() > 1;
        ui.horizontal(|ui| {
            ui.menu_button("Default  \u{E70D}", |ui| {
                ui.set_min_width(180.0);
                let _ = ui.radio(true, "Default");
                ui.separator();
                if mi(ui, "Send to New Tab Group", "", can_split) {
                    if let Some(id) = self.sessions.get(self.active).map(|s| s.id) {
                        self.group_of.insert(id, self.n_groups);
                        self.n_groups += 1;
                        self.group_current.push(id);
                        self.focus_terminal = true;
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let (caps, num) = config::lock_keys();
        ui.horizontal(|ui| {
            let left = match self.sessions.get(self.active) {
                Some(s) => match &s.status {
                    Status::Connected => "Ready".to_string(),
                    Status::Connecting => "Connecting...".to_string(),
                    Status::Disconnected(w) if w.is_empty() => "Not connected".to_string(),
                    Status::Disconnected(w) => w.clone(),
                },
                None => self.status_msg.clone(),
            };
            ui.label(RichText::new(left).size(12.0));
            // Global Options > Terminal > Appearance > Status Bar: fixed columns (log file name, CAPS, NUM) and
            // up to five chosen columns, drawn left to right in the chosen order (this layout runs right to left)
            let g = &self.settings.global;
            let tftp_on = self.tftp.is_some();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let on = Color32::from_gray(235);
                let off = Color32::from_gray(95);
                if g.status_num {
                    ui.label(RichText::new("NUM").size(12.0).color(if num { on } else { off }));
                }
                if g.status_caps {
                    ui.label(RichText::new("CAP").size(12.0).color(if caps { on } else { off }));
                }
                if let Some(s) = self.sessions.get(self.active) {
                    for col in g.status_columns.iter().take(5).rev() {
                        let text = match col.as_str() {
                            "Protocol Information" => s.cfg.summary(),
                            "Cursor Position" => {
                                let (r, c) = s.parser.screen().cursor_position();
                                format!("{}, {}", r + 1, c + 1)
                            }
                            "Rows and Columns" => format!("{} Rows, {} Cols", s.rows, s.cols),
                            "Emulation" => s.cfg.emulation.clone(),
                            "Elapsed Time" | "Elapsed Time w/Seconds" => match s.connected_at {
                                Some(t) => {
                                    let e = t.elapsed().as_secs();
                                    if col == "Elapsed Time" {
                                        format!("{:02}:{:02}", e / 3600, (e / 60) % 60)
                                    } else {
                                        format!("{:02}:{:02}:{:02}", e / 3600, (e / 60) % 60, e % 60)
                                    }
                                }
                                None => continue,
                            },
                            "Local Time" => {
                                let n = config::now();
                                format!("{:02}:{:02}", n.h, n.mi)
                            }
                            "TFTP Server Status" => (if tftp_on { "TFTP: On" } else { "TFTP: Off" }).to_string(),
                            _ => continue,
                        };
                        ui.add_space(14.0);
                        ui.label(RichText::new(text).size(12.0));
                    }
                    if let (true, Some(p)) = (g.status_log_name, &s.log_path) {
                        ui.add_space(14.0);
                        let name = std::path::Path::new(p).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.label(RichText::new(if s.log_paused { "Logging paused".to_string() } else { format!("Logging: {name}") }).size(12.0)).on_hover_text(p);
                    }
                }
            });
        });
    }

    // ----- terminal area ------------------------------------------------------------------

    fn terminal_area(&mut self, ui: &mut egui::Ui) {
        if self.sessions.is_empty() {
            return;
        }
        if self.active >= self.sessions.len() {
            self.active = self.sessions.len() - 1;
        }
        let full = ui.available_rect_before_wrap();
        let n = self.sessions.len();
        let mut rects: Vec<(usize, egui::Rect)> = match self.layout {
            Layout::Tabs => vec![(self.active, full)],
            Layout::TileV => (0..n)
                .map(|i| {
                    let w = full.width() / n as f32;
                    (i, egui::Rect::from_min_size(Pos2::new(full.min.x + w * i as f32, full.min.y), egui::vec2(w - 2.0, full.height())))
                })
                .collect(),
            Layout::Cascade => {
                let step = 26.0;
                let w = (full.width() - step * (n as f32 - 1.0)).max(300.0);
                let h = (full.height() - step * (n as f32 - 1.0)).max(200.0);
                (0..n).map(|i| (i, egui::Rect::from_min_size(full.min + egui::vec2(step * i as f32, step * i as f32), egui::vec2(w, h)))).collect()
            }
            Layout::TileH => (0..n)
                .map(|i| {
                    let h = full.height() / n as f32;
                    (i, egui::Rect::from_min_size(Pos2::new(full.min.x, full.min.y + h * i as f32), egui::vec2(full.width(), h - 2.0)))
                })
                .collect(),
        };
        if let Some(mid) = self.pane_max.filter(|_| self.layout != Layout::Tabs) {
            match self.sessions.iter().position(|s| s.id == mid) {
                Some(pos) => rects = vec![(pos, full)],
                None => self.pane_max = None,
            }
        }
        self.terminal_panes(ui, rects);
    }

    /// Tab groups side by side: each group has its own tab bar over the session it shows.
    fn group_area(&mut self, ui: &mut egui::Ui, show_tabs: bool) {
        let full = ui.available_rect_before_wrap();
        let n = self.n_groups.max(1);
        let gap = 6.0;
        let w = (full.width() - gap * (n as f32 - 1.0)) / n as f32;
        let strip_h = 25.0;
        let bottom = self.settings.global.tab_bottom;
        let mut panes = Vec::new();
        for g in 0..n {
            let col = egui::Rect::from_min_size(Pos2::new(full.min.x + (w + gap) * g as f32, full.min.y), egui::vec2(w, full.height()));
            let mut term_rect = col;
            if show_tabs {
                let strip = if bottom {
                    term_rect.max.y -= strip_h;
                    egui::Rect::from_min_max(Pos2::new(col.min.x, col.max.y - strip_h), col.max)
                } else {
                    term_rect.min.y += strip_h;
                    egui::Rect::from_min_size(col.min, egui::vec2(col.width(), strip_h))
                };
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(strip.shrink2(egui::vec2(2.0, 0.0))).id_salt(("tab_group", g)));
                child.add_space(3.0);
                self.tabs(&mut child, Some(g));
            }
            if let Some(i) = self.group_current.get(g).and_then(|id| self.index_of(*id)) {
                panes.push((i, term_rect));
            }
        }
        self.terminal_panes(ui, panes);
    }

    fn terminal_panes(&mut self, ui: &mut egui::Ui, mut rects: Vec<(usize, egui::Rect)>) {
        let active_now = self.active;
        rects.sort_by_key(|(i, _)| *i == active_now);
        let zoom = self.zoom;
        let dialog_open = self.any_dialog_open();
        let keywords = self.settings.keywords.clone();
        let highlight = self.settings.keyword_highlighting;
        let copy_on_select = self.settings.copy_on_select;
        let mut ctx_action: Option<&str> = None;
        let mut activate = None;
        let mut close_pane: Option<usize> = None;
        let mut pane_toggle: Option<(u64, bool)> = None;
        for (i, rect) in rects {
            let s = &mut self.sessions[i];
            if s.locked.is_some() {
                let (_, bg) = config::scheme(&s.cfg.color_scheme);
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(bg[0], bg[1], bg[2]));
                let mut unlock = false;
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(rect.center(), egui::vec2(260.0, 80.0))));
                child.label(RichText::new("🔒 Session locked").size(15.0));
                let mut pw = child.data_mut(|d| d.get_temp::<String>(egui::Id::new(("lockpw", s.id))).unwrap_or_default());
                let r = child.add(egui::TextEdit::singleline(&mut pw).password(true).hint_text("Password"));
                if r.lost_focus() && child.input(|i| i.key_pressed(egui::Key::Enter)) {
                    unlock = true;
                }
                if unlock && s.locked.as_deref() == Some(pw.as_str()) {
                    s.locked = None;
                    pw.clear();
                }
                child.data_mut(|d| d.insert_temp(egui::Id::new(("lockpw", s.id)), pw));
                continue;
            }
            let g = &self.settings.global;
            let opts = ViewOptions {
                keywords: &keywords,
                highlight,
                copy_on_select,
                font_size: s.cfg.font_size * zoom,
                accept_input: !dialog_open,
                confirm_paste: self.settings.paste_confirm,
                paste_prompt_lines: g.paste_prompt_lines.max(1),
                paste_delay: if self.settings.paste_delay_on { Some(self.settings.paste_delay_ms as u64) } else { None },
                paste_button: if g.paste_on_button { Some(g.paste_button.min(1)) } else { None },
                hide_pointer: g.hide_pointer_on_key,
                url_ctrl_click: g.url_ctrl_click,
                triple_click_newline: g.triple_click_newline,
                quote_chars: &g.quote_chars,
                quote_between: g.quote_between,
                browser: if g.browser_registry { "" } else { &g.browser_path },
            };
            let right_pastes = opts.paste_button == Some(1);
            // Tiled / cascaded windows have a title bar with the name, a mark on the active one and a close button.
            let mut rect = rect;
            if self.layout != Layout::Tabs {
                let bar = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 18.0));
                rect.min.y += 18.0;
                let active = i == active_now;
                let painter = ui.painter();
                painter.rect_filled(bar, 0.0, if active { Color32::from_rgb(0x00, 0x63, 0xb1) } else { Color32::from_gray(0x3a) });
                // as the original's tiled windows: "name - Connected"
                let state = match s.status {
                    Status::Connected => "Connected",
                    Status::Connecting => "Connecting",
                    Status::Disconnected(_) => "Disconnected",
                };
                // the name stops before the minimise / maximise / close buttons on a narrow pane
                let text_clip = egui::Rect::from_min_max(bar.min, Pos2::new(bar.max.x - 58.0, bar.max.y));
                painter.with_clip_rect(text_clip.intersect(painter.clip_rect())).text(
                    Pos2::new(bar.min.x + 6.0, bar.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{} - {state}", s.title),
                    egui::FontId::proportional(11.5),
                    Color32::WHITE,
                );
                // the bar first: a widget registered later sits on top, so the buttons below get their own clicks
                let br = ui.interact(bar, egui::Id::new(("pane_bar", s.id)), Sense::click());
                if br.clicked() && i != active_now {
                    activate = Some(i);
                }
                let x_rect = egui::Rect::from_center_size(Pos2::new(bar.max.x - 12.0, bar.center().y), egui::vec2(16.0, 16.0));
                let xr = ui.interact(x_rect, egui::Id::new(("pane_close", s.id)), Sense::click());
                if xr.hovered() {
                    ui.painter().rect_filled(x_rect, 2.0, Color32::from_rgb(0xc4, 0x2b, 0x1c));
                }
                ui.painter().text(x_rect.center(), egui::Align2::CENTER_CENTER, "✖", egui::FontId::proportional(10.0), Color32::WHITE);
                if xr.clicked() {
                    close_pane = Some(i);
                }
                // minimise (fold to the title bar) and maximise (fill the area) buttons
                let max_rect = egui::Rect::from_center_size(Pos2::new(bar.max.x - 30.0, bar.center().y), egui::vec2(16.0, 16.0));
                let min_rect = egui::Rect::from_center_size(Pos2::new(bar.max.x - 48.0, bar.center().y), egui::vec2(16.0, 16.0));
                let is_max = self.pane_max == Some(s.id);
                let is_min = self.pane_min.contains(&s.id);
                for (r, glyph, id, which) in [(min_rect, if is_min { "▭" } else { "–" }, "pane_min", 0), (max_rect, if is_max { "❐" } else { "□" }, "pane_max", 1)] {
                    let br2 = ui.interact(r, egui::Id::new((id, s.id)), Sense::click());
                    if br2.hovered() {
                        ui.painter().rect_filled(r, 2.0, Color32::from_white_alpha(40));
                    }
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(11.0), Color32::WHITE);
                    if br2.clicked() {
                        pane_toggle = Some((s.id, which == 1));
                        activate = Some(i); // the pane whose button was pressed becomes the active one
                    }
                }
            }
            if self.layout != Layout::Tabs && self.pane_min.contains(&s.id) && self.pane_max != Some(s.id) {
                continue; // folded to its title bar
            }
            let resp = term::show(ui, s, &opts, rect);
            if self.layout == Layout::Tabs {
                // the original frames the session window: a light line with a darker one inside it
                let p = ui.painter();
                p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_gray(0xa0)), egui::StrokeKind::Inside);
                p.rect_stroke(rect.shrink(1.0), 0.0, egui::Stroke::new(1.0, Color32::from_gray(0x69)), egui::StrokeKind::Inside);
            }
            if i == self.active && self.focus_terminal {
                resp.request_focus();
                self.focus_terminal = false;
            }
            if resp.clicked() || resp.secondary_clicked() || resp.has_focus() && self.layout != Layout::Tabs {
                if i != self.active {
                    activate = Some(i);
                }
                if self.side.is_some() && !self.pinned && resp.clicked() {
                    self.side = None;
                }
            }
            let has_sel = self.sessions[i].selection.is_some();
            let url_sel = self.sessions[i].selected_text().map(|t| t.trim().starts_with("http")).unwrap_or(false);
            // "Paste on right button" replaces the terminal's context menu
            if right_pastes {
                continue;
            }
            resp.context_menu(|ui| {
                ui.set_min_width(240.0);
                if mi(ui, "Copy", "", has_sel) {
                    ctx_action = Some("copy");
                }
                if mi(ui, "Paste", "", true) {
                    ctx_action = Some("paste");
                }
                if mi(ui, "Copy and Paste", "", has_sel) {
                    ctx_action = Some("copypaste");
                }
                if mi(ui, "Paste as Quotation", "", true) {
                    ctx_action = Some("quote");
                }
                ui.separator();
                if mi(ui, "Open URL", "", url_sel) {
                    ctx_action = Some("url");
                }
                mi(ui, "Lookup Selection", "", false);
                if mi(ui, "Add Keyword...", "", has_sel) {
                    ctx_action = Some("keyword");
                }
                ui.separator();
                if mi(ui, "Find...", "", true) {
                    ctx_action = Some("find");
                }
                if mi(ui, "Select All", "", true) {
                    ctx_action = Some("all");
                }
                if mi(ui, "Print Selection", "", has_sel) {
                    ctx_action = Some("print");
                }
                ui.separator();
                if mi(ui, "Clear Scrollback", "", true) {
                    ctx_action = Some("clear_sb");
                }
                if mi(ui, "Clear Screen", "", true) {
                    ctx_action = Some("clear");
                }
                if mi(ui, "Clear Screen and Scrollback", "", true) {
                    ctx_action = Some("clear_all");
                }
            });
            if ctx_action.is_some() {
                activate = Some(i);
            }
        }
        if let Some(i) = activate {
            self.active = i;
        }
        if let Some((id, is_max)) = pane_toggle {
            if is_max {
                self.pane_max = if self.pane_max == Some(id) { None } else { Some(id) };
            } else if let Some(p) = self.pane_min.iter().position(|x| *x == id) {
                self.pane_min.remove(p);
            } else {
                self.pane_min.push(id);
            }
        }
        if let Some(i) = close_pane {
            self.request_close(i);
        }
        if let Some(a) = ctx_action {
            let ctx = ui.ctx().clone();
            let idx = self.active;
            let s = &mut self.sessions[idx];
            match a {
                "copy" => {
                    if let Some(t) = s.selected_text() {
                        ctx.copy_text(t);
                    }
                }
                "paste" => {
                    // a menu paste, even with the Windows paste hotkeys turned off
                    s.paste_requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    self.focus_terminal = true;
                }
                "quote" => {
                    s.paste_quote = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    self.focus_terminal = true;
                }
                "copypaste" => {
                    if let Some(t) = s.selected_text() {
                        s.send_str(&t);
                    }
                }
                "url" => {
                    if let Some(t) = s.selected_text() {
                        // Global Options > Terminal > Web Browser
                        let g = &self.settings.global;
                        term::open_url(t.trim(), if g.browser_registry { "" } else { &g.browser_path });
                    }
                }
                "keyword" => {
                    if let Some(t) = s.selected_text() {
                        let t = t.trim().to_string();
                        if !t.is_empty() && !t.contains('\n') {
                            self.settings.keywords.push(t);
                            self.dirty = true;
                        }
                    }
                }
                "find" => self.find.open = true,
                "all" => s.select_all(),
                "print" => self.print_text(true),
                "clear_sb" => s.clear_scrollback(),
                "clear" => s.clear_screen(),
                _ => s.clear_all(),
            }
        }
    }

    // ----- dialogs ------------------------------------------------------------------------

    /// Dialogs follow the theme: on the dark theme their windows use the main window's colour (pop-up menus keep their own grey).
    fn dialogs(&mut self, ctx: &egui::Context) {
        let (dark, panel, _) = palette(&self.settings.theme);
        let saved = ctx.global_style().visuals.clone();
        if dark {
            ctx.global_style_mut(|s| {
                s.visuals.window_fill = panel;
                s.visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_gray(70));
            });
        }
        self.dialogs_inner(ctx);
        if dark {
            ctx.global_style_mut(|s| s.visuals = saved);
        }
    }

    fn dialogs_inner(&mut self, ctx: &egui::Context) {
        // ESC closes the sub-window that is on top (Quick Connect and Session Options handle their own).
        if self.quick.is_none() && self.options.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let closed = if self.prompt.is_some() {
                self.prompt = None;
                true
            } else if self.button_editor.is_some() {
                self.button_editor = None;
                true
            } else if self.scratch.is_some() {
                self.scratch = None;
                true
            } else if self.find.open {
                self.find.open = false;
                true
            } else if self.global_open {
                self.global_open = false;
                self.dirty = true;
                true
            } else if self.keymap_open {
                self.keymap_open = false;
                self.dirty = true;
                true
            } else if self.keyassist_open {
                self.keyassist_open = false;
                true
            } else if self.snapshots_open {
                self.snapshots_open = false;
                true
            } else if self.trace_open {
                self.trace_open = false;
                true
            } else if self.creds_open {
                self.creds_open = false;
                self.dirty = true;
                true
            } else if self.about_open {
                self.about_open = false;
                true
            } else if self.tftp_open {
                self.tftp_open = false;
                true
            } else {
                false
            };
            if closed {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            }
        }
        // "Change Password" in the Password properties opens a tab that runs passwd on the server.
        let mut pw_cfg: Option<SessionConfig> = None;
        if let Some(q) = &mut self.quick {
            if q.change_pw {
                q.change_pw = false;
                pw_cfg = Some(q.cfg.clone());
            }
        }
        if let Some(o) = &mut self.options {
            if o.change_pw {
                o.change_pw = false;
                pw_cfg = Some(o.cfg.clone());
            }
        }
        if let Some(mut cfg) = pw_cfg {
            if cfg.host.trim().is_empty() {
                self.status_msg = "Enter the hostname first.".into();
            } else {
                cfg.protocol = Protocol::Ssh;
                cfg.remote_command_on = true;
                cfg.remote_command = "passwd".into();
                cfg.name = format!("Change password - {}", cfg.display_name());
                cfg.start_log_on_connect = false;
                self.open_session(cfg, ctx);
            }
        }

        // Quick Connect
        if let Some(q) = &mut self.quick {
            match dialogs::quick_window(ctx, q, &mut self.settings) {
                QuickOut::Connect => {
                    let q = self.quick.take().unwrap();
                    let cfg = q.cfg.clone();
                    if q.save {
                        let mut c = q.cfg.clone();
                        // saved under the hostname, as the original does; a name already used in the folder gets
                        // " (1)", " (2)"... instead of a second session with the same name
                        let base = if c.name.trim().is_empty() { c.host.trim().to_string() } else { c.name.trim().to_string() };
                        c.name = self.free_session_name(&c.folder, &base);
                        self.settings.sessions.push(c);
                    }
                    self.dirty = true;
                    if q.open_in_tab {
                        self.open_session(cfg, ctx);
                    } else {
                        self.open_new_window(&cfg, None);
                    }
                }
                QuickOut::Cancel => {
                    self.quick = None;
                    self.dirty = true; // the global key and the startup check box are kept
                }
                QuickOut::Open => {}
            }
        }

        // Session options
        if let Some(d) = &mut self.options {
            match dialogs::options_window(ctx, d, &mut self.settings) {
                OptOut::Ok => {
                    let d = self.options.take().unwrap();
                    self.apply_options(d);
                }
                OptOut::Cancel => {
                    self.options = None;
                    self.dirty = true;
                }
                OptOut::Open => {}
            }
        }

        self.prompt_window(ctx);

        if self.global_open {
            match globaldlg::window(ctx, &mut self.settings, &mut self.global_state) {
                globaldlg::GlobalOut::Open => {}
                globaldlg::GlobalOut::Close => {
                    self.global_open = false;
                    self.dirty = true;
                }
                globaldlg::GlobalOut::Tool("create_public_key") => self.tool_create_public_key(ctx),
                globaldlg::GlobalOut::Tool(_) => self.tool_manage_agent_keys(ctx),
            }
        }
        if self.keymap_open {
            if !keymapdlg::window(ctx, &mut self.settings, &mut self.keymap_state) {
                self.keymap_open = false;
                self.dirty = true;
            }
        }
        if let Some(w) = self.wizard.as_mut() {
            match crate::wizard::window(ctx, w, &self.settings) {
                crate::wizard::WizardOut::Open => {}
                crate::wizard::WizardOut::Cancel => self.wizard = None,
                crate::wizard::WizardOut::Finish { cfg, dont_use_again } => {
                    // the name must identify the session: a name already used in the folder is refused and the
                    // wizard stays open on its name page
                    let taken = self.settings.sessions.iter().any(|s| s.folder == cfg.folder && s.display_name().eq_ignore_ascii_case(&cfg.name));
                    if taken {
                        self.error_msg = Some(format!("A session named \"{}\" already exists in this folder.", cfg.name));
                    } else {
                        // the wizard only creates the session (in the Session Manager), as in the original
                        self.settings.sessions.push(cfg);
                        self.selected = Some(self.settings.sessions.len() - 1);
                        if dont_use_again {
                            self.settings.global.new_session_wizard = false;
                        }
                        self.wizard = None;
                        self.dirty = true;
                    }
                }
            }
        }
        if self.keyassist_open {
            let mut open = true;
            let mut create = false;
            dialogs::dialog(ctx, "pubkey_assistant", "Public-Key Assistant", 460.0, &mut open, true, |ui| {
                let dir = std::env::var_os("USERPROFILE").map(|p| std::path::PathBuf::from(p).join(".ssh"));
                let mut found = false;
                if let Some(dir) = dir {
                    if let Ok(rd) = std::fs::read_dir(&dir) {
                        for e in rd.flatten() {
                            let p = e.path();
                            if p.extension().map(|x| x == "pub").unwrap_or(false) {
                                found = true;
                                let text = std::fs::read_to_string(&p).unwrap_or_default();
                                ui.label(RichText::new(p.file_name().unwrap_or_default().to_string_lossy()).strong());
                                ui.add(egui::TextEdit::multiline(&mut text.trim().to_string()).desired_rows(2).desired_width(480.0).font(egui::TextStyle::Monospace));
                                if ui.button("Copy public key").clicked() {
                                    ctx.copy_text(text.trim().to_string());
                                }
                                ui.separator();
                            }
                        }
                    }
                }
                if !found {
                    ui.label("No public keys found in your .ssh folder.");
                }
                if ui.button("Create Public Key...").clicked() {
                    create = true;
                }
            });
            if create {
                self.keyassist_open = false;
                let mut cfg = self.settings.defaults_for(Protocol::LocalShell);
                cfg.name = "Create Public Key".into();
                cfg.shell_command = "ssh-keygen.exe -t ed25519".into();
                self.open_session(cfg, ctx);
            } else if !open {
                self.keyassist_open = false;
            }
        }

        if self.snapshots_open {
            let mut open = true;
            let mut del = None;
            dialogs::dialog(ctx, "snapshots_win", "Manage Snapshots", 360.0, &mut open, true, |ui| {
                for (i, (n, c)) in self.settings.snapshots.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{n} ({} sessions)", c.len()));
                        if ui.small_button("Delete").clicked() {
                            del = Some(i);
                        }
                    });
                }
                if self.settings.snapshots.is_empty() {
                    ui.label("No snapshots saved.");
                }
            });
            if let Some(i) = del {
                self.settings.snapshots.remove(i);
                self.dirty = true;
            }
            if !open {
                self.snapshots_open = false;
            }
        }

        if let Some(text) = &mut self.scratch {
            let mut open = true;
            let mut send = false;
            dialogs::dialog(ctx, "scratchpad_win", "Scratchpad", 500.0, &mut open, true, |ui| {
                ui.add(egui::TextEdit::multiline(text).desired_rows(12).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                if ui.button("Send to session").clicked() {
                    send = true;
                }
            });
            if send {
                let t = text.replace("\r\n", "\r").replace('\n', "\r");
                if let Some(s) = self.sessions.get_mut(self.active) {
                    s.send_str(&t);
                }
            }
            if !open {
                self.scratch = None;
            }
        }

        if self.find.open {
            let mut open = true;
            let mut dir: Option<bool> = None;
            dialogs::dialog(ctx, "find_win", "Find", 380.0, &mut open, true, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find what:");
                    let r = ui.text_edit_singleline(&mut self.find.query);
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        dir = Some(false);
                    }
                });
                ui.checkbox(&mut self.find.case_sensitive, "Match case");
                ui.horizontal(|ui| {
                    if ui.button("< Previous").clicked() {
                        dir = Some(true);
                    }
                    if ui.button("Next >").clicked() {
                        dir = Some(false);
                    }
                });
                if !self.find.message.is_empty() {
                    ui.weak(&self.find.message);
                }
            });
            if let Some(back) = dir {
                let q = self.find.query.clone();
                let cs = self.find.case_sensitive;
                if let Some(s) = self.sessions.get_mut(self.active) {
                    self.find.message = if s.find(&q, back, cs) { String::new() } else { format!("\"{q}\" not found") };
                }
            }
            if !open {
                self.find.open = false;
            }
        }

        if let Some(be) = &mut self.button_editor {
            let mut open = true;
            let mut save = false;
            dialogs::dialog(ctx, "button_win", "Button", 340.0, &mut open, true, |ui| {
                egui::Grid::new("btn").num_columns(2).show(ui, |ui| {
                    ui.label("Label:");
                    ui.text_edit_singleline(&mut be.def.label);
                    ui.end_row();
                    ui.label("Send:");
                    ui.text_edit_singleline(&mut be.def.text);
                    ui.end_row();
                });
                ui.weak("Use \\r for Enter, \\t for Tab, \\e for Esc.");
                if ui.button("OK").clicked() {
                    save = true;
                }
            });
            if save {
                let be = self.button_editor.take().unwrap();
                match be.index {
                    Some(i) if i < self.bar_mut().len() => self.bar_mut()[i] = be.def,
                    _ => self.bar_mut().push(be.def),
                }
                self.dirty = true;
            } else if !open {
                self.button_editor = None;
            }
        }

        // File transfer progress (Xmodem / Ymodem)
        {
            let mut close = false;
            let mut cancel_out: Option<Vec<u8>> = None;
            if let Some(s) = self.sessions.get_mut(self.active) {
                if let Some(x) = s.xfer.as_mut() {
                    let title = x.label().to_string();
                    let mut still_open = true;
                    dialogs::dialog(ctx, "xfer_win", &title, 340.0, &mut still_open, false, |ui| {
                        ui.label(if x.name.is_empty() { "(waiting for the file name)".to_string() } else { x.name.clone() });
                        let frac = if x.total > 0 { (x.bytes_done as f32 / x.total as f32).clamp(0.0, 1.0) } else { 0.0 };
                        ui.add(egui::ProgressBar::new(frac).text(format!("{} / {} bytes", x.bytes_done, x.total)));
                        let col = if x.failed { Color32::from_rgb(0xe0, 0x50, 0x50) } else { ui.visuals().text_color() };
                        ui.label(RichText::new(x.status.clone()).color(col));
                        if x.done {
                            if ui.button("Close").clicked() {
                                close = true;
                            }
                        } else if ui.button("Cancel").clicked() {
                            cancel_out = Some(x.cancel());
                        }
                    });
                }
                if let Some(out) = cancel_out {
                    if let Some(c) = &s.conn {
                        c.send(&out);
                    }
                }
                if close {
                    s.xfer = None;
                }
            }
        }

        self.pass_dialog(ctx);
        if self.zm_open {
            let mut open = true;
            let mut add = false;
            let mut clear = false;
            let mut remove: Option<usize> = None;
            dialogs::dialog(ctx, "zm_list", "Zmodem Upload List", 440.0, &mut open, true, |ui| {
                ui.label("Files sent by Start Zmodem Upload (after the upload command of the session has run on the host):");
                egui::ScrollArea::vertical().max_height(180.0).auto_shrink([false, true]).show(ui, |ui| {
                    if self.zm_list.is_empty() {
                        ui.weak("(no files)");
                    }
                    for (i, p) in self.zm_list.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.small_button("✖").on_hover_text("Remove").clicked() {
                                remove = Some(i);
                            }
                            ui.label(p);
                        });
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Add...").clicked() {
                        add = true;
                    }
                    if ui.add_enabled(!self.zm_list.is_empty(), egui::Button::new("Clear")).clicked() {
                        clear = true;
                    }
                });
            });
            if add {
                if let Some(p) = config::browse_file("Add a file to the Zmodem upload list", None) {
                    self.zm_list.push(p);
                }
            }
            if let Some(i) = remove {
                self.zm_list.remove(i);
            }
            if clear {
                self.zm_list.clear();
            }
            if !open {
                self.zm_open = false;
            }
        }
        if self.trace_open {
            let mut open = true;
            dialogs::dialog(ctx, "trace_win", "Trace Options", 340.0, &mut open, true, |ui| {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    ui.checkbox(&mut s.cfg.ssh_verbose, "Show SSH debug messages (ssh -v)");
                    ui.weak("Takes effect the next time this tab connects.");
                }
            });
            if !open {
                self.trace_open = false;
            }
        }

        if self.creds_open {
            let mut open = true;
            dialogs::dialog(ctx, "creds_win", "Saved Credentials", 400.0, &mut open, true, |ui| {
                ui.set_min_width(380.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_min_size(egui::vec2(150.0, 160.0));
                        for (i, c) in self.settings.credentials.iter().enumerate() {
                            if ui.selectable_label(self.cred_sel == Some(i), if c.name.is_empty() { "(unnamed)" } else { &c.name }).clicked() {
                                self.cred_sel = Some(i);
                            }
                        }
                    });
                    ui.vertical(|ui| {
                        if let Some(i) = self.cred_sel.filter(|i| *i < self.settings.credentials.len()) {
                            let c = &mut self.settings.credentials[i];
                            egui::Grid::new("cred_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut c.name);
                                ui.end_row();
                                ui.label("Username:");
                                ui.text_edit_singleline(&mut c.user);
                                ui.end_row();
                                ui.label("Password:");
                                if self.cred_tmp_for != Some(i) {
                                    self.cred_tmp_for = Some(i);
                                    self.cred_tmp.clear();
                                }
                                let hint = if config::is_protected(&c.pass) { "(saved: type to replace)" } else { "" };
                                if ui.add(egui::TextEdit::singleline(&mut self.cred_tmp).password(true).hint_text(hint)).changed() {
                                    c.pass = config::protect(&self.cred_tmp);
                                }
                                ui.end_row();
                            });
                            ui.weak("(Password will be encrypted.)");
                        } else {
                            ui.weak("Select a credential or add a new one.");
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        self.settings.credentials.push(config::Credential { name: "New credential".into(), ..Default::default() });
                        self.cred_sel = Some(self.settings.credentials.len() - 1);
                    }
                    if ui.add_enabled(self.cred_sel.is_some(), egui::Button::new("Delete")).clicked() {
                        if let Some(i) = self.cred_sel.take() {
                            if i < self.settings.credentials.len() {
                                self.settings.credentials.remove(i);
                            }
                        }
                    }
                });
            });
            if !open {
                self.creds_open = false;
                self.dirty = true;
            }
        }

        // "Public Key Authentication Failed": the server refused the identity file chosen for the session.
        if let Some(idx) = self.sessions.iter().position(|s| s.pubkey_failed.is_some()) {
            let key = self.sessions[idx].pubkey_failed.clone().unwrap_or_default();
            let (user, host) = (self.sessions[idx].cfg.username.clone(), self.sessions[idx].cfg.host.clone());
            let mut open = true;
            let mut ok = false;
            dialogs::dialog(ctx, "pubkey_failed", "Public Key Authentication Failed", 440.0, &mut open, true, |ui| {
                ui.add_space(4.0);
                ui.label(format!("The server {host} did not accept the public key for {}.", if user.is_empty() { "this user" } else { &user }));
                ui.add_space(4.0);
                ui.label(RichText::new(&key).monospace().size(11.5));
                ui.add_space(4.0);
                ui.label("Make sure the matching public key is in the server's authorized_keys file, then try again. The connection continues with the next authentication method.");
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                        ok = true;
                    }
                });
            });
            if ok || !open {
                self.sessions[idx].pubkey_failed = None;
            }
        }
        dialogs::message_window(ctx, "app_error", &mut self.error_msg);
        // The ssh program's questions as windows: "New Host Key" and "Enter Secure Shell Password".
        if let Some(idx) = self.sessions.iter().position(|s| s.prompt_req.is_some()) {
            let req = self.sessions[idx].prompt_req.clone();
            let sid = self.sessions[idx].id;
            let (user, host) = (self.sessions[idx].cfg.username.clone(), self.sessions[idx].cfg.host.clone());
            match req {
                Some(term::SshPrompt::HostKey { detail }) => {
                    let host_line = detail.lines().find(|l| l.contains("authenticity of host")).unwrap_or("").to_string();
                    let shown_host = host_line.split('\'').nth(1).unwrap_or(&host).to_string();
                    let finger = detail.lines().find(|l| l.contains("fingerprint is")).unwrap_or("").trim().to_string();
                    let mut open = true;
                    let mut accept = false;
                    let mut cancel = false;
                    let show_id = egui::Id::new(("hk_view", sid));
                    let mut view: bool = ctx.data_mut(|d| d.get_temp(show_id).unwrap_or(false));
                    dialogs::dialog(ctx, "new_host_key", "New Host Key", 460.0, &mut open, true, |ui| {
                        ui.add_space(4.0);
                        ui.label("The server's host key is not cached in the registry. You have no guarantee that the server is the computer you think it is.");
                        ui.add_space(4.0);
                        ui.label(format!("Hostname: {shown_host}"));
                        if !finger.is_empty() {
                            ui.label(RichText::new(&finger).monospace().size(11.5));
                        }
                        ui.add_space(4.0);
                        ui.label("Accept and save the host key, or cancel the connection?");
                        if view {
                            egui::Frame::new().fill(ui.visuals().extreme_bg_color).stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)).inner_margin(4).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(&detail).monospace().size(11.0));
                            });
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new("View Host Key").min_size(egui::vec2(110.0, 22.0))).clicked() {
                                view = !view;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                                    cancel = true;
                                }
                                if ui.add(egui::Button::new("Accept & Save").min_size(egui::vec2(110.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                                    accept = true;
                                }
                            });
                        });
                    });
                    ctx.data_mut(|d| d.insert_temp(show_id, view));
                    if accept {
                        self.sessions[idx].answer_prompt(Some("yes"));
                    } else if cancel || !open {
                        self.sessions[idx].answer_prompt(None);
                    }
                }
                Some(term::SshPrompt::Password) => {
                    let pw_id = egui::Id::new(("pw_dlg", sid));
                    let (mut pw, mut save): (String, bool) = ctx.data_mut(|d| d.get_temp(pw_id).unwrap_or_default());
                    let mut open = true;
                    let (mut ok, mut skip, mut cancel) = (false, false, false);
                    dialogs::dialog(ctx, "ssh_password", "Enter Secure Shell Password", 380.0, &mut open, true, |ui| {
                        ui.add_space(4.0);
                        ui.label(format!("Enter a password for {}@{}", if user.is_empty() { "user" } else { &user }, host));
                        ui.add_space(4.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut pw).password(true).desired_width(f32::INFINITY));
                        if !r.has_focus() && !r.lost_focus() {
                            r.request_focus();
                        }
                        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            ok = true;
                        }
                        ui.checkbox(&mut save, "Save password");
                        ui.add_space(6.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                                cancel = true;
                            }
                            if ui.add(egui::Button::new("Skip").min_size(egui::vec2(72.0, 22.0))).clicked() {
                                skip = true;
                            }
                            if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                                ok = true;
                            }
                        });
                    });
                    ctx.data_mut(|d| d.insert_temp(pw_id, (pw.clone(), save)));
                    if ok {
                        if save && !pw.is_empty() {
                            let enc = config::protect(&pw);
                            self.sessions[idx].cfg.saved_password = enc.clone();
                            let (h, u, p) = (host.clone(), user.clone(), self.sessions[idx].cfg.port);
                            if let Some(s) = self.settings.sessions.iter_mut().find(|s| s.host == h && s.username == u && s.port == p) {
                                s.saved_password = enc;
                                self.dirty = true;
                            }
                        }
                        self.sessions[idx].answer_prompt(Some(&pw));
                        ctx.data_mut(|d| d.remove::<(String, bool)>(pw_id));
                    } else if skip {
                        self.sessions[idx].answer_prompt(Some(""));
                        ctx.data_mut(|d| d.remove::<(String, bool)>(pw_id));
                    } else if cancel || !open {
                        self.sessions[idx].answer_prompt(None);
                        ctx.data_mut(|d| d.remove::<(String, bool)>(pw_id));
                    }
                }
                None => {}
            }
        }
        // "Command Properties" (Command Manager: add / edit a command)
        if let Some((idx, mut def)) = self.cmd_edit.take() {
            let mut open = true;
            let mut ok = false;
            let mut clear = false;
            let mut cancel = false;
            dialogs::dialog(ctx, "cmd_props", "Command Properties", 560.0, &mut open, true, |ui| {
                dialogs::group(ui, "Action", |ui| {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(170.0);
                            ui.label("Function");
                            egui::ComboBox::from_id_salt("cmd_func").width(165.0).selected_text("Send String").show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Send String");
                            });
                            ui.checkbox(&mut def.disable_tooltip, "Disable tooltip");
                        });
                        ui.vertical(|ui| {
                            ui.label("Send String");
                            ui.add(egui::TextEdit::multiline(&mut def.text).desired_rows(3).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                        });
                    });
                });
                dialogs::group(ui, "Appearance", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Label:");
                        ui.add(egui::TextEdit::singleline(&mut def.label).desired_width(160.0));
                        ui.add_space(8.0);
                        ui.label("Icon color:");
                        egui::ComboBox::from_id_salt("cmd_color").width(120.0).selected_text(def.color.clone()).show_ui(ui, |ui| {
                            for (n, _) in config::COMMAND_COLORS {
                                ui.selectable_value(&mut def.color, n.to_string(), n);
                            }
                        });
                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 8.0, cmd_icon_color(&def.color));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.add(egui::TextEdit::singleline(&mut def.description).desired_width(f32::INFINITY));
                });
                dialogs::group(ui, "Send String commands", |ui| {
                    egui::Grid::new("cmd_help").num_columns(3).spacing([14.0, 2.0]).show(ui, |ui| {
                        let rows = [
                            ("\\r - sends a carriage return (CR)", "\\p - pauses for one second", "\\e - sends an escape"),
                            ("\\n - sends a newline (LF)", "\\t - sends a tab", "\\b - sends a backspace"),
                            ("\\\\ - sends a backslash", "\\v - pastes contents of Clipboard to active session", ""),
                        ];
                        for (a, b, c) in rows {
                            ui.label(RichText::new(a).size(10.5));
                            ui.label(RichText::new(b).size(10.5));
                            ui.label(RichText::new(c).size(10.5));
                            ui.end_row();
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Clear").min_size(egui::vec2(72.0, 22.0))).clicked() {
                        clear = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                            cancel = true;
                        }
                        let valid = !def.label.trim().is_empty() && !def.text.is_empty();
                        if ui.add_enabled(valid, egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0))).clicked() {
                            ok = true;
                        }
                    });
                });
            });
            if clear {
                def.text.clear();
                def.label.clear();
                def.description.clear();
                def.color = "Green".into();
                def.disable_tooltip = false;
            }
            if ok {
                match idx {
                    Some(i) if i < self.settings.commands.len() => self.settings.commands[i] = def,
                    _ => {
                        self.cmd_sel = Some(self.settings.commands.len());
                        self.settings.commands.push(def);
                    }
                }
                self.dirty = true;
            } else if open && !cancel {
                self.cmd_edit = Some((idx, def));
            }
        }
        // "Confirm Multi-line Paste": shown when the text to paste has more than one line.
        if let Some(idx) = self.sessions.iter().position(|s| s.pending_paste.is_some()) {
            let text = self.sessions[idx].pending_paste.clone().unwrap_or_default();
            let shown = text.replace('\r', "\n");
            let mut open = true;
            let mut paste = false;
            let mut dont_show: bool = ctx.data_mut(|d| d.get_temp(egui::Id::new("mp_dont")).unwrap_or(false));
            let mut delay_on = self.settings.paste_delay_on;
            let mut delay_ms = self.settings.paste_delay_ms;
            let mut cancel = false;
            dialogs::dialog(ctx, "multi_paste", "Confirm Multi-line Paste", 600.0, &mut open, true, |ui| {
                ui.add_space(4.0);
                ui.label("Clipboard contents:");
                ui.horizontal_top(|ui| {
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
                        .show(ui, |ui| {
                            egui::ScrollArea::both().id_salt("mp_scroll").max_height(200.0).min_scrolled_height(200.0).show(ui, |ui| {
                                ui.set_min_size(egui::vec2(470.0, 196.0));
                                let mut ro: &str = &shown;
                                ui.add(egui::TextEdit::multiline(&mut ro).font(egui::TextStyle::Monospace).frame(egui::Frame::NONE).desired_width(f32::INFINITY));
                            });
                        });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        let pb = egui::Button::new("Paste").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)));
                        if ui.add(pb).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            paste = true;
                        }
                        if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                            cancel = true;
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut dont_show, "Do not show this dialog again");
                    ui.add_space(30.0);
                    ui.checkbox(&mut delay_on, "Override line send delay:");
                    ui.add_enabled(delay_on, egui::DragValue::new(&mut delay_ms).range(0..=5000));
                    ui.label("milliseconds");
                });
            });
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("mp_dont"), dont_show));
            if paste {
                self.settings.paste_delay_on = delay_on;
                self.settings.paste_delay_ms = delay_ms;
                if dont_show {
                    self.settings.paste_confirm = false;
                }
                self.dirty = true;
                let delay = if delay_on { Some(delay_ms as u64) } else { None };
                self.sessions[idx].paste_text(&text, delay);
                self.sessions[idx].pending_paste = None;
                self.focus_terminal = true;
                ctx.data_mut(|d| d.remove::<bool>(egui::Id::new("mp_dont")));
            } else if !open || cancel {
                self.sessions[idx].pending_paste = None;
                self.focus_terminal = true;
            }
        }
        if let Some((text, act)) = self.confirm.clone() {
            if let Some(yes) = dialogs::confirm_window(ctx, "app_confirm", &text) {
                self.confirm = None;
                if yes {
                    self.run_confirmed(act, ctx);
                }
            }
        }
        if self.tftp_open {
            let mut open = true;
            let mut stop = false;
            ctx.request_repaint_after(Duration::from_millis(500));
            dialogs::dialog(ctx, "tftp_win", "TFTP Server", 500.0, &mut open, true, |ui| {
                if let Some(t) = &self.tftp {
                    ui.label(format!("Listening on UDP port {}", t.port));
                    ui.label(format!("Folder: {}", t.root.display()));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).show(ui, |ui| {
                        for l in t.log.lock().unwrap().iter() {
                            ui.monospace(l);
                        }
                    });
                    if ui.button("Stop TFTP Server").clicked() {
                        stop = true;
                    }
                } else {
                    ui.label("The TFTP server is not running.");
                }
            });
            if stop {
                if let Some(t) = &self.tftp {
                    t.stop();
                }
                self.tftp = None;
                self.status_msg = "TFTP server stopped".into();
            }
            if !open {
                self.tftp_open = false;
            }
        }

        if self.about_open {
            dialogs::dialog(ctx, "about_win", "About Yetty", 400.0, &mut self.about_open, true, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Yetty").strong().size(24.0));
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
                ui.label("A tabbed terminal client for Windows with SSH2, SFTP, Telnet, RLogin, serial and local shell sessions, file transfers, tab groups and scripting.");
                ui.add_space(6.0);
                ui.weak("SSH connections use the OpenSSH client that ships with Windows.");
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(RichText::new("License").strong());
                ui.label("Released under the MIT License.");
                ui.label("Copyright \u{00A9} 2026 Leonardo Berbert");
                ui.add_space(8.0);
                ui.label(RichText::new("Source code and issue tracker").strong());
                ui.hyperlink_to("github.com/leoberbert/Yetty", "https://github.com/leoberbert/Yetty");
                ui.add_space(4.0);
            });
        }
    }

    fn prompt_window(&mut self, ctx: &egui::Context) {
        let Some(p) = &mut self.prompt else { return };
        let mut open = true;
        let mut ok = false;
        let mut cancel = false;
        dialogs::raise(ctx, "prompt");
        dialogs::dialog(ctx, "prompt", p.title, 450.0, &mut open, true, |ui| {
            ui.label(p.label);
            let password = p.kind == Prompt::LockPassword;
            let r = ui.add(egui::TextEdit::singleline(&mut p.text).password(password).desired_width(420.0));
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
            // 0 = nothing to browse, 1 = open a file, 2 = save a file, 3 = choose a folder
            let browse = match p.kind {
                Prompt::SendAscii | Prompt::SendBinary | Prompt::ImportSettings | Prompt::ConvertKey | Prompt::XferSend(_) => 1,
                Prompt::ReceiveAscii | Prompt::LogFile | Prompt::SaveScript | Prompt::ExportSettings | Prompt::ExportSessions | Prompt::XferRecv(_) => 2,
                Prompt::TftpFolder => 3,
                _ => 0,
            };
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    ok = true;
                }
                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    cancel = true;
                }
                if browse != 0 && ui.add(egui::Button::new("Browse...").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    let picked = match browse {
                        1 => config::browse_file("Select file", None),
                        2 => config::browse_save("Save as"),
                        _ => config::browse_folder("Select folder"),
                    };
                    if let Some(path) = picked {
                        p.text = path;
                    }
                }
            });
        });
        if cancel {
            self.prompt = None;
            return;
        }
        if !ok {
            if !open {
                self.prompt = None;
            }
            return;
        }
        let p = self.prompt.take().unwrap();
        let text = p.text.trim().trim_matches('"').to_string();
        match p.kind {
            Prompt::SendAscii => match std::fs::read_to_string(&text) {
                Ok(t) => {
                    let t = t.replace("\r\n", "\r").replace('\n', "\r");
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.send_str(&t);
                    }
                    self.status_msg = "File sent".into();
                }
                Err(e) => self.status_msg = format!("Could not read the file: {e}"),
            },
            Prompt::SendBinary => match std::fs::read(&text) {
                Ok(b) => {
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        if let Some(c) = &s.conn {
                            c.send(&b);
                        }
                    }
                    self.status_msg = format!("Sent {} bytes", b.len());
                }
                Err(e) => self.status_msg = format!("Could not read the file: {e}"),
            },
            Prompt::ReceiveAscii => {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    self.status_msg = match s.start_capture(&text) {
                        Ok(()) => "Receiving text...".into(),
                        Err(e) => e,
                    };
                }
            }
            Prompt::LogFile => {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    self.status_msg = match s.start_log(Some(text)) {
                        Ok(p) => format!("Logging to {p}"),
                        Err(e) => e,
                    };
                }
            }
            Prompt::SaveScript => {
                if let Some(s) = self.sessions.iter_mut().find(|s| Some(s.id) == self.rec_target) {
                    if let Some(lines) = s.recording.take() {
                        if let Some(parent) = std::path::Path::new(&text).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let mut body = String::from("# Recorded script\n");
                        body.push_str(&lines.join("\n"));
                        self.status_msg = match std::fs::write(&text, body) {
                            Ok(()) => format!("Script saved to {text}"),
                            Err(e) => format!("Could not save the script: {e}"),
                        };
                    }
                }
            }
            Prompt::ExportSettings => {
                if let Ok(j) = serde_json::to_string_pretty(&self.settings) {
                    self.status_msg = match std::fs::write(&text, j) {
                        Ok(()) => format!("Settings exported to {text}"),
                        Err(e) => format!("Could not export: {e}"),
                    };
                }
            }
            Prompt::ImportSettings => match std::fs::read_to_string(&text).ok().and_then(|s| serde_json::from_str::<Settings>(&s).ok()) {
                Some(s) => {
                    self.settings = s;
                    self.dirty = true;
                    self.status_msg = "Settings imported".into();
                }
                None => self.status_msg = "That file is not a valid settings file".into(),
            },
            Prompt::ExportSessions => {
                let list: Vec<&SessionConfig> = self.selected.and_then(|i| self.settings.sessions.get(i)).into_iter().collect();
                let list: Vec<&SessionConfig> = if list.is_empty() { self.settings.sessions.iter().collect() } else { list };
                if let Ok(j) = serde_json::to_string_pretty(&list) {
                    self.status_msg = match std::fs::write(&text, j) {
                        Ok(()) => format!("Sessions exported to {text}"),
                        Err(e) => format!("Could not export: {e}"),
                    };
                }
            }
            Prompt::RenameCommand(i) => {
                let t = text.trim().to_string();
                if t.is_empty() {
                    self.error_msg = Some("Enter a name for the command.".into());
                } else if let Some(c) = self.settings.commands.get_mut(i) {
                    c.label = t;
                    self.dirty = true;
                }
            }
            Prompt::RenameCmdFolder(old) => {
                let t = text.trim().trim_matches('/').to_string();
                if t.is_empty() {
                    self.error_msg = Some("Enter a name for the folder.".into());
                } else if t.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    self.error_msg = Some("A folder name cannot contain any of these characters:\n\\ / : * ? \" < > |".into());
                } else {
                    let parent = old.rsplit_once('/').map(|x| x.0.to_string());
                    let new = match parent {
                        Some(p) => format!("{p}/{t}"),
                        None => t,
                    };
                    let fix = |p: &str| -> String {
                        if p == old {
                            new.clone()
                        } else if let Some(rest) = p.strip_prefix(&format!("{old}/")) {
                            format!("{new}/{rest}")
                        } else {
                            p.to_string()
                        }
                    };
                    for c in self.settings.commands.iter_mut() {
                        c.folder = fix(c.folder.trim_matches('/'));
                    }
                    for f in self.settings.command_folders.iter_mut() {
                        *f = fix(f);
                    }
                    self.cmd_sel_folder = Some(new);
                    self.dirty = true;
                }
            }
            Prompt::CmdFolder => {
                let t = text.trim_matches('/').to_string();
                if t.is_empty() {
                    self.error_msg = Some("Enter a name for the folder.".into());
                } else if t.chars().any(|c| "\\:*?\"<>|".contains(c)) {
                    self.error_msg = Some("A folder name cannot contain any of these characters:\n\\ : * ? \" < > |".into());
                } else if self.settings.command_folders.iter().any(|f| f.eq_ignore_ascii_case(&t)) {
                    self.error_msg = Some(format!("A folder named \"{t}\" already exists."));
                } else {
                    self.settings.command_folders.push(t);
                    self.dirty = true;
                }
            }
            Prompt::NewFolder => {
                let t = text.trim_matches('/').to_string();
                if t.is_empty() {
                    self.error_msg = Some("Enter a name for the folder.".into());
                } else if t.chars().any(|c| "\\:*?\"<>|".contains(c)) {
                    self.error_msg = Some("A folder name cannot contain any of these characters:\n\\ : * ? \" < > |".into());
                } else if self.settings.folders.iter().any(|f| f.eq_ignore_ascii_case(&t)) {
                    self.error_msg = Some(format!("A folder named \"{t}\" already exists."));
                } else {
                    self.settings.folders.push(t);
                    self.dirty = true;
                }
            }
            Prompt::DuplicateSession(i) => {
                let folder = self.settings.sessions.get(i).map(|s| s.folder.clone()).unwrap_or_default();
                if text.is_empty() {
                    self.error_msg = Some("Enter a name for the session.".into());
                } else if text.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    self.error_msg = Some("A session name cannot contain any of these characters:\n\\ / : * ? \" < > |".into());
                } else if self.settings.sessions.iter().any(|s| s.folder == folder && s.display_name().eq_ignore_ascii_case(&text)) {
                    self.error_msg = Some(format!("A session named \"{text}\" already exists in this folder."));
                } else if let Some(mut c) = self.settings.sessions.get(i).cloned() {
                    c.name = text;
                    self.settings.sessions.insert(i + 1, c);
                    self.selected = Some(i + 1);
                    self.dirty = true;
                }
            }
            Prompt::RenameSession(i) => {
                let folder = self.settings.sessions.get(i).map(|s| s.folder.clone()).unwrap_or_default();
                if text.is_empty() {
                    self.error_msg = Some("Enter a name for the session.".into());
                } else if text.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    self.error_msg = Some("A session name cannot contain any of these characters:\n\\ / : * ? \" < > |".into());
                } else if self.settings.sessions.iter().enumerate().any(|(k, s)| k != i && s.folder == folder && s.display_name().eq_ignore_ascii_case(&text)) {
                    self.error_msg = Some(format!("A session named \"{text}\" already exists in this folder."));
                } else if let Some(s) = self.settings.sessions.get_mut(i) {
                    s.name = text;
                    self.dirty = true;
                }
            }
            Prompt::RenameFolder(old) => {
                let parent = old.rsplit_once('/').map(|x| x.0.to_string());
                let new = match parent {
                    Some(p) => format!("{p}/{text}"),
                    None => text,
                };
                for s in &mut self.settings.sessions {
                    if s.folder == old {
                        s.folder = new.clone();
                    } else if let Some(rest) = s.folder.strip_prefix(&format!("{old}/")) {
                        s.folder = format!("{new}/{rest}");
                    }
                }
                for f in &mut self.settings.folders {
                    if *f == old {
                        *f = new.clone();
                    } else if let Some(rest) = f.strip_prefix(&format!("{old}/")) {
                        *f = format!("{new}/{rest}");
                    }
                }
                self.selected_folder = Some(new);
                self.dirty = true;
            }
            Prompt::SnapshotName => {
                if !text.is_empty() {
                    let cfgs: Vec<SessionConfig> = self.sessions.iter().map(|s| s.cfg.clone()).collect();
                    self.settings.snapshots.retain(|e| e.0 != text);
                    self.settings.snapshots.push((text.clone(), cfgs));
                    self.last_snapshot = Some(text);
                    self.dirty = true;
                }
            }
            Prompt::RenameTab(i) => {
                if let Some(s) = self.sessions.get_mut(i) {
                    s.title = text;
                    s.renamed = true;
                }
            }
            Prompt::SaveSession(id) => {
                let name = text.trim().to_string();
                if let (false, Some(i)) = (name.is_empty(), self.index_of(id)) {
                    let mut cfg = self.sessions[i].cfg.clone();
                    cfg.name = name.clone();
                    // a saved session of that name (in the root folder) is replaced, as Save Session does
                    match self.settings.sessions.iter().position(|x| x.name == name && x.folder.trim_matches('/').is_empty()) {
                        Some(k) => {
                            cfg.folder = self.settings.sessions[k].folder.clone();
                            self.settings.sessions[k] = cfg;
                            self.selected = Some(k);
                        }
                        None => {
                            cfg.folder.clear();
                            self.settings.sessions.push(cfg);
                            self.selected = Some(self.settings.sessions.len() - 1);
                        }
                    }
                    self.dirty = true;
                }
            }
            Prompt::XferSend(proto) => {
                let path = std::path::Path::new(&text);
                let made = if proto == crate::transfer::Proto::Kermit {
                    crate::transfer::Transfer::new_kermit_send(path)
                } else {
                    crate::transfer::Transfer::new_send(proto, path)
                };
                match made {
                    Ok(t) => {
                        if let Some(s) = self.sessions.get_mut(self.active) {
                            s.xfer = Some(t);
                        }
                    }
                    Err(e) => self.status_msg = e,
                }
            }
            Prompt::XferRecv(proto) => {
                let t = if proto == crate::transfer::Proto::Kermit {
                    crate::transfer::Transfer::new_kermit_receive(std::path::PathBuf::from(&text))
                } else {
                    crate::transfer::Transfer::new_receive(proto, std::path::PathBuf::from(&text))
                };
                if let Some(s) = self.sessions.get_mut(self.active) {
                    s.xfer = Some(t);
                }
            }
            Prompt::ConvertKey => config::open_console(&format!("ssh-keygen -p -o -f \"{text}\"")),
            Prompt::NewButtonBar => {
                let name = text.trim().to_string();
                if name.is_empty() || name.eq_ignore_ascii_case("default") || self.settings.button_bars.iter().any(|b| b.0.eq_ignore_ascii_case(&name)) {
                    self.status_msg = "Enter a name that is not used by another button bar.".into();
                } else {
                    self.settings.button_bars.push((name.clone(), Vec::new()));
                    if let Some(s) = self.sessions.get_mut(self.active) {
                        s.cfg.button_bar = name;
                    }
                    self.dirty = true;
                }
            }
            Prompt::TftpFolder => match crate::tftp::TftpServer::start("0.0.0.0:69", std::path::PathBuf::from(&text)) {
                Ok(s) => {
                    self.tftp = Some(s);
                    self.tftp_open = true;
                    self.status_msg = "TFTP server started".into();
                }
                Err(e) => self.status_msg = e,
            },
            Prompt::CredPassword(i) => {
                if let Some(c) = self.settings.credentials.get_mut(i) {
                    c.pass = config::protect(&text);
                    self.dirty = true;
                    self.status_msg = "Credentials password changed".into();
                }
            }
            Prompt::LockPassword => {
                if let Some(s) = self.sessions.get_mut(self.active) {
                    if !text.is_empty() {
                        s.locked = Some(text);
                    }
                }
            }
        }
    }

    fn any_dialog_open(&self) -> bool {
        self.quick.is_some()
            || self.wizard.is_some()
            || self.options.is_some()
            || self.prompt.is_some()
            || self.global_open
            || self.keymap_open
            || self.find.open
            || self.button_editor.is_some()
            || self.scratch.is_some()
            || self.keyassist_open
            || self.snapshots_open
            || self.trace_open
            || self.creds_open
            || self.about_open
            || self.tftp_open
            || self.error_msg.is_some()
            || self.confirm.is_some()
            || self.cmd_edit.is_some()
            || self.sessions.iter().any(|s| s.prompt_req.is_some())
            || self.sessions.iter().any(|s| s.pending_paste.is_some())
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        let wanted = std::env::var("YETTY_SHOT_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(120u32);
        if self.frames >= wanted && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let mut bytes = Vec::with_capacity(w * h * 4);
            for p in &img.pixels {
                bytes.extend_from_slice(&p.to_array());
            }
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = image::save_buffer(&path, &bytes, w as u32, h as u32, image::ColorType::Rgba8) {
                eprintln!("could not save screenshot: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frames > wanted + 900 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for YettyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for s in &mut self.sessions {
            if s.global_identity != self.settings.global_identity {
                s.global_identity = self.settings.global_identity.clone();
            }
            s.pump();
            if s.xfer.is_some() {
                ctx.request_repaint_after(Duration::from_millis(40));
            }
        }
        // The window's own close button: ask first while a session is connected.
        if self.screenshot.is_none() && !self.exit_ok && self.connected_count() > 0 && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_exit(ctx);
        }
        if let Some(i) = self.sessions.iter().position(|s| s.close_due) {
            self.close_tab(i); // "Close on disconnect"
        }
        if let Some(i) = self.sessions.iter().position(|s| s.log_prompt_due) {
            self.sessions[i].log_prompt_due = false;
            self.active = i;
            let p = self.sessions[i].default_log_path();
            self.prompt(Prompt::LogFile, "Log Session", "Log file:", p);
        }
        // logon scripts that became due after connecting
        let due: Vec<(u64, String)> = self
            .sessions
            .iter_mut()
            .filter(|s| s.logon_script_due)
            .map(|s| {
                s.logon_script_due = false;
                (s.id, s.cfg.logon_script_path.clone())
            })
            .collect();
        for (id, path) in due {
            self.start_script(id, &path);
        }
        self.run_scripts(ctx);
        if self.dirty && self.screenshot.is_none() && self.settings.auto_save {
            self.settings.save();
            self.dirty = false;
        }
        // window title: "<tab> - Yetty", or "not connected - Yetty"; Global Options > Appearance > Advanced >
        // Application Window > "Title bar:" replaces the program name
        let g = &self.settings.global;
        let app_name = if g.title_bar_on && !g.title_bar.trim().is_empty() { g.title_bar.trim().to_string() } else { "Yetty".to_string() };
        let title = match self.sessions.get(self.active) {
            Some(s) if s.is_connected() => format!("{} - {app_name}", s.title),
            _ => format!("not connected - {app_name}"),
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // The configuration passphrase comes first: nothing else is shown (or connected) before it is typed.
        if self.unlock_ui(&ctx) {
            return;
        }
        self.sync_groups();
        // Alt+letter is a shortcut or Meta, never typing: the window library also reports the letter as a text
        // event right after the key, which reached the focused box or the host ("g" after Alt+G). AltGr arrives as
        // Ctrl+Alt and keeps its text.
        ctx.input_mut(|i| {
            let mut after_alt = false;
            i.events.retain(|e| match e {
                egui::Event::Key { pressed: true, modifiers, .. } => {
                    after_alt = modifiers.alt && !modifiers.ctrl;
                    true
                }
                egui::Event::Text(_) if after_alt => {
                    after_alt = false;
                    false
                }
                _ => true,
            });
        });
        // Global Options > Terminal > "Exit when last session is disconnected"
        let any_connected = self.sessions.iter().any(|s| s.is_connected());
        if self.settings.global.exit_on_last_disconnect && self.was_connected && !any_connected && self.screenshot.is_none() {
            self.exit_ok = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.was_connected = any_connected;
        if !self.any_dialog_open() {
            self.shortcuts(&ctx);
        }
        // Typing always belongs to the session window: when a menu button or toolbar button still holds the
        // keyboard focus (a menu was closed with ESC, the window was just started...) and no menu, dialog or
        // text box is in use, the next key press hands the focus back to the terminal.
        if !self.sessions.is_empty()
            && !self.any_dialog_open()
            && !egui::Popup::is_any_open(&ctx)
            && !ctx.text_edit_focused()
            && !self.sm_hover
            && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { pressed: true, .. })))
        {
            if let Some(f) = ctx.memory(|m| m.focused()) {
                if !self.focus_terminal {
                    let is_term = self.sessions.iter().any(|s| egui::Id::new(("terminal", s.id)) == f);
                    if !is_term {
                        ctx.memory_mut(|m| m.surrender_focus(f));
                        self.focus_terminal = true;
                    }
                }
            } else {
                self.focus_terminal = true;
            }
        }
        self.handle_file_drops(&ctx);
        if self.side == Some(Side::CommandManager) && !self.settings.show_command_manager {
            self.side = None;
        }
        let (dark, panel_fill, central_fill) = palette(&self.settings.theme);
        let status_fill = if dark { Color32::from_gray(0x33) } else { panel_fill };
        let bar_frame = egui::Frame::new().fill(panel_fill).inner_margin(egui::Margin::symmetric(6, 2));
        if self.settings.show_menu_bar {
            egui::Panel::top("menu").frame(bar_frame).show_separator_line(false).show(ui, |ui| self.menu_bar(ui));
        }
        if self.settings.show_toolbar {
            egui::Panel::top("toolbar").frame(bar_frame).show_separator_line(false).show(ui, |ui| self.toolbar(ui));
        }
        if self.settings.show_status_bar {
            egui::Panel::bottom("status").frame(bar_frame.fill(status_fill)).show_separator_line(false).show(ui, |ui| self.status_bar(ui));
        }
        if self.settings.show_button_bar {
            // Global Options > Terminal > Appearance > "Show button bar on: bottom / top"
            let panel = if self.settings.global.button_bar_top { egui::Panel::top("buttons") } else { egui::Panel::bottom("buttons") };
            panel.frame(bar_frame).show(ui, |ui| self.button_bar(ui));
        }
        if self.settings.show_command_window {
            egui::Panel::bottom("command").frame(bar_frame.inner_margin(egui::Margin { left: 6, right: 2, top: 0, bottom: 2 })).show_separator_line(false).show(ui, |ui| self.command_window(ui));
        }
        if self.settings.show_hex_view {
            egui::Panel::bottom("hex").frame(bar_frame.inner_margin(6)).show(ui, |ui| self.hex_view(ui));
        }
        if self.settings.show_tabs && self.layout == Layout::Tabs && self.n_groups <= 1 {
            egui::Panel::bottom("groupbar")
                .frame(egui::Frame::new().fill(panel_fill).inner_margin(egui::Margin { left: 12, right: 4, top: 4, bottom: 4 }))
                .show_separator_line(false)
                .show(ui, |ui| self.group_bar(ui));
        }
        egui::Panel::left("stripe")
            .exact_size(20.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(panel_fill).inner_margin(egui::Margin::symmetric(1, 0)))
            .show(ui, |ui| self.side_stripe(ui));
        // Auto Hide: while the panel is not pinned it floats over the session and goes away
        // as soon as the person clicks anywhere outside it (a menu or pop-up on top does not count).
        if self.side.is_some() && !self.pinned {
            let pressed = ctx.input(|i| i.pointer.primary_pressed());
            if pressed {
                if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                    let on_top = ctx.layer_id_at(p).map(|l| l.order != egui::Order::Background).unwrap_or(false);
                    let stripe_rect = egui::Rect::from_min_max(
                        egui::pos2(0.0, ui.max_rect().min.y),
                        egui::pos2(22.0, ui.max_rect().max.y),
                    );
                    if !on_top && !self.side_rect.contains(p) && !stripe_rect.contains(p) {
                        self.side = None;
                    }
                }
            }
        }
        if let Some(side) = self.side {
            if self.pinned {
                let r = egui::Panel::left("side_panel")
                    .resizable(true)
                    .default_size(300.0)
                    .min_size(170.0)
                    .frame(egui::Frame::new().fill(panel_fill).inner_margin(4))
                    .show(ui, |ui| match side {
                        Side::SessionManager => self.session_manager(ui),
                        Side::ActiveSessions => self.active_sessions_panel(ui),
                        Side::CommandManager => self.command_manager(ui),
                    });
                self.side_rect = r.response.rect;
            } else {
                let avail = ui.available_rect_before_wrap();
                let w = 300.0_f32.min(avail.width() - 40.0).max(170.0);
                let area = egui::Area::new(egui::Id::new("side_overlay"))
                    .order(egui::Order::Middle)
                    .fixed_pos(avail.min)
                    .show(&ctx, |ui| {
                        egui::Frame::new()
                            .fill(panel_fill)
                            .stroke(egui::Stroke::new(1.0, Color32::from_gray(70)))
                            .inner_margin(4)
                            .show(ui, |ui| {
                                ui.set_width(w - 8.0);
                                ui.set_height(avail.height() - 8.0);
                                match side {
                                    Side::SessionManager => self.session_manager(ui),
                                    Side::ActiveSessions => self.active_sessions_panel(ui),
                                    Side::CommandManager => self.command_manager(ui),
                                }
                            });
                    });
                self.side_rect = area.response.rect;
            }
        } else {
            self.side_rect = egui::Rect::NOTHING;
        }
        // In the original the tab bar belongs to the Tabs view: tiled and cascaded sessions have window title
        // bars instead. "Only show tabs when there are more than one" and "Tab location" come from Global Options.
        let g = &self.settings.global;
        let tabs_wanted = self.settings.show_tabs && self.layout == Layout::Tabs && self.sessions.len() > usize::from(g.tabs_only_multiple);
        let tab_bottom = g.tab_bottom;
        let grouped = self.layout == Layout::Tabs && self.n_groups > 1;
        if tabs_wanted && !grouped {
            let frame = egui::Frame::new().fill(panel_fill).inner_margin(egui::Margin { left: 2, right: 2, top: 3, bottom: if tab_bottom { 3 } else { 0 } });
            let panel = if tab_bottom { egui::Panel::bottom("tabs") } else { egui::Panel::top("tabs") };
            panel.frame(frame).show_separator_line(false).show(ui, |ui| self.tabs(ui, None));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(central_fill)).show(ui, |ui| {
            if grouped {
                self.group_area(ui, tabs_wanted);
            } else {
                self.terminal_area(ui);
            }
        });

        self.dialogs(&ctx);
        self.screenshot_step(&ctx);
    }

    fn on_exit(&mut self) {
        // Auto Session > "Remember and connect to sessions from last use"
        if self.settings.global.auto_session == 2 {
            self.settings.global.last_sessions = self.sessions.iter().map(|s| s.cfg.clone()).collect();
        }
        if self.screenshot.is_none() && self.settings.auto_save {
            self.settings.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::natural_cmp;

    #[test]
    fn logical_order_compares_numbers_by_value() {
        let mut names = vec!["host10", "Host2", "host02b", "host1", "alpha", "host2"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, ["alpha", "host1", "Host2", "host2", "host02b", "host10"]);
    }
}
