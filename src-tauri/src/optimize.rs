//! On-device prompt optimization: rewrite selected/dictated text into a better prompt for an AI
//! assistant, without losing the original meaning. Runs entirely locally (Phase 1b wires a llama.cpp
//! GGUF model behind the `prompt` feature); no cloud, ever.
//!
//! This module owns the *behavior* (the system prompt + chat formatting) and the entry point
//! `optimize_text`. The model itself is loaded and run in the `prompt`-feature branch, which is
//! filled in as a separate, owner-built + owner-verified step — the from-source llama.cpp build and
//! the ~1 GB model can't be exercised from the sandbox.

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
        // Owner-built step (Task 5): load the GGUF model from the models dir and generate over
        // `build_prompt(trimmed)`. Until that lands, echo the input so the whole pipeline (hotkey →
        // capture → optimize → replace) is exercisable end to end with a no-op optimizer.
        Ok(trimmed.to_string())
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
        assert!(optimize_text("   ").is_err());
        assert!(optimize_text("").is_err());
    }

    #[cfg(not(feature = "prompt"))]
    #[test]
    fn without_the_feature_reports_unavailable() {
        let e = optimize_text("hello").unwrap_err();
        assert!(e.to_lowercase().contains("not available"));
    }

    #[cfg(feature = "prompt")]
    #[test]
    fn with_the_feature_echoes_until_model_is_wired() {
        assert_eq!(optimize_text("hello world").unwrap(), "hello world");
    }
}
