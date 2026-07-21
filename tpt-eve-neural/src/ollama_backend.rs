// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{prompts, Embedding, NeuralPatternLayer, SimilarityResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tpt_eve_core::{CausalRelationDraft, CausalRelationKind, EveError, Fact, FactSource, Value};

const DEFAULT_HOST: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3";
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct RawFact {
    subject: String,
    predicate: String,
    object: String,
}

#[derive(Debug, Deserialize)]
struct RawCausalRelation {
    cause: String,
    effect: String,
    relation: String,
}

/// NeuralPatternLayer backed by a local Ollama server.
///
/// Reads OLLAMA_HOST from the environment (default: http://localhost:11434).
/// Supports `embed()` via `/api/embeddings` with nomic-embed-text or any other
/// embedding model available in the local Ollama instance.
pub struct OllamaNeuralLayer {
    host: String,
    model: String,
    embed_model: String,
    client: reqwest::Client,
}

impl OllamaNeuralLayer {
    pub fn new() -> Self {
        let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
        OllamaNeuralLayer {
            host,
            model: DEFAULT_MODEL.to_string(),
            embed_model: DEFAULT_EMBED_MODEL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_embed_model(mut self, model: impl Into<String>) -> Self {
        self.embed_model = model.into();
        self
    }

    async fn generate(&self, prompt: &str) -> Result<String, EveError> {
        let url = format!("{}/api/generate", self.host);
        let req = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| EveError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let body: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

        Ok(body.response)
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
                source: FactSource::Extracted { confidence: 0.8 },
                confidence: 0.8,
            })
            .collect())
    }

    fn parse_causal_relations(text: &str) -> Result<Vec<CausalRelationDraft>, EveError> {
        let start = text.find('[').unwrap_or(0);
        let end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let slice = &text[start..end];

        let raw: Vec<RawCausalRelation> = serde_json::from_str(slice).map_err(|e| {
            EveError::ExtractionParseFailed(format!("Failed to parse causal relations: {e}"))
        })?;

        raw.into_iter()
            .map(|r| {
                let kind = match r.relation.as_str() {
                    "causes" => Ok(CausalRelationKind::Causes),
                    "precedes" => Ok(CausalRelationKind::Precedes),
                    "enables" => Ok(CausalRelationKind::Enables),
                    "prevents" => Ok(CausalRelationKind::Prevents),
                    other => Err(EveError::ExtractionParseFailed(format!(
                        "Unknown causal relation kind: {other}"
                    ))),
                }?;
                Ok(CausalRelationDraft {
                    cause: r.cause,
                    effect: r.effect,
                    relation: kind,
                })
            })
            .collect()
    }
}

impl Default for OllamaNeuralLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NeuralPatternLayer for OllamaNeuralLayer {
    async fn extract_facts(&self, text: &str) -> Result<Vec<Fact>, EveError> {
        let prompt = prompts::format_fact_prompt(text);
        let response = self.generate(&prompt).await?;
        Self::parse_facts(&response)
    }

    async fn extract_causal_relations(
        &self,
        text: &str,
    ) -> Result<Vec<CausalRelationDraft>, EveError> {
        let prompt = prompts::format_causal_prompt(text);
        let response = self.generate(&prompt).await?;
        Self::parse_causal_relations(&response)
    }

    async fn find_similar(
        &self,
        query: &str,
        candidates: Vec<String>,
    ) -> Result<Vec<SimilarityResult>, EveError> {
        let query_emb = self.embed(query).await?;
        let mut results = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let cand_emb = self.embed(&candidate).await?;
            let score = cosine_similarity(&query_emb.0, &cand_emb.0);
            results.push(SimilarityResult { candidate, score });
        }
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    async fn embed(&self, text: &str) -> Result<Embedding, EveError> {
        let url = format!("{}/api/embeddings", self.host);
        let req = EmbedRequest {
            model: self.embed_model.clone(),
            prompt: text.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| EveError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let body: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

        Ok(Embedding(body.embedding))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
