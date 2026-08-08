// src-tauri/src/engine.rs
//! The engine seam: one `Asr` facade over the two speech engines.
//!
//! Whisper (`asr::WhisperASR`) is the v1 accuracy engine and is always present. Moonshine
//! (`moonshine::MoonshineASR`) is the newer, faster engine, compiled in only under the
//! `moonshine` feature. Everything the app calls — list, download, load, transcribe, delete —
//! goes through this facade, which routes each call to the right engine by the model's
//! `EngineKind`. The rest of the app never needs to know which engine is active.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::asr::{EngineKind, ModelInfo, WhisperASR};

pub struct Asr {
    whisper: WhisperASR,
    #[cfg(feature = "moonshine")]
    moonshine: crate::moonshine::MoonshineASR,
    #[cfg(feature = "moonshine")]
    parakeet: crate::parakeet::ParakeetASR,
    #[cfg(feature = "moonshine")]
    dolphin: crate::dolphin::DolphinASR,
    #[cfg(feature = "moonshine")]
    nemo_ctc: crate::nemo_ctc::NemoCtcASR,
    /// The engine of the model currently loaded, so `transcribe` (which is not given a model
    /// id) routes to the same engine that was loaded.
    loaded_engine: Mutex<Option<EngineKind>>,
}

impl Asr {
    pub fn new() -> Result<Self> {
        let whisper = WhisperASR::new()?;
        #[cfg(feature = "moonshine")]
        let moonshine = crate::moonshine::MoonshineASR::new(whisper.models_dir().to_path_buf());
        #[cfg(feature = "moonshine")]
        let parakeet = crate::parakeet::ParakeetASR::new(whisper.models_dir().to_path_buf());
        #[cfg(feature = "moonshine")]
        let dolphin = crate::dolphin::DolphinASR::new(whisper.models_dir().to_path_buf());
        #[cfg(feature = "moonshine")]
        let nemo_ctc = crate::nemo_ctc::NemoCtcASR::new(whisper.models_dir().to_path_buf());
        Ok(Self {
            #[cfg(feature = "moonshine")]
            moonshine,
            #[cfg(feature = "moonshine")]
            parakeet,
            #[cfg(feature = "moonshine")]
            dolphin,
            #[cfg(feature = "moonshine")]
            nemo_ctc,
            whisper,
            loaded_engine: Mutex::new(None),
        })
    }

    /// Which engine runs a given model id. Whisper is the default for anything not recognised
    /// as another engine's model, so an unknown id never silently routes elsewhere.
    ///
    /// This is an instance method (not associated) because the transducer engine owns not just
    /// its built-in ids but any custom sherpa-onnx bundle the user has dropped on disk, which it
    /// can only know by looking.
    fn engine_of(&self, model_id: &str) -> EngineKind {
        #[cfg(feature = "moonshine")]
        if crate::moonshine::is_moonshine_model(model_id) {
            return EngineKind::Moonshine;
        }
        #[cfg(feature = "moonshine")]
        if crate::dolphin::is_dolphin_model(model_id) {
            return EngineKind::Dolphin;
        }
        #[cfg(feature = "moonshine")]
        if crate::nemo_ctc::is_nemo_ctc_model(model_id) {
            return EngineKind::NemoCtc;
        }
        #[cfg(feature = "moonshine")]
        if self.parakeet.owns(model_id) {
            return EngineKind::Parakeet;
        }
        let _ = model_id;
        EngineKind::Whisper
    }

    /// Every model from every compiled-in engine, as one catalogue for the UI.
    ///
    /// Recommendation is decided **here**, across all engines, not inside either one — otherwise
    /// each engine would badge its own best and the list would show two "Recommended" chips.
    /// See [`elect_recommended`] for the rule.
    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        // `mut` is only used when the moonshine extend below is compiled in.
        #[cfg_attr(not(feature = "moonshine"), allow(unused_mut))]
        let mut models = self.whisper.list_available_models();
        #[cfg(feature = "moonshine")]
        models.extend(self.moonshine.list_available_models());
        #[cfg(feature = "moonshine")]
        models.extend(self.parakeet.list_available_models());
        #[cfg(feature = "moonshine")]
        models.extend(self.dolphin.list_available_models());
        #[cfg(feature = "moonshine")]
        models.extend(self.nemo_ctc.list_available_models());

        elect_recommended(&mut models);
        models
    }

    /// Whether a model is fully present and usable (a whole Moonshine bundle, not a partial one).
    pub fn is_downloaded(&self, model_id: &str) -> bool {
        match self.engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.is_present(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::Parakeet => self.parakeet.is_present(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::Dolphin => self.dolphin.is_present(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::NemoCtc => self.nemo_ctc.is_present(model_id),
            _ => self.whisper.get_model_path(model_id).exists(),
        }
    }

    /// The on-disk location of a model — a file for Whisper, a directory for the sherpa engines.
    pub fn get_model_path(&self, model_id: &str) -> PathBuf {
        match self.engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.model_dir(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::Parakeet => self.parakeet.model_dir(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::Dolphin => self.dolphin.model_dir(model_id),
            #[cfg(feature = "moonshine")]
            EngineKind::NemoCtc => self.nemo_ctc.model_dir(model_id),
            _ => self.whisper.get_model_path(model_id),
        }
    }

    /// Whether live chunking helps this model. The sherpa engines (Moonshine, Parakeet) are fast
    /// enough that they always benefit; for Whisper it depends on whether the model keeps pace
    /// with speech on this machine.
    pub fn should_chunk(&self, model_id: &str) -> bool {
        match self.engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine | EngineKind::Parakeet | EngineKind::Dolphin | EngineKind::NemoCtc => {
                true
            }
            _ => crate::asr::should_chunk(model_id),
        }
    }

    pub fn load_model(&self, model_id: &str) -> Result<()> {
        let engine = self.engine_of(model_id);
        match engine {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self
                .moonshine
                .load_model(model_id, crate::asr::worker_threads())?,
            #[cfg(feature = "moonshine")]
            EngineKind::Parakeet => self
                .parakeet
                .load_model(model_id, crate::asr::worker_threads())?,
            #[cfg(feature = "moonshine")]
            EngineKind::Dolphin => self
                .dolphin
                .load_model(model_id, crate::asr::worker_threads())?,
            #[cfg(feature = "moonshine")]
            EngineKind::NemoCtc => self
                .nemo_ctc
                .load_model(model_id, crate::asr::worker_threads())?,
            _ => self.whisper.load_model(model_id)?,
        }
        *self.loaded_engine.lock().unwrap() = Some(engine);
        Ok(())
    }

    pub async fn download_model(
        &self,
        model_id: &str,
        on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        match self.engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.download_model(model_id, on_progress).await,
            #[cfg(feature = "moonshine")]
            EngineKind::Parakeet => self.parakeet.download_model(model_id, on_progress).await,
            #[cfg(feature = "moonshine")]
            EngineKind::Dolphin => self.dolphin.download_model(model_id, on_progress).await,
            #[cfg(feature = "moonshine")]
            EngineKind::NemoCtc => self.nemo_ctc.download_model(model_id, on_progress).await,
            _ => self.whisper.download_model(model_id, on_progress).await,
        }
    }

    pub fn transcribe(&self, audio: &[f32], language: Option<&str>) -> Result<String> {
        let engine = (*self.loaded_engine.lock().unwrap()).unwrap_or(EngineKind::Whisper);
        match engine {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.transcribe(audio, language),
            #[cfg(feature = "moonshine")]
            EngineKind::Parakeet => self.parakeet.transcribe(audio, language),
            #[cfg(feature = "moonshine")]
            EngineKind::Dolphin => self.dolphin.transcribe(audio, language),
            #[cfg(feature = "moonshine")]
            EngineKind::NemoCtc => self.nemo_ctc.transcribe(audio, language),
            _ => self.whisper.transcribe(audio, language),
        }
    }

    /// Remove a downloaded model — a single file for Whisper, the whole bundle for Moonshine.
    pub fn delete_model(&self, model_id: &str) -> std::io::Result<()> {
        let path = self.get_model_path(model_id);
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else if path.exists() {
            std::fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}

/// Set the single `recommended` model across the whole catalogue.
///
/// The rule is **English-first**: this is an English dictation product, so the default badge
/// points at the best *English* option and lets multilingual models be a deliberate, informed
/// choice. Among English models that keep up with speech (`realtime_factor <= 1.0`), recommend
/// the **most accurate**, breaking ties toward the **fastest**. Multilingual models are only
/// considered when no English model can keep pace. Everything below the real-time line is
/// excluded so the badge never points at a model that can't keep up.
///
/// With the real catalogue this elects **`moonshine-base-en`** ("AuraScribe English", accuracy 4,
/// ~0.15×): more accurate than `moonshine-tiny-en` ("English Mini", accuracy 3) and faster than
/// the equally-accurate multilingual `parakeet-v3` (~0.5×, 671 MB) and `indicconformer` (~0.6×).
/// The rule is why the badge never lands on the multilingual "AuraScribe European" (Parakeet),
/// even though its headline accuracy (5) is higher — Parakeet is the multilingual choice, not the
/// English default. The earlier rule was *accuracy-first over all models*, which elected the
/// multilingual Parakeet; the owner asked for AuraScribe English to be the recommendation.
fn elect_recommended(models: &mut [ModelInfo]) {
    for m in models.iter_mut() {
        m.recommended = false;
    }
    let realtime: Vec<&ModelInfo> = models.iter().filter(|m| m.realtime_factor <= 1.0).collect();
    let english: Vec<&ModelInfo> = realtime.iter().copied().filter(|m| !m.multilingual).collect();
    let candidates = if english.is_empty() { realtime } else { english };
    let best = candidates
        .iter()
        .max_by(|a, b| {
            a.accuracy
                .cmp(&b.accuracy)
                .then_with(|| b.realtime_factor.total_cmp(&a.realtime_factor))
        })
        .map(|m| m.id.clone());
    if let Some(best) = best {
        if let Some(m) = models.iter_mut().find(|m| m.id == best) {
            m.recommended = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::{EngineKind, ModelInfo};

    fn model(id: &str, accuracy: u8, realtime_factor: f32) -> ModelInfo {
        model_with_langs(id, accuracy, realtime_factor, false)
    }

    fn model_with_langs(id: &str, accuracy: u8, realtime_factor: f32, multilingual: bool) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            name: id.to_string(),
            engine: EngineKind::Moonshine,
            size_mb: 0,
            multilingual,
            speed: 1,
            accuracy,
            recommended: false,
            downloaded: false,
            path: None,
            realtime_factor,
            warning: None,
        }
    }

    /// The recommendation must land on the most accurate real-time **English** model. With the
    /// real catalogue values that is `moonshine-base-en` (accuracy 4) — never `moonshine-tiny-en`
    /// (accuracy 3) despite Mini being fractionally faster, and never the multilingual Parakeet or
    /// IndicConformer, even though Parakeet's headline accuracy (5) is higher. The English-first
    /// rule is why the badge sits on AuraScribe English, not the multilingual "AuraScribe
    /// European".
    #[test]
    fn recommends_most_accurate_realtime_english_model_not_any_multilingual() {
        let mut models = vec![
            model("moonshine-tiny-en", 3, 0.10),
            model("moonshine-base-en", 4, 0.15),
            model_with_langs("parakeet-v3-multilingual", 5, 0.5, true),
            model_with_langs("indicconformer-ml", 4, 0.6, true),
            model("whisper-large", 5, 3.0), // too slow — excluded despite top accuracy
        ];
        elect_recommended(&mut models);
        let recommended: Vec<&str> = models
            .iter()
            .filter(|m| m.recommended)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            recommended,
            ["moonshine-base-en"],
            "exactly AuraScribe English (moonshine-base-en) should be recommended"
        );
    }

    /// With no English real-time model in the list, the rule relaxes to the most accurate
    /// real-time model overall (a multilingual one is fine then).
    #[test]
    fn recommends_multilingual_when_no_english_model_keeps_up() {
        let mut models = vec![
            model_with_langs("parakeet-v3-multilingual", 5, 0.5, true),
            model_with_langs("indicconformer-ml", 4, 0.6, true),
        ];
        elect_recommended(&mut models);
        let recommended: Vec<&str> = models
            .iter()
            .filter(|m| m.recommended)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            recommended,
            ["parakeet-v3-multilingual"],
            "the most accurate multilingual real-time model is the fallback"
        );
    }

    /// A model slower than speech is never recommended, even if it is the most accurate one.
    #[test]
    fn never_recommends_a_model_that_cannot_keep_up() {
        let mut models = vec![
            model("slow-but-accurate", 5, 2.0),
            model("fast-enough", 4, 0.2),
        ];
        elect_recommended(&mut models);
        assert!(
            models.iter().find(|m| m.id == "fast-enough").unwrap().recommended,
            "the real-time model should be recommended"
        );
        assert!(
            !models.iter().find(|m| m.id == "slow-but-accurate").unwrap().recommended,
            "a model slower than real time must never be recommended"
        );
    }
}
