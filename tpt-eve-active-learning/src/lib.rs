// SPDX-License-Identifier: MIT OR Apache-2.0
//! Active learning layer for TPT Eve.
//!
//! Hypothesis generation from known facts, experiment design and execution
//! in a sandboxed logical environment, Bayesian belief updating, and
//! insight discovery — orchestrated by [`ActiveLearningEngine`].
mod belief;
mod experiment;
mod hypothesis;
mod layer;

pub use belief::{BeliefState, BeliefUpdater};
pub use experiment::{ExperimentDesign, ExperimentResult, ExperimentRunner};
pub use hypothesis::{Hypothesis, HypothesisGenerator};
pub use layer::ActiveLearningEngine;
