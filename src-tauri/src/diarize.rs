//! Speaker intelligence for the Notetaker — **all on-device, no cloud**.
//!
//! Three pure, unit-tested pieces that make a meeting transcript understand *who* is speaking and,
//! once the conversation makes it clear, *their name* — updated live while the meeting runs. See
//! `docs/superpowers/specs/2026-10-16-notetaker-speaker-intelligence-design.md`.
//!
//! - **Layer A — [`attribute_channel`]:** we capture the mic and the system/loopback audio in two
//!   separate buffers, so "You" vs "a guest" falls straight out of which channel carries the energy.
//!   Near-perfect on headphones, and free — no model, no guessing.
//! - **Layer B — [`OnlineClusterer`]:** online voice-fingerprint clustering (cosine centroids +
//!   periodic agglomerative self-correction) that splits the guests into Speaker 1/2/3…. The maths
//!   is here and fully testable; the actual embedding extraction is done elsewhere on-device.
//! - **Layer C — [`resolve_speaker_names`]:** reads the transcript for how people address and
//!   introduce each other ("thanks, Teresa", "Lenny, tell me…", "I'm Lenny") and, only when the
//!   evidence is strong and unconflicted, attaches a real name to a speaker's cluster. No guesses.
//!
//! None of this is copied from any competitor: the channel split, the clustering, and the naming
//! heuristics are our own, written from the behaviour we want and documented in the design.
//!
//! NOTE: this lands as a tested, standalone engine first; the next commit wires it into the live
//! meeting pipeline (`meeting.rs` + DB + UI), at which point this allow is removed.
#![allow(dead_code)]

use std::collections::HashMap;

// ===== Layer A: channel attribution ("You" vs a guest) ==========================================

/// The special cluster id for the local user (mic). Guests are 1, 2, 3, …; `-1` is "unknown".
pub const SELF_CLUSTER: i64 = 0;

/// Who a captured window belongs to, decided purely from the two channels' energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Mic-dominated — the local user ("You").
    Self_,
    /// Loopback-dominated — someone else in the call.
    Guest,
    /// Both channels carry real energy (cross-talk, or the user is on speakers not headphones).
    /// Attributed to the louder side, but flagged so diarization can down-weight it.
    Mixed,
}

/// Root-mean-square amplitude of a window — our cheap "how loud is this channel" measure.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    ((sum_sq / samples.len() as f64).sqrt()) as f32
}

/// Below this RMS a channel is treated as silence, so faint bleed never counts as speech.
const SILENCE_FLOOR: f32 = 0.004;
/// One channel must be this many times louder than the other to "own" the window outright;
/// within this ratio both are considered active (→ [`Channel::Mixed`]).
const DOMINANCE_RATIO: f32 = 2.0;

/// Decide whether a window is the user, a guest, or mixed, from each channel's RMS energy.
pub fn attribute_channel(mic_rms: f32, sys_rms: f32) -> Channel {
    let mic_on = mic_rms >= SILENCE_FLOOR;
    let sys_on = sys_rms >= SILENCE_FLOOR;
    match (mic_on, sys_on) {
        (true, false) => Channel::Self_,
        (false, true) => Channel::Guest,
        (false, false) => Channel::Guest, // silent window: default to the meeting side, not "You"
        (true, true) => {
            if mic_rms >= sys_rms * DOMINANCE_RATIO {
                Channel::Self_
            } else if sys_rms >= mic_rms * DOMINANCE_RATIO {
                Channel::Guest
            } else {
                Channel::Mixed
            }
        }
    }
}

// ===== Layer B: online guest diarization (clustering maths) =====================================

/// Cosine similarity of two equal-length vectors, in `[-1, 1]`. Returns 0 for a degenerate input.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..a.len() {
        dot += a[i] as f64 * b[i] as f64;
        na += a[i] as f64 * a[i] as f64;
        nb += b[i] as f64 * b[i] as f64;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())) as f32
}

/// A running guest cluster: a canonical id, the mean of its embeddings (centroid), and how many
/// windows it has absorbed (for the running average).
#[derive(Debug, Clone)]
struct Cluster {
    id: i64,
    centroid: Vec<f32>,
    count: u32,
}

/// Online voice-fingerprint clustering for the **guest** stream. Assigns each incoming embedding to
/// an existing speaker or opens a new one, and can re-cluster everything it has seen to fix drift.
///
/// Guests get ids 1, 2, 3, … (`SELF_CLUSTER` 0 is reserved for the mic). The embeddings are kept so
/// the periodic [`Self::recluster`] pass can re-derive stable clusters, exactly the "it self-corrects
/// mid-meeting" behaviour from the design.
#[derive(Debug, Clone)]
pub struct OnlineClusterer {
    clusters: Vec<Cluster>,
    /// Every (embedding, assigned id) seen so far — the material for re-clustering.
    history: Vec<(Vec<f32>, i64)>,
    next_id: i64,
    join_threshold: f32,
}

impl OnlineClusterer {
    /// `join_threshold`: minimum cosine similarity to an existing centroid to join it rather than
    /// start a new speaker. ~0.5–0.7 for typical speaker embeddings; tuned on-device.
    pub fn new(join_threshold: f32) -> Self {
        Self {
            clusters: Vec::new(),
            history: Vec::new(),
            next_id: SELF_CLUSTER + 1,
            join_threshold,
        }
    }

    /// Number of distinct guest speakers seen so far.
    pub fn speaker_count(&self) -> usize {
        self.clusters.len()
    }

    /// Assign one guest embedding, returning the speaker id it belongs to (joining the closest
    /// cluster above the threshold, or opening a new speaker).
    pub fn assign(&mut self, embedding: &[f32]) -> i64 {
        let best = self
            .clusters
            .iter()
            .map(|c| (c.id, cosine_similarity(&c.centroid, embedding)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let id = match best {
            Some((id, sim)) if sim >= self.join_threshold => {
                let c = self.clusters.iter_mut().find(|c| c.id == id).unwrap();
                update_centroid(&mut c.centroid, embedding, c.count);
                c.count += 1;
                id
            }
            _ => {
                let id = self.next_id;
                self.next_id += 1;
                self.clusters.push(Cluster {
                    id,
                    centroid: embedding.to_vec(),
                    count: 1,
                });
                id
            }
        };
        self.history.push((embedding.to_vec(), id));
        id
    }

    /// Re-cluster everything seen so far with average-linkage agglomerative clustering (cosine),
    /// merging clusters whose centroids are within `merge_threshold`. Returns a map from the *old*
    /// id each historical window had to the *new* canonical id, so callers can relabel stored
    /// segments. Ids are renumbered 1..=k in order of first appearance for a stable display order.
    pub fn recluster(&mut self, merge_threshold: f32) -> HashMap<i64, i64> {
        if self.history.is_empty() {
            return HashMap::new();
        }
        // Start with every window as its own group, then greedily merge the two most-similar groups
        // until nothing is close enough. Small N (windows per meeting) keeps this cheap.
        let mut groups: Vec<Vec<usize>> = (0..self.history.len()).map(|i| vec![i]).collect();

        loop {
            let mut best: Option<(usize, usize, f32)> = None;
            for i in 0..groups.len() {
                for j in (i + 1)..groups.len() {
                    let ci = group_centroid(&self.history, &groups[i]);
                    let cj = group_centroid(&self.history, &groups[j]);
                    let sim = cosine_similarity(&ci, &cj);
                    if best.map_or(true, |(_, _, b)| sim > b) {
                        best = Some((i, j, sim));
                    }
                }
            }
            match best {
                Some((i, j, sim)) if sim >= merge_threshold => {
                    let moved = groups.remove(j);
                    groups[i].extend(moved);
                }
                _ => break,
            }
        }

        // Renumber groups 1..=k by the earliest window index they contain (stable order), and build
        // the old-id -> new-id remap plus the rebuilt cluster centroids.
        groups.sort_by_key(|g| *g.iter().min().unwrap());
        let mut remap: HashMap<i64, i64> = HashMap::new();
        let mut new_clusters = Vec::new();
        for (gi, group) in groups.iter().enumerate() {
            let new_id = SELF_CLUSTER + 1 + gi as i64;
            let centroid = group_centroid(&self.history, group);
            for &idx in group {
                let old_id = self.history[idx].1;
                remap.entry(old_id).or_insert(new_id);
            }
            new_clusters.push(Cluster {
                id: new_id,
                centroid,
                count: group.len() as u32,
            });
        }
        // Apply the remap to history and swap in the rebuilt clusters.
        for h in self.history.iter_mut() {
            if let Some(&nid) = remap.get(&h.1) {
                h.1 = nid;
            }
        }
        self.next_id = SELF_CLUSTER + 1 + new_clusters.len() as i64;
        self.clusters = new_clusters;
        remap
    }
}

/// Fold a new sample into a running-mean centroid of `count` prior samples.
fn update_centroid(centroid: &mut [f32], sample: &[f32], count: u32) {
    let n = count as f32;
    for i in 0..centroid.len().min(sample.len()) {
        centroid[i] = (centroid[i] * n + sample[i]) / (n + 1.0);
    }
}

/// Mean vector of the history rows in `idxs`.
fn group_centroid(history: &[(Vec<f32>, i64)], idxs: &[usize]) -> Vec<f32> {
    let dim = history[idxs[0]].0.len();
    let mut acc = vec![0.0f32; dim];
    for &i in idxs {
        for d in 0..dim {
            acc[d] += history[i].0[d];
        }
    }
    let n = idxs.len() as f32;
    for d in 0..dim {
        acc[d] /= n;
    }
    acc
}

// ===== Layer C: online name resolution ==========================================================

/// One attributed line of transcript: which speaker cluster said it, and the words.
#[derive(Debug, Clone)]
pub struct Turn {
    pub cluster: i64,
    pub text: String,
}

/// How many consistent naming events a cluster needs before we commit a name to it. A single
/// self-introduction counts double (it is strong, first-person evidence).
const NAME_CONFIDENCE: u32 = 2;

/// Resolve real names for speaker clusters from how the conversation addresses and introduces
/// people. Returns only the clusters we are **confident** about — the rest stay "Speaker N".
///
/// Rules (all our own, derived from observed meeting behaviour):
/// - **Self-intro** ("I'm Lenny", "my name is …") names the *speaking* cluster.
/// - **Address / introduction** ("thanks, Teresa", "Lenny, tell me…", "Teresa is a speaker")
///   names the cluster being spoken *to* — the other active speaker.
/// - A name is committed only with >= [`NAME_CONFIDENCE`] consistent votes (or one self-intro),
///   and only if it clearly beats any competing name. Conflicted or one-off evidence → no name,
///   which is how we avoid the "Thanks, William" mislabel a single stray vocative would cause.
pub fn resolve_speaker_names(turns: &[Turn]) -> HashMap<i64, String> {
    // votes[cluster][normalized_name] = weight
    let mut votes: HashMap<i64, HashMap<String, u32>> = HashMap::new();
    let mut seq: Vec<i64> = Vec::new(); // clusters in turn order, to find the addressee

    for turn in turns {
        let speaker = turn.cluster;
        // Who they're talking to: the most recent earlier speaker who isn't them (robust to a run
        // of consecutive turns by the same speaker).
        let addressee = seq.iter().rev().find(|&&c| c != speaker).copied();
        collect_votes(&turn.text, speaker, addressee, &mut votes);
        seq.push(speaker);
    }

    // Commit the winning name per cluster, if it clears the confidence + margin gate.
    let mut resolved = HashMap::new();
    for (cluster, name_votes) in &votes {
        let mut ranked: Vec<(&String, u32)> = name_votes.iter().map(|(n, &v)| (n, v)).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        if let Some(&(name, top)) = ranked.first() {
            let runner = ranked.get(1).map(|&(_, v)| v).unwrap_or(0);
            if top >= NAME_CONFIDENCE && top > runner {
                resolved.insert(*cluster, name.clone());
            }
        }
    }
    resolved
}

/// Scan one line for naming events and add their votes. `speaker` said the line; `addressee` is the
/// cluster they are most likely talking to (the previous different speaker), if known.
fn collect_votes(
    text: &str,
    speaker: i64,
    addressee: Option<i64>,
    votes: &mut HashMap<i64, HashMap<String, u32>>,
) {
    let toks = tokenize(text);
    for (i, tok) in toks.iter().enumerate() {
        let word = tok.word.as_str();
        let lower = word.to_ascii_lowercase();

        // Self-introduction → names the *speaker*. Strong first-person evidence → weight 2.
        if matches!(lower.as_str(), "i'm" | "i’m")
            || (lower == "i" && next_word_is(&toks, i, "am"))
            || (lower == "name" && prev_word_is(&toks, i, "my") && next_word_is(&toks, i, "is"))
            || (lower == "this" && next_word_is(&toks, i, "is"))
        {
            if let Some(name) = first_candidate_from(&toks, i + 1) {
                add_vote(votes, speaker, &name, 2);
            }
            continue;
        }

        // Post-trigger address: "hey/hi/hello/welcome NAME", "thanks NAME", "thank you NAME" →
        // names the person being spoken to (the addressee).
        let is_thank = lower == "thanks" || (lower == "thank" && next_word_is(&toks, i, "you"));
        if matches!(lower.as_str(), "hey" | "hi" | "hello" | "welcome") || is_thank {
            let start = if lower == "thank" { i + 2 } else { i + 1 };
            if let (Some(name), Some(a)) = (candidate_at(&toks, start), addressee) {
                add_vote(votes, a, &name, 1);
            }
            continue;
        }

        if let Some(name) = candidate_word(word) {
            // Leading / pre-comma vocative: "NAME, …" at a sentence start, or right before an
            // address trigger ("NAME, thank you") → names the addressee. One vote per occurrence.
            let next_is_trigger = matches!(
                toks.get(i + 1).map(|t| t.word.to_ascii_lowercase()).as_deref(),
                Some("thank") | Some("thanks") | Some("welcome")
            );
            if tok.comma_after && (i == 0 || next_is_trigger) {
                if let Some(a) = addressee {
                    add_vote(votes, a, &name, 1);
                }
            }
        }

        // Introduction: a run of capitalized names ending right before "is a/an/the" → the first
        // name of the run (the given name), attributed to the introduced person (the addressee).
        if lower == "is"
            && matches!(
                toks.get(i + 1).map(|t| t.word.to_ascii_lowercase()).as_deref(),
                Some("a") | Some("an") | Some("the")
            )
        {
            if let (Some(name), Some(a)) = (run_start_before(&toks, i), addressee) {
                add_vote(votes, a, &name, 1);
            }
        }
    }
}

/// A tokenized word plus whether a comma immediately followed it (for "Name, thanks" detection).
struct Tok {
    word: String,
    comma_after: bool,
}

fn tokenize(text: &str) -> Vec<Tok> {
    text.split_whitespace()
        .map(|raw| {
            let comma_after = raw.ends_with(',') || raw.ends_with(",\"") || raw.ends_with(",'");
            let word = raw
                .trim_matches(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
                .to_string();
            Tok { word, comma_after }
        })
        .filter(|t| !t.word.is_empty())
        .collect()
}

fn next_word_is(toks: &[Tok], i: usize, w: &str) -> bool {
    toks.get(i + 1).map(|t| t.word.eq_ignore_ascii_case(w)).unwrap_or(false)
}
fn prev_word_is(toks: &[Tok], i: usize, w: &str) -> bool {
    i > 0 && toks[i - 1].word.eq_ignore_ascii_case(w)
}

/// A capitalized proper-noun candidate, normalized to Titlecase, or `None` if it's not name-like
/// (lowercase, too short/long, or a common non-name word).
fn candidate_word(word: &str) -> Option<String> {
    let w: String = word.chars().filter(|c| c.is_alphabetic() || *c == '\'' || *c == '’').collect();
    if w.len() < 2 || w.len() > 20 {
        return None;
    }
    let mut chars = w.chars();
    let first = chars.next()?;
    if !first.is_uppercase() {
        return None;
    }
    let lower = w.to_ascii_lowercase();
    if STOP_NAMES.contains(&lower.as_str()) {
        return None;
    }
    // Titlecase: first upper, rest lower.
    let normalized = {
        let mut s = String::new();
        s.push(first.to_ascii_uppercase());
        s.push_str(&w[first.len_utf8()..].to_ascii_lowercase());
        s
    };
    Some(normalized)
}

fn candidate_at(toks: &[Tok], i: usize) -> Option<String> {
    toks.get(i).and_then(|t| candidate_word(&t.word))
}

/// First candidate name at or after index `i`, scanning a couple of tokens (to skip a stray word).
fn first_candidate_from(toks: &[Tok], i: usize) -> Option<String> {
    for j in i..(i + 2).min(toks.len()) {
        if let Some(name) = candidate_word(&toks[j].word) {
            return Some(name);
        }
    }
    None
}

/// Walk back from `is_idx` over a run of candidate names and return the first (given name).
fn run_start_before(toks: &[Tok], is_idx: usize) -> Option<String> {
    if is_idx == 0 {
        return None;
    }
    let mut start = is_idx; // exclusive upper bound is is_idx
    while start > 0 && candidate_word(&toks[start - 1].word).is_some() {
        start -= 1;
    }
    if start < is_idx {
        candidate_word(&toks[start].word)
    } else {
        None
    }
}

fn add_vote(votes: &mut HashMap<i64, HashMap<String, u32>>, cluster: i64, name: &str, weight: u32) {
    let canon = canonical_name(votes, cluster, name);
    *votes.entry(cluster).or_default().entry(canon).or_insert(0) += weight;
}

/// Fold near-identical spellings together (Theresa≈Teresa) by matching against names this cluster
/// already has votes for, using the existing bounded Levenshtein helper.
fn canonical_name(votes: &HashMap<i64, HashMap<String, u32>>, cluster: i64, name: &str) -> String {
    if let Some(existing) = votes.get(&cluster) {
        for known in existing.keys() {
            let close = crate::expand::levenshtein_bounded(
                &known.to_ascii_lowercase(),
                &name.to_ascii_lowercase(),
                2,
            )
            .is_some();
            let same_initial = known.chars().next().map(|c| c.to_ascii_lowercase())
                == name.chars().next().map(|c| c.to_ascii_lowercase());
            if close && same_initial {
                return known.clone();
            }
        }
    }
    name.to_string()
}

/// Capitalized words that are never speaker names (sentence-openers, brands, places, fillers). Kept
/// small and high-value; the confidence gate catches the rest.
const STOP_NAMES: &[&str] = &[
    "i", "i'm", "ok", "okay", "yeah", "yes", "no", "so", "and", "but", "the", "a", "an", "well",
    "now", "then", "this", "that", "there", "here", "it", "we", "you", "they", "he", "she",
    "what", "when", "where", "why", "how", "who", "if", "my", "your", "our", "thanks", "thank",
    "hey", "hi", "hello", "welcome", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday",
    "sunday", "january", "february", "march", "april", "may", "june", "july", "august",
    "september", "october", "november", "december", "netflix", "prime", "hulu", "youtube",
    "disney", "google", "outlook", "salesforce", "slack", "calendly", "intercom", "apple",
    "kindle", "audible", "hbo", "persona", "stitch", "nudge",
];

#[cfg(test)]
mod tests {
    use super::*;

    // --- Layer A ---------------------------------------------------------------------------------
    #[test]
    fn rms_of_silence_is_zero_and_signal_is_positive() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms(&[0.0, 0.0, 0.0]), 0.0);
        assert!(rms(&[0.5, -0.5, 0.5]) > 0.0);
    }

    #[test]
    fn mic_dominant_is_self_loopback_dominant_is_guest() {
        assert_eq!(attribute_channel(0.2, 0.001), Channel::Self_);
        assert_eq!(attribute_channel(0.001, 0.2), Channel::Guest);
    }

    #[test]
    fn comparable_energy_on_both_channels_is_mixed() {
        assert_eq!(attribute_channel(0.10, 0.09), Channel::Mixed);
    }

    #[test]
    fn clearly_louder_channel_wins_even_when_both_active() {
        assert_eq!(attribute_channel(0.30, 0.05), Channel::Self_);
        assert_eq!(attribute_channel(0.05, 0.30), Channel::Guest);
    }

    #[test]
    fn total_silence_defaults_to_guest_never_you() {
        assert_eq!(attribute_channel(0.0, 0.0), Channel::Guest);
    }

    // --- Layer B ---------------------------------------------------------------------------------
    #[test]
    fn cosine_identical_is_one_orthogonal_is_zero() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn two_distinct_voices_make_two_clusters_similar_ones_join() {
        let mut c = OnlineClusterer::new(0.6);
        let a1 = vec![1.0, 0.0, 0.0];
        let a2 = vec![0.95, 0.1, 0.0];
        let b1 = vec![0.0, 1.0, 0.0];
        let id_a1 = c.assign(&a1);
        let id_b1 = c.assign(&b1);
        let id_a2 = c.assign(&a2);
        assert_ne!(id_a1, id_b1, "distinct voices must be different speakers");
        assert_eq!(id_a1, id_a2, "a near-identical voice joins its cluster");
        assert_eq!(c.speaker_count(), 2);
    }

    #[test]
    fn recluster_merges_an_oversplit_voice() {
        // Feed one voice with a strict threshold so it over-splits, then relax via recluster.
        let mut c = OnlineClusterer::new(0.999);
        c.assign(&[1.0, 0.0, 0.0]);
        c.assign(&[0.9, 0.2, 0.0]); // just different enough to start a 2nd cluster at 0.999
        assert!(c.speaker_count() >= 2);
        let remap = c.recluster(0.8);
        assert_eq!(c.speaker_count(), 1, "re-clustering should merge the one voice back together");
        assert!(!remap.is_empty());
    }

    // --- Layer C ---------------------------------------------------------------------------------
    #[test]
    fn address_names_the_other_speaker() {
        // cluster 0 = host, cluster 1 = guest. Host addresses the guest twice by name.
        let turns = vec![
            Turn { cluster: 1, text: "Hello, good to be here.".into() },
            Turn { cluster: 0, text: "Teresa, thank you so much for being here.".into() },
            Turn { cluster: 1, text: "Thanks for having me.".into() },
            Turn { cluster: 0, text: "Teresa, your book is wonderful.".into() },
        ];
        let names = resolve_speaker_names(&turns);
        assert_eq!(names.get(&1), Some(&"Teresa".to_string()));
        assert_eq!(names.get(&0), None, "host has no naming evidence yet");
    }

    #[test]
    fn self_introduction_names_the_speaker() {
        let turns = vec![Turn { cluster: 2, text: "Hi everyone, I'm Lenny and welcome.".into() }];
        let names = resolve_speaker_names(&turns);
        assert_eq!(names.get(&2), Some(&"Lenny".to_string()));
    }

    #[test]
    fn a_single_stray_name_is_not_committed() {
        // One vocative only → below the confidence gate → no name (the "Thanks, William" guard).
        let turns = vec![
            Turn { cluster: 1, text: "Thanks, William.".into() },
            Turn { cluster: 0, text: "Sure.".into() },
        ];
        let names = resolve_speaker_names(&turns);
        assert_eq!(names.get(&0), None);
    }

    #[test]
    fn spelling_variants_merge_theresa_and_teresa() {
        let turns = vec![
            Turn { cluster: 1, text: "Opening remarks.".into() },
            Turn { cluster: 0, text: "Theresa, welcome.".into() },
            Turn { cluster: 1, text: "Reply.".into() },
            Turn { cluster: 0, text: "Teresa, one more question.".into() },
        ];
        let names = resolve_speaker_names(&turns);
        // Two votes, merged across the spelling → committed once.
        assert_eq!(names.get(&1), Some(&"Theresa".to_string()));
    }

    #[test]
    fn introduction_pattern_names_the_introduced_guest() {
        let turns = vec![
            Turn { cluster: 1, text: "Glad to be on.".into() },
            Turn { cluster: 0, text: "Sarah Chen is a researcher and author.".into() },
            Turn { cluster: 1, text: "Thank you.".into() },
            Turn { cluster: 0, text: "Sarah, let's begin.".into() },
        ];
        let names = resolve_speaker_names(&turns);
        assert_eq!(names.get(&1), Some(&"Sarah".to_string()));
    }

    #[test]
    fn real_podcast_transcript_resolves_teresa_and_lenny_only() {
        // A faithful slice of the owner's pasted Wispr transcript. Clusters: 0 = Lenny (host),
        // 1 = Teresa (guest), 3 = the stray "Speaker 3". Expected: both hosts named, "William"
        // (a one-off mis-hearing) rejected in favour of the well-supported "Lenny", and the lone
        // Speaker-3 line left unnamed — i.e. we beat Wispr's "Thanks, William" mislabel.
        let t = |c: i64, s: &str| Turn { cluster: c, text: s.into() };
        let turns = vec![
            t(1, "Keep making bets. In parallel, start doing some discovery."),
            t(1, "No, you're always delivering, and you're always discovering."),
            t(0, "Theresa Torres is a speaker, a teacher, a consultant, and also the author of Continuous Discovery Habits."),
            t(0, "Theresa, thank you so much for being here."),
            t(0, "And again, thank you for being here, and welcome."),
            t(1, "Thanks, William. I'm excited to be here."),
            t(0, "How many PMs would you say that you've worked with?"),
            t(1, "I know through Product Talk Academy, we're at about 11,000 students."),
            t(1, "So the vast majority of product teams are saying things like, hey, Lenny, what do you like to watch?"),
            t(1, "Lenny, tell me about the last time you watched something on a streaming service."),
            t(0, "Just last night, I was watching Obi-Wan Kenobi on Disney+."),
            t(3, "Thank you so."),
        ];
        let names = resolve_speaker_names(&turns);
        assert_eq!(names.get(&1), Some(&"Theresa".to_string()), "guest resolves to Teresa/Theresa");
        assert_eq!(names.get(&0), Some(&"Lenny".to_string()), "host resolves to Lenny, not William");
        assert_eq!(names.get(&3), None, "the lone stray line stays Speaker 3 — no name");
    }

    #[test]
    fn brand_and_filler_words_are_never_names() {
        let turns = vec![
            Turn { cluster: 0, text: "Netflix, thank you for nothing.".into() },
            Turn { cluster: 1, text: "OK, so.".into() },
            Turn { cluster: 0, text: "Netflix, again.".into() },
        ];
        let names = resolve_speaker_names(&turns);
        assert!(names.is_empty(), "brands/fillers must not become speaker names");
    }
}
