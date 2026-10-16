//! Local **extractive** meeting summary — no model, no cloud, no network.
//!
//! Wispr-style summaries are cloud LLM calls; a local abstractive LLM is multi-GB (breaks
//! AuraScribe's lightweight/no-cloud non-negotiables). So the shippable summary is **extractive**:
//! it scores the transcript's own sentences by how many *significant* words they carry (a classic
//! Luhn/TF approach) and returns the top few in their original order, plus the salient keywords.
//! It is deterministic, dependency-free, instant, and unit-testable — and it cannot invent facts
//! that weren't said (no hallucination), which matters for a meeting record.
//!
//! A richer **abstractive** summary (fluent prose / action items) is a later opt-in via the local
//! `prompt` model (Qwen) — never bundled, never cloud. This module is the always-available default.

/// A local summary of a transcript.
#[derive(Clone, serde::Serialize)]
pub struct Summary {
    /// The most information-dense sentences, in the order they were said.
    pub key_points: Vec<String>,
    /// The most salient content words across the transcript.
    pub keywords: Vec<String>,
}

/// Very common words that carry little topic signal; excluded from scoring and keywords. Small on
/// purpose — a big stoplist is a liability, and the frequency weighting already de-emphasises the
/// rest.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "any", "can", "had", "her", "was",
    "one", "our", "out", "day", "get", "has", "him", "his", "how", "man", "new", "now", "old",
    "see", "two", "way", "who", "boy", "did", "its", "let", "put", "say", "she", "too", "use",
    "that", "this", "with", "have", "from", "they", "will", "would", "there", "their", "what",
    "about", "which", "when", "your", "them", "then", "than", "into", "just", "like", "also",
    "been", "were", "some", "could", "should", "because", "really", "actually", "going", "gonna",
    "okay", "yeah", "well", "know", "think", "want", "thing", "things", "something", "anything",
    "it's", "i'm", "we're", "that's", "don't", "doesn't", "let's", "i've",
];

/// Produce an extractive summary: up to `max_points` key sentences + up to 8 keywords.
pub fn summarize(transcript: &str, max_points: usize) -> Summary {
    let sentences = split_sentences(transcript);
    if sentences.is_empty() || max_points == 0 {
        return Summary { key_points: Vec::new(), keywords: Vec::new() };
    }

    // Word-frequency table over significant (content) words across the whole transcript.
    let mut freq: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for s in &sentences {
        for w in content_words(s) {
            *freq.entry(w).or_insert(0) += 1;
        }
    }
    if freq.is_empty() {
        // No content words (e.g. all stopwords/filler): fall back to the first sentences.
        let key_points = sentences.iter().take(max_points).cloned().collect();
        return Summary { key_points, keywords: Vec::new() };
    }

    // Score each sentence by the average frequency of its content words, so a long sentence padded
    // with filler doesn't automatically win over a short, dense one.
    let mut scored: Vec<(usize, f64, &String)> = sentences
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let words = content_words(s);
            let score = if words.is_empty() {
                0.0
            } else {
                let total: u32 = words.iter().map(|w| freq.get(w).copied().unwrap_or(0)).sum();
                total as f64 / words.len() as f64
            };
            (i, score, s)
        })
        .collect();

    // Pick the top-scoring sentences, then restore their original order so the summary reads in
    // sequence. Ties broken by earlier position (stable).
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    let mut chosen: Vec<(usize, &String)> =
        scored.into_iter().take(max_points).map(|(i, _, s)| (i, s)).collect();
    chosen.sort_by_key(|(i, _)| *i);
    let key_points = chosen.into_iter().map(|(_, s)| s.clone()).collect();

    // Keywords: the highest-frequency content words (ties broken alphabetically for determinism).
    let mut pairs: Vec<(&String, &u32)> = freq.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let keywords = pairs.into_iter().take(8).map(|(w, _)| w.clone()).collect();

    Summary { key_points, keywords }
}

/// Split text into sentences on `.`/`?`/`!`, trimming and dropping empties. Good enough for speech
/// transcripts, which rarely contain the abbreviations that trip naive splitters.
fn split_sentences(text: &str) -> Vec<String> {
    text.split(|c| c == '.' || c == '?' || c == '!')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// The significant (content) words of a sentence: lowercased, ≥3 letters, alphabetic, not a stopword.
fn content_words(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .map(|w| w.trim_matches('\'').to_lowercase())
        .filter(|w| w.chars().count() >= 3 && w.chars().all(|c| c.is_alphabetic() || c == '\''))
        .filter(|w| !STOPWORDS.contains(&w.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_densest_sentence_and_keywords() {
        let t = "We talked about pricing. The pricing model is the key decision for pricing this \
                 quarter. Lunch was fine.";
        let s = summarize(t, 1);
        assert_eq!(s.key_points.len(), 1);
        // The pricing-heavy sentence should win over the lunch aside.
        assert!(s.key_points[0].to_lowercase().contains("pricing"));
        assert!(s.keywords.contains(&"pricing".to_string()));
    }

    #[test]
    fn preserves_original_order_of_chosen_sentences() {
        let t = "Budget is the first topic and the budget matters. Then we covered hiring and \
                 hiring plans for hiring. A quick note at the end.";
        let s = summarize(t, 2);
        assert_eq!(s.key_points.len(), 2);
        // "Budget" sentence comes before the "hiring" sentence in the output.
        let joined = s.key_points.join(" | ");
        let bi = joined.find("Budget").unwrap();
        let hi = joined.find("hiring").unwrap();
        assert!(bi < hi, "chosen sentences should keep transcript order");
    }

    #[test]
    fn respects_max_points_and_handles_empty() {
        assert!(summarize("", 3).key_points.is_empty());
        assert!(summarize("Anything here.", 0).key_points.is_empty());
        let t = "One. Two. Three. Four. Five.";
        assert!(summarize(t, 2).key_points.len() <= 2);
    }

    #[test]
    fn short_transcript_returns_what_there_is() {
        let s = summarize("We shipped the release today.", 5);
        assert_eq!(s.key_points.len(), 1);
    }
}
