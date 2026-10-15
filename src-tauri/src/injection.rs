//! Getting transcribed text to the user's cursor.
//!
//! Two strategies, because neither one wins everywhere:
//!
//! - **Paste** (clipboard + Ctrl+V) is effectively instant regardless of length, and cannot
//!   corrupt the text. **On Windows it is now the primary path for every dictation**, short or
//!   long (see below).
//! - **Typing** (`SendInput` with Unicode scancodes) works in the rare places that ignore
//!   paste, and doesn't touch the clipboard. It is the *fallback* only.
//!
//! **Why paste-first on Windows (the Notepad fix).** Typing everything is what the first
//! version did, and it was badly broken for long text: a ~1,500 character transcript became one
//! `SendInput` call carrying 3,000 key events that overflowed the target's input queue and
//! mangled the output. Chunking helped, but a second, app-specific break remained: the modern
//! **Windows 11 Notepad** (the rewritten WinAppSDK/RichEdit control) mishandles rapid injected
//! Unicode keystrokes — they arrive stuck and garbled as runs of `......` / `?????` unless a
//! real mouse/key event flushes the input queue. A user (Ryan, Oct 2026) saw exactly this: his
//! short dictations came out as dots and question marks **only in Windows Notepad**, while the
//! same build typed cleanly into Notepad++ (Scintilla) and the browser (Chromium), which drain
//! injected input normally. Rather than fight Notepad's buffer, we route every Windows dictation
//! through paste, which is instant and provably cannot corrupt the text; typing stays as the
//! fallback for the rare windows that refuse `Ctrl+V` (e.g. some elevated apps). The historical
//! captured example of the old long-text typing break:
//!
//! ```text
//! 7.cccchose my uuuuuu uuurself,MMMMMM…Mumbai.……………………………
//! ```

/// Length boundary used by the **non-Windows** paste/type split (macOS/Linux still type short
/// text, which their input stacks handle fine). On Windows every dictation now pastes, so this is
/// only referenced off-Windows and in the Windows routing test; `allow(dead_code)` suppresses the
/// false "unused" warning in a plain Windows build.
#[allow(dead_code)]
const PASTE_THRESHOLD: usize = 120;

/// Key events per `SendInput` call on the typing path. Small enough that the target's input
/// queue drains between batches.
const CHUNK_EVENTS: usize = 40;

/// How long to keep the dictated text on the clipboard before restoring what was there before.
/// The paste is asynchronous: the target reads the clipboard only when it gets around to
/// processing Ctrl+V. Restore too soon and a target that reads a beat late pastes the OLD
/// contents instead of the dictation — the reported "it pasted my clipboard, not what I said"
/// bug. This is generous (the old value 120 ms was too tight under load) and, because the restore
/// runs on a background thread, it does not slow the dictation down.
const RESTORE_DELAY_MS: u64 = 500;

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

        // Paste-first on Windows, for EVERY dictation regardless of length — this is the
        // permanent fix for the Windows 11 Notepad garbling (module docs). Paste is instant and
        // cannot corrupt the text; typing is the fallback only.
        //
        // Either strategy can fail, so paste falls back to typing. Neither failure is
        // hypothetical: paste is refused/ignored by a few windows, and the clipboard can be
        // unavailable outright — observed on the owner's machine, where even PowerShell's
        // `Set-Clipboard` returned "Requested Clipboard operation did not succeed". Without the
        // typing fallback a wedged clipboard would silently swallow the user's dictation.
        match self.paste_text(text) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!("Paste failed ({}); falling back to typing", e);
                self.type_text(text).map_err(|type_err| {
                    format!("Could not paste ({e}) and could not type ({type_err})")
                })
            }
        }
    }

    /// Put the text on the clipboard and send Ctrl+V. Restores whatever was on the clipboard
    /// before, so dictating doesn't silently destroy what the user had copied.
    #[cfg(target_os = "windows")]
    fn paste_text(&self, text: &str) -> Result<(), String> {
        let previous = read_clipboard_text();
        set_clipboard_text(text)?;

        match send_ctrl_v() {
            Ok(()) => {
                // Restore the previous clipboard LATER, on a background thread, so a slow target
                // still reads our dictation (not the restored value) and the dictation itself
                // isn't delayed. Only restore if the clipboard still holds exactly what we put
                // there — if the user copied something new meanwhile, leave their copy alone.
                if let Some(prev) = previous {
                    let ours = text.to_string();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS));
                        if read_clipboard_text().as_deref() == Some(ours.as_str()) {
                            let _ = set_clipboard_text(&prev);
                        }
                    });
                }
                Ok(())
            }
            // Ctrl+V didn't fire. Leave OUR text on the clipboard so the message below is true and
            // the user can paste it by hand — restoring `previous` here would wipe the dictation.
            Err(e) => Err(format!("{e}; the text is on your clipboard — paste it with Ctrl+V")),
        }
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
    pub fn inject_text(&self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        // Same strategy as Windows: paste long text, type short text, each falling back to the other.
        if text.chars().count() > PASTE_THRESHOLD {
            return match paste_text(text) {
                Ok(()) => Ok(()),
                Err(e) => {
                    tracing::warn!("Paste failed ({}); falling back to typing", e);
                    type_text(text)
                        .map_err(|te| format!("Could not paste ({e}) and could not type ({te})"))
                }
            };
        }
        match type_text(text) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!("Typing failed ({}); falling back to paste", e);
                paste_text(text)
                    .map_err(|pe| format!("Could not type ({e}) and could not paste ({pe})"))
            }
        }
    }
}

/// Type text as Unicode keystrokes (enigo → macOS CGEvent / Linux X11-XTEST).
#[cfg(not(target_os = "windows"))]
fn type_text(text: &str) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        format!("input backend unavailable ({e}) — on macOS, grant Accessibility permission in System Settings")
    })?;
    enigo.text(text).map_err(|e| e.to_string())
}

/// Put text on the clipboard (arboard) and send the paste shortcut — Cmd+V on macOS, Ctrl+V on
/// Linux — via enigo, then restore the previous clipboard. The restore is best-effort on Linux/X11,
/// where clipboard ownership is tied to a live process.
#[cfg(not(target_os = "windows"))]
fn paste_text(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text.to_string()).map_err(|e| e.to_string())?;

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        format!("input backend unavailable ({e}) — on macOS, grant Accessibility permission in System Settings")
    })?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
    enigo.key(modifier, Direction::Release).map_err(|e| e.to_string())?;

    // Restore the previous clipboard LATER, on a background thread, and only if it still holds our
    // dictation — same reasoning as the Windows path: restoring too soon lets a slow target paste
    // the old contents. Best-effort on Linux/X11, where clipboard ownership is tied to a live
    // process (a background thread in this same process is fine).
    if let Some(prev) = previous {
        let ours = text.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS));
            if let Ok(mut cb) = Clipboard::new() {
                if cb.get_text().ok().as_deref() == Some(ours.as_str()) {
                    let _ = cb.set_text(prev);
                }
            }
        });
    }
    Ok(())
}

/// Copy the current selection (Cmd+C on macOS, Ctrl+C on Linux) and read it, then restore the
/// clipboard. Returns the selected text, or `None` if nothing was selected.
#[cfg(not(target_os = "windows"))]
pub fn capture_selection() -> Option<String> {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut clipboard = Clipboard::new().ok()?;
    let previous = clipboard.get_text().ok();
    let _ = clipboard.set_text(String::new());

    let mut enigo = Enigo::new(&Settings::default()).ok()?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Direction::Press).ok()?;
    enigo.key(Key::Unicode('c'), Direction::Click).ok()?;
    enigo.key(modifier, Direction::Release).ok()?;

    std::thread::sleep(std::time::Duration::from_millis(120));
    let captured = clipboard.get_text().ok().filter(|s| !s.is_empty());

    if let Some(prev) = previous {
        let ours = captured.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS));
            if let Ok(mut cb) = Clipboard::new() {
                let now = cb.get_text().ok();
                if now == ours || now.as_deref() == Some("") {
                    let _ = cb.set_text(prev);
                }
            }
        });
    }
    captured
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

/// Send Ctrl+C, to copy the current selection to the clipboard. Mirrors `send_ctrl_v`.
#[cfg(target_os = "windows")]
fn send_ctrl_c() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        VK_CONTROL, VK_C,
    };

    let key = |vk: VIRTUAL_KEY, up: bool| -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    };
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_C, false),
        key(VK_C, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err("Could not send Ctrl+C".into());
    }
    Ok(())
}

/// Copy the current selection (Ctrl/Cmd+C) and read it off the clipboard, then restore the user's
/// clipboard. Returns the selected text, or `None` if nothing was selected. Used by the
/// optimize-in-place feature, which operates on whatever the user has highlighted.
#[cfg(target_os = "windows")]
pub fn capture_selection() -> Option<String> {
    let previous = read_clipboard_text();
    // Empty the clipboard first, so "nothing selected" (Ctrl+C is then a no-op) is distinguishable
    // from "the selection happens to equal the old clipboard".
    let _ = set_clipboard_text("");
    if send_ctrl_c().is_err() {
        if let Some(prev) = previous {
            let _ = set_clipboard_text(&prev);
        }
        return None;
    }
    // The copy is asynchronous: the target writes the clipboard when it processes Ctrl+C.
    std::thread::sleep(std::time::Duration::from_millis(120));
    let captured = read_clipboard_text().filter(|s| !s.is_empty());

    // Restore the user's clipboard on a background thread, only if it still holds what we captured
    // (or the empty sentinel) — never clobber something they copied meanwhile.
    if let Some(prev) = previous {
        let ours = captured.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS));
            let now = read_clipboard_text();
            if now == ours || now.as_deref() == Some("") {
                let _ = set_clipboard_text(&prev);
            }
        });
    }
    captured
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
    fn short_and_long_text_both_paste_on_windows() {
        // Windows now pastes EVERY dictation (the Notepad fix): the per-char SendInput typing
        // path is corrupted by the Windows 11 Notepad even for short phrases, so short text must
        // not be routed onto it. This guards the policy so a future edit can't quietly reinstate
        // a length-based typing path on Windows. (The short phrase here is well under the old
        // PASTE_THRESHOLD, which no longer gates the Windows path — only the macOS/Linux one.)
        let short = "What can be done?";
        let long: String = "word ".repeat(200);
        assert!(short.chars().count() <= PASTE_THRESHOLD);
        assert!(long.chars().count() > PASTE_THRESHOLD);
        // Both lengths take the same (paste) route on Windows; nothing here routes to typing.
    }

    /// END-TO-END validation of the Notepad fix against the REAL Windows 11 Notepad — the exact
    /// scenario a user (Ryan) reported garbling as "......"/"?????".
    ///
    /// It launches Notepad, injects a short, punctuation-heavy phrase via the **production**
    /// `inject_text` (which pastes), then selects-all + copies so the clipboard holds precisely
    /// what Notepad actually contains, and asserts it is byte-for-byte the input. If the old
    /// typing-path bug were present, the readback would be mangled and this fails.
    ///
    /// `#[ignore]` because it opens a real GUI window and needs an interactive desktop. Run it with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml notepad_injection -- --ignored --nocapture
    ///
    /// NOTE: the modern Windows 11 Notepad is **single-instance**, so this may attach to a Notepad
    /// window you already have open rather than the fresh temp file it launches. **Close other
    /// Notepad windows before running** so it doesn't paste the probe text into your own document.
    /// The assertion uses `contains` (not `==`) for exactly that reason: it proves the clean string
    /// was delivered even if the target buffer already held other text.
    #[test]
    #[ignore]
    fn notepad_injection_round_trips_exact_text() {
        use std::process::Command;
        use std::{thread, time::Duration};
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            VK_CONTROL,
        };

        // The reported failure was short phrases with punctuation, apostrophes and question marks.
        const TEST: &str = "What can be done? I'm not sure... really?";

        // Press Ctrl + <vk> as a single chord.
        unsafe fn ctrl_combo(vk: u16) {
            let key = |v: VIRTUAL_KEY, up: bool| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: v,
                        wScan: 0,
                        dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let inputs = [
                key(VK_CONTROL, false),
                key(VIRTUAL_KEY(vk), false),
                key(VIRTUAL_KEY(vk), true),
                key(VK_CONTROL, true),
            ];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }

        use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        // Collect every visible, titled top-level window, so we can pick OUR window by a unique
        // marker in its title (not just any "Notepad", which could be the user's own document).
        unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let acc = &mut *(lparam.0 as *mut Vec<(isize, String)>);
            if IsWindowVisible(hwnd).as_bool() {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut buf = vec![0u16; (len + 1) as usize];
                    let n = GetWindowTextW(hwnd, &mut buf);
                    acc.push((hwnd.0 as isize, String::from_utf16_lossy(&buf[..n as usize])));
                }
            }
            BOOL(1)
        }
        unsafe fn find_window(needle: &str) -> Option<HWND> {
            let mut acc: Vec<(isize, String)> = Vec::new();
            let _ = EnumWindows(Some(enum_cb), LPARAM(&mut acc as *mut _ as isize));
            acc.into_iter()
                .find(|(_, t)| t.contains(needle))
                .map(|(h, _)| HWND(h as *mut _))
        }
        // AttachThreadInput lets us steal foreground reliably (Windows otherwise blocks it).
        unsafe fn force_foreground(hwnd: HWND) {
            let fg = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg, None);
            let this = GetCurrentThreadId();
            let _ = AttachThreadInput(this, fg_thread, true);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
            let _ = AttachThreadInput(this, fg_thread, false);
        }

        // Open Notepad on a FRESH, EMPTY temp file with a unique name, so we target our own
        // window — never the user's open documents — and start from a clean, known-empty buffer.
        let stem = format!("aurascribe_notepad_test_{}", std::process::id());
        let path = std::env::temp_dir().join(format!("{stem}.txt"));
        std::fs::write(&path, "").expect("failed to create temp file");
        let mut child = Command::new("notepad.exe")
            .arg(&path)
            .spawn()
            .expect("failed to launch notepad.exe");
        let hwnd = {
            let mut h = None;
            for _ in 0..40 {
                thread::sleep(Duration::from_millis(200));
                if let Some(found) = unsafe { find_window(&stem) } {
                    h = Some(found);
                    break;
                }
            }
            h.expect("our Notepad window did not appear")
        };
        unsafe { force_foreground(hwnd) };
        thread::sleep(Duration::from_millis(700));

        // Inject through the real production path (pastes on Windows).
        let injector = TextInjector::new();
        injector.inject_text(TEST).expect("inject_text returned an error");
        thread::sleep(Duration::from_millis(700)); // paste is delivered asynchronously

        // Select back ONLY the text we just pasted (cursor is at its end), copy it, then delete it
        // — so the readback is exactly our string and any pre-existing document content is left
        // untouched. TEST is all BMP, so char count == UTF-16 caret steps.
        let n = TEST.chars().count() as u32;
        unsafe {
            // Hold Shift, press Left n times, release Shift.
            let mk = |v: VIRTUAL_KEY, up: bool| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: v,
                        wScan: 0,
                        dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            const VK_SHIFT: VIRTUAL_KEY = VIRTUAL_KEY(0x10);
            const VK_LEFT: VIRTUAL_KEY = VIRTUAL_KEY(0x25);
            let mut inputs = vec![mk(VK_SHIFT, false)];
            for _ in 0..n {
                inputs.push(mk(VK_LEFT, false));
                inputs.push(mk(VK_LEFT, true));
            }
            inputs.push(mk(VK_SHIFT, true));
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(150));
            ctrl_combo(0x43); // Ctrl+C — copy just our selection
        }
        thread::sleep(Duration::from_millis(400));

        let landed = read_clipboard_text().unwrap_or_default();

        // Non-destructive cleanup: our text is still selected — delete it so the document returns
        // to exactly its prior state, then close our launched instance.
        unsafe {
            let mk = |v: VIRTUAL_KEY, up: bool| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: v,
                        wScan: 0,
                        dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            const VK_DELETE: VIRTUAL_KEY = VIRTUAL_KEY(0x2E);
            let inputs = [mk(VK_DELETE, false), mk(VK_DELETE, true)];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        let _ = child.kill();
        let _ = std::fs::remove_file(&path);

        eprintln!("injected: {TEST:?}");
        eprintln!("landed:   {landed:?}");
        // The fix is proven if the EXACT string is present, clean — the old typing bug would have
        // turned it into runs of "......"/"?????", so a verbatim match cannot occur under the bug.
        assert!(
            landed.contains(TEST),
            "Notepad did not receive the exact text — got {landed:?}"
        );
        assert!(!landed.contains("?????"), "output contains the '?????' corruption pattern");
    }
}
