// src-tauri/src/llm.rs
//! AI text cleanup using OpenRouter API (free models) or local Ollama

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupOptions {
    pub text: String,
    pub style: String, // casual, formal, code, technical, auto
    pub language: String,
    pub auto_punctuation: bool,
    pub remove_fillers: bool,
    pub fix_grammar: bool,
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LLMCleanup {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl LLMCleanup {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    pub async fn cleanup(&self, options: CleanupOptions) -> Result<String> {
        let start = Instant::now();
        let prompt = self.build_prompt(&options);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message { role: "system".to_string(), content: prompt },
                Message { role: "user".to_string(), content: options.text },
            ],
            temperature: 0.1,
            max_tokens: 4096,
            stream: false,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/aurascribe/aurascribe")
            .header("X-Title", "AuraScribe")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error: {}", error_text);
        }

        let chat_response: ChatResponse = response.json().await.context("Failed to parse response")?;

        let cleaned = chat_response.choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or(options.text);

        tracing::debug!("LLM cleanup took {:?}: {} chars -> {} chars",
            start.elapsed(), options.text.len(), cleaned.len());

        Ok(cleaned)
    }

    fn build_prompt(&self, options: &CleanupOptions) -> String {
        if let Some(custom) = &options.custom_prompt {
            return custom.clone();
        }

        let style_instruction = match options.style.as_str() {
            "casual" => "Write in a casual, conversational tone. Use contractions. Be friendly and natural.",
            "formal" => "Write in a professional, formal tone. Avoid contractions. Use complete sentences.",
            "code" => "Preserve ALL code formatting, variable names (camelCase, snake_case), function names, and technical terms EXACTLY. Do not modify code snippets.",
            "technical" => "Use precise technical terminology. Preserve acronyms, product names, and technical jargon. Be concise and accurate.",
            _ => "Auto-detect the appropriate style from context. Default to clear, natural English.",
        };

        let mut rules = vec![
            "Output ONLY the cleaned text, no explanations or formatting.",
            "Do not change the meaning or add information not in the original.",
        ];

        if options.auto_punctuation {
            rules.push("Add proper punctuation (periods, commas, question marks).");
        }
        if options.remove_fillers {
            rules.push("Remove filler words: um, uh, like, you know, actually, basically, literally, so, well, I mean.");
        }
        if options.fix_grammar {
            rules.push("Fix grammar, syntax, and sentence structure.");
        }
        rules.push(format!("Language: {}.", options.language));
        rules.push(style_instruction.to_string());

        format!(
            "You are a text cleanup assistant. Transform raw speech-to-text into clean, readable text.\n\nRules:\n{}\n\nExamples:\nRaw: \"um so i think we should like maybe schedule this for next tuesday if that works\"\nClean: \"I think we should schedule this for next Tuesday if that works.\"\n\nRaw: \"the function getUserData returns a promise that resolves to the user object\"\nClean: \"The function `getUserData` returns a Promise that resolves to the user object.\"\n\nRaw: \"hey team quick update the deploy failed because of a timeout error in the ci pipeline\"\nClean: \"Hey team, quick update: the deploy failed because of a timeout error in the CI pipeline.\"",
            rules.iter().enumerate().map(|(i, r)| format!("{}. {}", i + 1, r)).collect::<Vec<_>>().join("\n")
        )
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

/// Local Ollama integration
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    pub async fn cleanup(&self, options: CleanupOptions) -> Result<String> {
        let prompt = Self::build_prompt(&options);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            options: OllamaOptions {
                temperature: 0.1,
                num_predict: 4096,
            },
        };

        let response = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama API error: {}", response.status());
        }

        let ollama_response: OllamaResponse = response.json().await.context("Failed to parse Ollama response")?;

        Ok(ollama_response.response.trim().to_string())
    }

    fn build_prompt(options: &CleanupOptions) -> String {
        let style_instruction = match options.style.as_str() {
            "casual" => "Casual, conversational tone with contractions.",
            "formal" => "Professional, formal tone without contractions.",
            "code" => "Preserve ALL code formatting, variable names, and technical terms EXACTLY.",
            "technical" => "Precise technical terminology. Preserve acronyms and jargon.",
            _ => "Clear, natural English. Auto-detect style.",
        };

        format!(
            r#"Clean up this speech-to-text transcript. Rules:
1. {}
2. Output ONLY the cleaned text.
3. Do not add explanations.

Raw: {}
Clean:"#,
            style_instruction, options.text
        )
    }
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
}