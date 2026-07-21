// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal reasoning layer for TPT Eve.
//!
//! DAG-based causal graph construction, structural causal models with
//! multivariate OLS, intervention calculus (do-operator via mutilated graph),
//! counterfactual simulation (Pearl's abduction-action-prediction), and
//! confounder identification.
mod builder;
mod confounder;
mod counterfactual;
mod graph;
mod intervention;
mod layer;
mod observation;
mod scm;

pub use builder::CausalGraphBuilder;
pub use confounder::{estimate_causal_effect, identify_confounders};
pub use counterfactual::{simulate_counterfactual, Counterfactual, Simulation};
pub use graph::{CausalEdge, CausalGraph};
pub use intervention::{predict_intervention, Intervention, Prediction};
pub use layer::CausalReasoningEngine;
pub use observation::Observation;
pub use scm::StructuralCausalModel;
