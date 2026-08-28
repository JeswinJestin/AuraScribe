//! On-device prompt optimization: rewrite selected/dictated text into a better prompt for an AI
//! assistant, without losing the original meaning. Runs entirely locally (the `prompt` feature wires
//! a llama.cpp GGUF model); no cloud, ever.
//!
//! This module owns the *behavior* (the system prompt + chat formatting) in code that compiles
//! everywhere, and — under `#[cfg(feature = "prompt")]` — the actual model load + generation in the
//! `llm` submodule. That half is **owner-built + owner-verified**: it links llama.cpp from source and
//! needs a ~0.4 GB GGUF, neither of which can be exercised from the sandbox. See the Task-5 build
//! notes in `docs/superpowers/plans/2026-08-25-prompt-optimization-engine.md`, especially the
//! whisper.cpp/llama.cpp ggml symbol-collision caveat for a `--features "moonshine prompt"` build.

/// The optimizer's instructions. Intent-adaptive (structured prompt / cleanup / both, inferred from
/// the text) with a hard rule to preserve the original context. Kept as one const so it is
/// unit-testable and reviewable independently of any model.
pub const SYSTEM_PROMPT: &str = "You are a prompt-optimization assistant that runs entirely on the user's own device. \
You are given a piece of text the user dictated or selected. Rewrite it into the best possible form for talking to an \
AI assistant, without losing any of the original meaning or details.\n\n\
First infer, from the text itself, what the user wants:\n\
- If they ask (explicitly or implicitly) for a well-structured prompt, produce one with clear sections: Role, Context, \
Task, and the desired Output format (add Constraints only if the text implies them).\n\
- If they only want the text cleaned up or clarified, produce a single clear, specific, well-phrased request with no \
rigid template.\n\
- If it is a mix, do both: clean it up and structure it.\n\n\
Rules you must always follow:\n\
- Never lose or contradict the original context. Do not invent facts the user did not give, and do not drop details \
they did.\n\
- Preserve every concrete instruction, name, number, and constraint from the original text.\n\
- Match the user's language.\n\
- Output ONLY the rewritten result. No preamble, no commentary, no surrounding quotes.";

/// Wrap `user_text` in the model's chat template (Qwen2.5 / ChatML) with the system prompt, ready to
/// feed to the model. Returns the full prompt string ending at the assistant turn.
pub fn build_prompt(user_text: &str) -> String {
    format!(
        "<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n",
        user = user_text.trim(),
    )
}

/// Optimize `user_text` into a better prompt. Returns the rewrite, or an error if there is nothing
/// to optimize or the optimizer model is not available in this build / not downloaded yet.
///
/// This is CPU-heavy (seconds of generation) — call it from a blocking context, never on the async
/// runtime. `commands::optimize_selection` already wraps it in `spawn_blocking`.
pub fn optimize_text(user_text: &str) -> Result<String, String> {
    let trimmed = user_text.trim();
    if trimmed.is_empty() {
        return Err("Nothing selected to optimize".into());
    }

    #[cfg(not(feature = "prompt"))]
    {
        Err("Prompt optimization is not available in this build (the `prompt` feature is off)".into())
    }

    #[cfg(feature = "prompt")]
    {
        llm::generate(&build_prompt(trimmed)).map(|out| clean_output(&out))
    }
}

/// Tidy a raw generation into the text we inject: strip any chat-template markers the model echoed,
/// drop a wrapping pair of quotes it may have added despite the instruction, and trim. Kept out of
/// the feature gate so it is unit-testable on every build.
#[allow(dead_code)] // used only by the `prompt`-feature path, but always compiled + tested
pub fn clean_output(raw: &str) -> String {
    let mut s = raw.trim();
    // The turn-end / EOS markers should be caught by is_eog during generation, but strip them if the
    // model emitted the literal text anyway.
    for marker in ["<|im_end|>", "<|endoftext|>"] {
        if let Some(idx) = s.find(marker) {
            s = &s[..idx];
        }
    }
    let s = s.trim();
    // Remove one symmetric wrapping pair of quotes ("...", '...', or “...”), if the whole thing is
    // wrapped — the system prompt forbids them, but small models sometimes add them anyway.
    let unwrapped = [('"', '"'), ('\'', '\''), ('“', '”')]
        .iter()
        .find_map(|&(open, close)| {
            s.strip_prefix(open)
                .and_then(|inner| inner.strip_suffix(close))
                .filter(|inner| !inner.contains(open) && !inner.contains(close))
        })
        .unwrap_or(s);
    unwrapped.trim().to_string()
}

/// The llama.cpp-backed optimizer. Compiled only with the `prompt` feature.
///
/// Written against `llama-cpp-2 = 0.1.150` (pinned in Cargo.toml). The 0.1.x API shifts between
/// releases — if a first build fails here, check the installed version's docs and adjust these ~60
/// lines; the surrounding behavior (system prompt, chat template, cleanup) is stable and tested.
#[cfg(feature = "prompt")]
mod llm {
    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};

    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::{AddBos, LlamaModel};
    use llama_cpp_2::sampling::LlamaSampler;

    /// Cap on generated tokens. A prompt rewrite is short; this also bounds worst-case latency.
    const MAX_NEW_TOKENS: i32 = 1024;

    /// llama.cpp's global backend is process-wide and must be initialized exactly once.
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
    /// The loaded model, cached across calls (loading a GGUF is the slow part). Restart to switch
    /// models — good enough for the Phase-1 single-model POC.
    static MODEL: Mutex<Option<Arc<LlamaModel>>> = Mutex::new(None);

    fn models_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("AuraScribe")
            .join("models")
    }

    fn name_has(path: &std::path::Path, needle: &str) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_lowercase().contains(needle))
            .unwrap_or(false)
    }

    /// Locate the optimizer GGUF in the models directory. It is the only `.gguf` we use (Whisper uses
    /// `.bin`, the sherpa engines use `.onnx`), so any `.gguf` there is the optimizer. If both the
    /// 1.5B and 0.5B are present, prefer the 1.5B (the user opted into the heavier, higher-quality
    /// one); otherwise the 0.5B default; otherwise the first `.gguf` found.
    fn find_gguf() -> Option<PathBuf> {
        let mut ggufs: Vec<PathBuf> = std::fs::read_dir(models_dir())
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|e| e.eq_ignore_ascii_case("gguf"))
                    .unwrap_or(false)
            })
            .collect();
        ggufs.sort();
        ggufs
            .iter()
            .find(|p| name_has(p, "1.5b"))
            .cloned()
            .or_else(|| ggufs.iter().find(|p| name_has(p, "0.5b")).cloned())
            .or_else(|| ggufs.into_iter().next())
    }

    /// Initialize (once) and return the process-wide llama.cpp backend.
    fn backend() -> Result<&'static LlamaBackend, String> {
        if let Some(b) = BACKEND.get() {
            return Ok(b);
        }
        let b = LlamaBackend::init().map_err(|e| format!("llama.cpp backend init failed: {e}"))?;
        let _ = BACKEND.set(b); // ignore a lost init race; the stored one is equivalent
        BACKEND.get().ok_or_else(|| "llama.cpp backend unavailable".to_string())
    }

    /// Load (or return the cached) optimizer model.
    fn model(backend: &'static LlamaBackend) -> Result<Arc<LlamaModel>, String> {
        let mut guard = MODEL.lock().map_err(|_| "optimizer model lock poisoned".to_string())?;
        if let Some(m) = guard.as_ref() {
            return Ok(m.clone());
        }
        let path = find_gguf().ok_or_else(|| {
            "Prompt optimization model is not installed. Download it in Settings → Prompt optimization.".to_string()
        })?;
        // CPU by default (n_gpu_layers = 0) so the feature is portable; the 0.5B model is fast on CPU.
        // Raise this to offload layers to a GPU if one is available.
        let params = LlamaModelParams::default().with_n_gpu_layers(0);
        let loaded = LlamaModel::load_from_file(backend, &path, &params)
            .map_err(|e| format!("failed to load optimizer model {}: {e}", path.display()))?;
        let arc = Arc::new(loaded);
        *guard = Some(arc.clone());
        tracing::info!("Loaded prompt-optimizer model: {}", path.display());
        Ok(arc)
    }

    /// Run the optimizer over a fully-formatted chat `prompt`, returning the raw generated text
    /// (cleanup happens in the caller). Greedy sampling — deterministic, which is what we want for
    /// faithfully following the system prompt.
    pub fn generate(prompt: &str) -> Result<String, String> {
        let backend = backend()?;
        let model = model(backend)?;

        let tokens = model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| format!("tokenization failed: {e}"))?;
        let n_prompt = tokens.len() as i32;
        let n_len = n_prompt + MAX_NEW_TOKENS;

        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4);
        let n_ctx = (n_prompt + MAX_NEW_TOKENS + 8).max(512) as u32;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(n_ctx).expect("n_ctx >= 512")))
            .with_n_threads(threads)
            .with_n_threads_batch(threads);
        let mut ctx = model
            .new_context(backend, ctx_params)
            .map_err(|e| format!("llama context creation failed: {e}"))?;

        // Feed the whole prompt in one batch; only the last token needs logits.
        let mut batch = LlamaBatch::new(n_prompt.max(512) as usize, 1);
        let last = tokens.len().saturating_sub(1);
        for (i, tok) in tokens.iter().enumerate() {
            batch
                .add(*tok, i as i32, &[0], i == last)
                .map_err(|e| format!("batch add failed: {e}"))?;
        }
        ctx.decode(&mut batch).map_err(|e| format!("prompt decode failed: {e}"))?;

        let mut sampler = LlamaSampler::greedy();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut out = String::new();
        let mut n_cur = batch.n_tokens();

        while n_cur <= n_len {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);
            if model.is_eog_token(token) {
                break;
            }
            // special=false so a stray control token never lands in the user-facing text.
            let piece = model
                .token_to_piece(token, &mut decoder, false, None)
                .map_err(|e| format!("token decode failed: {e}"))?;
            out.push_str(&piece);

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| format!("batch add failed: {e}"))?;
            n_cur += 1;
            ctx.decode(&mut batch).map_err(|e| format!("decode failed: {e}"))?;
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_carries_the_core_rules() {
        let low = SYSTEM_PROMPT.to_lowercase();
        assert!(low.contains("never lose"), "must forbid losing context");
        assert!(low.contains("output only"), "must demand result-only output");
        assert!(low.contains("structured") || low.contains("structure"), "must offer structuring");
    }

    #[test]
    fn build_prompt_preserves_user_text_and_wraps_it() {
        let p = build_prompt("make me a poem about rain");
        assert!(p.contains("make me a poem about rain"), "user text must survive verbatim");
        assert!(p.contains("<|im_start|>system"));
        assert!(p.contains("<|im_start|>user"));
        assert!(p.trim_end().ends_with("<|im_start|>assistant"), "must end at the assistant turn");
    }

    #[test]
    fn build_prompt_trims_but_keeps_inner_whitespace() {
        let p = build_prompt("  git status  ");
        assert!(p.contains("git status"));
        assert!(!p.contains("  git status  "));
    }

    #[test]
    fn empty_or_blank_selection_errors() {
        // Feature-independent: the blank check runs before any model is touched.
        assert!(optimize_text("   ").is_err());
        assert!(optimize_text("").is_err());
    }

    #[cfg(not(feature = "prompt"))]
    #[test]
    fn without_the_feature_reports_unavailable() {
        let e = optimize_text("hello").unwrap_err();
        assert!(e.to_lowercase().contains("not available"));
    }

    #[test]
    fn clean_output_strips_markers_and_wrapping_quotes() {
        assert_eq!(clean_output("  hello world  "), "hello world");
        assert_eq!(clean_output("hello<|im_end|>"), "hello");
        assert_eq!(clean_output("\"wrapped\""), "wrapped");
        assert_eq!(clean_output("“smart quoted”"), "smart quoted");
        // A quote that is part of the content, not a wrapper, is left alone.
        assert_eq!(clean_output("say \"hi\" to them"), "say \"hi\" to them");
        assert_eq!(clean_output("Role: writer\nTask: rewrite<|endoftext|>"), "Role: writer\nTask: rewrite");
    }
}
