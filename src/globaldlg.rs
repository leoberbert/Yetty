//! Global Options: the original's category dialog (General ... SSH Host Keys), page by page as its
//! Global Options window shows them. What Yetty does not implement is shown greyed out with the
//! original's default values.

use crate::config::{self, Credential, Rgb, SessionConfig, Settings};
use crate::dialogs::{dialog, group};
use egui::{Color32, RichText, Sense, Stroke};
use serde::{Deserialize, Serialize};

const NA: &str = "Not available in Yetty";

/// Global Options values that have no older field in Settings (Settings::global).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalOpts {
    /// General > Dialogs "Show confirm disconnect dialog"
    pub confirm_disconnect: bool,
    /// "Show Session Manager or Connect dialog on startup"
    pub show_sm_on_startup: bool,
    /// "New session uses wizard"
    pub new_session_wizard: bool,
    /// "Save expanded folder state for session list"
    pub save_tree_state: bool,
    /// "Session list sort order": alphabetically (the original's "Sort Alphabetical"=1) or as arranged
    pub sort_alphabetical: bool,
    /// "Recently used session list:" (1-16)
    pub recent_list_size: u32,
    /// Auto Session: 0 do not connect, 1 local shell, 2 sessions from last use, 3 snapshot, 4 auto session
    pub auto_session: u8,
    pub auto_snapshot: String,
    /// the sessions open at the last exit (for auto_session 2)
    pub last_sessions: Vec<SessionConfig>,
    pub auto_reconnect: bool,
    pub reconnect_secs: u32,
    pub exit_on_last_disconnect: bool,
    /// "Prompt on multi-line paste for [n] or more newlines"
    pub paste_prompt_lines: u32,
    /// "Quotation character(s):" / "Insert pasted text between"
    pub quote_chars: String,
    pub quote_between: bool,
    /// "Paste on [middle|right] button": paste_button 0 middle, 1 right
    pub paste_on_button: bool,
    pub paste_button: u8,
    pub hide_pointer_on_key: bool,
    pub url_ctrl_click: bool,
    pub triple_click_newline: bool,
    /// Appearance "Show button bar on: [bottom|top]"
    pub button_bar_top: bool,
    /// Status Bar page
    pub status_log_name: bool,
    pub status_caps: bool,
    pub status_num: bool,
    pub status_columns: Vec<String>,
    /// Appearance > Advanced > Application Window "Title bar:"
    pub title_bar_on: bool,
    pub title_bar: String,
    /// Tabs/Tiling
    pub tab_bottom: bool,
    pub tabs_only_multiple: bool,
    pub alt_n_switch: bool,
    /// Web Browser
    pub browser_registry: bool,
    pub browser_path: String,
    /// Terminal > Advanced
    pub no_zoom_wheel: bool,
    pub no_zoom_keys: bool,
    pub swap_enter: bool,
    /// "Initially send commands to:" 0 Active Session, 1 All Sessions, 2 Visible Sessions, 3 Selected Tab Group(s)
    pub initial_send_target: u8,
    /// ANSI Color
    pub ansi_palette_name: String,
    pub custom_palettes: Vec<(String, Vec<[u8; 3]>)>,
}

impl Default for GlobalOpts {
    fn default() -> Self {
        Self {
            confirm_disconnect: true,
            show_sm_on_startup: false,
            new_session_wizard: true,
            save_tree_state: true,
            sort_alphabetical: true,
            recent_list_size: 4,
            auto_session: 0,
            auto_snapshot: String::new(),
            last_sessions: Vec::new(),
            auto_reconnect: false,
            reconnect_secs: 5,
            exit_on_last_disconnect: false,
            paste_prompt_lines: 1,
            quote_chars: "\"".into(),
            quote_between: true,
            paste_on_button: false,
            paste_button: 0,
            hide_pointer_on_key: true,
            url_ctrl_click: true,
            triple_click_newline: false,
            button_bar_top: false,
            status_log_name: true,
            status_caps: true,
            status_num: true,
            status_columns: STATUS_COLUMNS[..4].iter().map(|s| s.to_string()).collect(),
            title_bar_on: false,
            title_bar: String::new(),
            tab_bottom: false,
            tabs_only_multiple: false,
            alt_n_switch: true,
            browser_registry: true,
            browser_path: String::new(),
            no_zoom_wheel: false,
            no_zoom_keys: true,
            swap_enter: false,
            initial_send_target: 0,
            ansi_palette_name: "Standard".into(),
            custom_palettes: Vec::new(),
        }
    }
}

/// The status bar columns that can be chosen (Status Bar page), in the original's order; the first four
/// are the default selection.
pub const STATUS_COLUMNS: [&str; 8] = [
    "Protocol Information",
    "Cursor Position",
    "Rows and Columns",
    "Emulation",
    "Elapsed Time",
    "Elapsed Time w/Seconds",
    "Local Time",
    "TFTP Server Status",
];
const STATUS_MAX: usize = 5;

/// The original's built-in ANSI palettes (Global.ini): normal colours 0-7, then bold 8-15.
const PALETTES: &[(&str, [u32; 16])] = &[
    ("Birds of Paradise", [0x573d26, 0xbe2d26, 0x6ba18a, 0xe99d2a, 0x5a86ad, 0xac80a6, 0x74a6ad, 0xe0dbb7, 0x9b6c4a, 0xe84627, 0x95d8ba, 0xd0d150, 0xb8d3ed, 0xd19ecb, 0x93cfd7, 0xfff9d5]),
    ("Chalkboard", [0x000000, 0xc37372, 0x72c373, 0xc2c372, 0x7372c3, 0xc372c2, 0x72c2c3, 0xd9d9d9, 0x323232, 0xdbaaaa, 0xaadbaa, 0xdadbaa, 0xaaaadb, 0xdbaada, 0xaadadb, 0xffffff]),
    ("Ciapre", [0x181818, 0x810009, 0x48513b, 0xcc8b3f, 0x576d8c, 0x724d7c, 0x5c4f4b, 0xaea47f, 0x555555, 0xac3835, 0xa6a75d, 0xdcdf7c, 0x3097c6, 0xd33061, 0xf3dbb2, 0xf4f4f4]),
    ("Dark Pastel", [0x000000, 0xff5555, 0x55ff55, 0xffff55, 0x5555ff, 0xff55ff, 0x55ffff, 0xbbbbbb, 0x555555, 0xff5555, 0x55ff55, 0xffff55, 0x5555ff, 0xff55ff, 0x55ffff, 0xffffff]),
    ("Desert", [0x4d4d4d, 0xff2b2b, 0x98fb98, 0xf0e68c, 0xcd853f, 0xffdead, 0xffa0a0, 0xf5deb3, 0x555555, 0xff5555, 0x55ff55, 0xffff55, 0x87ceff, 0xff55ff, 0xffd700, 0xffffff]),
    ("Espresso", [0x353535, 0xd25252, 0xa5c261, 0xffc66d, 0x6c99bb, 0xd197d9, 0xbed6ff, 0xeeeeec, 0x535353, 0xf00c0c, 0xc2e075, 0xe1e48b, 0x8ab7d9, 0xefb5f7, 0xdcf4ff, 0xffffff]),
    ("Idletoes", [0x323232, 0xd25252, 0x7fe173, 0xffc66d, 0x4099ff, 0xf680ff, 0xbed6ff, 0xeeeeec, 0x535353, 0xf07070, 0x9dff91, 0xffe48b, 0x5eb7f7, 0xff9dff, 0xdcf4ff, 0xffffff]),
    ("Invisibone", [0x303030, 0xd370a3, 0x6d9e3f, 0xb58858, 0x6095c5, 0xac7bde, 0x3ba275, 0xcfcfcf, 0x686868, 0xffa7da, 0xa3d572, 0xefbd8b, 0x98cbfe, 0xe5b0ff, 0x75daa9, 0xffffff]),
    ("Novel", [0x000000, 0xcc0000, 0x009600, 0xd06b00, 0x0000cc, 0xcc00cc, 0x0087cc, 0xcccccc, 0x808080, 0xcc0000, 0x009600, 0xd06b00, 0x0000cc, 0xcc00cc, 0x0087cc, 0xffffff]),
    ("Solarized Dark", [0x073642, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5, 0x002b38, 0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3]),
    ("Solarized Darcula", [0x25292a, 0xf24840, 0x629655, 0xb68800, 0x2075c7, 0x797fd4, 0x15968d, 0xd2d8d9, 0x25292a, 0xf24840, 0x629655, 0xb68800, 0x2075c7, 0x797fd4, 0x15968d, 0xd2d8d9]),
    ("Solarized Light", [0x073642, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5, 0x002b38, 0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3]),
    ("Tomorrow", [0x8e908c, 0xff3334, 0x9ec400, 0xeab700, 0x5795e6, 0xb777e0, 0x54ced6, 0xefefef, 0x000000, 0xc82829, 0x718c00, 0xf5871f, 0x4271ae, 0x8959a8, 0x3e999f, 0xd6d6d6]),
    ("Zenburn", [0x4d4d4d, 0x705050, 0x60b48a, 0xf0dfaf, 0x506070, 0xdc8cc3, 0x8cd0d3, 0xdcdccc, 0x709080, 0xdca3a3, 0xc3bf9f, 0xe0cf9f, 0x94bff3, 0xec93d3, 0x93e0e3, 0xffffff]),
];

fn palette_of(v: &[u32; 16]) -> [Rgb; 16] {
    v.map(|c| [(c >> 16) as u8, (c >> 8) as u8, c as u8])
}

/// The colours of a palette by name: "Standard", a built-in one or a custom one.
fn named_palette(st: &Settings, name: &str) -> Option<[Rgb; 16]> {
    if name == "Standard" {
        return Some(config::ANSI_PALETTE);
    }
    if let Some((_, p)) = PALETTES.iter().find(|p| p.0 == name) {
        return Some(palette_of(p));
    }
    st.global.custom_palettes.iter().find(|p| p.0 == name).and_then(|p| <[Rgb; 16]>::try_from(p.1.as_slice()).ok())
}

fn builtin_palette(name: &str) -> bool {
    name == "Standard" || PALETTES.iter().any(|p| p.0 == name)
}

struct CredEdit {
    /// None = Add
    index: Option<usize>,
    name: String,
    user: String,
    /// a new password (empty = keep the saved one)
    pass: String,
    had_pass: bool,
    err: String,
}

#[derive(Clone)]
struct HostKey {
    /// line number in known_hosts
    line: usize,
    raw: String,
    host: String,
    port: u16,
    kind: String,
}

pub struct GlobalState {
    pub page: String,
    /// The settings and ANSI colours as they were when the dialog opened: Cancel puts them back.
    orig: Option<(Settings, [Rgb; 16])>,
    collapsed: Vec<&'static str>,
    cred_sel: Option<usize>,
    cred_edit: Option<CredEdit>,
    /// (in the Selected list, row)
    status_sel: Option<(bool, usize)>,
    palette_new: Option<String>,
    palette_err: String,
    scheme_sel: Option<usize>,
    host_keys: Option<Vec<HostKey>>,
    host_sel: Option<usize>,
    host_view: Option<HostKey>,
    host_delete: Option<HostKey>,
    fp_md5: bool,
    /// (identity path, md5, fingerprint text)
    fingerprint: Option<(String, bool, String)>,
    msg: Option<String>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            page: "general".into(),
            orig: None,
            collapsed: Vec::new(),
            cred_sel: None,
            cred_edit: None,
            status_sel: None,
            palette_new: None,
            palette_err: String::new(),
            scheme_sel: None,
            host_keys: None,
            host_sel: None,
            host_view: None,
            host_delete: None,
            fp_md5: false,
            fingerprint: None,
            msg: None,
        }
    }
}

pub enum GlobalOut {
    Open,
    Close,
    /// open a Yetty tool window: "create_public_key" (Create Identity File...), "manage_agent_keys"
    Tool(&'static str),
}

/// (depth, tree label, page id, page title)
const TREE: &[(usize, &str, &str, &str)] = &[
    (0, "General", "general", "General"),
    (1, "Configuration Paths", "cfgpaths", "Configuration Paths"),
    (1, "Auto Session", "autosession", "Auto Session"),
    (1, "Credentials", "credentials", "Credentials"),
    (0, "Terminal", "terminal", "Terminal"),
    (1, "Appearance", "appearance", "Global Appearance"),
    (2, "ANSI Color", "ansi", "ANSI Color"),
    (2, "Status Bar", "statusbar", "Status Bar"),
    (2, "Advanced", "appadvanced", "Advanced"),
    (1, "Tabs/Tiling", "tabs", "Tabs/Tiling"),
    (1, "Printing", "printing", "Global Printing"),
    (1, "Web Browser", "webbrowser", "Web Browser Setup"),
    (1, "TFTP Server", "tftp", "TFTP Server"),
    (1, "Advanced", "termadvanced", "Advanced"),
    (0, "Firewall", "firewall", "Firewall"),
    (0, "SSH2", "ssh2", "SSH2"),
    (1, "Agent Keys", "agentkeys", "Agent Keys"),
    (0, "SSH Host Keys", "hostkeys", "SSH Host Keys"),
];

const BTN: egui::Vec2 = egui::vec2(72.0, 22.0);

pub fn window(ctx: &egui::Context, st: &mut Settings, gs: &mut GlobalState) -> GlobalOut {
    if gs.orig.is_none() {
        gs.orig = Some((st.clone(), config::ansi_palette()));
    }
    if !TREE.iter().any(|t| t.2 == gs.page) {
        gs.page = "general".into();
    }
    let mut out = GlobalOut::Open;
    let mut open = true;
    let mut ok = false;
    let mut cancel = false;
    // ESC belongs to a sub-window while one is open
    let sub_open = gs.cred_edit.is_some() || gs.palette_new.is_some() || gs.host_view.is_some() || gs.host_delete.is_some() || gs.msg.is_some();
    dialog(ctx, "global_options", "Global Options", 600.0, &mut open, !sub_open, |ui| {
        ui.label("Category:");
        ui.add_space(2.0);
        ui.horizontal_top(|ui| {
            tree(ui, gs);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                let title = TREE.iter().find(|t| t.2 == gs.page).map(|t| t.3).unwrap_or("General");
                let (fill, ink) = if ui.visuals().dark_mode { (Color32::from_rgb(0x38, 0x3b, 0x40), Color32::from_gray(235)) } else { (Color32::from_rgb(0xc9, 0xd5, 0xe2), Color32::from_rgb(0x20, 0x20, 0x20)) };
                egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(5, 3)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(title).strong().color(ink).size(12.0));
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical().id_salt("global_page").max_height(452.0).auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;
                    page(ui, st, gs, &mut out);
                });
            });
        });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("Cancel").min_size(BTN)).clicked() {
                cancel = true;
            }
            if ui.add(egui::Button::new("OK").min_size(BTN).stroke(Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4)))).clicked() {
                ok = true;
            }
        });
    });
    sub_windows(ctx, st, gs);
    if ok || cancel || !open {
        if let Some((orig, pal)) = gs.orig.take() {
            if !ok {
                *st = orig;
                config::set_ansi_palette(pal);
            }
        }
        // the next opening starts on the same page, with fresh lists
        let page = std::mem::take(&mut gs.page);
        let collapsed = std::mem::take(&mut gs.collapsed);
        *gs = GlobalState { page, collapsed, ..Default::default() };
        out = GlobalOut::Close;
    }
    out
}

/// The category tree: indented, with expand/collapse arrows on the nodes that have children.
fn tree(ui: &mut egui::Ui, gs: &mut GlobalState) {
    egui::Frame::new().stroke(Stroke::new(1.0, Color32::from_gray(90))).fill(ui.visuals().extreme_bg_color).inner_margin(4).show(ui, |ui| {
        ui.set_min_size(egui::vec2(142.0, 474.0));
        ui.set_max_width(142.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.vertical(|ui| {
            let mut hide_below: Option<usize> = None;
            for (i, (depth, label, id, _)) in TREE.iter().enumerate() {
                if let Some(d) = hide_below {
                    if *depth > d {
                        continue;
                    }
                    hide_below = None;
                }
                let parent = TREE.get(i + 1).is_some_and(|n| n.0 > *depth);
                let collapsed = gs.collapsed.contains(id);
                if parent && collapsed {
                    hide_below = Some(*depth);
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(*depth as f32 * 12.0);
                    if parent {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(12.0, 16.0), Sense::click());
                        let c = r.center();
                        let col = if resp.hovered() { ui.visuals().strong_text_color() } else { ui.visuals().text_color() };
                        let pts = if collapsed {
                            vec![c + egui::vec2(-2.0, -4.0), c + egui::vec2(2.0, 0.0), c + egui::vec2(-2.0, 4.0)]
                        } else {
                            vec![c + egui::vec2(-4.0, -2.0), c + egui::vec2(0.0, 2.0), c + egui::vec2(4.0, -2.0)]
                        };
                        ui.painter().add(egui::Shape::line(pts, Stroke::new(1.2, col)));
                        if resp.clicked() {
                            if collapsed {
                                gs.collapsed.retain(|x| x != id);
                            } else {
                                gs.collapsed.push(*id);
                            }
                        }
                    } else {
                        ui.add_space(14.0);
                    }
                    if ui.selectable_label(gs.page == *id, RichText::new(*label).size(12.0)).clicked() {
                        gs.page = id.to_string();
                    }
                });
            }
        });
    });
}

// ----- greyed-out controls ("Not available in Yetty") ----------------------------

fn off(ui: &mut egui::Ui, w: impl egui::Widget) -> egui::Response {
    ui.add_enabled(false, w).on_disabled_hover_text(NA)
}

fn off_check(ui: &mut egui::Ui, on: bool, text: &str) {
    let mut v = on;
    off(ui, egui::Checkbox::new(&mut v, text));
}

fn off_radio(ui: &mut egui::Ui, on: bool, text: &str) {
    off(ui, egui::RadioButton::new(on, text));
}

fn off_text(ui: &mut egui::Ui, text: &str, width: f32) {
    let mut t = text;
    off(ui, egui::TextEdit::singleline(&mut t).desired_width(width));
}

fn off_button(ui: &mut egui::Ui, text: &str) {
    off(ui, egui::Button::new(text).min_size(egui::vec2(22.0, 20.0)));
}

fn off_label(ui: &mut egui::Ui, text: &str) {
    off(ui, egui::Label::new(text));
}

fn off_combo(ui: &mut egui::Ui, salt: &str, text: &str, width: f32) {
    ui.add_enabled_ui(false, |ui| {
        egui::ComboBox::from_id_salt(salt).width(width).selected_text(text).show_ui(ui, |_| {}).response.on_disabled_hover_text(NA);
    });
}

/// A read-only text field (the text can be selected and copied).
fn ro_text(ui: &mut egui::Ui, text: &str, width: f32) {
    let mut t = text;
    ui.add(egui::TextEdit::singleline(&mut t).desired_width(width));
}

/// A list with column headers like the original's list views. `size.x` INFINITY = the available width;
/// the last column takes what is left. Returns (clicked row, double-clicked row).
#[allow(clippy::too_many_arguments)]
fn table(ui: &mut egui::Ui, id: &str, cols: &[(&str, f32)], header: bool, rows: &[Vec<String>], sel: Option<usize>, size: egui::Vec2, enabled: bool) -> (Option<usize>, Option<usize>) {
    let mut clicked = None;
    let mut dbl = None;
    let width = size.x.min(ui.available_width());
    let fill = if enabled { ui.visuals().extreme_bg_color } else { ui.visuals().faint_bg_color };
    egui::Frame::new().stroke(Stroke::new(1.0, Color32::from_gray(110))).fill(fill).show(ui, |ui| {
        ui.set_width(width);
        ui.set_height(size.y);
        ui.spacing_mut().item_spacing.y = 0.0;
        let ink = if enabled { ui.visuals().text_color() } else { ui.visuals().weak_text_color() };
        let font = egui::FontId::proportional(12.0);
        let col_w = |i: usize, x0: f32, x: f32| if i + 1 >= cols.len() { (x0 + width - x).max(0.0) } else { cols[i].1 };
        let mut body_h = size.y;
        if header {
            let (hr, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), Sense::hover());
            let p = ui.painter().clone();
            let mut x = hr.min.x;
            for (i, (name, _)) in cols.iter().enumerate() {
                let w = col_w(i, hr.min.x, x);
                p.with_clip_rect(egui::Rect::from_min_max(egui::pos2(x, hr.min.y), egui::pos2(x + w, hr.max.y))).text(egui::pos2(x + 5.0, hr.center().y), egui::Align2::LEFT_CENTER, *name, font.clone(), ink);
                x += w;
                if i + 1 < cols.len() {
                    p.line_segment([egui::pos2(x - 0.5, hr.min.y + 2.0), egui::pos2(x - 0.5, hr.max.y - 2.0)], Stroke::new(1.0, Color32::from_gray(140)));
                }
            }
            p.line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(1.0, Color32::from_gray(140)));
            body_h -= 20.0;
        }
        egui::ScrollArea::vertical().id_salt(id).max_height(body_h).auto_shrink([false, false]).show(ui, |ui| {
            for (ri, row) in rows.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(width, 17.0), if enabled { Sense::click() } else { Sense::hover() });
                let selected = sel == Some(ri);
                let p = ui.painter().clone();
                if selected {
                    p.rect_filled(r, 0.0, ui.visuals().selection.bg_fill);
                } else if enabled && resp.hovered() {
                    p.rect_filled(r, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
                }
                let color = if selected { ui.visuals().selection.stroke.color } else { ink };
                let mut x = r.min.x;
                for (ci, cell) in row.iter().enumerate().take(cols.len().max(1)) {
                    let w = col_w(ci, r.min.x, x);
                    p.with_clip_rect(egui::Rect::from_min_max(egui::pos2(x, r.min.y), egui::pos2(x + w - 2.0, r.max.y))).text(egui::pos2(x + 5.0, r.center().y), egui::Align2::LEFT_CENTER, cell, font.clone(), color);
                    x += w;
                }
                if resp.clicked() {
                    clicked = Some(ri);
                }
                if resp.double_clicked() {
                    dbl = Some(ri);
                }
            }
        });
    });
    (clicked, dbl)
}

/// A small button with a painted triangle: 'u' up, 'd' down, 'r' right, 'l' left.
fn arrow(ui: &mut egui::Ui, dir: char, enabled: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 2.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let col = if enabled { ui.visuals().text_color() } else { ui.visuals().weak_text_color().gamma_multiply(0.6) };
    let c = r.center();
    let v = egui::vec2;
    let pts = match dir {
        'u' => vec![c + v(0.0, -5.0), c + v(6.0, 4.0), c + v(-6.0, 4.0)],
        'd' => vec![c + v(-6.0, -4.0), c + v(6.0, -4.0), c + v(0.0, 5.0)],
        'r' => vec![c + v(-4.0, -6.0), c + v(5.0, 0.0), c + v(-4.0, 6.0)],
        _ => vec![c + v(4.0, -6.0), c + v(4.0, 6.0), c + v(-5.0, 0.0)],
    };
    ui.painter().add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
    enabled && resp.clicked()
}

/// `n` buttons of the standard width, centred in the row.
fn centered_buttons(ui: &mut egui::Ui, n: usize, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let w = n as f32 * BTN.x + n.saturating_sub(1) as f32 * ui.spacing().item_spacing.x;
        ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
        add(ui);
    });
}

fn page(ui: &mut egui::Ui, st: &mut Settings, gs: &mut GlobalState, out: &mut GlobalOut) {
    let page = gs.page.clone();
    match page.as_str() {
        "general" => {
            group(ui, "Dialogs", |ui| {
                egui::Grid::new("global_dialogs").num_columns(2).spacing([24.0, 4.0]).min_col_width(180.0).show(ui, |ui| {
                    off_check(ui, false, "Show connection closed dialog");
                    ui.checkbox(&mut st.global.confirm_disconnect, "Show confirm disconnect dialog");
                    ui.end_row();
                });
            });
            group(ui, "View", |ui| {
                egui::Grid::new("global_view").num_columns(2).spacing([24.0, 4.0]).min_col_width(180.0).show(ui, |ui| {
                    ui.checkbox(&mut st.show_toolbar, "Show toolbar");
                    ui.checkbox(&mut st.show_menu_bar, "Show menu bar");
                    ui.end_row();
                    ui.checkbox(&mut st.show_status_bar, "Show status bar");
                    ui.checkbox(&mut st.show_connect_bar, "Show connect bar");
                    ui.end_row();
                });
            });
            group(ui, "Session management", |ui| {
                let g = &mut st.global;
                off_check(ui, true, "Use dockable Session Manager");
                ui.checkbox(&mut g.show_sm_on_startup, "Show Session Manager or Connect dialog on startup");
                ui.horizontal(|ui| {
                    ui.label("Session list sort order:");
                    egui::ComboBox::from_id_salt("global_sort")
                        .width(120.0)
                        .selected_text(if g.sort_alphabetical { "Alphabetically" } else { "Manually" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut g.sort_alphabetical, true, "Alphabetically");
                            ui.selectable_value(&mut g.sort_alphabetical, false, "Manually");
                        });
                });
                ui.checkbox(&mut g.save_tree_state, "Save expanded folder state for session list");
                off_check(ui, true, "When a session is renamed/moved, update sessions that use it as a firewall");
                ui.checkbox(&mut g.new_session_wizard, "New session uses wizard");
                ui.horizontal(|ui| {
                    ui.label("Recently used session list:");
                    ui.add(egui::DragValue::new(&mut g.recent_list_size).range(1..=16));
                });
                ui.horizontal(|ui| {
                    off_check(ui, true, "Warn when more than:");
                    off_text(ui, "20", 40.0);
                    ui.label("sessions are going to be connected");
                });
            });
        }
        "cfgpaths" => {
            group(ui, "Configuration folder", |ui| {
                ui.label("Configuration data is stored in the location below.");
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 32.0;
                    ro_text(ui, &config::data_dir().display().to_string(), w);
                    off_button(ui, "...");
                });
            });
            group(ui, "Personal data folder", |ui| {
                ui.label("Personal data such as usernames, passwords, and automated logon information can be stored separately from all other configuration data.");
                off_check(ui, false, "Store personal data separately");
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let w = ui.available_width() - 32.0;
                    off_text(ui, "", w);
                    off_button(ui, "...");
                });
            });
        }
        "autosession" => {
            let snaps: Vec<String> = st.snapshots.iter().map(|s| s.0.clone()).collect();
            group(ui, "Auto session setup", |ui| {
                let g = &mut st.global;
                ui.radio_value(&mut g.auto_session, 0, "Do not connect automatically on startup");
                ui.radio_value(&mut g.auto_session, 1, "Connect local shell");
                ui.radio_value(&mut g.auto_session, 2, "Remember and connect to sessions from last use");
                ui.horizontal(|ui| {
                    if ui.radio_value(&mut g.auto_session, 3, "Use snapshot:").clicked() && g.auto_snapshot.is_empty() {
                        if let Some(first) = snaps.first() {
                            g.auto_snapshot = first.clone();
                        }
                    }
                    ui.add_enabled_ui(g.auto_session == 3 && !snaps.is_empty(), |ui| {
                        egui::ComboBox::from_id_salt("global_snapshot").width(250.0).selected_text(g.auto_snapshot.clone()).show_ui(ui, |ui| {
                            for n in &snaps {
                                ui.selectable_value(&mut g.auto_snapshot, n.clone(), n.as_str());
                            }
                        });
                    });
                });
                off_radio(ui, g.auto_session == 4, "Use auto session");
                ui.horizontal_top(|ui| {
                    ui.add_space(22.0);
                    let w = ui.available_width() - 34.0;
                    table(ui, "global_auto_list", &[], false, &[], None, egui::vec2(w, 270.0), false);
                    ui.vertical(|ui| {
                        arrow(ui, 'u', false);
                        arrow(ui, 'd', false);
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(60.0);
                    off(ui, egui::Button::new("Add Session...").min_size(egui::vec2(90.0, 22.0)));
                    off(ui, egui::Button::new("Add Other").min_size(egui::vec2(90.0, 22.0)));
                    off(ui, egui::Button::new("Delete").min_size(BTN));
                });
            });
        }
        "credentials" => {
            group(ui, "Saved credentials", |ui| {
                let mut order: Vec<usize> = (0..st.credentials.len()).collect();
                order.sort_by_key(|&i| st.credentials[i].name.to_lowercase());
                let rows: Vec<Vec<String>> = order
                    .iter()
                    .map(|&i| {
                        let c = &st.credentials[i];
                        vec![c.name.clone(), c.user.clone(), if c.pass.is_empty() { String::new() } else { "Password".into() }]
                    })
                    .collect();
                if gs.cred_sel.is_some_and(|i| i >= st.credentials.len()) {
                    gs.cred_sel = None;
                }
                let sel_row = gs.cred_sel.and_then(|i| order.iter().position(|&o| o == i));
                let (clicked, dbl) = table(ui, "global_creds", &[("Title", 112.0), ("Username", 95.0), ("Authentication Methods", 0.0)], true, &rows, sel_row, egui::vec2(f32::INFINITY, 380.0), true);
                if let Some(r) = clicked {
                    gs.cred_sel = Some(order[r]);
                }
                let mut edit = dbl.map(|r| order[r]);
                ui.add_space(4.0);
                centered_buttons(ui, 3, |ui| {
                    if ui.add(egui::Button::new("Add...").min_size(BTN)).clicked() {
                        gs.cred_edit = Some(CredEdit { index: None, name: String::new(), user: String::new(), pass: String::new(), had_pass: false, err: String::new() });
                    }
                    if ui.add_enabled(gs.cred_sel.is_some(), egui::Button::new("Edit...").min_size(BTN)).clicked() {
                        edit = gs.cred_sel;
                    }
                    if ui.add_enabled(gs.cred_sel.is_some(), egui::Button::new("Delete").min_size(BTN)).clicked() {
                        if let Some(i) = gs.cred_sel.take() {
                            st.credentials.remove(i);
                        }
                    }
                });
                if let Some(c) = edit.and_then(|i| st.credentials.get(i).map(|c| (i, c))) {
                    let (i, c) = c;
                    gs.cred_edit = Some(CredEdit { index: Some(i), name: c.name.clone(), user: c.user.clone(), pass: String::new(), had_pass: !c.pass.is_empty(), err: String::new() });
                }
            });
        }
        "terminal" => {
            group(ui, "Options", |ui| {
                let g = &mut st.global;
                off_check(ui, false, "Save window state for each session");
                ui.checkbox(&mut g.auto_reconnect, "Auto reconnect");
                ui.horizontal(|ui| {
                    ui.label("Reconnect interval");
                    ui.add(egui::DragValue::new(&mut g.reconnect_secs).range(1..=3600));
                    ui.label("seconds");
                });
                off_check(ui, true, "Connect sessions in tabs");
                ui.checkbox(&mut g.exit_on_last_disconnect, "Exit when last session is disconnected");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut st.paste_confirm, "Prompt on multi-line paste for");
                    ui.add_enabled(st.paste_confirm, egui::DragValue::new(&mut g.paste_prompt_lines).range(1..=9999));
                    ui.label("or more newlines");
                });
                ui.horizontal(|ui| {
                    ui.label("Quotation character(s):");
                    ui.add(egui::TextEdit::singleline(&mut g.quote_chars).desired_width(75.0));
                    ui.checkbox(&mut g.quote_between, "Insert pasted text between");
                });
            });
            group(ui, "Mouse", |ui| {
                let g = &mut st.global;
                ui.checkbox(&mut st.copy_on_select, "Copy on select");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut g.paste_on_button, "Paste on");
                    ui.add_enabled_ui(g.paste_on_button, |ui| {
                        egui::ComboBox::from_id_salt("global_paste_button").width(60.0).selected_text(if g.paste_button == 1 { "right" } else { "middle" }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut g.paste_button, 0, "middle");
                            ui.selectable_value(&mut g.paste_button, 1, "right");
                        });
                    });
                    ui.label("button");
                });
                ui.checkbox(&mut g.hide_pointer_on_key, "Hide mouse pointer on keypress");
                ui.checkbox(&mut g.url_ctrl_click, "Open URL with CTRL+click");
                ui.checkbox(&mut g.triple_click_newline, "Triple-click selection includes newline");
            });
        }
        "appearance" => {
            group(ui, "Options", |ui| {
                let g = &mut st.global;
                egui::Grid::new("global_app_opts").num_columns(2).spacing([20.0, 4.0]).min_col_width(190.0).show(ui, |ui| {
                    ui.checkbox(&mut st.show_tabs, "Show session tabs");
                    off_check(ui, false, "Show horizontal scroll bar");
                    ui.end_row();
                    ui.checkbox(&mut st.show_command_window, "Show Command window");
                    off_check(ui, true, "Show vertical scroll bar");
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut st.show_button_bar, "Show button bar on:");
                        ui.add_enabled_ui(st.show_button_bar, |ui| {
                            egui::ComboBox::from_id_salt("global_button_bar").width(60.0).selected_text(if g.button_bar_top { "top" } else { "bottom" }).show_ui(ui, |ui| {
                                ui.selectable_value(&mut g.button_bar_top, false, "bottom");
                                ui.selectable_value(&mut g.button_bar_top, true, "top");
                            });
                        });
                    });
                    ui.end_row();
                });
                off_check(ui, true, "Use ClearType to smooth edges of screen fonts");
                egui::Grid::new("global_app_nums").num_columns(3).spacing([10.0, 4.0]).min_col_width(140.0).show(ui, |ui| {
                    for (label, value, unit) in [
                        ("Window left margin:", "3", "pixels (change applied after restart)"),
                        ("Maximum columns:", "256", ""),
                        ("Max button bar name width:", "125", "pixels"),
                        ("Hex view buffer:", "100", "lines"),
                    ] {
                        ui.label(label);
                        off_text(ui, value, 50.0);
                        ui.label(unit);
                        ui.end_row();
                    }
                });
            });
            group(ui, "Accessibility", |ui| {
                off_check(ui, false, "Use trackable insertion caret");
            });
        }
        "ansi" => ansi_page(ui, st, gs),
        "statusbar" => {
            group(ui, "Fixed columns", |ui| {
                let g = &mut st.global;
                ui.checkbox(&mut g.status_log_name, "Show log filename when logging");
                ui.checkbox(&mut g.status_caps, "Show CAPS lock state");
                ui.checkbox(&mut g.status_num, "Show NUM lock state");
            });
            group(ui, "Customizable columns", |ui| status_columns(ui, &mut st.global, gs));
            group(ui, "Options", |ui| {
                off_check(ui, false, "Show in full-screen mode");
            });
        }
        "appadvanced" => {
            group(ui, "Color schemes", |ui| {
                let mut names: Vec<&str> = config::SCHEMES.iter().map(|s| s.0).collect();
                names.sort_by_key(|n| n.to_lowercase());
                let rows: Vec<Vec<String>> = names.iter().map(|n| vec![n.to_string()]).collect();
                ui.horizontal_top(|ui| {
                    let w = ui.available_width() - 90.0;
                    if let (Some(r), _) = table(ui, "global_schemes", &[], false, &rows, gs.scheme_sel, egui::vec2(w, 196.0), true) {
                        gs.scheme_sel = Some(r);
                    }
                    ui.vertical(|ui| {
                        off(ui, egui::Button::new("Edit...").min_size(BTN));
                        off(ui, egui::Button::new("New...").min_size(BTN));
                        off(ui, egui::Button::new("Delete").min_size(BTN));
                    });
                });
            });
            group(ui, "Window transparency", |ui| {
                for label in ["Active window", "Inactive window"] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(40.0);
                            ui.label("Opaque");
                        });
                    });
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 60.0;
                        let mut v = 255u8;
                        off(ui, egui::Slider::new(&mut v, 0..=255));
                    });
                }
            });
            group(ui, "Application Window", |ui| {
                let g = &mut st.global;
                ui.horizontal(|ui| {
                    ui.checkbox(&mut g.title_bar_on, "Title bar:");
                    ui.add_enabled(g.title_bar_on, egui::TextEdit::singleline(&mut g.title_bar).desired_width(f32::INFINITY));
                });
                off_check(ui, true, "Preserve window size when opening sessions");
            });
        }
        "tabs" => {
            group(ui, "Appearance", |ui| {
                // a picture of the status indicators, as in the original
                egui::Frame::new().fill(Color32::from_gray(0x20)).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        for (mark, color, text) in [
                            ("✔", Color32::from_rgb(0x3c, 0xc8, 0x3c), "Connected"),
                            ("●", Color32::from_rgb(0xe0, 0x30, 0x30), "Disconnected"),
                            ("●", Color32::from_rgb(0x30, 0x50, 0xe8), "New Input"),
                            ("●", Color32::from_rgb(0x40, 0xb0, 0xa0), "Inactivity"),
                        ] {
                            ui.label(RichText::new(mark).color(color));
                            ui.label(RichText::new(text).color(Color32::from_gray(0xe8)));
                            ui.add_space(6.0);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    off_check(ui, true, "Show status indicators as:");
                    off_combo(ui, "global_tab_ind", "Icons", 110.0);
                });
                ui.indent("global_tab_ind_opts", |ui| {
                    off_check(ui, true, "Show new input status");
                    ui.horizontal(|ui| {
                        off_check(ui, false, "Show inactivity status after:");
                        off_text(ui, "5", 40.0);
                        ui.label("seconds");
                    });
                    off_check(ui, false, "Show script running status");
                });
                ui.horizontal(|ui| {
                    ui.label("Tab width:");
                    off_combo(ui, "global_tab_width", "Fit to label", 90.0);
                    off_check(ui, false, "Minimum:");
                    off_text(ui, "40", 40.0);
                    ui.label("pixels");
                });
            });
            group(ui, "Options", |ui| {
                let g = &mut st.global;
                egui::Grid::new("global_tab_opts").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("Double-click action:");
                    off_combo(ui, "global_tab_dbl", "Nothing", 140.0);
                    ui.end_row();
                    ui.label("Tab location:");
                    egui::ComboBox::from_id_salt("global_tab_loc").width(140.0).selected_text(if g.tab_bottom { "Bottom" } else { "Top" }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut g.tab_bottom, false, "Top");
                        ui.selectable_value(&mut g.tab_bottom, true, "Bottom");
                    });
                    ui.end_row();
                });
                off_check(ui, false, "Confirm when closing multiple connected tabbed or tiled sessions");
                ui.checkbox(&mut g.tabs_only_multiple, "Only show tabs when there are more than one");
                off_check(ui, false, "CTRL+TAB switches to most recently used tabbed or tiled session");
                ui.checkbox(&mut g.alt_n_switch, "ALT+N switches to the Nth tabbed or tiled session");
                off_check(ui, true, "Reuse disconnected tabs");
                off_check(ui, true, "Append (n) to session name when connecting to same session");
            });
        }
        "printing" => {
            off_check(ui, false, "Disable all printing");
            group(ui, "Margins", |ui| {
                ui.horizontal_top(|ui| {
                    egui::Grid::new("global_print_margins").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Left:");
                        off_text(ui, "0.500", 45.0);
                        ui.label("Top:");
                        off_text(ui, "0.500", 45.0);
                        ui.end_row();
                        ui.label("Right:");
                        off_text(ui, "0.500", 45.0);
                        ui.label("Bottom:");
                        off_text(ui, "0.500", 45.0);
                        ui.end_row();
                    });
                    ui.add_space(10.0);
                    ui.label("Margin units\nare in inches");
                });
            });
            group(ui, "Printer fonts", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Courier New 10pt");
                    ui.add_space(20.0);
                    off(ui, egui::Button::new("Choose...").min_size(BTN));
                });
            });
            group(ui, "Setup", |ui| {
                off(ui, egui::Button::new("Print Setup...").min_size(egui::vec2(82.0, 22.0)));
            });
            group(ui, "Pass-through printing", |ui| {
                ui.horizontal(|ui| {
                    off_check(ui, false, "Disable pass-through printing");
                    off_check(ui, false, "Buffer pass-through printing");
                });
                off_check(ui, false, "Use raw mode");
                ui.horizontal(|ui| {
                    off_check(ui, false, "Direct pass-through printing to port:");
                    off_combo(ui, "global_print_port", "", 40.0);
                    off(ui, egui::Button::new("Settings...").min_size(BTN));
                });
            });
            group(ui, "Other", |ui| {
                off_check(ui, false, "Force black on white");
                off_check(ui, false, "Enable print screen (PrtScn) key");
            });
        }
        "webbrowser" => {
            group(ui, "Web browser", |ui| {
                let g = &mut st.global;
                ui.checkbox(&mut g.browser_registry, "Use registry setting for web browser");
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.add_enabled(!g.browser_registry, egui::TextEdit::singleline(&mut g.browser_path).desired_width(ui.available_width() - 32.0));
                    if ui.add_enabled(!g.browser_registry, egui::Button::new("...")).clicked() {
                        if let Some(p) = config::browse_file("Select Web Browser", None) {
                            g.browser_path = p;
                        }
                    }
                });
                ui.indent("global_wb_note", |ui| {
                    ui.label("This option designates the Web browser that will be executed when opening a URL in Yetty.  Unselect the above checkbox and enter the path to your Web browser's executable to enable this option.");
                });
            });
            group(ui, "Default URL handler", |ui| {
                off(ui, egui::Button::new("Make Yetty the Default Application").min_size(egui::vec2(250.0, 22.0)));
            });
        }
        "tftp" => {
            // The TFTP server (Transfer > Start TFTP Server) asks for its folder each time and has no saved settings.
            group(ui, "Service", |ui| {
                off_check(ui, false, "Start TFTP server when:");
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    off_combo(ui, "global_tftp_when", "Yetty starts", 180.0);
                });
            });
            group(ui, "Local directories", |ui| {
                egui::Grid::new("global_tftp_dirs").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                    for label in ["Upload to:", "Download from:"] {
                        ui.label(label);
                        off_text(ui, "", 280.0);
                        off_button(ui, "...");
                        ui.end_row();
                    }
                });
            });
            group(ui, "Security", |ui| {
                off_check(ui, false, "Only allow connections from IP addresses of open terminal sessions");
            });
            group(ui, "Options", |ui| {
                egui::Grid::new("global_tftp_opts").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("Listen Port:");
                    off_text(ui, "69", 50.0);
                    ui.label("");
                    ui.end_row();
                    ui.label("Log File:");
                    off_text(ui, "", 300.0);
                    off_button(ui, "...");
                    ui.end_row();
                });
                off_check(ui, false, "Allow overwrite of existing files");
            });
            group(ui, "Log file name substitutions", |ui| {
                egui::Grid::new("global_tftp_subst").num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                    for row in [["%Y - four-digit year", "%D - 2-digit day", "%m - 2-digit minute"], ["%M - 2-digit month", "%h - 2-digit hour", "See Help for more"]] {
                        for cell in row {
                            off_label(ui, cell);
                        }
                        ui.end_row();
                    }
                });
            });
        }
        "termadvanced" => {
            group(ui, "Options", |ui| {
                let g = &mut st.global;
                off_check(ui, false, "Enable execute escape sequence");
                off_check(ui, false, "Disable resize");
                off_check(ui, false, "Connect to multiple sessions sequentially");
                ui.checkbox(&mut g.no_zoom_wheel, "Disable session zooming with mouse scrollwheel");
                ui.checkbox(&mut g.no_zoom_keys, "Disable session zooming with keyboard shortcuts");
            });
            group(ui, "Command window", |ui| {
                let g = &mut st.global;
                ui.checkbox(&mut g.swap_enter, "Swap ENTER and CTRL+ENTER in Command window");
                ui.checkbox(&mut st.cmd_send_immediately, "Send characters immediately");
                egui::Grid::new("global_cmdwin").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("Initially send commands to:");
                    const TARGETS: [(u8, &str); 4] = [(0, "Active Session"), (1, "All Sessions"), (3, "Selected Tab Group(s)"), (2, "Visible Sessions")];
                    let cur = TARGETS.iter().find(|t| t.0 == g.initial_send_target).map(|t| t.1).unwrap_or("Active Session");
                    egui::ComboBox::from_id_salt("global_send_target").width(130.0).selected_text(cur).show_ui(ui, |ui| {
                        for (v, name) in TARGETS {
                            ui.selectable_value(&mut g.initial_send_target, v, name);
                        }
                    });
                    ui.label("");
                    ui.end_row();
                    ui.label("Command window history:");
                    off_text(ui, "200", 50.0);
                    ui.label("commands");
                    ui.end_row();
                });
            });
            group(ui, "Audio bell", |ui| {
                ui.horizontal(|ui| {
                    ui.label("File:");
                    let w = ui.available_width() - 60.0;
                    off_text(ui, "", w);
                    off_button(ui, "...");
                    off_button(ui, "▶");
                });
            });
            group(ui, "Custom menu and toolbar file", |ui| {
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 32.0;
                    off_text(ui, "", w);
                    off_button(ui, "...");
                });
            });
        }
        "firewall" => {
            group(ui, "Firewalls", |ui| {
                table(ui, "global_firewalls", &[("Name", 78.0), ("Type", 160.0), ("Hostname", 105.0), ("Port", 0.0)], true, &[], None, egui::vec2(f32::INFINITY, 385.0), true);
                ui.add_space(4.0);
                centered_buttons(ui, 3, |ui| {
                    off(ui, egui::Button::new("Add...").min_size(BTN));
                    off(ui, egui::Button::new("Edit...").min_size(BTN));
                    off(ui, egui::Button::new("Delete").min_size(BTN));
                });
            });
        }
        "ssh2" => ssh2_page(ui, st, gs, out),
        "agentkeys" => {
            ui.label("Keys to add to agent at startup:");
            ui.horizontal_top(|ui| {
                let w = ui.available_width() - 34.0;
                table(ui, "global_agent_keys", &[], false, &[], None, egui::vec2(w, 392.0), false);
                ui.vertical(|ui| {
                    off(ui, egui::Button::new(RichText::new("+").size(16.0).strong()).min_size(egui::vec2(24.0, 24.0)));
                    off(ui, egui::Button::new("✖").min_size(egui::vec2(24.0, 24.0)));
                });
            });
            if ui.add(egui::Button::new("Manage Agent Keys...").min_size(egui::vec2(128.0, 22.0))).clicked() {
                *out = GlobalOut::Tool("manage_agent_keys");
            }
        }
        "hostkeys" => {
            if gs.host_keys.is_none() {
                gs.host_keys = Some(load_host_keys());
            }
            let keys = gs.host_keys.clone().unwrap_or_default();
            if gs.host_sel.is_some_and(|i| i >= keys.len()) {
                gs.host_sel = None;
            }
            ui.label("Accepted host keys:");
            let rows: Vec<Vec<String>> = keys.iter().map(|k| vec![k.host.clone(), k.port.to_string(), k.kind.clone()]).collect();
            let (clicked, dbl) = table(ui, "global_host_keys", &[("Host", 242.0), ("Port", 80.0), ("Type", 0.0)], true, &rows, gs.host_sel, egui::vec2(f32::INFINITY, 282.0), true);
            if let Some(r) = clicked {
                gs.host_sel = Some(r);
            }
            if let Some(r) = dbl {
                gs.host_view = Some(keys[r].clone());
            }
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                let sel = gs.host_sel.and_then(|i| keys.get(i)).cloned();
                if ui.add_enabled(sel.is_some(), egui::Button::new("View Key...").min_size(BTN)).clicked() {
                    gs.host_view = sel.clone();
                }
                off(ui, egui::Button::new("Import").min_size(BTN));
                off(ui, egui::Button::new("Export Key...").min_size(BTN));
                if ui.add_enabled(sel.is_some(), egui::Button::new("Delete").min_size(BTN)).clicked() {
                    gs.host_delete = sel;
                }
            });
            group(ui, "Host key database location", |ui| {
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 32.0;
                    ro_text(ui, &config::ssh_dir(), w);
                    off_button(ui, "...");
                });
            });
            group(ui, "Digital certificates", |ui| {
                off_check(ui, false, "Check certificate revocation lists");
            });
        }
        _ => {}
    }
}

/// Status Bar > Customizable columns: Selected (5 max) and Available lists with move buttons.
fn status_columns(ui: &mut egui::Ui, g: &mut GlobalOpts, gs: &mut GlobalState) {
    g.status_columns.retain(|c| STATUS_COLUMNS.contains(&c.as_str()));
    let avail: Vec<&str> = STATUS_COLUMNS.iter().copied().filter(|c| !g.status_columns.iter().any(|s| s == c)).collect();
    if let Some((left, i)) = gs.status_sel {
        if (left && i >= g.status_columns.len()) || (!left && i >= avail.len()) {
            gs.status_sel = None;
        }
    }
    let list_w = (ui.available_width() - 40.0) / 2.0;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(format!("Selected ({STATUS_MAX} max)"));
            let rows: Vec<Vec<String>> = g.status_columns.iter().map(|c| vec![c.clone()]).collect();
            let sel = gs.status_sel.filter(|s| s.0).map(|s| s.1);
            if let (Some(r), _) = table(ui, "global_status_sel", &[], false, &rows, sel, egui::vec2(list_w, 128.0), true) {
                gs.status_sel = Some((true, r));
            }
        });
        ui.vertical(|ui| {
            ui.add_space(20.0);
            let left = gs.status_sel.filter(|s| s.0).map(|s| s.1);
            if arrow(ui, 'u', left.is_some_and(|i| i > 0)) {
                if let Some(i) = left {
                    g.status_columns.swap(i, i - 1);
                    gs.status_sel = Some((true, i - 1));
                }
            }
            if arrow(ui, 'd', left.is_some_and(|i| i + 1 < g.status_columns.len())) {
                if let Some(i) = left {
                    g.status_columns.swap(i, i + 1);
                    gs.status_sel = Some((true, i + 1));
                }
            }
            ui.add_space(46.0);
            match gs.status_sel {
                Some((false, j)) => {
                    if arrow(ui, 'l', g.status_columns.len() < STATUS_MAX) {
                        g.status_columns.push(avail[j].to_string());
                        gs.status_sel = Some((true, g.status_columns.len() - 1));
                    }
                }
                Some((true, i)) => {
                    if arrow(ui, 'r', true) {
                        g.status_columns.remove(i);
                        gs.status_sel = None;
                    }
                }
                None => {
                    arrow(ui, 'r', false);
                }
            }
        });
        ui.vertical(|ui| {
            ui.label("Available");
            let rows: Vec<Vec<String>> = avail.iter().map(|c| vec![c.to_string()]).collect();
            let sel = gs.status_sel.filter(|s| !s.0).map(|s| s.1);
            if let (Some(r), _) = table(ui, "global_status_avail", &[], false, &rows, sel, egui::vec2(list_w, 128.0), true) {
                gs.status_sel = Some((false, r));
            }
        });
    });
}

fn ansi_page(ui: &mut egui::Ui, st: &mut Settings, gs: &mut GlobalState) {
    let mut pal = config::ansi_palette();
    let mut changed = false;
    group(ui, "Palette name", |ui| {
        ui.horizontal(|ui| {
            let mut name = st.global.ansi_palette_name.clone();
            egui::ComboBox::from_id_salt("global_palette").width(230.0).selected_text(name.clone()).show_ui(ui, |ui| {
                ui.selectable_value(&mut name, "Standard".to_string(), "Standard");
                for (n, _) in PALETTES {
                    ui.selectable_value(&mut name, n.to_string(), *n);
                }
                for (n, _) in &st.global.custom_palettes {
                    ui.selectable_value(&mut name, n.clone(), n.as_str());
                }
            });
            if name != st.global.ansi_palette_name {
                if let Some(p) = named_palette(st, &name) {
                    pal = p;
                    changed = true;
                }
                st.global.ansi_palette_name = name;
            }
            let custom = !builtin_palette(&st.global.ansi_palette_name);
            if ui.add_enabled(custom, egui::Button::new("Delete").min_size(BTN)).clicked() {
                let n = st.global.ansi_palette_name.clone();
                st.global.custom_palettes.retain(|p| p.0 != n);
                st.global.ansi_palette_name = "Standard".into();
                pal = config::ANSI_PALETTE;
                changed = true;
            }
            if ui.add(egui::Button::new("New...").min_size(BTN)).clicked() {
                gs.palette_new = Some(String::new());
                gs.palette_err.clear();
            }
        });
    });
    for (title, range, salt) in [("Normal colors", 0..8, "global_reset_normal"), ("Bold colors", 8..16, "global_reset_bold")] {
        group(ui, title, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().interact_size = egui::vec2(26.0, 26.0);
                ui.spacing_mut().item_spacing.x = 12.0;
                for c in &mut pal[range.clone()] {
                    changed |= ui.color_edit_button_srgb(c).changed();
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                if ui.push_id(salt, |ui| ui.add(egui::Button::new("Reset").min_size(BTN))).inner.clicked() {
                    if let Some(p) = named_palette(st, &st.global.ansi_palette_name) {
                        pal[range.clone()].copy_from_slice(&p[range.clone()]);
                        changed = true;
                    }
                }
            });
        });
    }
    if changed {
        config::set_ansi_palette(pal);
        st.ansi_palette = pal.to_vec();
    }
}

fn ssh2_page(ui: &mut egui::Ui, st: &mut Settings, gs: &mut GlobalState, out: &mut GlobalOut) {
    let path = st.global_identity.trim().to_string();
    let md5 = gs.fp_md5;
    if gs.fingerprint.as_ref().is_none_or(|(p, m, _)| *p != path || *m != md5) {
        let fp = if !path.is_empty() && std::path::Path::new(&path).is_file() { fingerprint(&path, md5) } else { String::new() };
        gs.fingerprint = Some((path.clone(), md5, fp));
    }
    let private = path.strip_suffix(".pub").unwrap_or(&path).to_string();
    group(ui, "Public key", |ui| {
        let _ = ui.radio(true, "Use identity or certificate file (when blank, keys in the SSH Agent are tried)");
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.add(egui::TextEdit::singleline(&mut st.global_identity).desired_width(ui.available_width() - 32.0));
            if ui.button("...").clicked() {
                if let Some(p) = config::browse_file("Select Identity File", Some(config::ssh_dir().as_str())) {
                    st.global_identity = p;
                }
            }
        });
        off_radio(ui, false, "Use a certificate from your personal CAPI store or a PKCS #11 provider DLL");
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            off_combo(ui, "global_capi", "CAPI", 60.0);
            off_label(ui, "DLL:");
            let w = ui.available_width() - 32.0;
            off_text(ui, "", w);
            off_button(ui, "...");
        });
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            off_label(ui, "Certificate to use:");
            let w = ui.available_width() - 32.0;
            off_text(ui, "<Try all certificates>", w);
            off_button(ui, "...");
        });
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            off_check(ui, false, "Get username from certificate:");
            off_combo(ui, "global_cert_user", "Common Name", 85.0);
            let w = ui.available_width();
            off_text(ui, "", w);
        });
        off_check(ui, true, "Use certificate as raw SSH2 key (server does not support X.509)");
        off_check(ui, false, "Suppress expired certificate warning");
        ui.horizontal(|ui| {
            off_check(ui, false, "OpenSSH certificate:");
            let w = ui.available_width() - 32.0;
            off_text(ui, "", w);
            off_button(ui, "...");
        });
        ui.horizontal(|ui| {
            ui.label("Fingerprint:");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("global_fp_kind").width(90.0).selected_text(if gs.fp_md5 { "MD5" } else { "SHA-2" }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut gs.fp_md5, false, "SHA-2");
                    ui.selectable_value(&mut gs.fp_md5, true, "MD5");
                });
                ui.label("Display as:");
            });
        });
        let fp = gs.fingerprint.as_ref().map(|f| f.2.clone()).unwrap_or_default();
        ui.indent("global_fp", |ui| {
            ui.label(RichText::new(fp).monospace().size(11.0));
        });
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            if ui.add(egui::Button::new("Create Identity File...").min_size(egui::vec2(116.0, 22.0))).clicked() {
                *out = GlobalOut::Tool("create_public_key");
            }
            if ui.add_enabled(!path.is_empty(), egui::Button::new("Export Public Key...").min_size(egui::vec2(116.0, 22.0))).clicked() {
                match public_key_text(&path) {
                    Ok(text) => {
                        if let Some(dest) = config::browse_save("Export Public Key") {
                            gs.msg = Some(match std::fs::write(&dest, format!("{text}\n")) {
                                Ok(()) => format!("The public key was exported to {dest}."),
                                Err(e) => format!("Could not write {dest}: {e}"),
                            });
                        }
                    }
                    Err(e) => gs.msg = Some(e),
                }
            }
            let can_change = !private.is_empty() && std::path::Path::new(&private).is_file();
            if ui.add_enabled(can_change, egui::Button::new("Change Passphrase...").min_size(egui::vec2(116.0, 22.0))).clicked() {
                config::open_console(&format!("ssh-keygen -p -f \"{private}\""));
            }
        });
    });
    group(ui, "Advanced", |ui| {
        egui::Grid::new("global_ssh2_adv").num_columns(2).spacing([40.0, 4.0]).min_col_width(190.0).show(ui, |ui| {
            off_check(ui, true, "Add keys to agent");
            off_check(ui, true, "Enable deprecated GSSAPI");
            ui.end_row();
            off_check(ui, false, "Enable OpenSSH agent forwarding");
            off_check(ui, false, "Cache session password");
            ui.end_row();
        });
    });
}

/// The small windows of the pages: credential editor, new palette name, host key view / delete, messages.
fn sub_windows(ctx: &egui::Context, st: &mut Settings, gs: &mut GlobalState) {
    if let Some(e) = &mut gs.cred_edit {
        let mut open = true;
        let mut ok = false;
        let mut cancel = false;
        let title = if e.index.is_some() { "Edit Credentials" } else { "Add Credentials" };
        dialog(ctx, "global_cred_edit", title, 320.0, &mut open, true, |ui| {
            egui::Grid::new("global_cred_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Title:");
                ui.add(egui::TextEdit::singleline(&mut e.name).desired_width(220.0));
                ui.end_row();
                ui.label("Username:");
                ui.add(egui::TextEdit::singleline(&mut e.user).desired_width(220.0));
                ui.end_row();
                ui.label("Password:");
                let hint = if e.had_pass { "(saved: type to replace)" } else { "" };
                ui.add(egui::TextEdit::singleline(&mut e.pass).password(true).hint_text(hint).desired_width(220.0));
                ui.end_row();
            });
            ui.weak("(Password will be encrypted.)");
            if !e.err.is_empty() {
                ui.colored_label(Color32::from_rgb(0xe0, 0x40, 0x40), e.err.as_str());
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(BTN)).clicked() {
                    cancel = true;
                }
                if ui.add(egui::Button::new("OK").min_size(BTN)).clicked() {
                    ok = true;
                }
            });
        });
        if ok {
            let name = e.name.trim().to_string();
            if name.is_empty() {
                e.err = "Enter a title for the credentials.".into();
            } else if st.credentials.iter().enumerate().any(|(i, c)| Some(i) != e.index && c.name.eq_ignore_ascii_case(&name)) {
                e.err = format!("Credentials named \"{name}\" already exist.");
            } else {
                let pass = if !e.pass.is_empty() {
                    config::protect(&e.pass)
                } else {
                    e.index.and_then(|i| st.credentials.get(i)).map(|c| c.pass.clone()).unwrap_or_default()
                };
                let cred = Credential { name, user: e.user.clone(), pass };
                match e.index {
                    Some(i) if i < st.credentials.len() => st.credentials[i] = cred,
                    _ => {
                        st.credentials.push(cred);
                        gs.cred_sel = Some(st.credentials.len() - 1);
                    }
                }
                gs.cred_edit = None;
            }
        } else if cancel || !open {
            gs.cred_edit = None;
        }
    }

    if let Some(name) = &mut gs.palette_new {
        let mut open = true;
        let mut ok = false;
        let mut cancel = false;
        let err = gs.palette_err.clone();
        dialog(ctx, "global_palette_new", "New Palette", 300.0, &mut open, true, |ui| {
            ui.label("Palette name:");
            ui.add(egui::TextEdit::singleline(&mut *name).desired_width(f32::INFINITY)).request_focus();
            if !err.is_empty() {
                ui.colored_label(Color32::from_rgb(0xe0, 0x40, 0x40), err.as_str());
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(BTN)).clicked() {
                    cancel = true;
                }
                if ui.add(egui::Button::new("OK").min_size(BTN)).clicked() {
                    ok = true;
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
        });
        if ok {
            let n = name.trim().to_string();
            if n.is_empty() {
                gs.palette_err = "Enter a name for the palette.".into();
            } else if builtin_palette(&n) {
                gs.palette_err = format!("\"{n}\" is a built-in palette.");
            } else {
                let colours = config::ansi_palette().to_vec();
                match st.global.custom_palettes.iter_mut().find(|p| p.0 == n) {
                    Some(p) => p.1 = colours,
                    None => st.global.custom_palettes.push((n.clone(), colours)),
                }
                st.global.ansi_palette_name = n;
                gs.palette_new = None;
            }
        } else if cancel || !open {
            gs.palette_new = None;
        }
    }

    if let Some(k) = &gs.host_view {
        let mut open = true;
        let mut close = false;
        dialog(ctx, "global_host_view", "View Host Key", 460.0, &mut open, true, |ui| {
            egui::Grid::new("global_hk_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("Host:");
                ui.label(k.host.as_str());
                ui.end_row();
                ui.label("Port:");
                ui.label(k.port.to_string());
                ui.end_row();
                ui.label("Type:");
                ui.label(k.kind.as_str());
                ui.end_row();
            });
            ui.label("Key:");
            let mut text = k.raw.as_str();
            ui.add(egui::TextEdit::multiline(&mut text).font(egui::TextStyle::Monospace).desired_rows(5).desired_width(f32::INFINITY));
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Close").min_size(BTN)).clicked() {
                    close = true;
                }
            });
        });
        if close || !open {
            gs.host_view = None;
        }
    }

    if let Some(k) = gs.host_delete.clone() {
        if let Some(yes) = crate::dialogs::confirm_window(ctx, "global_host_delete", &format!("Delete the host key of {} (port {})?", k.host, k.port)) {
            gs.host_delete = None;
            if yes {
                if let Err(e) = delete_host_key(&k) {
                    gs.msg = Some(e);
                }
                gs.host_keys = None;
                gs.host_sel = None;
            }
        }
    }

    crate::dialogs::message_window(ctx, "global_msg", &mut gs.msg);
}

// ----- SSH2 identity and known hosts ------------------------------------------------

fn no_window(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The key's fingerprint as the original shows it: hex bytes separated by colons (SHA-256 or MD5).
fn fingerprint(path: &str, md5: bool) -> String {
    let Some(exe) = crate::conn::find_program("ssh-keygen.exe") else { return "ssh-keygen was not found".into() };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["-l", "-E", if md5 { "md5" } else { "sha256" }, "-f", path]).stdin(std::process::Stdio::null());
    no_window(&mut cmd);
    let Ok(o) = cmd.output() else { return "Could not run ssh-keygen".into() };
    if !o.status.success() {
        return "Could not read the key file".into();
    }
    let text = String::from_utf8_lossy(&o.stdout);
    let Some(tok) = text.split_whitespace().nth(1) else { return String::new() };
    let hex: Vec<String> = if let Some(h) = tok.strip_prefix("MD5:") {
        h.split(':').map(str::to_string).collect()
    } else if let Some(bytes) = tok.strip_prefix("SHA256:").and_then(base64_decode) {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    } else {
        return tok.to_string();
    };
    hex.chunks(22).map(|c| c.join(":")).collect::<Vec<_>>().join(":\n")
}

/// The public key line of an identity file: the file itself (.pub), the .pub next to it, or ssh-keygen -y.
fn public_key_text(path: &str) -> Result<String, String> {
    if path.ends_with(".pub") {
        return std::fs::read_to_string(path).map(|t| t.trim().to_string()).map_err(|e| format!("Could not read {path}: {e}"));
    }
    if let Ok(t) = std::fs::read_to_string(format!("{path}.pub")) {
        return Ok(t.trim().to_string());
    }
    let exe = crate::conn::find_program("ssh-keygen.exe").ok_or("ssh-keygen was not found")?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["-y", "-f", path]).stdin(std::process::Stdio::null());
    no_window(&mut cmd);
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()),
        _ => Err("The public key could not be read (a key with a passphrase needs its .pub file next to it).".into()),
    }
}

fn known_hosts_path() -> std::path::PathBuf {
    std::path::PathBuf::from(config::ssh_dir()).join("known_hosts")
}

fn split_host_port(h: &str) -> (String, u16) {
    if let Some(rest) = h.strip_prefix('[') {
        if let Some((name, port)) = rest.split_once("]:") {
            return (name.to_string(), port.parse().unwrap_or(22));
        }
        if let Some(name) = rest.strip_suffix(']') {
            return (name.to_string(), 22);
        }
    }
    (h.to_string(), 22)
}

/// The entries of the OpenSSH known_hosts file, sorted by host.
fn load_host_keys() -> Vec<HostKey> {
    let Ok(text) = std::fs::read_to_string(known_hosts_path()) else { return Vec::new() };
    let mut keys = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let mut parts = t.split_whitespace();
        let mut hosts = parts.next().unwrap_or("");
        if hosts.starts_with('@') {
            // @cert-authority / @revoked
            hosts = parts.next().unwrap_or("");
        }
        let kind = parts.next().unwrap_or("").to_string();
        let (host, port) = if hosts.starts_with("|1|") {
            ("(hashed)".to_string(), 22)
        } else {
            let names: Vec<(String, u16)> = hosts.split(',').map(split_host_port).collect();
            let port = names.first().map(|n| n.1).unwrap_or(22);
            (names.iter().map(|n| n.0.as_str()).collect::<Vec<_>>().join(", "), port)
        };
        keys.push(HostKey { line: i, raw: line.to_string(), host, port, kind });
    }
    keys.sort_by_key(|k| k.host.to_lowercase());
    keys
}

fn delete_host_key(k: &HostKey) -> Result<(), String> {
    let path = known_hosts_path();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.get(k.line) != Some(&k.raw.as_str()) {
        return Err("The known_hosts file has changed; the list was reloaded.".into());
    }
    lines.remove(k.line);
    let mut out = lines.join(nl);
    if !lines.is_empty() {
        out.push_str(nl);
    }
    std::fs::write(&path, out).map_err(|e| format!("Could not write {}: {e}", path.display()))
}
