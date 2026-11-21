//! Dialogs: Quick Connect and the Session Options category dialog.

use crate::authdlg::auth_props_window;
use crate::config::*;
use egui::{Color32, RichText};

const HEAD_FILL: Color32 = Color32::from_rgb(0xc9, 0xd5, 0xe2);
const FOCUS: Color32 = Color32::from_rgb(0x00, 0x78, 0xd4);

pub fn group<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(2.0);
    ui.label(RichText::new(title).size(12.0));
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(80)))
        .inner_margin(6)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A dialog like the original's: a small left-aligned title with a close button, and a content area of
/// `inner_width` points. Sets `*open` to false when the close button is pressed.
pub fn dialog<R>(ctx: &egui::Context, id: &str, title: &str, inner_width: f32, open: &mut bool, esc: bool, content: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    if esc && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        *open = false; // ESC closes the dialog, like the original
    }
    // a dialog opened from another dialog must come out in front of it, also the second time
    // (not the two main dialogs: egui keeps the relative order of windows that are raised in the same frame,
    // so raising the parent as well would leave the child behind it)
    if id != "session_options" && id != "quick_connect" && id != "global_options" {
        raise(ctx, id);
    }
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(0).corner_radius(2);
    let total = inner_width + 20.0;
    let mut close = false;
    let res = egui::Window::new(id)
        .id(egui::Id::new(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .frame(frame)
        .fixed_size([total, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
            egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 6, top: 5, bottom: 4 }).show(ui, |ui| {
                ui.set_width(total - 16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let r = ui.add(egui::Button::new(RichText::new("✖").size(11.0)).frame(false));
                        if r.hovered() {
                            ui.painter().rect_filled(r.rect, 2.0, Color32::from_rgb(0xc4, 0x2b, 0x1c));
                        }
                        if r.clicked() {
                            close = true;
                        }
                    });
                });
            });
            egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 2, bottom: 10 }).show(ui, |ui| {
                ui.set_width(inner_width);
                content(ui)
            })
            .inner
        });
    if close {
        *open = false;
    }
    res.and_then(|r| r.inner)
}

fn heading(ui: &mut egui::Ui, text: &str) {
    // The title bar of a page follows the theme (light on a light theme, dark on a dark one).
    let (fill, ink) = if ui.visuals().dark_mode { (Color32::from_rgb(0x38, 0x3b, 0x40), Color32::from_gray(235)) } else { (HEAD_FILL, Color32::from_rgb(0x20, 0x20, 0x20)) };
    egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(5, 3)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(text).strong().color(ink).size(12.0));
    });
    ui.add_space(6.0);
}

/// The Authentication list with its up/down arrows and gear.
fn auth_list(ui: &mut egui::Ui, cfg: &mut SessionConfig, sel: &mut usize, gear: &mut Option<String>, props_btn: bool) {
    ui.label(RichText::new("Authentication").size(12.0));
    if *sel >= cfg.auth.len() {
        *sel = 0;
    }
    egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(80))).inner_margin(6).show(ui, |ui| {
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(90)))
                .inner_margin(3)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(172.0, 82.0));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, a) in cfg.auth.iter_mut().enumerate() {
                            let fill = if *sel == i { Color32::from_rgb(0x00, 0x63, 0xb1) } else { Color32::TRANSPARENT };
                            let inner = egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(2, 0)).show(ui, |ui| {
                                ui.set_width(168.0);
                                ui.horizontal(|ui| {
                                    let cb = ui.checkbox(&mut a.enabled, "");
                                    ui.label(RichText::new(&a.name).size(11.5).color(if *sel == i { Color32::WHITE } else { ui.visuals().text_color() }));
                                    cb
                                })
                                .inner
                            });
                            // the row's click area starts after the check box: a widget registered later sits on top,
                            // so a row covering the box would take its clicks and the method could not be turned off
                            let cb = inner.inner;
                            let mut row = inner.response.rect;
                            row.min.x = cb.rect.max.x;
                            let r = ui.interact(row, ui.id().with(("auth_row", i)), egui::Sense::click());
                            if r.clicked() || cb.clicked() {
                                *sel = i;
                            }
                            if r.double_clicked() {
                                *gear = Some(a.name.clone());
                            }
                        }
                    });
                });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                if ui.small_button("⬆").on_hover_text("Move up").clicked() && *sel > 0 {
                    cfg.auth.swap(*sel, *sel - 1);
                    *sel -= 1;
                }
                if ui.small_button("⬇").on_hover_text("Move down").clicked() && *sel + 1 < cfg.auth.len() {
                    cfg.auth.swap(*sel, *sel + 1);
                    *sel += 1;
                }
                if !props_btn {
                    ui.add_space(8.0);
                    if ui.small_button("⚙").on_hover_text("Properties of the selected method").clicked() {
                        *gear = cfg.auth.get(*sel).map(|a| a.name.clone());
                    }
                }
            });
            if props_btn {
                ui.vertical(|ui| {
                    if ui.add(egui::Button::new("Properties...").min_size(egui::vec2(84.0, 22.0))).clicked() {
                        *gear = cfg.auth.get(*sel).map(|a| a.name.clone());
                    }
                });
            }
        });
    });
}

/// An ordered list of algorithms with check boxes and up/down arrows (a short scrolling list, as in the original).
fn alg_editor(ui: &mut egui::Ui, title: &str, list: &mut Vec<AuthMethod>, sel: &mut usize, default: &[&str]) {
    group(ui, title, |ui| {
        if *sel >= list.len() {
            *sel = 0;
        }
        ui.horizontal(|ui| {
            egui::Frame::new().fill(ui.visuals().extreme_bg_color).stroke(egui::Stroke::new(1.0, Color32::from_gray(90))).inner_margin(3).show(ui, |ui| {
                ui.set_min_width(200.0);
                ui.set_max_width(200.0);
                egui::ScrollArea::vertical().id_salt(("alg", title)).max_height(104.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.vertical(|ui| {
                        for i in 0..list.len() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut list[i].enabled, "");
                                if ui.selectable_label(*sel == i, RichText::new(alg_label(&list[i].name)).size(11.5)).clicked() {
                                    *sel = i;
                                }
                            });
                        }
                    });
                });
            });
            ui.vertical(|ui| {
                if ui.small_button("⬆").clicked() && *sel > 0 {
                    list.swap(*sel, *sel - 1);
                    *sel -= 1;
                }
                if ui.small_button("⬇").clicked() && *sel + 1 < list.len() {
                    list.swap(*sel, *sel + 1);
                    *sel += 1;
                }
                if ui.add_sized([48.0, 18.0], egui::Button::new(RichText::new("Reset").size(11.0))).clicked() {
                    *list = alg_list(default);
                }
            });
        });
    });
}
/// Properties of one authentication method (opened with the gear or a double-click).
#[allow(dead_code)]
fn auth_props_window_old(ctx: &egui::Context, cfg: &mut SessionConfig, open: &mut Option<String>, st: &mut Settings) {
    let Some(method) = open.clone() else { return };
    let mut o = true;
    let mut close = false;
    egui::Window::new(format!("{method} Properties"))
        .id(egui::Id::new("auth_props"))
        .open(&mut o)
        .collapsible(false)
        .resizable(false)
        .fixed_size([430.0, 0.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            match method.as_str() {
                "PublicKey" => {
                    group(ui, "Public key", |ui| {
                        ui.radio_value(&mut cfg.pubkey_use_global, true, "Use global public key setting");
                        ui.radio_value(&mut cfg.pubkey_use_global, false, "Use session public key setting");
                    });
                    let global = cfg.pubkey_use_global;
                    group(ui, if global { "Global identity file" } else { "Session identity file" }, |ui| {
                        let field: &mut String = if global { &mut st.global_identity } else { &mut cfg.identity_file };
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(field).desired_width(320.0).hint_text("private key file"));
                            if ui.button("...").on_hover_text("Browse").clicked() {
                                if let Some(p) = browse_file("Select the private key file", Some(&ssh_dir())) {
                                    *field = p;
                                }
                            }
                        });
                        ui.label(RichText::new("Keys found in your .ssh folder:").size(11.5));
                        let keys = detected_keys();
                        if keys.is_empty() {
                            ui.weak("(none found)");
                        }
                        for k in keys {
                            let short = std::path::Path::new(&k).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(k.clone());
                            if ui.selectable_label(*field == k, short).on_hover_text(&k).clicked() {
                                *field = k.clone();
                            }
                        }
                        if ui.button("Clear").clicked() {
                            field.clear();
                        }
                    });
                    // The chosen key is also kept in the session itself, so it is saved with it.
                    if global && !st.global_identity.trim().is_empty() {
                        cfg.identity_file = st.global_identity.clone();
                    }
                    ui.checkbox(&mut cfg.pubkey_agent, "Also try the keys loaded in ssh-agent");
                    if ui.button("Create Identity File...").clicked() {
                        let _ = std::process::Command::new("cmd.exe").args(["/C", "start", "Yetty key generator", "cmd.exe", "/K", "ssh-keygen -t ed25519"]).spawn();
                    }
                }
                "Password" | "Keyboard Interactive" => {
                    group(ui, "Saved password", |ui| {
                        ui.label("Password to send when the host asks for it (optional):");
                        ui.add(egui::TextEdit::singleline(&mut cfg.saved_password).password(true).desired_width(f32::INFINITY));
                        ui.weak("It is stored as plain text in your Yetty settings file. Leave it empty to type the password in the tab.");
                    });
                }
                "GSSAPI" => {
                    group(ui, "GSSAPI", |ui| {
                        ui.checkbox(&mut cfg.gssapi_delegate, "Delegate credentials to the server");
                        ui.weak("Uses the Kerberos ticket of the signed-in Windows user.");
                    });
                }
                _ => {}
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    close = true;
                }
            });
        });
    if close || !o {
        *open = None;
    }
}

/// The connection fields shared by Quick Connect and the Connection page. Returns true when the gear was clicked.
pub fn connection_form(ui: &mut egui::Ui, cfg: &mut SessionConfig, st: &Settings, salt: &str, sel_auth: &mut usize) -> Option<String> {
    let mut gear: Option<String> = None;
    ui.horizontal(|ui| {
        ui.add_sized([62.0, 20.0], egui::Label::new("Protocol:"));
        let shown = if cfg.protocol == Protocol::Sftp { Protocol::Ssh } else { cfg.protocol };
        let mut pick = shown;
        egui::ComboBox::from_id_salt((salt, "proto")).width(112.0).selected_text(pick.label()).show_ui(ui, |ui| {
            for p in Protocol::PICK {
                ui.selectable_value(&mut pick, p, p.label());
            }
        });
        if pick != shown {
            let (n, d, f, h, u) = (cfg.name.clone(), cfg.description.clone(), cfg.folder.clone(), cfg.host.clone(), cfg.username.clone());
            *cfg = st.defaults_for(pick);
            cfg.name = n;
            cfg.description = d;
            cfg.folder = f;
            cfg.host = h;
            cfg.username = u;
        }
        if cfg.protocol.is_ssh() {
            let mut sftp = cfg.protocol == Protocol::Sftp;
            if ui.checkbox(&mut sftp, "SFTP session").changed() {
                cfg.protocol = if sftp { Protocol::Sftp } else { Protocol::Ssh };
            }
        }
    });
    match cfg.protocol {
        Protocol::Serial => {
            egui::Grid::new((salt, "serial")).num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
                ui.label("Port:");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt((salt, "com")).width(90.0).selected_text(cfg.com_port.clone()).show_ui(ui, |ui| {
                        let mut ports = crate::conn::serial_ports();
                        if ports.is_empty() {
                            ports = (1..=8).map(|i| format!("COM{i}")).collect();
                        }
                        for p in ports {
                            ui.selectable_value(&mut cfg.com_port, p.clone(), p);
                        }
                    });
                });
                ui.end_row();
                ui.label("Baud rate:");
                egui::ComboBox::from_id_salt((salt, "baud")).width(90.0).selected_text(cfg.baud.to_string()).show_ui(ui, |ui| {
                    for b in [110u32, 300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
                        ui.selectable_value(&mut cfg.baud, b, b.to_string());
                    }
                });
                ui.end_row();
                ui.label("Data bits:");
                egui::ComboBox::from_id_salt((salt, "db")).width(90.0).selected_text(cfg.data_bits.to_string()).show_ui(ui, |ui| {
                    for b in [5u8, 6, 7, 8] {
                        ui.selectable_value(&mut cfg.data_bits, b, b.to_string());
                    }
                });
                ui.end_row();
                ui.label("Parity:");
                egui::ComboBox::from_id_salt((salt, "par"))
                    .width(90.0)
                    .selected_text(match cfg.parity {
                        Parity::None => "None",
                        Parity::Odd => "Odd",
                        Parity::Even => "Even",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut cfg.parity, Parity::None, "None");
                        ui.selectable_value(&mut cfg.parity, Parity::Odd, "Odd");
                        ui.selectable_value(&mut cfg.parity, Parity::Even, "Even");
                    });
                ui.end_row();
                ui.label("Stop bits:");
                egui::ComboBox::from_id_salt((salt, "sb")).width(90.0).selected_text(cfg.stop_bits.to_string()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.stop_bits, 1, "1");
                    ui.selectable_value(&mut cfg.stop_bits, 2, "2");
                });
                ui.end_row();
                ui.label("Flow control:");
                egui::ComboBox::from_id_salt((salt, "flow"))
                    .width(90.0)
                    .selected_text(match cfg.flow {
                        FlowControl::None => "None",
                        FlowControl::XonXoff => "XON/XOFF",
                        FlowControl::RtsCts => "RTS/CTS",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut cfg.flow, FlowControl::None, "None");
                        ui.selectable_value(&mut cfg.flow, FlowControl::XonXoff, "XON/XOFF");
                        ui.selectable_value(&mut cfg.flow, FlowControl::RtsCts, "RTS/CTS");
                    });
                ui.end_row();
            });
        }
        Protocol::LocalShell => {
            ui.horizontal(|ui| {
                ui.add_sized([62.0, 20.0], egui::Label::new("Command:"));
                ui.add(egui::TextEdit::singleline(&mut cfg.shell_command).desired_width(f32::INFINITY).hint_text("cmd.exe (default)"));
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.add_sized([62.0, 20.0], egui::Label::new("Hostname:"));
                let r = ui.add(egui::TextEdit::singleline(&mut cfg.host).id(egui::Id::new((salt, "host"))).desired_width(f32::INFINITY));
                let _ = r;
            });
            ui.horizontal(|ui| {
                ui.add_sized([62.0, 20.0], egui::Label::new("Port:"));
                ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535));
                ui.add_space(20.0);
                ui.label("Firewall:");
                egui::ComboBox::from_id_salt((salt, "fw")).width(110.0).selected_text(cfg.firewall.clone()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.firewall, "None".to_string(), "None");
                });
            });
            if cfg.protocol != Protocol::Raw {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut cfg.use_credentials, false, "Username:");
                    ui.add_enabled(!cfg.use_credentials, egui::TextEdit::singleline(&mut cfg.username).desired_width(f32::INFINITY));
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut cfg.use_credentials, true, "Credentials:");
                    ui.add_enabled_ui(cfg.use_credentials, |ui| {
                        egui::ComboBox::from_id_salt((salt, "cred")).width(150.0).selected_text(cfg.credentials.clone()).show_ui(ui, |ui| {
                            if st.credentials.is_empty() {
                                ui.label("No saved credentials (Tools > Manage Saved Credentials)");
                            }
                            for c in &st.credentials {
                                ui.selectable_value(&mut cfg.credentials, c.name.clone(), &c.name);
                            }
                        });
                    });
                });
            }
            if cfg.protocol.is_ssh() {
                ui.add_space(4.0);
                // Quick Connect has the gear button (as the original); Session Options has "Properties..."
                auth_list(ui, cfg, sel_auth, &mut gear, salt != "quick");
            }
        }
    }
    gear
}

// ---------------------------------------------------------------------------
// Quick Connect

pub struct QuickDlg {
    pub cfg: SessionConfig,
    pub save: bool,
    pub open_in_tab: bool,
    pub sel_auth: usize,
    pub auth_open: Option<String>,
    pub change_pw: bool,
    pub error: Option<String>,
    first: bool,
}

impl QuickDlg {
    pub fn new(cfg: SessionConfig) -> Self {
        Self { cfg, save: true, open_in_tab: true, sel_auth: 0, auth_open: None, change_pw: false, error: None, first: true }
    }
}

pub enum QuickOut {
    Open,
    Connect,
    Cancel,
}

pub fn quick_window(ctx: &egui::Context, d: &mut QuickDlg, st: &mut Settings) -> QuickOut {
    let mut out = QuickOut::Open;
    let mut open = true;
    // 390 points wide in total, like the original's Quick Connect.
    let esc_ok = d.auth_open.is_none() && d.error.is_none();
    dialog(ctx, "quick_connect", "Quick Connect", 370.0, &mut open, esc_ok, |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            let cfg = &mut d.cfg;
            let snapshot = st.clone();
            if let Some(m) = connection_form(ui, cfg, &snapshot, "quick", &mut d.sel_auth) {
                d.auth_open = Some(m);
            }
            if d.first {
                ui.memory_mut(|m| m.request_focus(egui::Id::new(("quick", "host"))));
                d.first = false;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut st.quick_connect_at_start, "Show quick connect on startup");
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.checkbox(&mut d.save, "Save session");
                    ui.checkbox(&mut d.open_in_tab, "Open in a tab");
                });
            });
            ui.add_space(4.0);
            let valid = !d.cfg.protocol.uses_host() || !d.cfg.host.trim().is_empty();
            ui.horizontal(|ui| {
                ui.add_enabled(false, egui::Button::new("Done").min_size(egui::vec2(72.0, 22.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                        out = QuickOut::Cancel;
                    }
                    let enter = valid && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.add_enabled(valid, egui::Button::new("Connect").min_size(egui::vec2(72.0, 22.0))).clicked() || enter {
                        match validate_session(&d.cfg, None, st) {
                            Ok(()) => out = QuickOut::Connect,
                            Err(e) => d.error = Some(e),
                        }
                    }
                });
            });
        });
    if d.auth_open.is_some() {
        out = QuickOut::Open;
    }
    if auth_props_window(ctx, &mut d.cfg, &mut d.auth_open, st) {
        d.change_pw = true;
    }
    message_window(ctx, "quick_error", &mut d.error);
    if !open {
        out = QuickOut::Cancel;
    }
    out
}

// ---------------------------------------------------------------------------
// Session Options

#[derive(Clone, Copy, PartialEq)]
pub enum EditTarget {
    Saved(usize),
    New,
    Active,
    DefaultSession,
    DefaultSerial,
    DefaultLocal,
}

pub struct OptionsDlg {
    pub target: EditTarget,
    pub cfg: SessionConfig,
    pub page: String,
    pub title: String,
    sel_auth: usize,
    sel_step: Option<usize>,
    step_edit: Option<(Option<usize>, LogonStep)>,
    sel_fwd: Option<usize>,
    fwd_edit: Option<(Option<usize>, Forward)>,
    kw_new: String,
    auth_open: Option<String>,
    pub change_pw: bool,
    pub error: Option<String>,
    map_sel: Option<usize>,
    map_edit: Option<(Option<usize>, MappedKey, bool)>,
    tab_title_on: bool,
    alg_sel: [usize; 4],
    pub show_name: bool,
}

impl OptionsDlg {
    pub fn new(target: EditTarget, cfg: SessionConfig, title: String, page: &str) -> Self {
        let show_name = matches!(target, EditTarget::Saved(_) | EditTarget::New);
        let tab_title_on = !cfg.tab_title.is_empty();
        // "page+key" opens the page with the PublicKey properties on top (used by the screenshot check).
        let step_open = page.ends_with("+step");
        let fwd_open = page.ends_with("+fwd");
        let page = page.strip_suffix("+fwd").unwrap_or(page);
        let page = page.strip_suffix("+step").unwrap_or(page);
        let (page, auth_open) = match page.strip_suffix("+key") {
            Some(p) => (p, Some("PublicKey".to_string())),
            None => (page, None),
        };
        Self {
            target,
            cfg,
            page: page.into(),
            title,
            sel_auth: 0,
            sel_step: None,
            step_edit: if step_open { Some((None, LogonStep { expect: String::new(), send: String::new(), hide: false, trailing_cr: true, credentials: String::new() })) } else { None },
            sel_fwd: None,
            fwd_edit: if fwd_open { Some((None, Forward { name: String::new(), kind: ForwardKind::Local, port: 8080, host: "localhost".into(), dest_port: 80 })) } else { None },
            kw_new: String::new(),
            auth_open,
            change_pw: false,
            error: None,
            map_sel: None,
            map_edit: None,
            tab_title_on,
            alg_sel: [0; 4],
            show_name,
        }
    }
}

pub enum OptOut {
    Open,
    Ok,
    Cancel,
}

fn tree_for(p: Protocol) -> Vec<(u8, &'static str, &'static str)> {
    let mut v = vec![(0, "Connection", "connection"), (1, "Logon Actions", "logon")];
    if p.is_ssh() {
        v.extend([
            (1, "SSH2", "ssh2"),
            (2, "SFTP Session", "sftp"),
            (2, "Advanced", "ssh_adv"),
            (2, "Host Key", "hostkey"),
            (1, "Port Forwarding", "fwd"),
            (2, "Remote/X11", "x11"),
        ]);
    }
    v.extend([
        (0, "Terminal", "terminal"),
        (1, "Emulation", "emulation"),
        (2, "Modes", "modes"),
        (2, "Emacs", "emacs"),
        (2, "Mapped Keys", "mapped"),
        (2, "Advanced", "term_adv"),
        (1, "Appearance", "appearance"),
        (2, "Window", "window"),
        (1, "Keyword Highlighting", "keywords"),
        (2, "Advanced", "kw_adv"),
        (0, "Log File", "log"),
        (0, "Printing", "printing"),
        (0, "X/Y/Zmodem", "zmodem"),
    ]);
    v
}

fn check_tip(ui: &mut egui::Ui, v: &mut bool, text: &str) {
    ui.checkbox(v, text);
}

pub fn options_window(ctx: &egui::Context, d: &mut OptionsDlg, st: &mut Settings) -> OptOut {
    let mut out = OptOut::Open;
    let mut open = true;
    let title = d.title.clone();
    // about 537 x 513 points, like the original's Session Options
    let esc_ok = d.auth_open.is_none() && d.step_edit.is_none() && d.fwd_edit.is_none() && d.map_edit.is_none() && d.error.is_none();
    dialog(ctx, "session_options", &title, 517.0, &mut open, esc_ok, |ui| {
            ui.label("Category:");
            ui.add_space(2.0);
            let tree = tree_for(d.cfg.protocol);
            if !tree.iter().any(|t| t.2 == d.page) {
                d.page = "connection".into();
            }
            ui.horizontal_top(|ui| {
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0, Color32::from_gray(90)))
                    .fill(ui.visuals().extreme_bg_color)
                    .inner_margin(4)
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(142.0, 392.0));
                        ui.set_max_width(142.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.vertical(|ui| {
                        for (depth, label, id) in &tree {
                            ui.horizontal(|ui| {
                                ui.add_space(*depth as f32 * 14.0);
                                let has_child = tree.iter().any(|t| t.0 > *depth) && tree.iter().skip_while(|t| t.2 != *id).nth(1).map(|n| n.0 > *depth).unwrap_or(false);
                                ui.label(RichText::new(if has_child { "v" } else { " " }).size(10.0).weak());
                                let sel = d.page == *id;
                                let r = ui.selectable_label(sel, RichText::new(*label).size(12.0));
                                if r.clicked() {
                                    d.page = id.to_string();
                                }
                            });
                        }
                        });
                    });
                ui.vertical(|ui| {
                    ui.set_width(336.0);
                    let label = tree.iter().find(|t| t.2 == d.page).map(|t| t.1).unwrap_or("Connection");
                    heading(ui, match d.page.as_str() { "terminal" => "Terminal", "sftp" => "SFTP Session Options", "ssh_adv" => "SSH2 Advanced Options", "window" => "Session Window", _ => label });
                    egui::ScrollArea::vertical().id_salt("opt_page").max_height(380.0).auto_shrink([false, false]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        let page = d.page.clone();
                        page_body(ui, &page, d, st);
                    });
                });
            });
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    out = OptOut::Cancel;
                }
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, FOCUS))).clicked() {
                    match validate_session(&d.cfg, Some(d.target), st) {
                        Ok(()) => out = OptOut::Ok,
                        Err(e) => d.error = Some(e),
                    }
                }
            });
        });
    if !open {
        out = OptOut::Cancel;
    }

    if d.auth_open.is_some() {
        out = OptOut::Open;
    }
    if auth_props_window(ctx, &mut d.cfg, &mut d.auth_open, st) {
        d.change_pw = true;
    }
    message_window(ctx, "options_error", &mut d.error);

    // Sub-dialogs (Add/Edit of a logon step and of a forwarded port).
    if let Some(mut se) = d.step_edit.take() {
        let (mut apply, mut cancel, mut o) = (false, false, true);
        let cred_names: Vec<String> = st.credentials.iter().map(|c| c.name.clone()).collect();
        dialog(ctx, "step_edit", "Expect/Send Properties", 400.0, &mut o, true, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(305.0);
                    group(ui, "Expect", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Expect:");
                            ui.add(egui::TextEdit::singleline(&mut se.1.expect).desired_width(f32::INFINITY));
                        });
                    });
                    group(ui, "Send", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Send:");
                            ui.add(egui::TextEdit::singleline(&mut se.1.send).desired_width(f32::INFINITY));
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut se.1.hide, "Hide");
                            ui.add_space(24.0);
                            ui.checkbox(&mut se.1.trailing_cr, "Send trailing carriage return");
                        });
                    });
                    group(ui, "Credentials (optional)", |ui| {
                        let shown = if se.1.credentials.is_empty() { "<not set>".to_string() } else { se.1.credentials.clone() };
                        egui::ComboBox::from_id_salt("step_cred").width(250.0).selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut se.1.credentials, String::new(), "<not set>");
                            for n in &cred_names {
                                ui.selectable_value(&mut se.1.credentials, n.clone(), n);
                            }
                        });
                    });
                    group(ui, "Commands", |ui| {
                        let left = [
                            "\\r - Sends a carriage return (CR)",
                            "\\n - Sends a linefeed (LF)",
                            "\\b - Sends a backspace",
                            "\\v - Pastes Clipboard contents into active session",
                            "\\p - Pauses for one second",
                            "\\s - Sends username from specified credentials",
                            "\\w - Sends password from specified credentials",
                        ];
                        let right = ["\\e - Sends an Escape", "\\t - Sends a tab", "\\\\ - Sends a backslash"];
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for t in left {
                            ui.label(RichText::new(t).size(10.5));
                        }
                        ui.add_space(3.0);
                        for t in right {
                            ui.label(RichText::new(t).size(10.5));
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.add_space(4.0);
                    if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, FOCUS))).clicked() {
                        apply = true;
                    }
                    if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                        cancel = true;
                    }
                });
            });
        });
        if !o {
            cancel = true;
        }
        if apply {
            match se.0 {
                Some(i) if i < d.cfg.logon_steps.len() => d.cfg.logon_steps[i] = se.1,
                _ => d.cfg.logon_steps.push(se.1),
            }
        } else if !cancel && o {
            d.step_edit = Some(se);
        }
    }
    if let Some(mut fe) = d.fwd_edit.take() {
        let (mut apply, mut cancel, mut o) = (false, ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)), true);
        raise(ctx, "fwd_edit");
        dialog(ctx, "fwd_edit", "Port Forward", 330.0, &mut o, true, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut fe.1.kind, ForwardKind::Local, "Local (-L)");
                ui.radio_value(&mut fe.1.kind, ForwardKind::Remote, "Remote (-R)");
                ui.radio_value(&mut fe.1.kind, ForwardKind::Dynamic, "Dynamic (-D)");
            });
            egui::Grid::new("fwd_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut fe.1.name);
                ui.end_row();
                ui.label(if fe.1.kind == ForwardKind::Remote { "Remote port:" } else { "Local port:" });
                ui.add(egui::DragValue::new(&mut fe.1.port).range(1..=65535));
                ui.end_row();
                if fe.1.kind != ForwardKind::Dynamic {
                    ui.label("Destination host:");
                    ui.text_edit_singleline(&mut fe.1.host);
                    ui.end_row();
                    ui.label("Destination port:");
                    ui.add(egui::DragValue::new(&mut fe.1.dest_port).range(1..=65535));
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    apply = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if apply {
            match fe.0 {
                Some(i) if i < d.cfg.forwards.len() => d.cfg.forwards[i] = fe.1,
                _ => d.cfg.forwards.push(fe.1),
            }
        } else if !cancel && o {
            d.fwd_edit = Some(fe);
        }
    }
    if let Some(mut me) = d.map_edit.take() {
        // "Map a Key": press the key, choose what it does
        if me.2 {
            let pressed = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                    _ => None,
                })
            });
            if let Some((key, m)) = pressed {
                let mut name = String::new();
                if m.ctrl {
                    name.push_str("Ctrl+");
                }
                if m.alt {
                    name.push_str("Alt+");
                }
                if m.shift {
                    name.push_str("Shift+");
                }
                name.push_str(key.name());
                me.1.key = name;
                me.2 = false;
            }
            ctx.request_repaint();
        }
        let esc = !me.2 && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let (mut apply, mut cancel, mut o) = (false, esc, true);
        raise(ctx, "map_edit");
        dialog(ctx, "map_edit", "Map a Key", 420.0, &mut o, true, |ui| {
            egui::Grid::new("map_edit_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Key combination:");
                ui.horizontal(|ui| {
                    ui.label(if me.2 { "Press a key...".to_string() } else if me.1.key.is_empty() { "(none)".to_string() } else { me.1.key.clone() });
                    if ui.small_button("Capture").clicked() {
                        me.2 = true;
                    }
                });
                ui.end_row();
                ui.label("Function:");
                egui::ComboBox::from_id_salt("map_func").width(160.0).selected_text(if me.1.func.is_empty() { "Send String".to_string() } else { me.1.func.clone() }).show_ui(ui, |ui| {
                    for f in ["Send String", "Send Hex String", "Copy", "Paste", "Clear Screen", "Send Break", "Disconnect"] {
                        ui.selectable_value(&mut me.1.func, f.to_string(), f);
                    }
                });
                ui.end_row();
                let needs_arg = matches!(me.1.func.as_str(), "" | "Send String" | "Send Hex String");
                ui.label("Argument:");
                ui.add_enabled(needs_arg, egui::TextEdit::singleline(&mut me.1.send).desired_width(220.0).hint_text("\\r = Enter, \\e = Esc"));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!me.1.key.is_empty(), egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    apply = true;
                }
                if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    cancel = true;
                }
            });
        });
        if apply {
            if me.1.func.is_empty() {
                me.1.func = "Send String".into();
            }
            match me.0 {
                Some(i) if i < d.cfg.mapped_keys.len() => d.cfg.mapped_keys[i] = me.1,
                _ => {
                    d.cfg.mapped_keys.push(me.1);
                    d.map_sel = Some(d.cfg.mapped_keys.len() - 1);
                }
            }
        } else if !cancel && o {
            d.map_edit = Some(me);
        }
    }
    out
}

fn page_body(ui: &mut egui::Ui, page: &str, d: &mut OptionsDlg, st: &mut Settings) {
    match page {
        "connection" => {
            if d.show_name {
                egui::Grid::new("names").num_columns(2).spacing([8.0, 5.0]).show(ui, |ui| {
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.name).desired_width(260.0));
                    ui.end_row();
                    ui.label("Folder:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.folder).desired_width(260.0).hint_text("e.g. Servers/Lab"));
                    ui.end_row();
                    ui.label("Description:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.description).desired_width(260.0));
                    ui.end_row();
                });
                ui.separator();
            }
            let snapshot = st.clone();
            if let Some(m) = connection_form(ui, &mut d.cfg, &snapshot, "opt", &mut d.sel_auth) {
                d.auth_open = Some(m);
            }
        }
        "logon" => {
            ui.horizontal(|ui| {
                ui.checkbox(&mut d.cfg.automate_logon, "Automate logon");
                ui.add_space(10.0);
                ui.add_enabled(d.cfg.automate_logon, egui::Checkbox::new(&mut d.cfg.send_initial_cr, "Send initial carriage return"));
            });
            ui.horizontal(|ui| {
                egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(90))).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(290.0, 176.0));
                    ui.set_max_width(290.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::Grid::new("steps").num_columns(4).min_col_width(60.0).spacing([10.0, 3.0]).show(ui, |ui| {
                        for h in ["Expect", "Send", "Hide", "Send trail..."] {
                            ui.label(RichText::new(h).size(11.5));
                        }
                        ui.end_row();
                        for (i, s) in d.cfg.logon_steps.iter().enumerate() {
                            let sel = d.sel_step == Some(i);
                            if ui.selectable_label(sel, if s.expect.is_empty() { " " } else { &s.expect }).clicked() {
                                d.sel_step = Some(i);
                            }
                            ui.label(if s.hide { "•••" } else { s.send.as_str() });
                            ui.label(if s.hide { "✔" } else { "" });
                            ui.label(if s.trailing_cr { "✔" } else { "" });
                            ui.end_row();
                        }
                    });
                    });
                });
                ui.vertical(|ui| {
                    if ui.small_button("⬆").clicked() {
                        if let Some(i) = d.sel_step {
                            if i > 0 {
                                d.cfg.logon_steps.swap(i, i - 1);
                                d.sel_step = Some(i - 1);
                            }
                        }
                    }
                    if ui.small_button("⬇").clicked() {
                        if let Some(i) = d.sel_step {
                            if i + 1 < d.cfg.logon_steps.len() {
                                d.cfg.logon_steps.swap(i, i + 1);
                                d.sel_step = Some(i + 1);
                            }
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                let big = egui::vec2(66.0, 22.0);
                if ui.add(egui::Button::new("Add...").min_size(big)).clicked() {
                    d.step_edit = Some((None, LogonStep { expect: String::new(), send: String::new(), hide: false, trailing_cr: true, credentials: String::new() }));
                }
                let sel = d.sel_step.filter(|i| *i < d.cfg.logon_steps.len());
                if ui.add_enabled(sel.is_some(), egui::Button::new("Edit...").min_size(big)).clicked() {
                    if let Some(i) = sel {
                        d.step_edit = Some((Some(i), d.cfg.logon_steps[i].clone()));
                    }
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new("Delete").min_size(big)).clicked() {
                    if let Some(i) = sel {
                        d.cfg.logon_steps.remove(i);
                        d.sel_step = None;
                    }
                }
            });
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.logon_script, "Logon script:");
                    ui.add_enabled_ui(d.cfg.logon_script, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut d.cfg.logon_script_path).desired_width(190.0));
                        if ui.add(egui::Button::new("...").min_size(egui::vec2(24.0, 20.0))).clicked() {
                            if let Some(p) = browse_file("Select logon script", None) {
                                d.cfg.logon_script_path = p;
                            }
                        }
                    });
                });
                ui.add_enabled_ui(d.cfg.logon_script, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.label("Arguments:");
                        ui.add(egui::TextEdit::singleline(&mut d.cfg.logon_script_args).desired_width(215.0));
                    });
                });
            });
            group(ui, "SSH1/SSH2", |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.remote_command_on, "Remote command:");
                    ui.add_enabled(d.cfg.remote_command_on, egui::TextEdit::singleline(&mut d.cfg.remote_command).desired_width(f32::INFINITY));
                });
                ui.checkbox(&mut d.cfg.display_logon_prompts, "Display logon prompts in terminal window");
            });
        }
        "ssh2" => {
            egui::Grid::new("ssh2_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Hostname:");
                ui.add(egui::TextEdit::singleline(&mut d.cfg.host).desired_width(f32::INFINITY));
                ui.end_row();
                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut d.cfg.port).range(1..=65535));
                ui.end_row();
                ui.label("Firewall:");
                egui::ComboBox::from_id_salt("ssh2_fw").width(220.0).selected_text(d.cfg.firewall.clone()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.cfg.firewall, "None".to_string(), "None");
                });
                ui.end_row();
                ui.label("Username:");
                ui.add(egui::TextEdit::singleline(&mut d.cfg.username).desired_width(f32::INFINITY));
                ui.end_row();
            });
            let mut gear: Option<String> = None;
            auth_list(ui, &mut d.cfg, &mut d.sel_auth, &mut gear, true);
            if gear.is_some() {
                d.auth_open = gear;
            }
            alg_editor(ui, "Key exchange", &mut d.cfg.kex, &mut d.alg_sel[0], KEX_DEFAULT);
        }
        "sftp" => {
            group(ui, "Initial directories", |ui| {
                egui::Grid::new("sftp_grid").num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Local directory:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.sftp_local_dir).desired_width(230.0));
                    if ui.button("...").on_hover_text("Browse").clicked() {
                        if let Some(p) = browse_folder("Choose the local directory") {
                            d.cfg.sftp_local_dir = p;
                        }
                    }
                    ui.end_row();
                    ui.label("Remote directory:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.sftp_start_dir).desired_width(230.0));
                    ui.end_row();
                });
            });
        }
        "ssh_adv" => {
            alg_editor(ui, "Cipher", &mut d.cfg.ciphers, &mut d.alg_sel[1], CIPHERS_DEFAULT);
            alg_editor(ui, "MAC", &mut d.cfg.macs, &mut d.alg_sel[2], MACS_DEFAULT);
            group(ui, "Options", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Compression:");
                    let mut zlib = d.cfg.compression;
                    egui::ComboBox::from_id_salt("compression").width(110.0).selected_text(if zlib { "zlib" } else { "None" }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut zlib, false, "None");
                        ui.selectable_value(&mut zlib, true, "zlib");
                    });
                    d.cfg.compression = zlib;
                    ui.add_space(8.0);
                    ui.label("Level:");
                    ui.add_enabled(zlib, egui::DragValue::new(&mut d.cfg.compression_level).range(1..=9));
                });
                check_tip(ui, &mut d.cfg.agent_forward, "Enable OpenSSH agent forwarding");
                ui.horizontal(|ui| {
                    ui.label("Send keep-alive every");
                    ui.add(egui::DragValue::new(&mut d.cfg.keepalive_secs).range(0..=3600));
                    ui.label("seconds (0 = off)");
                });
            });
        }        "hostkey" => {
            alg_editor(ui, "Host key algorithms", &mut d.cfg.hostkey_algos, &mut d.alg_sel[3], HOSTKEY_DEFAULT);
            group(ui, "Known hosts", |ui| {
                ui.label("Known hosts file (optional):");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.known_hosts_file).desired_width(260.0).hint_text("default: .ssh\\known_hosts"));
                    if ui.button("...").on_hover_text("Browse").clicked() {
                        if let Some(p) = browse_file("Known hosts file", Some(&ssh_dir())) {
                            d.cfg.known_hosts_file = p;
                        }
                    }
                });
            });
            group(ui, "Host key checking", |ui| {
                ui.radio_value(&mut d.cfg.strict_host_key, "ask".to_string(), "Ask before trusting a new host key");
                ui.radio_value(&mut d.cfg.strict_host_key, "accept-new".to_string(), "Accept new host keys automatically");
                ui.radio_value(&mut d.cfg.strict_host_key, "no".to_string(), "Do not verify host keys");
            });
        }
        "fwd" => {
            ui.label("Forwarded ports:");
            egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(90))).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
                ui.set_min_size(egui::vec2(330.0, 160.0));
                egui::Grid::new("fwds").num_columns(3).min_col_width(60.0).spacing([10.0, 3.0]).show(ui, |ui| {
                    for h in ["Name", "Type", "Forward"] {
                        ui.label(RichText::new(h).size(11.5));
                    }
                    ui.end_row();
                    for (i, f) in d.cfg.forwards.iter().enumerate() {
                        let sel = d.sel_fwd == Some(i);
                        if ui.selectable_label(sel, if f.name.is_empty() { "(unnamed)" } else { &f.name }).clicked() {
                            d.sel_fwd = Some(i);
                        }
                        ui.label(match f.kind {
                            ForwardKind::Local => "Local",
                            ForwardKind::Remote => "Remote",
                            ForwardKind::Dynamic => "Dynamic",
                        });
                        ui.label(match f.kind {
                            ForwardKind::Dynamic => format!(":{}", f.port),
                            _ => format!(":{} → {}:{}", f.port, f.host, f.dest_port),
                        });
                        ui.end_row();
                    }
                });
            });
            ui.horizontal(|ui| {
                if ui.button("Add...").clicked() {
                    d.fwd_edit = Some((None, Forward { name: String::new(), kind: ForwardKind::Local, port: 8080, host: "localhost".into(), dest_port: 80 }));
                }
                let sel = d.sel_fwd.filter(|i| *i < d.cfg.forwards.len());
                if ui.add_enabled(sel.is_some(), egui::Button::new("Edit...")).clicked() {
                    if let Some(i) = sel {
                        d.fwd_edit = Some((Some(i), d.cfg.forwards[i].clone()));
                    }
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
                    if let Some(i) = sel {
                        d.cfg.forwards.remove(i);
                        d.sel_fwd = None;
                    }
                }
            });
        }
        "x11" => {
            group(ui, "Remote/X11", |ui| {
                check_tip(ui, &mut d.cfg.x11, "Forward X11 packets");
                ui.add_enabled_ui(d.cfg.x11, |ui| {
                    check_tip(ui, &mut d.cfg.x11_trusted, "Trusted X11 forwarding (-Y)");
                    ui.horizontal(|ui| {
                        ui.label("X display location:");
                        ui.add(egui::TextEdit::singleline(&mut d.cfg.x11_display).desired_width(150.0));
                    });
                });
                check_tip(ui, &mut d.cfg.agent_forward, "Enable OpenSSH agent forwarding");
            });
            ui.weak("X11 forwarding needs an X server running on this computer.");
        }
        "terminal" => {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    check_tip(ui, &mut d.cfg.audible_bell, "Audio bell");
                    check_tip(ui, &mut d.cfg.visual_bell, "Visual bell");
                    check_tip(ui, &mut d.cfg.close_on_disconnect, "Close on disconnect");
                    check_tip(ui, &mut d.cfg.clear_on_disconnect, "Clear on disconnect");
                    check_tip(ui, &mut d.cfg.auto_reconnect, "Auto reconnect");
                });
                ui.vertical(|ui| {
                    check_tip(ui, &mut d.cfg.scroll_bottom_output, "Scroll to bottom on output");
                    check_tip(ui, &mut d.cfg.scroll_bottom_keypress, "Scroll to bottom on keypress");
                });
            });
            group(ui, "Mouse", |ui| {
                check_tip(ui, &mut d.cfg.send_wheel_remote, "Send scroll wheel events to remote");
                check_tip(ui, &mut d.cfg.mouse_report, "Send mouse events to programs that ask for them");
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.add_enabled(d.cfg.mouse_report, egui::Checkbox::new(&mut d.cfg.shift_local_mouse, "Shift forces local mouse"));
                });
                check_tip(ui, &mut d.cfg.cursor_click, "Position cursor on click (command line)");
            });
            group(ui, "Anti-idle", |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.anti_idle_string_on, "Send string:");
                    ui.add_enabled(d.cfg.anti_idle_string_on, egui::TextEdit::singleline(&mut d.cfg.anti_idle_string).desired_width(70.0));
                    ui.label("every");
                    ui.add_enabled(d.cfg.anti_idle_string_on, egui::DragValue::new(&mut d.cfg.anti_idle_secs).range(1..=86400));
                    ui.label("seconds");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.anti_idle_noop_on, "Send protocol NO-OP");
                    ui.label("every");
                    ui.add_enabled(d.cfg.anti_idle_noop_on, egui::DragValue::new(&mut d.cfg.anti_idle_noop_secs).range(1..=86400));
                    ui.label("seconds");
                });
            });
            group(ui, "Selection", |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.word_delims_on, "Set word delimiter characters:");
                    ui.add_enabled(d.cfg.word_delims_on, egui::TextEdit::singleline(&mut d.cfg.word_delims).desired_width(80.0));
                });
            });
        }
        "emulation" => {
            group(ui, "Display and keyboard", |ui| {
                check_tip(ui, &mut d.cfg.jump_scroll, "Jump scroll (fast output is shown in jumps)");
                check_tip(ui, &mut d.cfg.minimize_drawing, "Minimize drawing (redraw at most every 50 ms)");
                ui.horizontal(|ui| {
                    ui.label("Alternate keyboard:");
                    egui::ComboBox::from_id_salt("alt_keyboard").width(150.0).selected_text(d.cfg.alt_keyboard.clone()).show_ui(ui, |ui| {
                        for k in ["None", "Linux", "Xterm R5 / VT220"] {
                            ui.selectable_value(&mut d.cfg.alt_keyboard, k.to_string(), k);
                        }
                    });
                });
            });
            group(ui, "Emulation", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Terminal:");
                    egui::ComboBox::from_id_salt("emu").width(130.0).selected_text(d.cfg.emulation.clone()).show_ui(ui, |ui| {
                        for e in ["Xterm", "VT100", "VT102", "VT220", "ANSI", "Linux", "SCO ANSI", "Wyse50", "VShell"] {
                            ui.selectable_value(&mut d.cfg.emulation, e.to_string(), e);
                        }
                    });
                    check_tip(ui, &mut d.cfg.ansi_color, "ANSI Color");
                });
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    check_tip(ui, &mut d.cfg.xterm256, "Enable xterm-256color");
                    check_tip(ui, &mut d.cfg.use_color_scheme, "Use color scheme");
                });
                check_tip(ui, &mut d.cfg.truecolor, "Enable Xterm-Truecolor");
                ui.horizontal(|ui| {
                    ui.label("Answerback:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.answerback).desired_width(200.0).hint_text("sent when the host asks (ENQ)"));
                });
            });
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    group(ui, "Size", |ui| {
                        egui::Grid::new("emu_size").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                            ui.label("Logical rows:");
                            ui.add(egui::DragValue::new(&mut d.cfg.rows).range(2..=300));
                            ui.end_row();
                            ui.label("Logical columns:");
                            ui.add(egui::DragValue::new(&mut d.cfg.cols).range(10..=1000));
                            ui.end_row();
                        });
                    });
                });
                ui.vertical(|ui| {
                    group(ui, "On resize", |ui| {
                        let mode = if d.cfg.scale_font { 1 } else if d.cfg.resize_retain { 2 } else { 0 };
                        let mut m = mode;
                        ui.radio_value(&mut m, 0, "Synchronize view to size");
                        ui.radio_value(&mut m, 1, "Scale the font");
                        ui.radio_value(&mut m, 2, "Retain size and font");
                        if m != mode {
                            d.cfg.scale_font = m == 1;
                            d.cfg.resize_retain = m == 2;
                        }
                    });
                });
            });
            group(ui, "Scrollback", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Scrollback buffer:");
                    ui.add(egui::DragValue::new(&mut d.cfg.scrollback).range(0..=128000));
                });
            });
        }
        "modes" => {
            group(ui, "Terminal modes", |ui| {
                check_tip(ui, &mut d.cfg.line_wrap, "Auto wrap mode");
                check_tip(ui, &mut d.cfg.origin_mode, "Origin mode");
                check_tip(ui, &mut d.cfg.insert_mode, "Insert mode");
                check_tip(ui, &mut d.cfg.newline_mode, "New line mode (received LF also returns the cursor)");
                check_tip(ui, &mut d.cfg.local_echo, "Local echo");
            });
            group(ui, "Keys", |ui| {
                check_tip(ui, &mut d.cfg.app_cursor_keys, "Application cursor keys");
                check_tip(ui, &mut d.cfg.app_keypad, "Application keypad");
                check_tip(ui, &mut d.cfg.cursor_switching, "Enable cursor key mode switching (the host may change it)");
            });
            ui.weak("Mode changes apply the next time the session connects.");
        }
        "emacs" => {
            group(ui, "Emacs", |ui| {
                check_tip(ui, &mut d.cfg.meta_key, "Use the Alt key as the Meta key");
                ui.add_enabled_ui(d.cfg.meta_key, |ui| {
                    ui.radio_value(&mut d.cfg.meta_8bit, false, "Send ESC before the character (7-bit)");
                    ui.radio_value(&mut d.cfg.meta_8bit, true, "Set the high bit of the character (8-bit)");
                });
            });
        }
        "mapped" => {
            group(ui, "Mapped keys", |ui| {
                egui::Frame::new().stroke(egui::Stroke::new(1.0, Color32::from_gray(90))).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(310.0, 120.0));
                    egui::Grid::new("map_grid").num_columns(3).min_col_width(90.0).spacing([8.0, 3.0]).show(ui, |ui| {
                        for h in ["Key Combination", "Function", "Argument"] {
                            ui.label(RichText::new(h).size(11.5));
                        }
                        ui.end_row();
                        for (i, k) in d.cfg.mapped_keys.iter().enumerate() {
                            if ui.selectable_label(d.map_sel == Some(i), &k.key).clicked() {
                                d.map_sel = Some(i);
                            }
                            ui.label(if k.func.is_empty() { "Send String" } else { k.func.as_str() });
                            ui.label(&k.send);
                            ui.end_row();
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("Map a Key...").clicked() {
                        d.map_edit = Some((None, MappedKey { key: String::new(), func: "Send String".into(), send: String::new() }, true));
                    }
                    let sel = d.map_sel.filter(|i| *i < d.cfg.mapped_keys.len());
                    if ui.add_enabled(sel.is_some(), egui::Button::new("Edit...")).clicked() {
                        if let Some(i) = sel {
                            d.map_edit = Some((Some(i), d.cfg.mapped_keys[i].clone(), false));
                        }
                    }
                    if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
                        if let Some(i) = sel {
                            d.cfg.mapped_keys.remove(i);
                            d.map_sel = None;
                        }
                    }
                });
            });
            group(ui, "Other mappings", |ui| {
                let mut bs_del = !d.cfg.backspace_bs;
                if ui.checkbox(&mut bs_del, "Backspace sends delete").changed() {
                    d.cfg.backspace_bs = !bs_del;
                }
                check_tip(ui, &mut d.cfg.delete_sends_del, "Delete sends backspace");
                check_tip(ui, &mut d.cfg.home_end_tilde, "Home and End send ESC[1~ and ESC[4~");
            });
            group(ui, "CUA settings", |ui| {
                check_tip(ui, &mut d.cfg.cua, "Use Windows copy and paste hotkeys");
            });
        }
        "term_adv" => {
            group(ui, "Keys", |ui| {
                ui.label("Enter key sends:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.cfg.enter_sends, "CR".to_string(), "CR");
                    ui.radio_value(&mut d.cfg.enter_sends, "CRLF".to_string(), "CR + LF");
                    ui.radio_value(&mut d.cfg.enter_sends, "LF".to_string(), "LF");
                });
            });
        }
        "appearance" => {
            group(ui, "Current color scheme", |ui| {
                egui::ComboBox::from_id_salt("scheme").width(220.0).height(320.0).selected_text(d.cfg.color_scheme.clone()).show_ui(ui, |ui| {
                    for (name, fg, bg) in SCHEMES {
                        let txt = RichText::new(format!(" {name} "))
                            .color(Color32::from_rgb(fg[0], fg[1], fg[2]))
                            .background_color(Color32::from_rgb(bg[0], bg[1], bg[2]))
                            .monospace();
                        ui.selectable_value(&mut d.cfg.color_scheme, name.to_string(), txt);
                    }
                });
                let (fg, bg) = scheme(&d.cfg.color_scheme);
                egui::Frame::new().fill(Color32::from_rgb(bg[0], bg[1], bg[2])).inner_margin(8).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("user@host:~$ ls -la").monospace().color(Color32::from_rgb(fg[0], fg[1], fg[2])));
                });
            });
            group(ui, "Font", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Font:");
                    ui.label(RichText::new("Monospace (built-in)").monospace());
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(egui::DragValue::new(&mut d.cfg.font_size).range(8.0..=40.0).speed(0.5));
                });
            });
            group(ui, "Cursor", |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.cfg.cursor_style, CursorStyle::Block, "Block");
                    ui.radio_value(&mut d.cfg.cursor_style, CursorStyle::Underline, "Underline");
                    ui.radio_value(&mut d.cfg.cursor_style, CursorStyle::VerticalBar, "Vertical bar");
                });
                check_tip(ui, &mut d.cfg.blinking_cursor, "Blinking cursor");
            });
            group(ui, "Character encoding", |ui| {
                egui::ComboBox::from_id_salt("enc").width(150.0).selected_text(d.cfg.encoding.clone()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.cfg.encoding, "UTF-8".to_string(), "UTF-8");
                    ui.selectable_value(&mut d.cfg.encoding, "ISO-8859-1".to_string(), "ISO-8859-1");
                });
            });
        }
        "window" => {
            group(ui, "Session window", |ui| {
                check_tip(ui, &mut d.cfg.show_scrollbar, "Show vertical scroll bar");
                check_tip(ui, &mut d.cfg.show_hscrollbar, "Show horizontal scroll bar (when the window is narrower than the terminal)");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.cfg.initial_pos_on, "Initial position:");
                    ui.add_enabled(d.cfg.initial_pos_on, egui::DragValue::new(&mut d.cfg.initial_x).prefix("x "));
                    ui.add_enabled(d.cfg.initial_pos_on, egui::DragValue::new(&mut d.cfg.initial_y).prefix("y "));
                    ui.weak("(the Yetty window moves there when the session opens)");
                });
                ui.horizontal(|ui| {
                    ui.label("Button bar:");
                    let shown = if d.cfg.button_bar.is_empty() { "Default".to_string() } else { d.cfg.button_bar.clone() };
                    egui::ComboBox::from_id_salt("session_button_bar").width(150.0).selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut d.cfg.button_bar, String::new(), "Default");
                        for (n, _) in &st.button_bars {
                            ui.selectable_value(&mut d.cfg.button_bar, n.clone(), n.as_str());
                        }
                    });
                });
                check_tip(ui, &mut d.cfg.resize_retain, "Disable resize");
                check_tip(ui, &mut d.cfg.window_on_top, "Session window always on top");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.tab_title_on, "Title bar:");
                    ui.add_enabled(d.tab_title_on, egui::TextEdit::singleline(&mut d.cfg.tab_title).desired_width(180.0));
                });
                if !d.tab_title_on {
                    d.cfg.tab_title.clear();
                }
                ui.horizontal(|ui| {
                    ui.label("Margin:");
                    ui.add(egui::DragValue::new(&mut d.cfg.margin).range(0.0..=40.0));
                    ui.label("pixels");
                });
                ui.horizontal(|ui| {
                    check_tip(ui, &mut d.cfg.tab_color_on, "Tab color");
                    ui.add_enabled(d.cfg.tab_color_on, |ui: &mut egui::Ui| ui.color_edit_button_srgb(&mut d.cfg.tab_color));
                });
            });
        }
        "keywords" => {
            group(ui, "Keyword highlighting", |ui| {
                check_tip(ui, &mut d.cfg.keyword_highlighting, "Highlight keywords in this session");
                check_tip(ui, &mut st.keyword_highlighting, "Keyword highlighting on (all sessions)");
            });
            group(ui, "Keywords", |ui| {
                let mut remove = None;
                for (i, k) in st.keywords.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.monospace(k);
                        if ui.small_button("✖").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    st.keywords.remove(i);
                }
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut d.kw_new).desired_width(180.0));
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ui.button("Add").clicked() || enter) && !d.kw_new.trim().is_empty() {
                        st.keywords.push(d.kw_new.trim().to_string());
                        d.kw_new.clear();
                    }
                });
            });
        }
        "kw_adv" => {
            group(ui, "Highlighting style", |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.cfg.highlight_style, "Color".to_string(), "Color");
                    ui.radio_value(&mut d.cfg.highlight_style, "Bold".to_string(), "Bold");
                    ui.radio_value(&mut d.cfg.highlight_style, "Reverse".to_string(), "Reverse video");
                });
                check_tip(ui, &mut d.cfg.highlight_case, "Match case");
                check_tip(ui, &mut d.cfg.highlight_line, "Highlight the whole line");
                ui.horizontal(|ui| {
                    ui.label("Highlight color:");
                    ui.color_edit_button_srgb(&mut d.cfg.highlight_color);
                });
            });
        }
        "log" => {
            group(ui, "Log file name", |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.log_file).desired_width(290.0));
                    if ui.button("...").on_hover_text("Browse").clicked() {
                        if let Some(p) = browse_save("Log file") {
                            d.cfg.log_file = p;
                        }
                    }
                });
            });
            group(ui, "Options", |ui| {
                egui::Grid::new("log_opts").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    check_tip(ui, &mut d.cfg.log_prompt, "Prompt for filename");
                    ui.radio_value(&mut d.cfg.log_append, false, "Overwrite file");
                    ui.end_row();
                    check_tip(ui, &mut d.cfg.start_log_on_connect, "Start log upon connect");
                    ui.radio_value(&mut d.cfg.log_append, true, "Append to file");
                    ui.end_row();
                    check_tip(ui, &mut d.cfg.log_raw, "Raw log");
                    ui.label("");
                    ui.end_row();
                });
                check_tip(ui, &mut d.cfg.log_midnight, "Start new log at midnight (must use %D in log file name)");
                check_tip(ui, &mut d.cfg.log_timestamp, "Timestamp each line");
            });
            group(ui, "Custom log data", |ui| {
                egui::Grid::new("log_custom").num_columns(2).spacing([8.0, 5.0]).show(ui, |ui| {
                    ui.label("Upon connect:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.log_custom_connect).desired_width(220.0));
                    ui.end_row();
                    ui.label("Upon disconnect:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.log_custom_disconnect).desired_width(220.0));
                    ui.end_row();
                    ui.label("On each line:");
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.log_custom_line).desired_width(220.0));
                    ui.end_row();
                });
                let any = !d.cfg.log_custom_connect.is_empty() || !d.cfg.log_custom_disconnect.is_empty() || !d.cfg.log_custom_line.is_empty();
                ui.add_enabled(any, egui::Checkbox::new(&mut d.cfg.log_only_custom, "Log only custom data"));
            });
            group(ui, "Log file name and custom log data substitutions", |ui| {
                ui.label(RichText::new("%H - hostname      %M - 2-digit month      %h - 2-digit hour").size(11.5));
                ui.label(RichText::new("%S - session name  %D - 2-digit day        %m - minute · %s - second · %t - millisecond · %Y - year").size(11.5));
            });
        }        "printing" => {
            group(ui, "Printing", |ui| {
                ui.label("Header text:");
                ui.add(egui::TextEdit::singleline(&mut d.cfg.print_header).desired_width(f32::INFINITY));
                ui.label("Footer text:");
                ui.add(egui::TextEdit::singleline(&mut d.cfg.print_footer).desired_width(f32::INFINITY));
                ui.weak("Print sends the screen text to the default printer.");
            });
        }
        _ => {
            group(ui, "Transfer folders", |ui| {
                ui.label("Upload folder:");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.upload_dir).desired_width(280.0));
                    if ui.button("...").on_hover_text("Browse").clicked() {
                        if let Some(p) = browse_folder("Choose the upload folder") {
                            d.cfg.upload_dir = p;
                        }
                    }
                });
                ui.label("Download folder:");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut d.cfg.download_dir).desired_width(280.0));
                    if ui.button("...").on_hover_text("Browse").clicked() {
                        if let Some(p) = browse_folder("Choose the download folder") {
                            d.cfg.download_dir = p;
                        }
                    }
                });
                ui.weak("Used by the Send/Receive commands of the Transfer menu.");
            });
            group(ui, "Xmodem", |ui| {
                check_tip(ui, &mut d.cfg.xmodem_1k, "Send with 1K blocks (Xmodem-1K)");
            });
            group(ui, "Zmodem", |ui| {
                ui.label("Upload command (run on the host before Start Zmodem Upload):");
                ui.add(egui::TextEdit::singleline(&mut d.cfg.zmodem_upload_cmd).desired_width(180.0));
                check_tip(ui, &mut d.cfg.zmodem_auto_download, "Start a download when the host sends a Zmodem request (sz)");
            });
        }
    }
}

// Field checks, like the original's: what is wrong is said in a message box and the dialog stays open.

const BAD_NAME_CHARS: &str = "\\/:*?\"<>|";

fn host_ok(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || ".-_:[]%".contains(c))
}

/// `target` is None for Quick Connect (no session name to check).
pub fn validate_session(cfg: &SessionConfig, target: Option<EditTarget>, st: &Settings) -> Result<(), String> {
    if let Some(t) = target {
        if matches!(t, EditTarget::Saved(_) | EditTarget::New) {
            let name = cfg.name.trim();
            if name.is_empty() {
                return Err("Enter a name for the session.".into());
            }
            if name.chars().any(|c| BAD_NAME_CHARS.contains(c)) {
                return Err(format!("A session name cannot contain any of these characters:\n{}", BAD_NAME_CHARS.replace('\\', "\\ ")));
            }
            for part in cfg.folder.split('/') {
                if part.chars().any(|c| BAD_NAME_CHARS.contains(c)) {
                    return Err("A folder name cannot contain any of these characters:\n\\ : * ? \" < > |".into());
                }
            }
            let own = if let EditTarget::Saved(i) = t { Some(i) } else { None };
            let folder = cfg.folder.trim_matches('/');
            let clash = st.sessions.iter().enumerate().any(|(i, s)| Some(i) != own && s.display_name().eq_ignore_ascii_case(name) && s.folder.trim_matches('/') == folder);
            if clash {
                return Err(format!("A session named \"{name}\" already exists in this folder."));
            }
        }
    }
    match cfg.protocol {
        Protocol::Serial => {
            if cfg.com_port.trim().is_empty() {
                return Err("Select the serial port.".into());
            }
            if cfg.baud == 0 {
                return Err("The baud rate must be greater than zero.".into());
            }
        }
        Protocol::LocalShell => {}
        p => {
            let h = cfg.host.trim();
            if h.is_empty() {
                return Err("Enter the hostname.".into());
            }
            if !host_ok(h) {
                return Err("The hostname contains characters that are not allowed (spaces and symbols).".into());
            }
            if cfg.port == 0 {
                return Err("The port must be between 1 and 65535.".into());
            }
            if cfg.use_credentials && p != Protocol::Raw && cfg.credentials.trim().is_empty() {
                return Err("Choose a saved credential, or select Username.".into());
            }
        }
    }
    if cfg.protocol.is_ssh() && !cfg.identity_file.trim().is_empty() && !std::path::Path::new(cfg.identity_file.trim()).is_file() {
        return Err(format!("The identity file was not found:\n{}", cfg.identity_file.trim()));
    }
    if cfg.protocol.is_ssh() && !cfg.known_hosts_file.trim().is_empty() {
        let p = std::path::Path::new(cfg.known_hosts_file.trim());
        if p.parent().map(|d| !d.as_os_str().is_empty() && !d.is_dir()).unwrap_or(false) {
            return Err("The folder of the known hosts file does not exist.".into());
        }
    }
    if cfg.remote_command_on && cfg.remote_command.trim().is_empty() {
        return Err("Enter the remote command, or clear the Remote command check box.".into());
    }
    if cfg.logon_script {
        let p = cfg.logon_script_path.trim().trim_matches('"');
        if p.is_empty() {
            return Err("Enter the logon script file, or clear the Logon script check box.".into());
        }
        if !std::path::Path::new(p).is_file() {
            return Err(format!("The logon script file was not found:\n{p}"));
        }
    }
    for (i, s) in cfg.logon_steps.iter().enumerate() {
        if cfg.automate_logon && s.expect.is_empty() && s.send.is_empty() {
            return Err(format!("Logon action {} has nothing to expect or send.", i + 1));
        }
    }
    for f in &cfg.forwards {
        if f.port == 0 {
            return Err(format!("The port forward \"{}\" needs a port between 1 and 65535.", f.name));
        }
        if f.kind != ForwardKind::Dynamic && (f.host.trim().is_empty() || f.dest_port == 0) {
            return Err(format!("The port forward \"{}\" needs a destination host and port.", f.name));
        }
    }
    for (i, m) in cfg.mapped_keys.iter().enumerate() {
        if m.key.trim().is_empty() {
            return Err("A mapped key has no key combination.".into());
        }
        if cfg.mapped_keys.iter().skip(i + 1).any(|o| o.key.trim().eq_ignore_ascii_case(m.key.trim())) {
            return Err(format!("The key {} is mapped more than once.", m.key));
        }
    }
    if cfg.anti_idle_string_on && cfg.anti_idle_string.is_empty() {
        return Err("Enter the anti-idle string, or clear the Send string check box.".into());
    }
    if cfg.word_delims_on && cfg.word_delims.is_empty() {
        return Err("Enter the word delimiter characters, or clear the check box.".into());
    }
    if cfg.log_file.chars().any(|c| "<>|\"?*".contains(c)) {
        return Err("The log file name contains characters that are not allowed.".into());
    }
    if cfg.tab_title.chars().count() > 120 {
        return Err("The title is too long.".into());
    }
    Ok(())
}

/// A Yes / No question with a warning sign. `Some(true)` = Yes (Enter), `Some(false)` = No (ESC, X).
pub fn confirm_window(ctx: &egui::Context, id: &str, text: &str) -> Option<bool> {
    let mut open = true;
    let mut answer: Option<bool> = None;
    dialog(ctx, id, "Yetty", 340.0, &mut open, true, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⚠").size(30.0).color(egui::Color32::from_rgb(0xf5, 0xc2, 0x18)));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.set_width(270.0);
                ui.add_space(6.0);
                ui.label(text);
            });
        });
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("No").min_size(egui::vec2(72.0, 22.0))).clicked() {
                answer = Some(false);
            }
            if ui.add(egui::Button::new("Yes").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, FOCUS))).clicked() {
                answer = Some(true);
            }
        });
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            answer = Some(true);
        }
    });
    if !open && answer.is_none() {
        answer = Some(false);
    }
    answer
}

/// A small message box (ESC or OK closes it).
pub fn message_window(ctx: &egui::Context, id: &str, msg: &mut Option<String>) {
    let Some(text) = msg.clone() else { return };
    let mut open = true;
    let mut ok = false;
    dialog(ctx, id, "Yetty", 300.0, &mut open, true, |ui| {
        ui.add_space(4.0);
        ui.label(text);
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("OK").min_size(egui::vec2(72.0, 22.0)).stroke(egui::Stroke::new(1.0, FOCUS))).clicked() {
                ok = true;
            }
        });
    });
    if !open || ok {
        *msg = None;
    }
}
/// Put the window with this id in front of the other windows (egui remembers the old stacking order of
/// a window that was closed, so a sub-window reopened later would come out behind its parent).
pub fn raise(ctx: &egui::Context, id: &str) {
    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new(id)));
}
