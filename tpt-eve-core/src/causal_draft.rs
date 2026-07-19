// SPDX-License-Identifier: MIT OR Apache-2.0
use serde::{Deserialize, Serialize};

/// A causal relationship as extracted from text by the Neural Pattern Layer,
/// before it has been validated or integrated into a causal graph.
///
/// Phase 1 stub type: `eve-neural`'s `extract_causal_relations` produces
/// these now so the trait surface matches the spec from day one, but no
/// Phase 1 component consumes them yet — that's Phase 2's `eve-causal` crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalRelationDraft {
    pub cause: String,
    pub effect: String,
    pub relation: CausalRelationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CausalRelationKind {
    Causes,
    Precedes,
    Enables,
    Prevents,
}
