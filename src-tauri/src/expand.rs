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
///
/// When `fuzzy` is on, an extra conservative pass (`fuzzy_correct`) also fixes **near-miss**
/// mishears of the user's own dictionary terms — e.g. "cubernetties" → "Kubernetes" — not just
/// exact matches. It is opt-in because fuzzy matching can, in principle, rewrite an unrelated word
/// that happens to be close to a dictionary term; off by default, normal dictation is untouched.
pub fn apply(
    text: &str,
    dictionary: &[DictionaryRow],
    snippets: &[SnippetRow],
    fuzzy: bool,
) -> String {
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

    // Catch near-misses of the user's own vocabulary that the exact pass above left behind.
    if fuzzy {
        out = fuzzy_correct(&out, dictionary);
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

/// Maximum edit distance allowed for a near-miss, by the dictionary term's length. Tight on short
/// words (collisions are cheap there) and never above 2, so a correction is always a *small* fix.
fn fuzzy_threshold(term_len: usize) -> usize {
    match term_len {
        0..=4 => 0, // too short to fuzzy-match safely — exact pass only
        5..=7 => 1,
        _ => 2,
    }
}

/// Conservative near-miss correction toward the user's own dictionary terms.
///
/// For each alphabetic token the exact pass didn't already fix, find eligible dictionary terms
/// (whole-word, case-insensitive, ≥5 chars, alphabetic) whose lowercased form is within
/// `fuzzy_threshold` edits of the token, with the **same first letter** and a length within ±2.
/// The token is replaced only when exactly one *distinct* replacement wins — ambiguous or tied
/// matches are left alone, so a wrong guess never silently overwrites the user's words.
fn fuzzy_correct(text: &str, dictionary: &[DictionaryRow]) -> String {
    let eligible: Vec<(String, &str)> = dictionary
        .iter()
        .filter(|e| e.whole_word != 0 && e.case_sensitive == 0)
        .map(|e| (e.word.to_lowercase(), e.replacement.as_str()))
        .filter(|(w, _)| w.chars().count() >= 5 && w.chars().all(|c| c.is_alphabetic()))
        .collect();
    if eligible.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut token = String::new();

    // Flush the current alphabetic token, correcting it if a single clear near-miss match exists.
    let flush = |token: &mut String, out: &mut String| {
        if token.is_empty() {
            return;
        }
        let lower = token.to_lowercase();
        let tlen = lower.chars().count();
        let mut best: Option<(usize, &str)> = None; // (distance, replacement)
        let mut tied = false;
        for (word, replacement) in &eligible {
            let wlen = word.chars().count();
            if lower == *word {
                // Exact (e.g. replacement equals term, or already-correct) — leave as typed.
                best = None;
                tied = false;
                break;
            }
            if word.chars().next() != lower.chars().next() {
                continue;
            }
            let max = fuzzy_threshold(wlen);
            if max == 0 || tlen.abs_diff(wlen) > 2 {
                continue;
            }
            if let Some(d) = levenshtein_bounded(&lower, word, max) {
                match best {
                    None => best = Some((d, replacement)),
                    Some((bd, br)) => {
                        if d < bd {
                            best = Some((d, replacement));
                            tied = false;
                        } else if d == bd && br != *replacement {
                            tied = true; // two different terms equally close — don't guess
                        }
                    }
                }
            }
        }
        match best {
            Some((_, replacement)) if !tied => out.push_str(replacement),
            _ => out.push_str(token),
        }
        token.clear();
    };

    for ch in text.chars() {
        if ch.is_alphabetic() {
            token.push(ch);
        } else {
            flush(&mut token, &mut out);
            out.push(ch);
        }
    }
    flush(&mut token, &mut out);
    out
}

/// Levenshtein distance, early-exiting once it provably exceeds `max` (returns `None` then). Keeps
/// the fuzzy pass cheap: it only ever runs the full DP on tokens already close in length.
pub(crate) fn levenshtein_bounded(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        let mut row_min = curr[0];
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
            row_min = row_min.min(curr[j + 1]);
        }
        if row_min > max {
            return None; // whole row already past the budget — no path can recover
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    let d = prev[b.len()];
    if d <= max {
        Some(d)
    } else {
        None
    }
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
            apply("We deployed kubernetes today.", &d, &[], false),
            "We deployed Kubernetes today."
        );
    }

    #[test]
    fn dictionary_whole_word_does_not_match_inside_another_word() {
        let d = vec![dict("cs", "CS", false, true)];
        // Must not turn "physics" into "physiCS".
        assert_eq!(apply("I study physics and cs.", &d, &[], false), "I study physics and CS.");
    }

    #[test]
    fn snippet_expands_a_spoken_phrase() {
        let s = vec![snip("my email", "jeswin@example.com")];
        // Case-insensitive whole-phrase match, wherever it appears in the sentence.
        assert_eq!(
            apply("Reach me at my email please.", &[], &s, false),
            "Reach me at jeswin@example.com please."
        );
    }

    #[test]
    fn snippet_expands_every_occurrence() {
        let s = vec![snip("my email", "jeswin@example.com")];
        assert_eq!(
            apply("My email is my email.", &[], &s, false),
            "jeswin@example.com is jeswin@example.com."
        );
    }

    #[test]
    fn longer_snippet_trigger_wins() {
        let s = vec![
            snip("my email", "short@example.com"),
            snip("my work email", "work@example.com"),
        ];
        assert_eq!(apply("Send my work email please.", &[], &s, false), "Send work@example.com please.");
    }

    #[test]
    fn snippet_expansion_is_inserted_verbatim_not_re_corrected() {
        // A dictionary rule for "google" must not rewrite the inside of a snippet expansion.
        let d = vec![dict("google", "Google", false, true)];
        let s = vec![snip("my site", "google.com")];
        assert_eq!(apply("Visit my site.", &d, &s, false), "Visit google.com.");
    }

    #[test]
    fn case_sensitive_dictionary_only_matches_exact_case() {
        let d = vec![dict("IT", "information technology", true, true)];
        // "it" (lowercase) is left alone; only the exact-case "IT" is expanded.
        assert_eq!(
            apply("it is an IT problem.", &d, &[], false),
            "it is an information technology problem."
        );
    }

    #[test]
    fn empty_inputs_are_a_no_op() {
        assert_eq!(apply("nothing to do here.", &[], &[], false), "nothing to do here.");
    }

    // --- fuzzy near-miss correction (opt-in) -----------------------------------------------------

    #[test]
    fn fuzzy_corrects_a_near_miss_of_a_dictionary_term() {
        let d = vec![dict("kubernetes", "Kubernetes", false, true)];
        // One/two-character mishears of a long term get fixed only when fuzzy is on.
        assert_eq!(
            apply("we deployed kuberneties today.", &d, &[], true),
            "we deployed Kubernetes today."
        );
        // Off by default: the mishear is left untouched.
        assert_eq!(
            apply("we deployed kuberneties today.", &d, &[], false),
            "we deployed kuberneties today."
        );
    }

    #[test]
    fn fuzzy_preserves_punctuation_and_other_words() {
        let d = vec![dict("postgres", "Postgres", false, true)];
        assert_eq!(apply("use postgress, please.", &d, &[], true), "use Postgres, please.");
    }

    #[test]
    fn fuzzy_does_not_touch_far_words() {
        let d = vec![dict("kubernetes", "Kubernetes", false, true)];
        // "container" is nowhere near "kubernetes" — must be left alone.
        assert_eq!(apply("a container ran.", &d, &[], true), "a container ran.");
    }

    #[test]
    fn fuzzy_skips_short_terms() {
        // 4-char term: too short to fuzzy-match (threshold 0), so "reactx" is NOT turned into React.
        let d = vec![dict("sql", "SQL", false, true)];
        assert_eq!(apply("the sqm server.", &d, &[], true), "the sqm server.");
    }

    #[test]
    fn fuzzy_leaves_ambiguous_ties_alone() {
        // Two different replacements equally close (distance 1) to "fooo" → don't guess.
        let d = vec![
            dict("foob", "FooB", false, true),
            dict("fooc", "FooC", false, true),
        ];
        assert_eq!(apply("the fooo thing.", &d, &[], true), "the fooo thing.");
    }

    #[test]
    fn fuzzy_respects_distance_budget() {
        // "kubernetes" (len 10) allows up to 2 edits; a 3-edit mangle is left alone.
        let d = vec![dict("kubernetes", "Kubernetes", false, true)];
        assert_eq!(apply("kuberxyzes ran.", &d, &[], true), "kuberxyzes ran.");
    }
}
