// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{prompts, CloudProvider, Embedding, NeuralPatternLayer};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tpt_eve_core::{EveError, Fact, FactSource, Value};

// ── Anthropic types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

// ── OpenAI / OpenRouter types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiContent,
}

#[derive(Debug, Deserialize)]
struct OpenAiContent {
    content: String,
}

// ── Embeddings (OpenAI) ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Debug, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

// ── Shared fact parsing ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawFact {
    subject: String,
    predicate: String,
    object: String,
}

fn parse_facts(text: &str) -> Result<Vec<Fact>, EveError> {
    let start = text.find('[').unwrap_or(0);
    let end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
    let slice = &text[start..end];

    let raw: Vec<RawFact> = serde_json::from_str(slice)
        .map_err(|e| EveError::ExtractionParseFailed(format!("Failed to parse facts: {e}")))?;

    Ok(raw
        .into_iter()
        .map(|f| Fact {
            subject: Value::Symbol(f.subject),
            predicate: Value::Symbol(f.predicate),
            object: Value::Text(f.object),
            source: FactSource::Extracted { confidence: 0.9 },
            confidence: 0.9,
        })
        .collect())
}

// ── CloudNeuralLayer ─────────────────────────────────────────────────────────

/// NeuralPatternLayer backed by a cloud LLM (Anthropic, OpenAI, or OpenRouter).
///
/// Reads `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or `OPENROUTER_API_KEY` from
/// the environment when no key is provided at construction time.
pub struct CloudNeuralLayer {
    provider: CloudProvider,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl CloudNeuralLayer {
    pub fn new(provider: CloudProvider, api_key: impl Into<String>) -> Self {
        let model = match provider {
            CloudProvider::Anthropic => "claude-haiku-4-5-20251001".to_string(),
            CloudProvider::OpenAi => "gpt-4o-mini".to_string(),
            CloudProvider::OpenRouter => "openai/gpt-4o-mini".to_string(),
        };
        CloudNeuralLayer {
            provider,
            api_key: api_key.into(),
            model,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env(provider: CloudProvider) -> Self {
        let var_name = match provider {
            CloudProvider::Anthropic => "ANTHROPIC_API_KEY",
            CloudProvider::OpenAi => "OPENAI_API_KEY",
            CloudProvider::OpenRouter => "OPENROUTER_API_KEY",
        };
        let key = std::env::var(var_name).unwrap_or_else(|_| {
            tracing::warn!("{var_name} is not set — cloud API calls will fail with 401");
            String::new()
        });
        Self::new(provider, key)
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    async fn complete(&self, prompt: &str) -> Result<String, EveError> {
        match self.provider {
            CloudProvider::Anthropic => self.complete_anthropic(prompt).await,
            CloudProvider::OpenAi => {
                self.complete_openai(prompt, "https://api.openai.com/v1/chat/completions")
                    .await
            }
            CloudProvider::OpenRouter => {
                self.complete_openai(prompt, "https://openrouter.ai/api/v1/chat/completions")
                    .await
            }
        }
    }

    async fn complete_anthropic(&self, prompt: &str) -> Result<String, EveError> {
        let req = AnthropicRequest {
            model: &self.model,
            max_tokens: 1024,
            messages: vec![AnthropicMessage {
                role: "user",
                content: prompt,
            }],
        };
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .map_err(|e| EveError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let body: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

        body.content
            .into_iter()
            .filter_map(|b| if b.kind == "text" { b.text } else { None })
            .next()
            .ok_or_else(|| EveError::ExtractionParseFailed("Empty response from Anthropic".into()))
    }

    async fn complete_openai(&self, prompt: &str, url: &str) -> Result<String, EveError> {
        let req = OpenAiRequest {
            model: &self.model,
            messages: vec![OpenAiMessage {
                role: "user",
                content: prompt,
            }],
        };
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| EveError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let body: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| EveError::ExtractionParseFailed("Empty response from API".into()))
    }
}

#[async_trait]
impl NeuralPatternLayer for CloudNeuralLayer {
    async fn extract_facts(&self, text: &str) -> Result<Vec<Fact>, EveError> {
        let prompt = prompts::format_fact_prompt(text);
        let response = self.complete(&prompt).await?;
        parse_facts(&response)
    }

    async fn embed(&self, text: &str) -> Result<Embedding, EveError> {
        match self.provider {
            CloudProvider::Anthropic => Err(EveError::NotSupported(
                "Anthropic does not support embeddings",
            )),
            CloudProvider::OpenRouter => Err(EveError::NotSupported(
                "OpenRouter embed not supported via this client",
            )),
            CloudProvider::OpenAi => {
                let req = EmbedRequest {
                    model: "text-embedding-3-small",
                    input: text,
                };
                let resp = self
                    .client
                    .post("https://api.openai.com/v1/embeddings")
                    .bearer_auth(&self.api_key)
                    .json(&req)
                    .send()
                    .await
                    .map_err(|e| EveError::Transport(e.to_string()))?;

                let body: EmbedResponse = resp
                    .json()
                    .await
                    .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

                body.data
                    .into_iter()
                    .next()
                    .map(|d| Embedding(d.embedding))
                    .ok_or_else(|| {
                        EveError::ExtractionParseFailed("No embedding data in response".to_string())
                    })
            }
        }
    }
}
