// SPDX-License-Identifier: MIT OR Apache-2.0
//! Symbolic reasoning layer for TPT Eve.
//!
//! Provides working memory (fact storage with indexed lookup), forward-chaining
//! inference via naive fixpoint recomputation, consistency checking, and
//! question answering. Fact persistence uses an embedded `redb` database at
//! `~/.tpt/eve/working_memory.redb`.
mod consistency;
mod inference;
mod layer;
mod lowering;
mod persistence;
mod rule;
mod working_memory;

pub mod qa;

pub use consistency::{ConsistencyChecker, Contradiction};
pub use inference::{Inference, InferenceEngine};
pub use layer::SymbolicReasoningLayer;
pub use lowering::{
    query_to_patterns, rule_to_runtime, term_to_pattern_term, term_to_value, triple_to_fact,
    triple_to_pattern,
};
pub use persistence::WorkingMemoryStore;
pub use rule::Rule;
pub use working_memory::WorkingMemory;
