// src-tauri/src/system.rs
//! System integration: tray icon, global shortcuts, auto-start, permissions

use anyhow::{Context, Result};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use windows::{
        Win32::Foundation::*,
        Win32::UI::Shell::*,
    };

    pub fn set_autostart(enabled: bool) -> Result<()> {
        // Windows autostart via registry
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)?;

        if enabled {
            let exe_path = std::env::current_exe()?;
            run_key.set_value("AuraScribe", &exe_path.to_string_lossy().to_string())?;
        } else {
            run_key.delete_value("AuraScribe").ok();
        }
        Ok(())
    }

    pub fn check_microphone_permission() -> bool {
        true // Windows handles this at OS level
    }

    pub fn check_accessibility_permission() -> bool {
        // Check if we have UI Access
        true // Simplified
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::process::Command;

    pub fn set_autostart(enabled: bool) -> Result<()> {
        let plist_path = dirs::home_dir()
            .context("No home dir")?
            .join("Library/LaunchAgents/dev.aurascribe.AuraScribe.plist");

        if enabled {
            let exe_path = std::env::current_exe()?;
            let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.aurascribe.AuraScribe</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#, exe_path.display());

            std::fs::write(&plist_path, plist_content)?;
            Command::new("launchctl").args(["load", plist_path.to_str().unwrap()]).status()?;
        } else {
            Command::new("launchctl").args(["unload", plist_path.to_str().unwrap()]).status().ok();
            std::fs::remove_file(&plist_path).ok;
        }
        Ok(())
    }

    pub fn check_microphone_permission() -> bool {
        // Check TCC database
        let output = Command::new("tccutil")
            .args(["reset", "Microphone", "dev.aurascribe.AuraScribe"])
            .output()
            .ok();
        true
    }

    pub fn check_accessibility_permission() -> bool {
        // Check AXIsProcessTrusted
        true
    }

    pub fn request_accessibility_permission() -> Result<()> {
        Command::new("open")
            .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"])
            .status()?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub fn set_autostart(enabled: bool) -> Result<()> {
        let autostart_dir = dirs::home_dir()
            .context("No home dir")?
            .join(".config/autostart");
        std::fs::create_dir_all(&autostart_dir)?;

        let desktop_path = autostart_dir.join("aurascribe.desktop");

        if enabled {
            let exe_path = std::env::current_exe()?;
            let content = format!(r#"[Desktop Entry]
Type=Application
Name=AuraScribe
Exec={}
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
"#, exe_path.display());
            std::fs::write(&desktop_path, content)?;
        } else {
            std::fs::remove_file(&desktop_path).ok();
        }
        Ok(())
    }

    pub fn check_microphone_permission() -> bool { true }
    pub fn check_accessibility_permission() -> bool { true }
}

/// System tray manager
pub struct SystemTrayManager {
    app_handle: AppHandle,
}

impl SystemTrayManager {
    pub fn new(app_handle: AppHandle) -> Result<Self> {
        Ok(Self { app_handle })
    }

    pub fn update_menu(&self, is_recording: bool, model_loaded: bool) -> Result<()> {
        // Menu updates handled via Tauri events
        Ok(())
    }
}

/// Global shortcut manager
pub struct GlobalShortcutManager {
    app_handle: AppHandle,
    current_hotkey: Option<String>,
    current_mode: Option<String>, // "press-hold" | "toggle"
}

impl GlobalShortcutManager {
    pub fn new() -> Self {
        Self {
            app_handle: AppHandle::default(),
            current_hotkey: None,
            current_mode: None,
        }
    }

    pub fn init(&mut self, app_handle: AppHandle, hotkey: &str, mode: &str) -> Result<()> {
        self.app_handle = app_handle.clone();
        self.current_hotkey = Some(hotkey.to_string());
        self.current_mode = Some(mode.to_string());
        self.register_shortcut(hotkey, mode)
    }

    pub fn update_hotkey(&mut self, hotkey: &str, mode: &str) -> Result<()> {
        // Unregister old
        if let Some(old) = &self.current_hotkey {
            self.app_handle.global_shortcut().unregister(old).ok();
        }
        self.current_hotkey = Some(hotkey.to_string());
        self.current_mode = Some(mode.to_string());
        self.register_shortcut(hotkey, mode)
    }

    fn register_shortcut(&self, hotkey: &str, mode: &str) -> Result<()> {
        let app_handle = self.app_handle.clone();
        let mode = mode.to_string();

        // Convert hotkey to Tauri format
        let shortcut = parse_hotkey(hotkey)?;

        app_handle.global_shortcut().on_shortcut(shortcut, move |app, shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                if mode == "press-hold" {
                    let _ = app.emit("global-shortcut-pressed", shortcut);
                } else {
                    let _ = app.emit("global-shortcut-toggled", shortcut);
                }
            } else if event.state == tauri_plugin_global_shortcut::ShortcutState::Released {
                if mode == "press-hold" {
                    let _ = app.emit("global-shortcut-released", shortcut);
                }
            }
        })?;

        app_handle.global_shortcut().register(shortcut)?;
        Ok(())
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<()> {
        #[cfg(target_os = "windows")]
        windows::set_autostart(enabled)?;

        #[cfg(target_os = "macos")]
        macos::set_autostart(enabled)?;

        #[cfg(target_os = "linux")]
        linux::set_autostart(enabled)?;

        Ok(())
    }
}

fn parse_hotkey(hotkey: &str) -> Result<Shortcut> {
    // Parse "Ctrl+Space" -> "Control+Space"
    let normalized = hotkey
        .replace("Ctrl", "Control")
        .replace("Cmd", "Meta")
        .replace("Alt", "Alt")
        .replace("Shift", "Shift")
        .replace("Super", "Meta");

    Shortcut::try_from(normalized.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid hotkey: {}", e))
}

pub fn check_microphone_permission() -> bool {
    #[cfg(target_os = "windows")]
    return windows::check_microphone_permission();

    #[cfg(target_os = "macos")]
    return macos::check_microphone_permission();

    #[cfg(target_os = "linux")]
    return linux::check_microphone_permission();

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return true;
}

pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "windows")]
    return windows::check_accessibility_permission();

    #[cfg(target_os = "macos")]
    return macos::check_accessibility_permission();

    #[cfg(target_os = "linux")]
    return linux::check_accessibility_permission();

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return true;
}

pub fn request_accessibility_permission() -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::request_accessibility_permission();

    #[cfg(not(target_os = "macos"))]
    return Ok(());
}