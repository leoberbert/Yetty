//! New Session Wizard (File > New Session when the wizard is enabled): protocol, host or port, then name.

use crate::config::{FlowControl, Parity, Protocol, SessionConfig, Settings};
use egui::{vec2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind};

const PAGE_HEIGHT: f32 = 230.0;
const FIELD_W: f32 = 270.0;

pub struct WizardState {
    /// 0 = connection type, 1 = host (or serial port), 2 = session name.
    pub page: u8,
    /// The protocol picked on page 1 (SSH2 also when "SFTP session" is checked).
    pub protocol: Protocol,
    pub sftp: bool,
    /// The session being built: host, port, username/credentials, serial settings, name, description.
    pub cfg: SessionConfig,
    /// "Do not use this wizard when creating sessions".
    pub dont_use: bool,
    name_edited: bool,
    focus: bool,
}

pub enum WizardOut {
    Open,
    Cancel,
    Finish {
        cfg: Box<SessionConfig>,
        dont_use_again: bool,
    },
}

impl WizardState {
    pub fn new(settings: &Settings) -> Self {
        Self {
            page: 0,
            protocol: Protocol::Ssh,
            sftp: false,
            cfg: settings.defaults_for(Protocol::Ssh),
            dont_use: false,
            name_edited: false,
            focus: false,
        }
    }

    /// Local Shell has nothing to ask on the second page.
    fn has_details_page(&self) -> bool {
        self.protocol != Protocol::LocalShell
    }

    fn details_ok(&self) -> bool {
        match self.protocol {
            Protocol::Serial => !self.cfg.com_port.trim().is_empty(),
            Protocol::LocalShell => true,
            _ => !self.cfg.host.trim().is_empty() && self.cfg.port != 0,
        }
    }

    fn default_name(&self) -> String {
        match self.protocol {
            Protocol::Serial => format!("Serial-{}", self.cfg.com_port.trim()),
            Protocol::LocalShell => "Local Shell".into(),
            _ => self.cfg.host.trim().to_string(),
        }
    }

    fn go_to_name_page(&mut self) {
        if !self.name_edited {
            self.cfg.name = self.default_name();
        }
        self.page = 2;
        self.focus = true;
    }

    fn set_protocol(&mut self, p: Protocol, settings: &Settings) {
        let keep = (
            self.cfg.host.clone(),
            self.cfg.username.clone(),
            self.cfg.description.clone(),
        );
        self.protocol = p;
        self.cfg = settings.defaults_for(p);
        (self.cfg.host, self.cfg.username, self.cfg.description) = keep;
        if p != Protocol::Ssh {
            self.sftp = false;
        }
    }

    fn finish(&self) -> SessionConfig {
        let mut cfg = self.cfg.clone();
        cfg.protocol = if self.protocol == Protocol::Ssh && self.sftp {
            Protocol::Sftp
        } else {
            self.protocol
        };
        cfg.host = cfg.host.trim().to_string();
        cfg.username = cfg.username.trim().to_string();
        cfg.name = cfg.name.trim().to_string();
        if !cfg.protocol.uses_host() {
            cfg.use_credentials = false;
        }
        cfg
    }
}

enum Act {
    Back,
    Next,
    Finish,
    Cancel,
}

pub fn window(ctx: &egui::Context, st: &mut WizardState, settings: &Settings) -> WizardOut {
    let mut open = true;
    let act = crate::dialogs::dialog(
        ctx,
        "new_session_wizard",
        "New Session Wizard",
        500.0,
        &mut open,
        true,
        |ui| {
            ui.add_space(4.0);
            ui.horizontal_top(|ui| {
                picture(ui);
                ui.add_space(8.0);
                let w = ui.available_width();
                ui.allocate_ui_with_layout(
                    vec2(w, PAGE_HEIGHT),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_size(vec2(w, PAGE_HEIGHT));
                        ui.spacing_mut().item_spacing.y = 6.0;
                        match st.page {
                            0 => page_protocol(ui, st, settings),
                            1 => page_details(ui, st, settings),
                            _ => page_name(ui, st),
                        }
                    },
                );
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            let mut act = None;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_next = st.page == 0 || (st.page == 1 && st.details_ok());
                let can_finish = st.page == 2 && !st.cfg.name.trim().is_empty();
                if slot(ui, true, true, "Cancel") {
                    act = Some(Act::Cancel);
                }
                if slot(ui, st.page != 1, can_finish, "Finish") {
                    act = Some(Act::Finish);
                }
                ui.add_space(6.0);
                if slot(ui, st.page != 2, can_next, "Next >") {
                    act = Some(Act::Next);
                }
                if slot(ui, st.page != 0, true, "< Back") {
                    act = Some(Act::Back);
                }
            });
            act
        },
    )
    .flatten();
    if !open {
        return WizardOut::Cancel;
    }
    match act {
        Some(Act::Cancel) => WizardOut::Cancel,
        Some(Act::Finish) => WizardOut::Finish {
            cfg: Box::new(st.finish()),
            dont_use_again: st.dont_use,
        },
        Some(Act::Next) => {
            if st.page == 0 && st.has_details_page() {
                st.page = 1;
                st.focus = true;
            } else {
                st.go_to_name_page();
            }
            WizardOut::Open
        }
        Some(Act::Back) => {
            st.page = if st.page == 2 && st.has_details_page() {
                1
            } else {
                0
            };
            st.focus = st.page == 1;
            WizardOut::Open
        }
        None => WizardOut::Open,
    }
}

/// A wizard button in its fixed place; a hidden button leaves its place empty, like the original.
fn slot(ui: &mut egui::Ui, shown: bool, enabled: bool, label: &str) -> bool {
    let size = vec2(72.0, 22.0);
    if shown {
        ui.add_enabled(enabled, egui::Button::new(label).min_size(size))
            .clicked()
    } else {
        ui.allocate_exact_size(size, Sense::hover());
        false
    }
}

fn page_protocol(ui: &mut egui::Ui, st: &mut WizardState, settings: &Settings) {
    ui.label("This wizard will help you create a new session for connecting to a remote server.");
    ui.add_space(2.0);
    ui.label("What type of connection do you want to establish?");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Protocol:");
        ui.add_space(8.0);
        let mut pick = st.protocol;
        egui::ComboBox::from_id_salt("wiz_proto")
            .width(110.0)
            .selected_text(pick.label())
            .show_ui(ui, |ui| {
                for p in Protocol::PICK {
                    ui.selectable_value(&mut pick, p, p.label());
                }
            });
        if pick != st.protocol {
            st.set_protocol(pick, settings);
        }
        ui.add_space(6.0);
        if st.protocol == Protocol::Ssh {
            ui.checkbox(&mut st.sftp, "SFTP session");
        }
    });
    let rest = ui.available_height() - 22.0;
    if rest > 0.0 {
        ui.add_space(rest);
    }
    ui.checkbox(
        &mut st.dont_use,
        "Do not use this wizard when creating sessions",
    );
}

fn page_details(ui: &mut egui::Ui, st: &mut WizardState, settings: &Settings) {
    match st.protocol {
        Protocol::Serial => page_serial(ui, st),
        _ => page_host(ui, st, settings),
    }
}

fn page_host(ui: &mut egui::Ui, st: &mut WizardState, settings: &Settings) {
    let p = st.protocol;
    let user_rows = p != Protocol::Raw;
    ui.label("What is the name or IP address of the remote host?");
    if user_rows {
        ui.label("The username can be left blank.");
    }
    ui.add_space(6.0);
    let focus = std::mem::take(&mut st.focus);
    let cfg = &mut st.cfg;
    egui::Grid::new("wiz_host")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.label("Hostname:");
            let r = ui.add(egui::TextEdit::singleline(&mut cfg.host).desired_width(FIELD_W));
            if focus {
                r.request_focus();
            }
            ui.end_row();
            ui.label("Port:");
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535));
            ui.end_row();
            if p != Protocol::Rdp {
                ui.label("Firewall:");
                egui::ComboBox::from_id_salt("wiz_fw")
                    .width(FIELD_W - 8.0)
                    .selected_text(cfg.firewall.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut cfg.firewall, "None".to_string(), "None");
                    });
                ui.end_row();
            }
            if p == Protocol::Rdp {
                ui.label("Username:");
                ui.add(egui::TextEdit::singleline(&mut cfg.username).desired_width(FIELD_W));
                ui.end_row();
            } else if user_rows {
                ui.radio_value(&mut cfg.use_credentials, false, "Username:");
                ui.add_enabled(
                    !cfg.use_credentials,
                    egui::TextEdit::singleline(&mut cfg.username).desired_width(FIELD_W),
                );
                ui.end_row();
                ui.radio_value(&mut cfg.use_credentials, true, "Credentials:");
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(cfg.use_credentials, |ui| {
                        egui::ComboBox::from_id_salt("wiz_cred")
                            .width(FIELD_W - 36.0)
                            .selected_text(cfg.credentials.clone())
                            .show_ui(ui, |ui| {
                                if settings.credentials.is_empty() {
                                    ui.label(
                                        "No saved credentials (Tools > Manage Saved Credentials)",
                                    );
                                }
                                for c in &settings.credentials {
                                    ui.selectable_value(
                                        &mut cfg.credentials,
                                        c.name.clone(),
                                        &c.name,
                                    );
                                }
                            });
                    });
                    ui.add_enabled(false, egui::Button::new("🔑").min_size(vec2(22.0, 20.0)))
                        .on_disabled_hover_text("Not available in Yetty");
                });
                ui.end_row();
            }
        });
}

fn page_serial(ui: &mut egui::Ui, st: &mut WizardState) {
    ui.label("What serial port and settings do you want to use?");
    ui.add_space(6.0);
    st.focus = false;
    let cfg = &mut st.cfg;
    egui::Grid::new("wiz_serial")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.label("Port:");
            egui::ComboBox::from_id_salt("wiz_com")
                .width(110.0)
                .selected_text(cfg.com_port.clone())
                .show_ui(ui, |ui| {
                    let mut ports = crate::conn::serial_ports();
                    if ports.is_empty() {
                        ports = crate::conn::default_serial_ports();
                    }
                    for p in ports {
                        ui.selectable_value(&mut cfg.com_port, p.clone(), p);
                    }
                });
            ui.end_row();
            ui.label("Baud rate:");
            egui::ComboBox::from_id_salt("wiz_baud")
                .width(110.0)
                .selected_text(cfg.baud.to_string())
                .show_ui(ui, |ui| {
                    for b in [
                        110u32, 300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400,
                        460800, 921600,
                    ] {
                        ui.selectable_value(&mut cfg.baud, b, b.to_string());
                    }
                });
            ui.end_row();
            ui.label("Data bits:");
            egui::ComboBox::from_id_salt("wiz_db")
                .width(110.0)
                .selected_text(cfg.data_bits.to_string())
                .show_ui(ui, |ui| {
                    for b in [5u8, 6, 7, 8] {
                        ui.selectable_value(&mut cfg.data_bits, b, b.to_string());
                    }
                });
            ui.end_row();
            ui.label("Parity:");
            let parity = match cfg.parity {
                Parity::None => "None",
                Parity::Odd => "Odd",
                Parity::Even => "Even",
            };
            egui::ComboBox::from_id_salt("wiz_par")
                .width(110.0)
                .selected_text(parity)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.parity, Parity::None, "None");
                    ui.selectable_value(&mut cfg.parity, Parity::Odd, "Odd");
                    ui.selectable_value(&mut cfg.parity, Parity::Even, "Even");
                });
            ui.end_row();
            ui.label("Stop bits:");
            egui::ComboBox::from_id_salt("wiz_sb")
                .width(110.0)
                .selected_text(cfg.stop_bits.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.stop_bits, 1, "1");
                    ui.selectable_value(&mut cfg.stop_bits, 2, "2");
                });
            ui.end_row();
            ui.label("Flow control:");
            let flow = match cfg.flow {
                FlowControl::None => "None",
                FlowControl::XonXoff => "XON/XOFF",
                FlowControl::RtsCts => "RTS/CTS",
            };
            egui::ComboBox::from_id_salt("wiz_flow")
                .width(110.0)
                .selected_text(flow)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.flow, FlowControl::None, "None");
                    ui.selectable_value(&mut cfg.flow, FlowControl::XonXoff, "XON/XOFF");
                    ui.selectable_value(&mut cfg.flow, FlowControl::RtsCts, "RTS/CTS");
                });
            ui.end_row();
        });
}

fn page_name(ui: &mut egui::Ui, st: &mut WizardState) {
    ui.label("The wizard is now ready to create the new session for you.");
    ui.label("What name do you want to use to uniquely identify the new session?");
    ui.add_space(6.0);
    let focus = std::mem::take(&mut st.focus);
    egui::Grid::new("wiz_name")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.label("Session name:");
            let r = ui.add(egui::TextEdit::singleline(&mut st.cfg.name).desired_width(205.0));
            if focus {
                r.request_focus();
            }
            if r.changed() {
                st.name_edited = true;
            }
            ui.end_row();
            ui.label("Description:");
            ui.add(
                egui::TextEdit::multiline(&mut st.cfg.description)
                    .desired_rows(3)
                    .desired_width(205.0),
            );
            ui.end_row();
        });
}

/// The teal picture on the left: two server towers linked by a cable to a desktop PC.
fn picture(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(120.0, PAGE_HEIGHT), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, Color32::from_rgb(0x00, 0x80, 0x80));
    let at = |x: f32, y: f32| rect.min + vec2(x, y);
    let r = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(at(x, y), vec2(w, h));
    let face = Color32::from_gray(0xd6);
    let side = Color32::from_gray(0x9a);
    let edge = Stroke::new(1.0, Color32::from_gray(0x50));
    let led = Color32::from_rgb(0x20, 0xd0, 0x40);

    // cable from the big server down the right side to the PC
    let (a, b, c, d) = (
        at(88.0, 48.0),
        at(124.0, 60.0),
        at(80.0, 150.0),
        at(96.0, 190.0),
    );
    let cable: Vec<Pos2> = (0..=32)
        .map(|i| {
            let t = i as f32 / 32.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            Pos2::new(
                w[0] * a.x + w[1] * b.x + w[2] * c.x + w[3] * d.x,
                w[0] * a.y + w[1] * b.y + w[2] * c.y + w[3] * d.y,
            )
        })
        .collect();
    p.add(egui::Shape::line(
        cable,
        Stroke::new(1.5, Color32::from_rgb(0x20, 0x30, 0xff)),
    ));

    // server towers: front, darker right side, drive bays and a status light
    for (x, y, w, h) in [(28.0, 34.0, 22.0, 42.0), (54.0, 16.0, 30.0, 60.0)] {
        p.rect_filled(r(x + w, y + 3.0, 5.0, h - 3.0), 0.0, side);
        p.rect_filled(r(x, y, w, h), 0.0, face);
        p.rect_stroke(r(x, y, w, h), 0.0, edge, StrokeKind::Inside);
        for k in 0..3 {
            let yy = y + 10.0 + k as f32 * 6.0;
            p.line_segment(
                [at(x + 4.0, yy), at(x + w - 4.0, yy)],
                Stroke::new(1.0, side),
            );
        }
        p.rect_filled(r(x + 4.0, y + 4.0, 3.0, 2.0), 0.0, led);
    }

    // desktop PC: monitor with a blue screen on a flat system unit
    p.rect_filled(r(78.0, 138.0, 5.0, 44.0), 0.0, side);
    p.rect_filled(r(24.0, 134.0, 54.0, 48.0), 0.0, face);
    p.rect_stroke(r(24.0, 134.0, 54.0, 48.0), 0.0, edge, StrokeKind::Inside);
    p.rect_filled(
        r(31.0, 140.0, 40.0, 32.0),
        0.0,
        Color32::from_rgb(0x00, 0x00, 0xf0),
    );
    p.rect_stroke(r(31.0, 140.0, 40.0, 32.0), 0.0, edge, StrokeKind::Outside);
    p.rect_filled(r(40.0, 182.0, 22.0, 5.0), 0.0, side);
    p.rect_filled(r(10.0, 187.0, 90.0, 24.0), 0.0, face);
    p.rect_stroke(r(10.0, 187.0, 90.0, 24.0), 0.0, edge, StrokeKind::Inside);
    p.line_segment([at(62.0, 196.0), at(90.0, 196.0)], Stroke::new(1.0, side));
    p.line_segment([at(62.0, 202.0), at(90.0, 202.0)], Stroke::new(1.0, side));
    p.rect_filled(r(16.0, 203.0, 3.0, 2.0), 0.0, led);
}
