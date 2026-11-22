//! Keymap Editor: an on-screen keyboard; click a key, choose what it does, Map / Unmap, Save / Load.

use crate::config::{self, MappedKey, Settings};
use crate::dialogs::{dialog, group};
use egui::{Color32, RichText};

#[derive(Default)]
pub struct KeymapState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub selected: Option<egui::Key>,
    pub func: String,
    pub arg: String,
    pub message: String,
}

const FUNCS: [&str; 7] = ["Send String", "Send Hex String", "Copy", "Paste", "Clear Screen", "Send Break", "Disconnect"];

const ROWS: &[&[&str]] = &[
    &["Escape", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"],
    &["Backtick", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "Minus", "Equals", "Backspace"],
    &["Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "OpenBracket", "CloseBracket", "Backslash"],
    &["A", "S", "D", "F", "G", "H", "J", "K", "L", "Semicolon", "Quote", "Enter"],
    &["Z", "X", "C", "V", "B", "N", "M", "Comma", "Period", "Slash"],
    &["Space", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"],
];

fn key_by_name(name: &str) -> Option<egui::Key> {
    egui::Key::ALL.iter().copied().find(|k| k.name() == name || format!("{k:?}") == name)
}

fn combo(k: egui::Key, s: &KeymapState) -> String {
    let mut c = String::new();
    if s.ctrl {
        c.push_str("Ctrl+");
    }
    if s.alt {
        c.push_str("Alt+");
    }
    if s.shift {
        c.push_str("Shift+");
    }
    c.push_str(k.name());
    c
}

/// Returns false when the window was closed.
pub fn window(ctx: &egui::Context, st: &mut Settings, s: &mut KeymapState) -> bool {
    let mut open = true;
    let mut close = false;
    if s.func.is_empty() {
        s.func = FUNCS[0].to_string();
    }
    dialog(ctx, "keymap_editor", "Keymap Editor", 640.0, &mut open, true, |ui| {
        ui.label("Click a key on the keyboard, choose what it does and press Map. Keys that already do something are blue.");
        ui.horizontal(|ui| {
            ui.label("With:");
            ui.checkbox(&mut s.ctrl, "Ctrl");
            ui.checkbox(&mut s.alt, "Alt");
            ui.checkbox(&mut s.shift, "Shift");
        });
        egui::Frame::new().stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)).fill(ui.visuals().extreme_bg_color).inner_margin(8).show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            for row in ROWS {
                ui.horizontal(|ui| {
                    for name in *row {
                        let Some(k) = key_by_name(name) else { continue };
                        let combo_name = combo(k, s);
                        let mapped = st.default_session.mapped_keys.iter().any(|m| m.key.trim().eq_ignore_ascii_case(&combo_name));
                        let selected = s.selected == Some(k);
                        let label = match *name {
                            "Escape" => "Esc".to_string(),
                            "Backtick" => "`".to_string(),
                            "Minus" => "-".to_string(),
                            "Equals" => "=".to_string(),
                            "Backspace" => "Bksp".to_string(),
                            "OpenBracket" => "[".to_string(),
                            "CloseBracket" => "]".to_string(),
                            "Backslash" => "\\".to_string(),
                            "Semicolon" => ";".to_string(),
                            "Quote" => "'".to_string(),
                            "Comma" => ",".to_string(),
                            "Period" => ".".to_string(),
                            "Slash" => "/".to_string(),
                            "PageUp" => "PgUp".to_string(),
                            "PageDown" => "PgDn".to_string(),
                            "ArrowUp" => "↑".to_string(),
                            "ArrowDown" => "↓".to_string(),
                            "ArrowLeft" => "←".to_string(),
                            "ArrowRight" => "→".to_string(),
                            "Insert" => "Ins".to_string(),
                            "Delete" => "Del".to_string(),
                            other => other.to_string(),
                        };
                        let w = match *name {
                            "Space" => 120.0,
                            "Backspace" | "Enter" | "Tab" => 52.0,
                            n if n.len() > 2 => 38.0,
                            _ => 30.0,
                        };
                        let mut b = egui::Button::new(RichText::new(label).size(11.0)).min_size(egui::vec2(w, 26.0));
                        if mapped {
                            b = b.fill(Color32::from_rgb(0x00, 0x63, 0xb1));
                        }
                        if selected {
                            b = b.stroke(egui::Stroke::new(2.0, Color32::from_rgb(0xff, 0xc8, 0x3d)));
                        }
                        if ui.add(b).clicked() {
                            s.selected = Some(k);
                            let cn = combo(k, s);
                            if let Some(m) = st.default_session.mapped_keys.iter().find(|m| m.key.trim().eq_ignore_ascii_case(&cn)) {
                                s.func = if m.func.is_empty() { FUNCS[0].to_string() } else { m.func.clone() };
                                s.arg = m.send.clone();
                            } else {
                                s.arg.clear();
                            }
                            s.message.clear();
                        }
                    }
                });
            }
        });
        group(ui, "Selected key", |ui| {
            match s.selected {
                None => {
                    ui.label("No key selected.");
                }
                Some(k) => {
                    let cn = combo(k, s);
                    ui.label(RichText::new(&cn).strong());
                    ui.horizontal(|ui| {
                        ui.label("Function:");
                        egui::ComboBox::from_id_salt("km_func").width(170.0).selected_text(s.func.clone()).show_ui(ui, |ui| {
                            for f in FUNCS {
                                ui.selectable_value(&mut s.func, f.to_string(), f);
                            }
                        });
                    });
                    let needs_arg = matches!(s.func.as_str(), "Send String" | "Send Hex String");
                    ui.add_enabled_ui(needs_arg, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Argument:");
                            ui.add(egui::TextEdit::singleline(&mut s.arg).desired_width(380.0));
                        });
                    });
                    ui.horizontal(|ui| {
                        let exists = st.default_session.mapped_keys.iter().any(|m| m.key.trim().eq_ignore_ascii_case(&cn));
                        if ui.add(egui::Button::new("Map").min_size(egui::vec2(72.0, 22.0))).clicked() {
                            let entry = MappedKey { key: cn.clone(), func: s.func.clone(), send: if needs_arg { s.arg.clone() } else { String::new() } };
                            if let Some(m) = st.default_session.mapped_keys.iter_mut().find(|m| m.key.trim().eq_ignore_ascii_case(&cn)) {
                                *m = entry;
                            } else {
                                st.default_session.mapped_keys.push(entry);
                            }
                            s.message = format!("{cn} mapped.");
                        }
                        if ui.add_enabled(exists, egui::Button::new("Unmap").min_size(egui::vec2(72.0, 22.0))).clicked() {
                            st.default_session.mapped_keys.retain(|m| !m.key.trim().eq_ignore_ascii_case(&cn));
                            s.message = format!("{cn} unmapped.");
                        }
                        if !s.message.is_empty() {
                            ui.weak(&s.message);
                        }
                    });
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new("Save...").min_size(egui::vec2(72.0, 22.0))).clicked() {
                if let Some(p) = config::browse_save("Save keymap") {
                    match serde_json::to_string_pretty(&st.default_session.mapped_keys) {
                        Ok(t) => s.message = match std::fs::write(&p, t) {
                            Ok(_) => "Keymap saved.".into(),
                            Err(e) => format!("Could not save: {e}"),
                        },
                        Err(e) => s.message = format!("Could not save: {e}"),
                    }
                }
            }
            if ui.add(egui::Button::new("Load...").min_size(egui::vec2(72.0, 22.0))).clicked() {
                if let Some(p) = config::browse_file("Load keymap", None) {
                    match std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Vec<MappedKey>>(&t).ok()) {
                        Some(v) => {
                            st.default_session.mapped_keys = v;
                            s.message = "Keymap loaded.".into();
                        }
                        None => s.message = "That file is not a keymap.".into(),
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Close").min_size(egui::vec2(72.0, 22.0))).clicked() {
                    close = true;
                }
            });
        });
    });
    open && !close
}
