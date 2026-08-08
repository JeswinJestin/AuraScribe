//! Personal dictionary + snippets, applied to a transcript before it is injected.
//!
//! This is the step that makes the Words and Snippets screens *do* something. It runs after
//! `cleanup::clean` and before injection, on the final text:
//!
//! 1. **Dictionary** — spoken-form corrections. "kubernetes" → "Kubernetes", "google" →
//!    "Google". Each entry may be case-sensitive and/or whole-word (defaults: insensitive
//!    match, whole word).
//! 2. **Snippets** — say a short trigger, insert a longer canned expansion. "my email" →
//!    "you@example.com". Triggers are matched case-insensitively as whole phrases.
//!
//! Order matters and is deliberate: **dictionary first, then snippets.** Dictionary entries are
//! single-word spelling fixes for what Whisper heard; running them first normalises the
//! transcript. Snippet *expansions* are inserted verbatim afterwards, so a dictionary rule can
//! never rewrite the inside of a canned block of text the user pasted in.
//!
//! All matching is boundary-aware: a rule for "cs" never fires inside "physics". Everything is
//! pure string processing — no network, no allocation-per-char hot loops that would show up
//! next to transcription latency.

use crate::db::{DictionaryRow, SnippetRow};

/// Apply the personal dictionary then snippet expansions to `text`.
pub fn apply(text: &str, dictionary: &[DictionaryRow], snippets: &[SnippetRow]) -> String {
    let mut out = text.to_string();

    for entry in dictionary {
        out = replace_phrase(
            &out,
            &entry.word,
            &entry.replacement,
            entry.case_sensitive != 0,
            entry.whole_word != 0,
        );
    }

    // Longer triggers first, so "my work email" wins over "my email" when both exist and
    // overlap. Snippet triggers are always whole-phrase and case-insensitive.
    let mut snips: Vec<&SnippetRow> = snippets.iter().collect();
    snips.sort_by_key(|s| std::cmp::Reverse(s.trigger.chars().count()));
    for snip in snips {
        out = replace_phrase(&out, &snip.trigger, &snip.expansion, false, true);
    }

    // Replacements can leave doubled spaces (e.g. an expansion that starts/ends blank); tidy.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Replace every boundary-respecting occurrence of `needle` in `haystack` with `replacement`.
///
/// - `case_sensitive` matches the literal case; otherwise ASCII-insensitively.
/// - `whole_word` requires a non-alphanumeric boundary (or string edge) on both sides, so
///   "cs" does not match inside "physics" and the phrase "my email" does not match inside
///   "clarify email". With it off, any substring occurrence is replaced.
///
/// Empty `needle` is a no-op (guards against an accidental infinite match).
fn replace_phrase(
    haystack: &str,
    needle: &str,
    replacement: &str,
    case_sensitive: bool,
    whole_word: bool,
) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }

    let hay_cmp = if case_sensitive {
        haystack.to_string()
    } else {
        haystack.to_lowercase()
    };
    let needle_cmp = if case_sensitive {
        needle.to_string()
    } else {
        needle.to_lowercase()
    };

    let bytes = hay_cmp.as_bytes();
    let mut result = String::with_capacity(haystack.len());
    let mut last = 0usize;
    let mut search_start = 0usize;

    while let Some(rel) = hay_cmp[search_start..].find(&needle_cmp) {
        let start = search_start + rel;
        let end = start + needle_cmp.len();

        let boundary_ok = !whole_word
            || ((start == 0 || !bytes[start - 1].is_ascii_alphanumeric())
                && (end == bytes.len() || !bytes[end].is_ascii_alphanumeric()));

        if boundary_ok {
            result.push_str(&haystack[last..start]);
            result.push_str(replacement);
            last = end;
        }
        // Advance past this match (at least one char) so overlapping matches still progress.
        search_start = end.max(start + 1);
        if search_start > bytes.len() {
            break;
        }
    }
    result.push_str(&haystack[last..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(word: &str, replacement: &str, cs: bool, ww: bool) -> DictionaryRow {
        DictionaryRow {
            id: 0,
            word: word.into(),
            replacement: replacement.into(),
            case_sensitive: cs as i32,
            whole_word: ww as i32,
            created_at: 0,
        }
    }

    fn snip(trigger: &str, expansion: &str) -> SnippetRow {
        SnippetRow {
            id: 0,
            trigger: trigger.into(),
            expansion: expansion.into(),
            description: None,
            created_at: 0,
        }
    }

    #[test]
    fn dictionary_corrects_a_whole_word_case_insensitively() {
        let d = vec![dict("kubernetes", "Kubernetes", false, true)];
        assert_eq!(
            apply("We deployed kubernetes today.", &d, &[]),
            "We deployed Kubernetes today."
        );
    }

    #[test]
    fn dictionary_whole_word_does_not_match_inside_another_word() {
        let d = vec![dict("cs", "CS", false, true)];
        // Must not turn "physics" into "physiCS".
        assert_eq!(apply("I study physics and cs.", &d, &[]), "I study physics and CS.");
    }

    #[test]
    fn snippet_expands_a_spoken_phrase() {
        let s = vec![snip("my email", "jeswin@example.com")];
        // Case-insensitive whole-phrase match, wherever it appears in the sentence.
        assert_eq!(
            apply("Reach me at my email please.", &[], &s),
            "Reach me at jeswin@example.com please."
        );
    }

    #[test]
    fn snippet_expands_every_occurrence() {
        let s = vec![snip("my email", "jeswin@example.com")];
        assert_eq!(
            apply("My email is my email.", &[], &s),
            "jeswin@example.com is jeswin@example.com."
        );
    }

    #[test]
    fn longer_snippet_trigger_wins() {
        let s = vec![
            snip("my email", "short@example.com"),
            snip("my work email", "work@example.com"),
        ];
        assert_eq!(apply("Send my work email please.", &[], &s), "Send work@example.com please.");
    }

    #[test]
    fn snippet_expansion_is_inserted_verbatim_not_re_corrected() {
        // A dictionary rule for "google" must not rewrite the inside of a snippet expansion.
        let d = vec![dict("google", "Google", false, true)];
        let s = vec![snip("my site", "google.com")];
        assert_eq!(apply("Visit my site.", &d, &s), "Visit google.com.");
    }

    #[test]
    fn case_sensitive_dictionary_only_matches_exact_case() {
        let d = vec![dict("IT", "information technology", true, true)];
        // "it" (lowercase) is left alone; only the exact-case "IT" is expanded.
        assert_eq!(
            apply("it is an IT problem.", &d, &[]),
            "it is an information technology problem."
        );
    }

    #[test]
    fn empty_inputs_are_a_no_op() {
        assert_eq!(apply("nothing to do here.", &[], &[]), "nothing to do here.");
    }
}
