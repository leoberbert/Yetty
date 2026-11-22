//! Menu captions with their mnemonic marked by "&", as in the original program (Menus/default.mnu).

pub const CAPTIONS: &[&str] = &[
    "&About SecureCRT...",
    "&Active Sessions",
    "&Always on Top",
    "&Aqua",
    "&Auto Print",
    "&Auto Save Options",
    "&Blue",
    "&Button Bar",
    "&Cancel",
    "&Carbon",
    "&Cascade",
    "&Check At Startup",
    "&Close",
    "&Command Window",
    "&Configure",
    "&Connect...",
    "&Copy",
    "&Create Public Key...",
    "&Deregister...",
    "&Disconnect",
    "&Edit",
    "&Edit...",
    "&Eject page",
    "&Enter License Data...",
    "&Export Public Key from Certificate...",
    "&File",
    "&Find...",
    "&Full Screen",
    "&Global Options...",
    "&Go to Command Window",
    "&Help",
    "&Help Topics",
    "&Horizontal Scroll Bar",
    "&Keymap Editor...",
    "&Keyword Highlighting",
    "&Legacy Themes",
    "&Local Shell Defaults...",
    "&Log Session",
    "&Manage Snapshots...",
    "&Menu Bar",
    "&Move",
    "&Native",
    "&Office 2010 Blue",
    "&Office White",
    "&Open Snapshot",
    "&Open URL",
    "&Options",
    "&Order SecureCRT...",
    "&Paste",
    "&Print",
    "&Public-Key Assistant...",
    "&Quick Connect...",
    "&RDP Defaults...",
    "&Receive ASCII...",
    "&Reconnect",
    "&Reset",
    "&Restore",
    "&Run...",
    "&Save As...",
    "&Screen",
    "&Script",
    "&Send ASCII...",
    "&Serial Defaults...",
    "&Session Options...",
    "&Shaded",
    "&Single Tab Group",
    "&Size",
    "&Start Recording Script",
    "&Status Bar",
    "&System",
    "&Tabs",
    "&Toggle Menu Bar",
    "&Toolbar",
    "&Trace Options",
    "&Transfer",
    "&Update",
    "&Update Now...",
    "&Use Firewall",
    "&VS Blue",
    "&Vertical Scroll Bar",
    "&View",
    "&View Available...",
    "&Window",
    "&Window Transparency",
    "&Zmodem Upload List...",
    "&Zoom",
    "1&00%",
    "A&lways on Top",
    "Add Key&word...",
    "Blac&k",
    "C&ancel Recording Script",
    "C&ommand Manager",
    "C&opy and Paste",
    "Change Configuration Pa&ssphrase...",
    "Change Creden&tials Password",
    "Clear &Screen",
    "Clear Screen and Scrollbac&k",
    "Clear Scroll&back",
    "Clo&ne Session",
    "Conn&ect Local Shell",
    "Connect &SFTP Tab",
    "Connect Ba&r",
    "Connect in Ta&b/Tile...",
    "Contact &Support...",
    "Convert Private Key to &OpenSSH Format...",
    "D&efault Session...",
    "D&isconnect All",
    "E&xit",
    "E&xport Settings...",
    "Go to Acti&ve Sessions",
    "Go to Comman&d Manager",
    "Go to Session &Manager",
    "He&x View",
    "I&mport Settings from XML File...",
    "Impo&rt Settings from Text File...",
    "K&eyword List Properties...",
    "Loc&k Session...",
    "Lookup Selectio&n",
    "M&anage Saved Credentials...",
    "Ma&nage Agent Keys...",
    "Ma&ximize",
    "Mi&nimize",
    "Office 2010 Bla&ck",
    "Office 2010 Sil&ver",
    "Open &File...",
    "Open Scratc&hpad",
    "Paste as &Quotation",
    "Pause Lo&gging",
    "Print Se&lection",
    "Print Set&up...",
    "R&ename Window",
    "Ra&w Log Session",
    "Re&ceive Xmodem...",
    "Recei&ve Ymodem...",
    "Receive Ker&mit...",
    "Reconnect &All",
    "S&end Break",
    "S&ilver",
    "S&top Recording Script...",
    "Sa&ve Snapshot",
    "Save Settings &Now",
    "Save Settings No&w",
    "Se&lection",
    "Se&nd Xmodem...",
    "SecureCRT &Web Page...",
    "Select &All",
    "Sen&d Ymodem...",
    "Send &Binary...",
    "Send &Kermit...",
    "Sessio&n Manager",
    "Session Ta&bs",
    "Start &TFTP Server",
    "Start Zmodem &Upload",
    "Th&emes",
    "Tile &Horizontally",
    "Tile &Vertically",
    "Too&ls",
    "VS &Dark",
    "VS &White",
    "Zoom &in",
    "Zoom &out",
];

use std::collections::HashMap;
use std::sync::OnceLock;

fn table() -> &'static HashMap<String, (usize, char)> {
    static T: OnceLock<HashMap<String, (usize, char)>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for c in CAPTIONS {
            let Some(at) = c.find('&') else { continue };
            let Some(ch) = c[at + 1..].chars().next() else { continue };
            let plain = format!("{}{}", &c[..at], &c[at + 1..]).replace("SecureCRT", "Yetty");
            let idx = c[..at].replace("SecureCRT", "Yetty").chars().count();
            m.entry(plain).or_insert((idx, ch.to_ascii_lowercase()));
        }
        m
    })
}

/// The mnemonic of a menu caption: (character index in the caption, the letter).
pub fn lookup(caption: &str) -> Option<(usize, char)> {
    table().get(caption).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captions_give_the_original_mnemonics() {
        assert_eq!(lookup("Connect..."), Some((0, 'c')));
        assert_eq!(lookup("Tools"), Some((3, 'l')));
        assert_eq!(lookup("Send Xmodem..."), Some((2, 'n')));
        assert_eq!(lookup("About Yetty..."), Some((0, 'a')));
        assert_eq!(lookup("No such item"), None);
    }

    #[test]
    fn submenus_and_window_menu_have_mnemonics() {
        for (caption, letter) in [
            ("Themes", 'e'),
            ("Zoom", 'z'),
            ("Configure", 'c'),
            ("Print", 'p'),
            ("Open Snapshot", 'o'),
            ("Save Snapshot", 'v'),
            ("Tabs", 't'),
            ("Tile Vertically", 'v'),
            ("Tile Horizontally", 'h'),
            ("Cascade", 'c'),
            ("Single Tab Group", 's'),
            ("Window Transparency", 'w'),
        ] {
            assert_eq!(lookup(caption).map(|m| m.1), Some(letter), "{caption}");
        }
    }
}
