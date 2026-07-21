// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{Embedding, NeuralPatternLayer, SimilarityResult};
use async_trait::async_trait;
use tpt_eve_core::{CausalRelationDraft, EveError, Fact, FactSource, Value};

/// Mock NeuralPatternLayer for testing/demos. Returns deterministic facts
/// for the TypeScript example from spec.txt.
#[derive(Debug, Clone, Default)]
pub struct MockNeuralLayer {
    /// Optional override for extracted facts.
    facts: Option<Vec<Fact>>,
    /// Optional override for extracted causal relations.
    relations: Option<Vec<CausalRelationDraft>>,
}

impl MockNeuralLayer {
    pub fn new() -> Self {
        MockNeuralLayer {
            facts: None,
            relations: None,
        }
    }

    pub fn with_facts(facts: Vec<Fact>) -> Self {
        MockNeuralLayer {
            facts: Some(facts),
            relations: None,
        }
    }

    pub fn with_relations(relations: Vec<CausalRelationDraft>) -> Self {
        MockNeuralLayer {
            facts: None,
            relations: Some(relations),
        }
    }
}

#[async_trait]
impl NeuralPatternLayer for MockNeuralLayer {
    async fn extract_facts(&self, text: &str) -> Result<Vec<Fact>, EveError> {
        if let Some(ref facts) = self.facts {
            return Ok(facts.clone());
        }

        // Return deterministic facts for spec.txt TypeScript example
        let facts = vec![
            Fact {
                subject: Value::Symbol("TypeScript".into()),
                predicate: Value::Symbol("is".into()),
                object: Value::Text("programming language".into()),
                source: FactSource::Extracted { confidence: 0.95 },
                confidence: 0.95,
            },
            Fact {
                subject: Value::Symbol("TypeScript".into()),
                predicate: Value::Symbol("provides".into()),
                object: Value::Text("compile-time type checking".into()),
                source: FactSource::Extracted { confidence: 0.95 },
                confidence: 0.95,
            },
            Fact {
                subject: Value::Symbol("TypeScript".into()),
                predicate: Value::Symbol("compiles".into()),
                object: Value::Text("to JavaScript".into()),
                source: FactSource::Extracted { confidence: 0.92 },
                confidence: 0.92,
            },
        ];

        // Filter to only return facts relevant to the input text
        if text.contains("TypeScript") || text.is_empty() {
            Ok(facts)
        } else {
            Ok(vec![])
        }
    }

    async fn extract_causal_relations(
        &self,
        _text: &str,
    ) -> Result<Vec<CausalRelationDraft>, EveError> {
        if let Some(ref relations) = self.relations {
            return Ok(relations.clone());
        }
        // Return deterministic causal relations for the TypeScript example
        use tpt_eve_core::CausalRelationKind;
        Ok(vec![
            CausalRelationDraft {
                cause: "TypeScript".into(),
                effect: "JavaScript".into(),
                relation: CausalRelationKind::Causes,
            },
            CausalRelationDraft {
                cause: "TypeScript".into(),
                effect: "compile-time type checking".into(),
                relation: CausalRelationKind::Enables,
            },
        ])
    }

    async fn embed(&self, _text: &str) -> Result<Embedding, EveError> {
        // Return a dummy embedding for testing
        Ok(Embedding(vec![0.1; 384]))
    }

    async fn find_similar(
        &self,
        _query: &str,
        mut candidates: Vec<String>,
    ) -> Result<Vec<SimilarityResult>, EveError> {
        // Return all candidates with decreasing scores
        let results: Vec<SimilarityResult> = candidates
            .drain(..)
            .enumerate()
            .map(|(i, candidate)| SimilarityResult {
                candidate,
                score: 1.0 - (i as f32 * 0.1),
            })
            .collect();
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn extracts_typescript_facts() {
        let mock = MockNeuralLayer::new();
        let facts = mock.extract_facts("TypeScript facts").await.unwrap();
        assert_eq!(facts.len(), 3);
        assert_eq!(facts[0].subject, Value::Symbol("TypeScript".into()));
    }

    #[tokio::test]
    async fn embed_returns_vector() {
        let mock = MockNeuralLayer::new();
        let emb = mock.embed("test").await.unwrap();
        assert_eq!(emb.0.len(), 384);
    }

    #[tokio::test]
    async fn extracts_causal_relations() {
        let mock = MockNeuralLayer::new();
        let relations = mock.extract_causal_relations("anything").await.unwrap();
        assert_eq!(relations.len(), 2);
        assert_eq!(relations[0].cause, "TypeScript");
    }
}
