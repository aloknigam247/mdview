/// Detect whether the OS is currently in light mode. Returns `None` if detection
/// is unsupported on this platform or fails (caller should fall back to Dark).
pub fn os_prefers_light() -> Option<bool> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            .ok()?;
        let value: u32 = key.get_value("AppsUseLightTheme").ok()?;
        Some(value != 0)
    }
    #[cfg(not(windows))]
    None
}
