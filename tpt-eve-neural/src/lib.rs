// SPDX-License-Identifier: MIT OR Apache-2.0
mod cloud_backend;
mod jsonrpc;
mod mock_backend;
mod ollama_backend;
pub(crate) mod prompts;
mod spark_backend;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tpt_eve_core::{CausalRelationDraft, Fact};

pub use cloud_backend::CloudNeuralLayer;
pub use jsonrpc::JsonRpcClient;
pub use mock_backend::MockNeuralLayer;
pub use ollama_backend::OllamaNeuralLayer;
pub use spark_backend::SparkNeuralLayer;

/// Embedding vector for similarity search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    pub fn new(values: Vec<f32>) -> Self {
        Embedding(values)
    }
}

/// Similarity search result.
#[derive(Debug, Clone, PartialEq)]
pub struct SimilarityResult {
    pub candidate: String,
    pub score: f32,
}

/// Trait for neural pattern extraction from text.
#[async_trait]
pub trait NeuralPatternLayer: Send + Sync {
    /// Extract facts from the given text.
    async fn extract_facts(&self, text: &str) -> Result<Vec<Fact>, tpt_eve_core::EveError>;

    /// Extract causal relationships from the given text.
    async fn extract_causal_relations(
        &self,
        text: &str,
    ) -> Result<Vec<CausalRelationDraft>, tpt_eve_core::EveError> {
        let _ = text;
        Err(tpt_eve_core::EveError::NotSupported(
            "extract_causal_relations",
        ))
    }

    /// Generate embeddings for the given text (optional, returns NotSupported by default).
    async fn embed(&self, text: &str) -> Result<Embedding, tpt_eve_core::EveError> {
        let _ = text;
        Err(tpt_eve_core::EveError::NotSupported("embed"))
    }

    /// Find similar concepts/examples (optional, returns NotSupported by default).
    async fn find_similar(
        &self,
        query: &str,
        candidates: Vec<String>,
    ) -> Result<Vec<SimilarityResult>, tpt_eve_core::EveError> {
        let _ = (query, candidates);
        Err(tpt_eve_core::EveError::NotSupported("find_similar"))
    }
}

/// Cloud provider options for external LLM API calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Anthropic,
    OpenAi,
    OpenRouter,
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudProvider::Anthropic => write!(f, "anthropic"),
            CloudProvider::OpenAi => write!(f, "openai"),
            CloudProvider::OpenRouter => write!(f, "openrouter"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_layer_extracts_facts() {
        let mock = MockNeuralLayer::new();
        let facts = mock
            .extract_facts("TypeScript is a programming language.")
            .await
            .unwrap();
        assert!(!facts.is_empty());
    }
}
