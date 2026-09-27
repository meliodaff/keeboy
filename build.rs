fn main() {
    #[cfg(windows)]
    {
        // Embeds assets/keeboy.ico as icon resource #1 so Explorer, Alt+Tab, the taskbar
        // and the tray all show the keycap instead of the generic Windows app icon.
        println!("cargo:rerun-if-changed=assets/keeboy.rc");
        println!("cargo:rerun-if-changed=assets/keeboy.ico");
        embed_resource::compile("assets/keeboy.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
