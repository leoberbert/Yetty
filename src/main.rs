#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod authdlg;
mod certkey;
mod config;
mod conn;
mod dialogs;
mod globaldlg;
mod kermit;
mod keymapdlg;
mod mnemonics;
mod nativedlg;
mod sftplist;
mod sysfonts;
mod term;
mod tftp;
mod transfer;
mod wizard;
mod zmodem;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let shot_at = args.iter().position(|a| a == "--screenshot");
    let screenshot = shot_at.and_then(|i| args.get(i + 1).cloned());
    // Anything else on the command line is a connection, e.g. `ssh user@host`, `telnet host`, `local`.
    let connect: Vec<String> = args
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(i, _)| Some(*i) != shot_at && Some(i.wrapping_sub(1)) != shot_at)
        .map(|(_, a)| a.clone())
        .collect();
    // `--session "name"` opens a saved session by its name.
    let connect = if connect.first().map(|c| c == "--session").unwrap_or(false) {
        Some(format!("session:{}", connect[1..].join(" ")))
    } else if connect.is_empty() {
        None
    } else {
        Some(connect.join(" "))
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Yetty")
        .with_inner_size([1180.0, 740.0])
        .with_min_inner_size([640.0, 400.0]);
    // window class (X11) / app id (Wayland): matches yetty.desktop, so the shell shows the right icon
    #[cfg(target_os = "linux")]
    {
        viewport = viewport.with_app_id("yetty");
    }
    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/yetty.png")) {
        let img = img.to_rgba8();
        viewport = viewport.with_icon(egui::IconData {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        });
    }
    if screenshot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if screenshot.is_some() {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    let shot = screenshot.clone();
    eframe::run_native(
        "Yetty",
        options,
        Box::new(move |cc| Ok(Box::new(app::YettyApp::new(cc, shot, connect)))),
    )
}
