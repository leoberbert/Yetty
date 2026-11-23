//! Properties of one authentication method: PublicKey, Password / Keyboard Interactive, GSSAPI.

use crate::config::*;
use crate::dialogs::group;
use egui::{Color32, RichText};

type Backup = (SessionConfig, String);

/// Opened with Properties..., the gear or a double-click. Returns true when "Change Password" was pressed.
pub fn auth_props_window(
    ctx: &egui::Context,
    cfg: &mut SessionConfig,
    open: &mut Option<String>,
    st: &mut Settings,
) -> bool {
    let Some(method) = open.clone() else {
        return false;
    };
    let backup_id = egui::Id::new("auth_backup");
    let pw_id = egui::Id::new("auth_pw_state");
    let adv_id = egui::Id::new("auth_gss_adv");
    if ctx.data(|d| d.get_temp::<Backup>(backup_id)).is_none() {
        ctx.data_mut(|d| {
            d.insert_temp::<Backup>(backup_id, (cfg.clone(), st.global_identity.clone()))
        });
    }
    let mut pwst: (bool, String, String) = ctx.data(|d| d.get_temp(pw_id)).unwrap_or((
        !cfg.saved_password.is_empty(),
        String::new(),
        String::new(),
    ));
    let mut adv: bool = ctx.data(|d| d.get_temp(adv_id)).unwrap_or(false);
    let pp_id = egui::Id::new("auth_pp_state");
    let mut ppst: (bool, String, String) = ctx.data(|d| d.get_temp(pp_id)).unwrap_or((
        !cfg.key_passphrase.is_empty(),
        String::new(),
        String::new(),
    ));
    let is_pw = method == "Password" || method == "Keyboard Interactive";
    let mut ok = false;
    let mut cancel = false;
    let mut change_pw = false;
    let mut o = true;
    // The Password box is small in the original; the key and GSSAPI boxes are wider.
    let width = match method.as_str() {
        "Password" | "Keyboard Interactive" => 225.0,
        "GSSAPI" => 360.0,
        _ => 380.0,
    };
    crate::dialogs::dialog(
        ctx,
        "auth_props",
        &format!("{method} Properties"),
        width,
        &mut o,
        true,
        |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            match method.as_str() {
                "PublicKey" => {
                    group(ui, "Public key", |ui| {
                        ui.radio_value(
                            &mut cfg.pubkey_use_global,
                            true,
                            "Use global public key setting",
                        );
                        ui.radio_value(
                            &mut cfg.pubkey_use_global,
                            false,
                            "Use session public key setting",
                        );
                    });
                    let global = cfg.pubkey_use_global;
                    group(
                        ui,
                        if global {
                            "Global public key setting"
                        } else {
                            "Session public key setting"
                        },
                        |ui| {
                            let field: &mut String = if global {
                                &mut st.global_identity
                            } else {
                                &mut cfg.identity_file
                            };
                            ui.label("Use identity or certificate file:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(field)
                                        .desired_width(300.0)
                                        .hint_text("private key file"),
                                );
                                if ui.button("...").on_hover_text("Browse").clicked() {
                                    if let Some(p) = browse_file(
                                        "Select the identity or certificate file",
                                        Some(&ssh_dir()),
                                    ) {
                                        *field = p;
                                    }
                                }
                            });
                            ui.label(RichText::new("Keys found in your .ssh folder:").size(11.5));
                            // read the .ssh folder once per dialog, not at every frame
                            let keys_id = egui::Id::new("auth_keys_cache");
                            let keys = ui
                                .ctx()
                                .data(|d| d.get_temp::<Vec<String>>(keys_id))
                                .unwrap_or_else(|| {
                                    let k = detected_keys();
                                    ui.ctx().data_mut(|d| d.insert_temp(keys_id, k.clone()));
                                    k
                                });
                            if keys.is_empty() {
                                ui.weak("(none found)");
                            }
                            for k in keys {
                                let short = std::path::Path::new(&k)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or(k.clone());
                                if ui
                                    .selectable_label(*field == k, short)
                                    .on_hover_text(&k)
                                    .clicked()
                                {
                                    *field = k.clone();
                                }
                            }
                            if ui.button("Clear").clicked() {
                                field.clear();
                            }
                            // type and fingerprint of the chosen key
                            let path = field.trim().to_string();
                            if !path.is_empty() {
                                let fp_id = egui::Id::new(("auth_fp", path.clone()));
                                let info = ui
                                    .ctx()
                                    .data(|d| d.get_temp::<String>(fp_id))
                                    .unwrap_or_else(|| {
                                        let t = key_fingerprint(&path);
                                        ui.ctx().data_mut(|d| d.insert_temp(fp_id, t.clone()));
                                        t
                                    });
                                ui.label(RichText::new(info).size(11.0).weak());
                            }
                        },
                    );
                    // The chosen key is also kept in the session itself, so it is saved with it.
                    if global && !st.global_identity.trim().is_empty() {
                        cfg.identity_file = st.global_identity.clone();
                    }
                    group(ui, "Options", |ui| {
                        ui.checkbox(
                            &mut cfg.pubkey_agent,
                            "Also try the keys loaded in ssh-agent",
                        );
                        ui.checkbox(
                            &mut cfg.identities_only,
                            "Use only the identity file above (ignore other keys)",
                        );
                    });
                    group(ui, "Passphrase", |ui| {
                        ui.checkbox(&mut ppst.0, "Store passphrase");
                        ui.add_enabled_ui(ppst.0, |ui| {
                            let hint = if !cfg.key_passphrase.is_empty() {
                                "(saved: type to replace)"
                            } else {
                                ""
                            };
                            ui.add(
                                egui::TextEdit::singleline(&mut ppst.1)
                                    .password(true)
                                    .desired_width(f32::INFINITY)
                                    .hint_text(hint),
                            );
                            ui.label("Confirm passphrase");
                            ui.add(
                                egui::TextEdit::singleline(&mut ppst.2)
                                    .password(true)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.weak("(Passphrase will be encrypted.)");
                        });
                        if ppst.0 && ppst.1 != ppst.2 {
                            ui.colored_label(
                                Color32::from_rgb(0xe0, 0x60, 0x60),
                                "The two passphrases are not the same.",
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create Identity File...").clicked() {
                            crate::config::open_keygen_console();
                        }
                        let key_path = if cfg.pubkey_use_global {
                            st.global_identity.clone()
                        } else {
                            cfg.identity_file.clone()
                        };
                        if ui
                            .add_enabled(
                                !key_path.trim().is_empty(),
                                egui::Button::new("Copy Public Key"),
                            )
                            .on_hover_text("Copies the public key to the clipboard")
                            .clicked()
                        {
                            if let Some(t) = public_key_text(key_path.trim()) {
                                ui.ctx().copy_text(t);
                            }
                        }
                    });
                }
                "Password" | "Keyboard Interactive" => {
                    group(ui, "Password", |ui| {
                        ui.checkbox(&mut pwst.0, "Store password");
                        ui.add_enabled_ui(pwst.0, |ui| {
                            let hint = if !cfg.saved_password.is_empty() {
                                "(saved: type to replace)"
                            } else {
                                ""
                            };
                            ui.add(
                                egui::TextEdit::singleline(&mut pwst.1)
                                    .password(true)
                                    .desired_width(f32::INFINITY)
                                    .hint_text(hint),
                            );
                            ui.label("Confirm password");
                            ui.add(
                                egui::TextEdit::singleline(&mut pwst.2)
                                    .password(true)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.weak("(Password will be encrypted.)");
                        });
                    });
                    if pwst.0 && pwst.1 != pwst.2 {
                        ui.colored_label(
                            Color32::from_rgb(0xe0, 0x60, 0x60),
                            "The two passwords are not the same.",
                        );
                    }
                    if method == "Password" {
                        group(ui, "Change Remote Password", |ui| {
                            if ui
                                .add(
                                    egui::Button::new("Change Password")
                                        .min_size(egui::vec2(120.0, 22.0)),
                                )
                                .on_hover_text("Opens a tab that runs passwd on the server")
                                .clicked()
                            {
                                change_pw = true;
                            }
                        });
                    }
                }
                "GSSAPI" => {
                    egui::Grid::new("gss_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Method:");
                            egui::ComboBox::from_id_salt("gss_method")
                                .width(150.0)
                                .selected_text(cfg.gssapi_method.clone())
                                .show_ui(ui, |ui| {
                                    for m in ["Auto Detect", "SSPI", "Kerberos (GSSAPI)"] {
                                        ui.selectable_value(
                                            &mut cfg.gssapi_method,
                                            m.to_string(),
                                            m,
                                        );
                                    }
                                });
                            ui.end_row();
                            ui.label("Delegation:");
                            egui::ComboBox::from_id_salt("gss_deleg")
                                .width(150.0)
                                .selected_text(cfg.gssapi_delegation.clone())
                                .show_ui(ui, |ui| {
                                    for m in ["Full", "None"] {
                                        ui.selectable_value(
                                            &mut cfg.gssapi_delegation,
                                            m.to_string(),
                                            m,
                                        );
                                    }
                                });
                            ui.end_row();
                        });
                    if adv {
                        group(ui, "Advanced", |ui| {
                            ui.label("Server principal name (optional):");
                            ui.add(
                                egui::TextEdit::singleline(&mut cfg.gssapi_spn)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("host@server.example.com"),
                            );
                        });
                    }
                }
                _ => {}
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Cancel").min_size(egui::vec2(72.0, 22.0)))
                    .clicked()
                {
                    cancel = true;
                }
                if ui
                    .add(
                        egui::Button::new("OK")
                            .min_size(egui::vec2(72.0, 22.0))
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xd4))),
                    )
                    .clicked()
                {
                    ok = true;
                }
                if method == "GSSAPI"
                    && ui
                        .add(
                            egui::Button::new(if adv { "Advanced <<" } else { "Advanced >>" })
                                .min_size(egui::vec2(88.0, 22.0)),
                        )
                        .clicked()
                {
                    adv = !adv;
                }
            });
        },
    );
    if ok && is_pw {
        if !pwst.0 {
            cfg.saved_password.clear();
        } else if !pwst.1.is_empty() {
            if pwst.1 == pwst.2 {
                cfg.saved_password = protect(&pwst.1);
            } else {
                ok = false; // the two passwords differ: keep the dialog open
            }
        }
    }
    if ok && method == "PublicKey" {
        if !ppst.0 {
            cfg.key_passphrase = String::new();
        } else if !ppst.1.is_empty() {
            if ppst.1 == ppst.2 {
                cfg.key_passphrase = protect(&ppst.1);
            } else {
                ok = false; // the two passphrases differ: keep the dialog open
            }
        }
    }
    if cancel || !o {
        if let Some((c, g)) = ctx.data(|d| d.get_temp::<Backup>(backup_id)) {
            *cfg = c;
            st.global_identity = g;
        }
    }
    if ok || cancel || !o || change_pw {
        *open = None;
        ctx.data_mut(|d| {
            d.clear_temp_for::<Backup>(backup_id);
            d.clear_temp_for::<(bool, String, String)>(pw_id);
            d.clear_temp_for::<bool>(adv_id);
            d.clear_temp_for::<Vec<String>>(egui::Id::new("auth_keys_cache"));
            d.clear_temp_for::<(bool, String, String)>(pp_id);
        });
    } else {
        ctx.data_mut(|d| {
            d.insert_temp(pw_id, pwst);
            d.insert_temp(pp_id, ppst);
            d.insert_temp(adv_id, adv);
        });
    }
    change_pw
}

/// Type, size and fingerprint of a private key file (through ssh-keygen -l).
fn key_fingerprint(path: &str) -> String {
    let Some(exe) = crate::conn::find_program("ssh-keygen.exe") else {
        return "ssh-keygen was not found".into();
    };
    match std::process::Command::new(exe)
        .args(["-l", "-f", path])
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Ok(o) => {
            let e = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if e.is_empty() {
                "Could not read the key file".into()
            } else {
                e
            }
        }
        Err(_) => "Could not run ssh-keygen".into(),
    }
}

/// The public key text: the .pub file next to the key, or what ssh-keygen -y prints.
fn public_key_text(path: &str) -> Option<String> {
    if let Ok(t) = std::fs::read_to_string(format!("{path}.pub")) {
        return Some(t.trim().to_string());
    }
    let exe = crate::conn::find_program("ssh-keygen.exe")?;
    let o = std::process::Command::new(exe)
        .args(["-y", "-f", path])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if o.status.success() {
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        None
    }
}

trait ClearTemp {
    fn clear_temp_for<T: 'static + Clone + Send + Sync>(&mut self, id: egui::Id);
}

impl ClearTemp for egui::util::IdTypeMap {
    fn clear_temp_for<T: 'static + Clone + Send + Sync>(&mut self, id: egui::Id) {
        self.remove::<T>(id);
    }
}
