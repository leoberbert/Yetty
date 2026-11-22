// Embeds the application icon (and version info from Cargo.toml) into yetty.exe,
// so Explorer, the taskbar and shortcuts show the Yetty icon.
fn main() {
    println!("cargo:rerun-if-changed=assets/yetty.ico");
    #[cfg(windows)]
    {
        winresource::WindowsResource::new()
            .set_icon("assets/yetty.ico")
            .set("ProductName", "Yetty")
            .set("FileDescription", "Yetty")
            .compile()
            .expect("failed to embed the Windows icon");
    }
}
