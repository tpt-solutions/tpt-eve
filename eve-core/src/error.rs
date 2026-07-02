use thiserror::Error;

/// Shared error type for all `eve-*` crates. Kept dependency-free (no
/// `eve-dsl`/`eve-symbolic` types referenced directly) so `eve-core` stays
/// at the bottom of the workspace dependency graph.
#[derive(Debug, Error)]
pub enum EveError {
    #[error("transport error: {0}")]
    Transport(String),

    #[error("rpc error {code}: {message}")]
    Rpc { code: i32, message: String },

    #[error("failed to parse extraction response: {0}")]
    ExtractionParseFailed(String),

    #[error("operation not supported by this backend: {0}")]
    NotSupported(&'static str),

    #[error("persistence error: {0}")]
    Persistence(String),

    #[error("dsl error: {0}")]
    Dsl(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("causal error: {0}")]
    Causal(String),
}
