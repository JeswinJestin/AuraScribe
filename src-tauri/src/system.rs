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

/// The window that had keyboard focus when dictation started — captured so we can paste back
/// into it even if focus moved. Returned as an `isize` so it can be stored in `AppState` without
/// pulling the Windows HWND type through the shared struct. 0 means "none".
///
/// This is what makes **click-to-stop on the overlay** reliable: clicking the overlay can move the
/// foreground away from the app you were dictating into, and injection pastes into whatever is
/// focused — so without restoring focus first, the text landed in the wrong place (or nowhere).
/// Stopping with the hotkey never moved focus, which is why only the mouse-stop path was broken.
#[cfg(target_os = "windows")]
pub fn capture_foreground_window() -> isize {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow().0 as isize }
}

#[cfg(not(target_os = "windows"))]
pub fn capture_foreground_window() -> isize {
    0
}

/// Bring the captured window back to the foreground just before injecting, so the paste lands
/// where the user was typing. A no-op if the handle is stale/zero or already foreground.
#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsWindow, SetForegroundWindow,
    };

    if hwnd == 0 {
        return;
    }
    let target = HWND(hwnd as *mut core::ffi::c_void);
    unsafe {
        if GetForegroundWindow().0 as isize == hwnd {
            return; // already focused — nothing to do (the hotkey-stop path)
        }
        if !IsWindow(target).as_bool() {
            return; // the window went away
        }
        let _ = SetForegroundWindow(target);
    }
    // Let the focus change settle before the paste keystroke is delivered.
    std::thread::sleep(std::time::Duration::from_millis(40));
}

#[cfg(not(target_os = "windows"))]
pub fn focus_window(_hwnd: isize) {}

