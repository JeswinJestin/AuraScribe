// src-tauri/src/injection.rs
//! Cross-platform text injection into active application

use anyhow::{Context, Result};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
mod windows_injection {
    use super::*;
    use windows::{
        Win32::Foundation::*,
        Win32::UI::Accessibility::*,
        Win32::UI::Input::KeyboardAndMouse::*,
        Win32::UI::WindowsAndMessaging::*,
        Win32::System::Com::*,
    };

    pub fn inject_text(text: &str) -> Result<()> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

            // Get the foreground window
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                anyhow::bail!("No foreground window");
            }

            // Check if it's an editable control
            let mut focus = HWND(0);
            let _ = GetGUIThreadInfo(0, &mut focus as *mut _ as _);

            // Use SendInput for text injection
            let mut inputs = Vec::new();

            // Convert string to UTF-16
            let utf16: Vec<u16> = text.encode_utf16().collect();

            for &ch in &utf16 {
                let mut input = INPUT::default();
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                };
                inputs.push(input);

                // Key up
                let mut input_up = INPUT::default();
                input_up.r#type = INPUT_KEYBOARD;
                input_up.Anonymous.ki = KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                inputs.push(input_up);
            }

            if !inputs.is_empty() {
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }

            Ok(())
        }
    }

    pub fn get_active_window_title() -> Result<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return Ok(String::new());
            }

            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            Ok(String::from_utf16_lossy(&title[..len as usize]))
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_injection {
    use super::*;
    use objc2::rc::Retained;
    use objc2_foundation::{NSString, NSObject};
    use objc2_app_kit::{NSEvent, NSApplication, NSPasteboard, NSPasteboardTypeString};
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    pub fn inject_text(text: &str) -> Result<()> {
        // Method 1: Pasteboard (most reliable)
        unsafe {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            let str = NSString::from_str(text);
            pasteboard.setString_forType(&str, NSPasteboardTypeString);

            // Simulate Cmd+V
            let src = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
            let cmd_down = CGEventCreateKeyboardEvent(src, 0x37, true); // Cmd
            let v_down = CGEventCreateKeyboardEvent(src, 0x09, true);   // V
            let v_up = CGEventCreateKeyboardEvent(src, 0x09, false);
            let cmd_up = CGEventCreateKeyboardEvent(src, 0x37, false);

            CGEventSetFlags(cmd_down, kCGEventFlagMaskCommand);
            CGEventSetFlags(v_down, kCGEventFlagMaskCommand);
            CGEventSetFlags(v_up, kCGEventFlagMaskCommand);

            CGEventPost(kCGHIDEventTap, cmd_down);
            CGEventPost(kCGHIDEventTap, v_down);
            CGEventPost(kCGHIDEventTap, v_up);
            CGEventPost(kCGHIDEventTap, cmd_up);
        }
        Ok(())
    }

    pub fn get_active_app_name() -> Result<String> {
        unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            let app = workspace.frontmostApplication();
            if let Some(app) = app {
                let name = app.localizedName();
                Ok(name.to_string())
            } else {
                Ok(String::new())
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_injection {
    use super::*;
    use std::process::Command;

    pub fn inject_text(text: &str) -> Result<()> {
        // Try ydotool first (Wayland), then xdotool (X11)
        if Command::new("which").arg("ydotool").output().is_ok() {
            let mut child = Command::new("ydotool")
                .arg("type")
                .arg(text)
                .spawn()
                .context("Failed to start ydotool")?;
            child.wait().context("ydotool failed")?;
        } else if Command::new("which").arg("xdotool").output().is_ok() {
            let mut child = Command::new("xdotool")
                .arg("type")
                .arg("--clearmodifiers")
                .arg(text)
                .spawn()
                .context("Failed to start xdotool")?;
            child.wait().context("xdotool failed")?;
        } else {
            anyhow::bail!("Neither ydotool nor xdotool found. Install one for text injection.");
        }
        Ok(())
    }

    pub fn get_active_window() -> Result<String> {
        if Command::new("which").arg("xdotool").output().is_ok() {
            let output = Command::new("xdotool")
                .arg("getactivewindow")
                .arg("getwindowname")
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok(String::new())
        }
    }
}

pub struct TextInjector {
    app_handle: AppHandle,
}

impl TextInjector {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn inject(&self, text: &str) -> Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }

        let cleaned = self.clean_text(text);

        #[cfg(target_os = "windows")]
        {
            windows_injection::inject_text(&cleaned)?;
        }

        #[cfg(target_os = "macos")]
        {
            macos_injection::inject_text(&cleaned)?;
        }

        #[cfg(target_os = "linux")]
        {
            linux_injection::inject_text(&cleaned)?;
        }

        // Emit event for UI
        let _ = self.app_handle.emit("text-injected", &cleaned);

        Ok(())
    }

    fn clean_text(&self, text: &str) -> String {
        // Remove any control characters that could cause issues
        text.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect()
    }

    pub fn get_active_context(&self) -> Result<AppContext> {
        #[cfg(target_os = "windows")]
        {
            let title = windows_injection::get_active_window_title()?;
            Ok(AppContext { title, platform: "windows" })
        }

        #[cfg(target_os = "macos")]
        {
            let name = macos_injection::get_active_app_name()?;
            Ok(AppContext { title: name, platform: "macos" })
        }

        #[cfg(target_os = "linux")]
        {
            let title = linux_injection::get_active_window()?;
            Ok(AppContext { title, platform: "linux" })
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(AppContext { title: String::new(), platform: "unknown" })
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AppContext {
    pub title: String,
    pub platform: &'static str,
}

pub fn init(app_handle: AppHandle) -> Result<TextInjector> {
    Ok(TextInjector::new(app_handle))
}