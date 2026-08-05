use tracing::info;

#[cfg(target_os = "windows")]
pub fn set_startup(auto_start: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let key = hkcu.open_subkey_with_flags(path, KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;

    if auto_start {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("AuraScribe", &exe_path.to_string_lossy().to_string())
            .map_err(|e| e.to_string())?;
        info!("Added to startup");
    } else {
        key.delete_value("AuraScribe").ok();
        info!("Removed from startup");
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_startup(_auto_start: bool) -> Result<(), String> {
    Err("Startup management is not yet implemented on this platform".into())
}

