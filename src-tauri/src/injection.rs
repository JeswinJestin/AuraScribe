pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "windows")]
    pub fn inject_text(&self, text: &str) -> Result<(), String> {
        use std::process::Command;
        // Use PowerShell to send keys for simple text injection
        Command::new("powershell")
            .args(["-Command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
            .output()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn inject_text(&self, _text: &str) -> Result<(), String> {
        Err("Text injection not implemented on this platform".into())
    }
}
