// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared types for the TPT Eve cognitive architecture.
//!
//! Defines the core vocabulary used across all Eve crates:
//!
//! - [`Value`] — a typed atom (`Symbol` or `Text`).
//! - [`Fact`] — a subject–predicate–object triple with provenance and confidence.
//! - [`Pattern`] — a triple where any slot may be bound or variable.
//! - [`EveError`] — the shared error type.
//! - [`CausalRelationDraft`] / [`CausalRelationKind`] — Phase 2 causal-relation types
//!   defined here so `tpt-eve-neural`'s trait surface matches the spec from day one.
mod causal_draft;
mod error;
mod fact;
mod pattern;

pub use causal_draft::{CausalRelationDraft, CausalRelationKind};
pub use error::EveError;
pub use fact::{Fact, FactSource, Value};
pub use pattern::{Pattern, PatternTerm};
