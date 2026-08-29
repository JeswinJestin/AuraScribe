//! On-device disk-footprint reporting and cleanup for the models directory.
//!
//! The app's real weight is downloaded models, not the tiny transcript DB (text is cheap). Nothing
//! used to clean up **orphaned** model files (leftovers from removed engines / earlier experiments)
//! or **partial** downloads (`*.part` left by a failed/cancelled fetch), so gigabytes could strand
//! on disk unnoticed. This module measures every entry in the models directory, classifies it
//! against the live catalogue, and reclaims what the app can no longer use.
//!
//! The logic is kept pure — it takes a directory path and the set of on-disk names the catalogue
//! knows about — so it is unit-testable against a temp dir with no models, DB, or Tauri state.

use std::collections::HashSet;
use std::path::Path;

/// The suffix a download uses for an incomplete file. A `*.part` is always reclaimable: it is not a
/// usable model, and the app re-downloads from scratch, so a stale one is pure dead weight.
pub const PARTIAL_SUFFIX: &str = ".part";

/// What a models-directory entry is, for the storage view and cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// A model the current catalogue knows about (its on-disk name matches). Never auto-deleted.
    Model,
    /// A file/dir in the models folder that no catalogue model claims — a leftover. Reclaimable.
    Orphan,
    /// An incomplete download (`*.part`). Reclaimable.
    Partial,
}

impl EntryKind {
    /// Reclaimable entries are the ones cleanup removes and the "reclaim" total sums.
    pub fn is_reclaimable(self) -> bool {
        matches!(self, EntryKind::Orphan | EntryKind::Partial)
    }
}

/// One entry in the models directory, with its measured size.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageEntry {
    /// The on-disk name (a subdirectory for sherpa models, a `ggml-*.bin` file for Whisper).
    pub name: String,
    pub size_bytes: u64,
    pub kind: EntryKind,
}

/// The full footprint picture handed to the UI.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StorageReport {
    pub entries: Vec<StorageEntry>,
    /// Sum of entries the catalogue recognises (legitimate models).
    pub models_bytes: u64,
    /// Sum of orphan + partial entries — what "Reclaim" frees.
    pub reclaimable_bytes: u64,
    /// The transcript database (`aurascribe.db` + its `-wal`/`-shm` sidecars).
    pub db_bytes: u64,
    /// models_bytes + reclaimable_bytes + db_bytes.
    pub total_bytes: u64,
}

/// Size of a file, or the recursive sum of a directory's files. Best-effort: unreadable entries
/// contribute 0 rather than aborting the whole scan (a storage report must never fail loudly).
pub fn entry_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let Ok(read) = std::fs::read_dir(path) else {
            return 0;
        };
        return read.flatten().map(|e| entry_size(&e.path())).sum();
    }
    0 // symlink or other — don't follow, don't count
}

/// Classify a single entry name against the set of names the catalogue knows.
fn classify(name: &str, known: &HashSet<String>) -> EntryKind {
    if name.ends_with(PARTIAL_SUFFIX) {
        EntryKind::Partial
    } else if known.contains(name) || is_optimizer_model(name) {
        EntryKind::Model
    } else {
        EntryKind::Orphan
    }
}

/// The prompt-optimizer model is a `.gguf` file whose name is not in the ASR catalogue (that lists
/// only the speech engines). It is still a real, user-installed model — treat any `.gguf` as a Model
/// so "Reclaim" never deletes it. (Regression guard: a manual/auto reclaim once wiped a 469 MB
/// optimizer GGUF because it was misclassified as an orphan.)
fn is_optimizer_model(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .map(|e| e.eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
}

/// Scan `models_dir`, classifying and measuring every top-level entry. `known` is the set of on-disk
/// basenames that correspond to real catalogue models (subdir names for sherpa engines,
/// `ggml-<id>.bin` for Whisper). A missing/empty directory yields an empty list.
pub fn scan_models_dir(models_dir: &Path, known: &HashSet<String>) -> Vec<StorageEntry> {
    let Ok(read) = std::fs::read_dir(models_dir) else {
        return Vec::new();
    };
    let mut entries: Vec<StorageEntry> = read
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            Some(StorageEntry {
                kind: classify(&name, known),
                size_bytes: entry_size(&e.path()),
                name,
            })
        })
        .collect();
    // Largest first — that is what the user wants to act on.
    entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    entries
}

/// Build a full report from a models-dir scan plus a pre-measured database size.
pub fn build_report(entries: Vec<StorageEntry>, db_bytes: u64) -> StorageReport {
    let models_bytes = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Model)
        .map(|e| e.size_bytes)
        .sum();
    let reclaimable_bytes = entries
        .iter()
        .filter(|e| e.kind.is_reclaimable())
        .map(|e| e.size_bytes)
        .sum();
    StorageReport {
        entries,
        models_bytes,
        reclaimable_bytes,
        db_bytes,
        total_bytes: models_bytes + reclaimable_bytes + db_bytes,
    }
}

/// Delete every reclaimable (orphan + partial) entry in `models_dir`, returning the bytes freed.
/// Only ever touches entries the catalogue does **not** recognise, so a real model can never be
/// removed by this path. Best-effort per entry: a failed delete is skipped, not fatal.
pub fn reclaim(models_dir: &Path, known: &HashSet<String>) -> u64 {
    let mut freed = 0;
    for entry in scan_models_dir(models_dir, known) {
        if !entry.kind.is_reclaimable() {
            continue;
        }
        let path = models_dir.join(&entry.name);
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if removed.is_ok() {
            freed += entry.size_bytes;
        }
    }
    freed
}

/// Delete only the partial (`*.part`) downloads, returning the bytes freed. Called at startup so a
/// failed/cancelled download never strands gigabytes; safe because a `*.part` is never a usable
/// model. Orphaned full models are left for the user to reclaim deliberately from the storage view.
pub fn remove_partials(models_dir: &Path) -> u64 {
    let Ok(read) = std::fs::read_dir(models_dir) else {
        return 0;
    };
    let mut freed = 0;
    for e in read.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.ends_with(PARTIAL_SUFFIX) {
            continue;
        }
        let path = e.path();
        let size = entry_size(&path);
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if removed.is_ok() {
            freed += size;
        }
    }
    freed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A self-cleaning unique temp directory — no external `tempfile` dependency (the project weighs
    /// every dep). Mirrors the `std::env::temp_dir()` pattern used elsewhere, with RAII cleanup.
    struct TmpDir(PathBuf);

    impl TmpDir {
        fn new() -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("aura-storage-{}-{}", std::process::id(), n));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TmpDir(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn known_set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// A temp dir with: a known model (subdir with two files), an orphan `ggml-*.bin`, a `*.part`
    /// partial, and an orphan subdir.
    fn fixture() -> TmpDir {
        let dir = TmpDir::new();
        let root = dir.path();

        // Known model: moonshine-base-en/ with 300 bytes total.
        let model = root.join("moonshine-base-en");
        fs::create_dir_all(&model).unwrap();
        fs::write(model.join("model.onnx"), vec![0u8; 200]).unwrap();
        fs::write(model.join("tokens.txt"), vec![0u8; 100]).unwrap();

        // Orphan Whisper file (500 bytes) — no catalogue model claims it.
        fs::write(root.join("ggml-large-v3.bin"), vec![0u8; 500]).unwrap();

        // Partial download (1000 bytes).
        fs::write(root.join("ggml-turbo.bin.part"), vec![0u8; 1000]).unwrap();

        // Orphan subdir (50 bytes).
        let orphan_dir = root.join("leftover-experiment");
        fs::create_dir_all(&orphan_dir).unwrap();
        fs::write(orphan_dir.join("junk"), vec![0u8; 50]).unwrap();

        dir
    }

    #[test]
    fn entry_size_sums_files_and_recurses_dirs() {
        let dir = fixture();
        assert_eq!(entry_size(&dir.path().join("moonshine-base-en")), 300);
        assert_eq!(entry_size(&dir.path().join("ggml-large-v3.bin")), 500);
        assert_eq!(entry_size(&dir.path().join("does-not-exist")), 0);
    }

    #[test]
    fn scan_classifies_model_orphan_and_partial() {
        let dir = fixture();
        let known = known_set(&["moonshine-base-en"]);
        let entries = scan_models_dir(dir.path(), &known);

        let by_name = |n: &str| entries.iter().find(|e| e.name == n).unwrap().kind;
        assert_eq!(by_name("moonshine-base-en"), EntryKind::Model);
        assert_eq!(by_name("ggml-large-v3.bin"), EntryKind::Orphan);
        assert_eq!(by_name("ggml-turbo.bin.part"), EntryKind::Partial);
        assert_eq!(by_name("leftover-experiment"), EntryKind::Orphan);
    }

    #[test]
    fn report_totals_split_models_from_reclaimable() {
        let dir = fixture();
        let known = known_set(&["moonshine-base-en"]);
        let report = build_report(scan_models_dir(dir.path(), &known), 880);
        assert_eq!(report.models_bytes, 300);
        assert_eq!(report.reclaimable_bytes, 500 + 1000 + 50);
        assert_eq!(report.db_bytes, 880);
        assert_eq!(report.total_bytes, 300 + 1550 + 880);
    }

    #[test]
    fn reclaim_removes_only_orphans_and_partials() {
        let dir = fixture();
        let known = known_set(&["moonshine-base-en"]);
        let freed = reclaim(dir.path(), &known);
        assert_eq!(freed, 500 + 1000 + 50);

        // The real model survives; every reclaimable entry is gone.
        assert!(dir.path().join("moonshine-base-en").exists());
        assert!(!dir.path().join("ggml-large-v3.bin").exists());
        assert!(!dir.path().join("ggml-turbo.bin.part").exists());
        assert!(!dir.path().join("leftover-experiment").exists());
    }

    #[test]
    fn optimizer_gguf_is_a_model_and_is_never_reclaimed() {
        // Regression: a .gguf (the prompt-optimizer model) is not in the ASR catalogue, so it used to
        // be classified as an orphan and DELETED by Reclaim — wiping a 469 MB model the user placed.
        let dir = TmpDir::new();
        let gguf = dir.path().join("qwen2.5-0.5b-instruct-q4_k_m.gguf");
        fs::write(&gguf, vec![0u8; 700]).unwrap();
        fs::write(dir.path().join("ggml-old.bin"), vec![0u8; 200]).unwrap(); // an actual orphan

        let known = known_set(&[]); // catalogue does NOT list the gguf
        let entries = scan_models_dir(dir.path(), &known);
        let kind = |n: &str| entries.iter().find(|e| e.name == n).unwrap().kind;
        assert_eq!(kind("qwen2.5-0.5b-instruct-q4_k_m.gguf"), EntryKind::Model, "a .gguf must be a Model");
        assert_eq!(kind("ggml-old.bin"), EntryKind::Orphan);

        // Reclaim frees only the real orphan; the optimizer model survives.
        let freed = reclaim(dir.path(), &known);
        assert_eq!(freed, 200);
        assert!(gguf.exists(), "the optimizer .gguf must NOT be reclaimed");
        assert!(!dir.path().join("ggml-old.bin").exists());
    }

    #[test]
    fn remove_partials_only_touches_dot_part() {
        let dir = fixture();
        let freed = remove_partials(dir.path());
        assert_eq!(freed, 1000);
        assert!(!dir.path().join("ggml-turbo.bin.part").exists());
        // Orphans that are not partials are left for a deliberate reclaim.
        assert!(dir.path().join("ggml-large-v3.bin").exists());
        assert!(dir.path().join("moonshine-base-en").exists());
    }

    #[test]
    fn missing_directory_is_empty_not_an_error() {
        let dir = TmpDir::new();
        let missing = dir.path().join("nope");
        assert!(scan_models_dir(&missing, &known_set(&[])).is_empty());
        assert_eq!(reclaim(&missing, &known_set(&[])), 0);
        assert_eq!(remove_partials(&missing), 0);
    }
}
