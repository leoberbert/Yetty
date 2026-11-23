//! Where the fonts of `install_fallback_fonts` come from.
//!
//! Windows: the fonts the system ships (Consolas, Segoe UI, Segoe MDL2 Assets, ...), exactly as before.
//! Linux: Microsoft's fonts are not there and cannot be shipped, so each role has a list of files to look for,
//! best match first. A person who wants the exact Windows look can drop the real fonts (`consola.ttf`,
//! `segoeui.ttf`, `segmdl2.ttf`, ...) in `~/.local/share/fonts` or in `<config dir>/yetty/fonts`: they win over
//! the substitutes. Without them: menus and dialogs use the bundled Selawik (Microsoft's open font, the same
//! metrics as Segoe UI), the toolbar and menu icons a bundled icon font with the same code points as
//! Segoe MDL2 Assets (see `tools/make-icons-font.py`), and the terminal Yetty Mono: Hack with the cell width
//! of Consolas (see `tools/make-mono-font.py`).

/// The bytes of the font for `role` (`win_consolas`, `win_segoeui`, `fb_mdl2`, `fb_symbols`, `fb_cjk_sc`,
/// `fb_cjk_jp`, `fb_korean`, `fb_cjk_tc`), if the system has one.
pub fn load(role: &str) -> Option<Vec<u8>> {
    imp::load(role)
}

#[cfg(windows)]
mod imp {
    pub fn load(role: &str) -> Option<Vec<u8>> {
        let file = match role {
            "win_consolas" => "consola.ttf",
            "win_segoeui" => "segoeui.ttf",
            "fb_mdl2" => "segmdl2.ttf",
            "fb_symbols" => "seguisym.ttf",
            "fb_cjk_sc" => "msyh.ttc",
            "fb_cjk_jp" => "msgothic.ttc",
            "fb_korean" => "malgun.ttf",
            "fb_cjk_tc" => "msjh.ttc",
            _ => return None,
        };
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        std::fs::read(std::path::Path::new(&windir).join("Fonts").join(file)).ok()
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    /// File names (lower case) to look for, best first.
    fn candidates(role: &str) -> &'static [&'static str] {
        match role {
            // Consolas, if the person installed it; otherwise the bundled Yetty Mono (see `bundled`)
            "win_consolas" => &["consola.ttf"],
            // Segoe UI, if the person installed it; otherwise the bundled Selawik (see `bundled`), Microsoft's
            // open font with the same metrics
            "win_segoeui" => &["segoeui.ttf", "selawik-regular.ttf", "selawk.ttf"],
            // the icon font: the real one if the person installed it, else the bundled one (see `bundled`)
            "fb_mdl2" => &["segmdl2.ttf", "segoeicons.ttf", "segoefluenticons.ttf"],
            "fb_symbols" => &[
                "seguisym.ttf",
                "notosanssymbols2-regular.ttf",
                "notosanssymbols-regular.ttf",
                "dejavusans.ttf",
            ],
            "fb_cjk_sc" => &[
                "msyh.ttc",
                "notosanscjk-regular.ttc",
                "notosanscjksc-regular.otf",
                "wqy-microhei.ttc",
                "wqy-zenhei.ttc",
                "droidsansfallbackfull.ttf",
            ],
            "fb_cjk_jp" => &[
                "msgothic.ttc",
                "notosanscjkjp-regular.otf",
                "ipagp.ttf",
                "takao-gothic.ttf",
            ],
            "fb_korean" => &[
                "malgun.ttf",
                "notosanscjkkr-regular.otf",
                "nanumgothic.ttf",
                "unifont.ttf",
            ],
            "fb_cjk_tc" => &["msjh.ttc", "notosanscjktc-regular.otf"],
            _ => &[],
        }
    }

    fn bundled(role: &str) -> Option<Vec<u8>> {
        match role {
            "fb_mdl2" => Some(include_bytes!("../assets/fonts/yetty-icons.ttf").to_vec()),
            "win_segoeui" => Some(include_bytes!("../assets/fonts/selawk.ttf").to_vec()),
            "win_consolas" => Some(include_bytes!("../assets/fonts/yetty-mono.ttf").to_vec()),
            _ => None,
        }
    }

    fn font_dirs() -> Vec<PathBuf> {
        let mut dirs = vec![crate::config::data_dir().join("fonts")];
        if let Some(home) = crate::config::home_dir() {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
        if let Some(x) = std::env::var_os("XDG_DATA_HOME").filter(|p| !p.is_empty()) {
            dirs.push(PathBuf::from(x).join("fonts"));
        }
        for d in std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
            .split(':')
            .filter(|d| !d.is_empty())
        {
            dirs.push(Path::new(d).join("fonts"));
        }
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/local/share/fonts".into());
        dirs
    }

    fn walk(dir: &Path, depth: u32, out: &mut Vec<(String, PathBuf)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth < 6 {
                    walk(&p, depth + 1, out);
                }
            } else if let Some(n) = p.file_name() {
                out.push((n.to_string_lossy().to_lowercase(), p));
            }
        }
    }

    /// Every font file under the font folders, found once (the folders can hold thousands of files).
    /// The person's own folders come first, so their fonts win.
    static INDEX: std::sync::LazyLock<Vec<(String, PathBuf)>> = std::sync::LazyLock::new(|| {
        let mut out = Vec::new();
        let mut seen = Vec::new();
        for d in font_dirs() {
            if !seen.contains(&d) {
                walk(&d, 0, &mut out);
                seen.push(d);
            }
        }
        out
    });

    pub fn load(role: &str) -> Option<Vec<u8>> {
        for want in candidates(role) {
            let idx = &*INDEX;
            // earlier folders (the person's own) come first in the index
            if let Some((_, p)) = idx.iter().find(|(n, _)| n == want) {
                if let Ok(bytes) = std::fs::read(p) {
                    return Some(bytes);
                }
            }
        }
        bundled(role)
    }
}
