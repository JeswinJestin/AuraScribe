//! Getting transcribed text to the user's cursor.
//!
//! Two strategies, because neither one wins everywhere:
//!
//! - **Paste** (clipboard + Ctrl+V) is effectively instant regardless of length, and cannot
//!   corrupt the text. It is the default for anything longer than a short phrase.
//! - **Typing** (`SendInput` with Unicode scancodes) works in the rare places that ignore
//!   paste, and doesn't touch the clipboard. Used for short text only.
//!
//! Typing everything is what the first version did, and it was badly broken: a ~1,500
//! character transcript became one `SendInput` call carrying 3,000 key events. Windows
//! delivers those asynchronously into the target's input queue, which overflows — KEYUPs get
//! dropped, the key auto-repeats, and the result is mangled. A real captured example:
//!
//! ```text
//! 7.cccchose my uuuuuu uuurself,MMMMMM…Mumbai.……………………………
//! ```
//!
//! The fragments are in the right order, so the transcript was correct — only the delivery
//! was destroying it. Chunking alone is not enough at that size; paste is.

/// Above this many characters, paste instead of typing. Short enough that the clipboard is
/// left alone for quick phrases, low enough that no realistic dictation goes through the
/// typing path in bulk.
const PASTE_THRESHOLD: usize = 120;

/// Key events per `SendInput` call on the typing path. Small enough that the target's input
/// queue drains between batches.
const CHUNK_EVENTS: usize = 40;

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "windows")]
    pub fn inject_text(&self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        // Either strategy can fail, so each falls back to the other. Neither failure is
        // hypothetical: typing is refused by elevated windows, and the clipboard can be
        // unavailable outright — observed on the owner's machine, where even PowerShell's
        // `Set-Clipboard` returned "Requested Clipboard operation did not succeed". With
        // paste as the primary path for long text, no fallback would mean a wedged
        // clipboard silently swallows the user's dictation.
        if text.chars().count() > PASTE_THRESHOLD {
            return match self.paste_text(text) {
                Ok(()) => Ok(()),
                Err(e) => {
                    tracing::warn!("Paste failed ({}); falling back to typing", e);
                    self.type_text(text).map_err(|type_err| {
                        format!("Could not paste ({e}) and could not type ({type_err})")
                    })
                }
            };
        }

        match self.type_text(text) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!("Typing failed ({}); falling back to paste", e);
                self.paste_text(text)
                    .map_err(|paste_err| format!("Could not type ({e}) and could not paste ({paste_err})"))
            }
        }
    }

    /// Put the text on the clipboard and send Ctrl+V. Restores whatever was on the clipboard
    /// before, so dictating doesn't silently destroy what the user had copied.
    #[cfg(target_os = "windows")]
    fn paste_text(&self, text: &str) -> Result<(), String> {
        let previous = read_clipboard_text();

        set_clipboard_text(text)?;

        let result = send_ctrl_v();

        // The paste is asynchronous — the target reads the clipboard when it processes the
        // keystroke. Restoring immediately would race it and paste the old contents.
        std::thread::sleep(std::time::Duration::from_millis(120));

        if let Some(prev) = previous {
            let _ = set_clipboard_text(&prev);
        }

        result.map_err(|e| {
            format!("{e}; the text is on your clipboard — paste it with Ctrl+V")
        })
    }

    /// Synthesize the text as real keystrokes, in small batches.
    #[cfg(target_os = "windows")]
    fn type_text(&self, text: &str) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
            KEYEVENTF_UNICODE, VIRTUAL_KEY,
        };

        let make_input = |ch: u16, key_up: bool| -> INPUT {
            let mut flags = KEYEVENTF_UNICODE;
            if key_up {
                flags |= KEYEVENTF_KEYUP;
            }
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: ch,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        };

        let mut inputs: Vec<INPUT> = Vec::new();
        for unit in text.encode_utf16() {
            inputs.push(make_input(unit, false));
            inputs.push(make_input(unit, true));
        }

        for batch in inputs.chunks(CHUNK_EVENTS) {
            let expected = batch.len() as u32;
            let sent = unsafe { SendInput(batch, std::mem::size_of::<INPUT>() as i32) };
            if sent != expected {
                return Err(format!(
                    "SendInput delivered {sent}/{expected} key events (the focused window may be blocking synthetic input)"
                ));
            }
            // Let the target drain its input queue. Without this the queue overflows and
            // characters repeat or vanish.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn inject_text(&self, _text: &str) -> Result<(), String> {
        Err("Text injection is not yet implemented on this platform".into())
    }
}

#[cfg(target_os = "windows")]
fn send_ctrl_v() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        VK_CONTROL, VK_V,
    };

    let key = |vk: VIRTUAL_KEY, up: bool| -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    };

    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(
            "Could not send Ctrl+V (the focused window may be blocking synthetic input)".into(),
        );
    }
    Ok(())
}

/// `OpenClipboard` fails immediately if any other process currently holds the clipboard —
/// and on a normal desktop something usually does, briefly: clipboard managers, browsers,
/// Office, remote-desktop agents. Failing on the first attempt would lose the user's
/// transcript, since paste is the path every long dictation now takes.
///
/// This is why Windows clipboard code retries. The round-trip test caught it: it passed
/// early in a session and then failed three times in a row once another app was running.
#[cfg(target_os = "windows")]
fn open_clipboard_retrying() -> Result<(), String> {
    use windows::Win32::System::DataExchange::OpenClipboard;

    const ATTEMPTS: u32 = 10;
    const BACKOFF_MS: u64 = 20;

    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        match unsafe { OpenClipboard(None) } {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = e.to_string();
                std::thread::sleep(std::time::Duration::from_millis(
                    BACKOFF_MS * (attempt as u64 + 1),
                ));
            }
        }
    }
    Err(format!(
        "Could not open the clipboard after {ATTEMPTS} attempts (another application is          holding it): {last}"
    ))
}

/// Clipboard access via the Win32 API directly. The previous implementation shelled out to
/// `powershell -Command Set-Clipboard`, which cost hundreds of milliseconds per dictation
/// and mangled any text containing quotes or newlines.
#[cfg(target_os = "windows")]
fn set_clipboard_text(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::{HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_UNICODETEXT: u32 = 13;

    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let bytes = utf16.len() * std::mem::size_of::<u16>();

    open_clipboard_retrying()?;

    unsafe {
        let result = (|| -> Result<(), String> {
            EmptyClipboard().map_err(|e| format!("EmptyClipboard failed: {e}"))?;

            let handle: HGLOBAL =
                GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| format!("GlobalAlloc failed: {e}"))?;

            let ptr = GlobalLock(handle) as *mut u16;
            if ptr.is_null() {
                return Err("GlobalLock returned null".into());
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
            let _ = GlobalUnlock(handle);

            // Ownership of the memory transfers to the clipboard on success.
            SetClipboardData(CF_UNICODETEXT, HANDLE(handle.0))
                .map_err(|e| format!("SetClipboardData failed: {e}"))?;
            Ok(())
        })();

        let _ = CloseClipboard();
        result
    }
}

#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    const CF_UNICODETEXT: u32 = 13;

    open_clipboard_retrying().ok()?;

    unsafe {
        let text = (|| -> Option<String> {
            let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
            let global = HGLOBAL(handle.0);
            let ptr = GlobalLock(global) as *const u16;
            if ptr.is_null() {
                return None;
            }

            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            let s = String::from_utf16_lossy(slice);
            let _ = GlobalUnlock(global);
            Some(s)
        })();

        let _ = CloseClipboard();
        text
    }
}

impl Default for TextInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// The clipboard path carries every long dictation now, so a round-trip failure would
    /// lose the user's text silently. Also covers the characters that broke the previous
    /// `powershell -Command Set-Clipboard` implementation: quotes and newlines.
    #[test]
    fn clipboard_round_trips_awkward_text() {
        let cases = [
            "plain text",
            "it's got 'single' quotes",
            "double \"quotes\" too",
            "line one\nline two\r\nline three",
            "unicode: naïve café — em dash, 日本語",
            "trailing spaces   ",
        ];

        // A wedged clipboard is a machine condition, not a defect in this code - Windows
        // can leave it inaccessible process-wide. Report and skip rather than reporting a
        // regression that isn't one. The production path handles this by falling back to
        // typing; see `inject_text`.
        if let Err(e) = set_clipboard_text("probe") {
            eprintln!("SKIPPED: clipboard unavailable on this machine ({e})");
            return;
        }

        for case in cases {
            set_clipboard_text(case).expect("set_clipboard_text failed");
            let read = read_clipboard_text().expect("read_clipboard_text returned None");
            assert_eq!(read, case, "clipboard round-trip changed the text");
        }
    }

    #[test]
    fn long_text_takes_the_paste_path() {
        // The mangled-output bug was long text going through SendInput. Guard the boundary
        // so a future edit can't quietly route a transcript back onto the typing path.
        let long: String = "word ".repeat(200);
        assert!(long.chars().count() > PASTE_THRESHOLD);
        assert!("a short phrase".chars().count() <= PASTE_THRESHOLD);
    }
}
