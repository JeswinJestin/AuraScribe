//! Local, deterministic cleanup pass for raw Whisper output.
//!
//! Pure string processing (no network, no LLM) so it stays effectively free
//! relative to transcription latency — this is what lets AuraScribe hit
//! sub-second turnaround instead of the multi-second cloud round trip.

const FILLERS: &[&str] = &[
    "um", "uh", "erm", "uhh", "umm", "hmm",
    "like", "you know", "i mean", "sort of", "kind of",
    "actually", "basically", "literally",
];

#[derive(Debug, Clone, Copy)]
pub struct CleanupOptions {
    pub remove_fillers: bool,
    pub auto_punctuation: bool,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            remove_fillers: true,
            auto_punctuation: true,
        }
    }
}

pub fn clean(text: &str, options: CleanupOptions) -> String {
    // Whisper narrates non-speech audio as bracketed annotations ("[Music]",
    // "[BLANK_AUDIO]", "(upbeat music)"). Typing those into the user's document is
    // never what they meant, so drop them before anything else.
    let mut cleaned = strip_non_speech_annotations(text);

    if cleaned.is_empty() {
        return cleaned;
    }

    if options.remove_fillers {
        cleaned = strip_fillers(&cleaned);
    }

    if options.auto_punctuation {
        cleaned = normalize_punctuation(&cleaned);
        cleaned = capitalize_sentences(&cleaned);
    }

    collapse_whitespace(&cleaned)
}

/// Removes `[...]` / `(...)` non-speech annotations that Whisper emits for silence, music
/// and background noise. Parenthesised spans are only dropped when they look like an audio
/// description, so ordinary dictated asides survive.
fn strip_non_speech_annotations(text: &str) -> String {
    const AUDIO_WORDS: &[&str] = &[
        "music", "audio", "silence", "blank", "noise", "sound", "sounds", "laughter",
        "applause", "chatter", "typing", "coughing", "sighs", "inaudible", "indistinct",
        "background", "beep", "static", "clicking", "breathing", "whispering",
    ];

    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        let closing = match c {
            '[' => Some(']'),
            '(' => Some(')'),
            _ => None,
        };

        let Some(close) = closing else {
            out.push(c);
            continue;
        };

        let mut inner = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == close {
                closed = true;
                break;
            }
            inner.push(c);
        }

        if !closed {
            // Unbalanced — keep the text as the user dictated it.
            out.push(c);
            out.push_str(&inner);
            continue;
        }

        let lower = inner.to_lowercase();
        let looks_like_audio = c == '['
            || AUDIO_WORDS
                .iter()
                .any(|w| lower.split(|ch: char| !ch.is_alphanumeric()).any(|t| t == *w));

        if !looks_like_audio {
            out.push(c);
            out.push_str(&inner);
            out.push(close);
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_fillers(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().collect();

    // Remove multi-word fillers first ("you know", "i mean", "sort of", "kind of")
    let multi_word: Vec<&str> = FILLERS.iter().copied().filter(|f| f.contains(' ')).collect();
    let mut joined = words.join(" ");
    for phrase in &multi_word {
        joined = replace_word_boundary(&joined, phrase, "");
    }
    words = joined.split_whitespace().collect();

    let single_word: std::collections::HashSet<&str> = FILLERS
        .iter()
        .copied()
        .filter(|f| !f.contains(' '))
        .collect();

    words
        .into_iter()
        .filter(|w| {
            let bare = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            !single_word.contains(bare.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn replace_word_boundary(text: &str, phrase: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut result = String::new();
    let mut last = 0;
    let mut search_start = 0;

    while let Some(pos) = lower[search_start..].find(&phrase_lower) {
        let start = search_start + pos;
        let end = start + phrase_lower.len();

        let before_ok = start == 0
            || !lower.as_bytes()[start - 1].is_ascii_alphanumeric();
        let after_ok = end == lower.len()
            || !lower.as_bytes()[end].is_ascii_alphanumeric();

        if before_ok && after_ok {
            result.push_str(&text[last..start]);
            result.push_str(replacement);
            last = end;
        }
        search_start = end.max(start + 1);
    }
    result.push_str(&text[last..]);
    result
}

fn normalize_punctuation(text: &str) -> String {
    const PUNCT: [char; 6] = ['.', '?', '!', ',', ';', ':'];

    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if PUNCT.contains(&c) {
            // Speech-to-text often leaves a space before punctuation; drop it.
            while out.ends_with(' ') {
                out.pop();
            }
            out.push(c);
            // Collapse runs of the same mark ("..." -> ".").
            while i + 1 < chars.len() && chars[i + 1] == c {
                i += 1;
            }
        } else if c.is_whitespace() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(c);
        }

        i += 1;
    }

    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        return trimmed;
    }
    if !trimmed.ends_with(['.', '?', '!']) {
        format!("{}.", trimmed)
    } else {
        trimmed
    }
}

fn capitalize_sentences(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for c in text.chars() {
        if capitalize_next && c.is_alphabetic() {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
            if c == '.' || c == '?' || c == '!' {
                capitalize_next = true;
            } else if !c.is_whitespace() {
                capitalize_next = false;
            }
        }
    }

    capitalize_standalone_i(&result)
}

fn capitalize_standalone_i(text: &str) -> String {
    let mut words: Vec<String> = text.split(' ').map(|w| w.to_string()).collect();
    for word in &mut words {
        let core: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        if core == "i" {
            *word = word.replacen('i', "I", 1);
        }
    }
    words.join(" ")
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fillers_and_punctuates() {
        let opts = CleanupOptions::default();
        let out = clean(
            "um so i think we should like maybe schedule the meeting for next tuesday",
            opts,
        );
        assert!(!out.to_lowercase().contains(" um "));
        assert!(out.starts_with('S') || out.starts_with("I"));
        assert!(out.ends_with('.'));
    }

    #[test]
    fn capitalizes_standalone_i() {
        let opts = CleanupOptions::default();
        let out = clean("i think i am ready", opts);
        assert!(out.contains("I "));
    }

    #[test]
    fn leaves_empty_text_alone() {
        assert_eq!(clean("", CleanupOptions::default()), "");
    }

    #[test]
    fn strips_space_before_punctuation() {
        let out = clean("i went to the store . then i bought milk", CleanupOptions::default());
        assert!(!out.contains(" ."), "unexpected space before period in {out:?}");
        assert_eq!(out, "I went to the store. Then I bought milk.");
    }

    #[test]
    fn collapses_repeated_marks() {
        let out = clean("wait!!! really???", CleanupOptions::default());
        assert_eq!(out, "Wait! Really?");
    }

    #[test]
    fn strips_whisper_non_speech_annotations() {
        let opts = CleanupOptions::default();
        // Real output captured from a live session.
        assert_eq!(clean("[Music] [Music] [Music]", opts), "");
        assert_eq!(clean("Hi, hello. [BLANK_AUDIO]", opts), "Hi, hello.");
        let out = clean("[typing sounds] [indistinct chatter] Hi, so it works", opts);
        assert_eq!(out, "Hi, so it works.");
    }

    #[test]
    fn strips_parenthesised_audio_descriptions_only() {
        let opts = CleanupOptions::default();
        assert_eq!(clean("hello (upbeat music) world", opts), "Hello world.");
        // A genuine dictated aside must survive.
        let out = clean("the total (roughly ten) is fine", opts);
        assert!(out.contains("(roughly ten)"), "lost a real aside: {out:?}");
    }

    #[test]
    fn keeps_text_when_fillers_disabled() {
        let out = clean(
            "um i think so",
            CleanupOptions { remove_fillers: false, auto_punctuation: true },
        );
        assert!(out.to_lowercase().contains("um"));
    }

    /// Prints real before/after pairs so cleanup quality can be eyeballed:
    /// `cargo test -- --nocapture show_cleanup_samples`
    #[test]
    fn show_cleanup_samples() {
        let samples = [
            "um so i think we should like maybe schedule the meeting for next tuesday if that works for everyone",
            "hey team quick update the deploy failed because of a timeout error in the ci pipeline",
            "uh basically i mean the thing is you know it just doesn't work",
            "i went to the store . then i bought milk",
            "what do you think    is this actually working",
        ];
        for s in samples {
            println!("RAW  : {}", s);
            println!("CLEAN: {}\n", clean(s, CleanupOptions::default()));
        }
    }
}
