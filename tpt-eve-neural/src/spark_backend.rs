// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{jsonrpc::JsonRpcClient, prompts, NeuralPatternLayer};
use async_trait::async_trait;
use serde::Deserialize;
use tpt_eve_core::{EveError, Fact, FactSource, Value};

#[cfg(windows)]
const DEFAULT_PATH: &str = r"\\.\pipe\tpt-spark";
#[cfg(not(windows))]
const DEFAULT_PATH: &str = "/tmp/tpt-spark.sock";

#[derive(Debug, Deserialize)]
struct SparkFact {
    subject: String,
    predicate: String,
    object: String,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct SparkInferResult {
    facts: Vec<SparkFact>,
}

/// NeuralPatternLayer backed by a running tpt-spark process via JSON-RPC.
///
/// On Windows: connects to `\\.\pipe\tpt-spark` (named pipe).
/// On Unix: connects to a Unix domain socket (default `/tmp/tpt-spark.sock`).
pub struct SparkNeuralLayer {
    path: String,
}

impl SparkNeuralLayer {
    pub fn new() -> Self {
        SparkNeuralLayer {
            path: DEFAULT_PATH.to_string(),
        }
    }

    pub fn with_path(path: impl Into<String>) -> Self {
        SparkNeuralLayer { path: path.into() }
    }

    #[cfg(windows)]
    async fn connect(&self) -> Result<JsonRpcClient, EveError> {
        JsonRpcClient::connect_pipe(&self.path).await
    }

    #[cfg(not(windows))]
    async fn connect(&self) -> Result<JsonRpcClient, EveError> {
        JsonRpcClient::connect_socket(&self.path).await
    }
}

impl Default for SparkNeuralLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NeuralPatternLayer for SparkNeuralLayer {
    async fn extract_facts(&self, text: &str) -> Result<Vec<Fact>, EveError> {
        let mut client = self.connect().await?;
        let prompt = prompts::format_fact_prompt(text);
        let params = serde_json::json!({ "prompt": prompt });
        let result: SparkInferResult = client.request("spark_infer", params).await?;

        Ok(result
            .facts
            .into_iter()
            .map(|f| {
                let confidence = f.confidence.unwrap_or(0.8);
                Fact {
                    subject: Value::Symbol(f.subject),
                    predicate: Value::Symbol(f.predicate),
                    object: Value::Text(f.object),
                    source: FactSource::Extracted { confidence },
                    confidence,
                }
            })
            .collect())
    }
}
