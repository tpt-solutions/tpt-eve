// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;

use eve_core::{CausalRelationDraft, EveError};

use crate::{
    builder::CausalGraphBuilder,
    confounder::{estimate_causal_effect, identify_confounders},
    counterfactual::{simulate_counterfactual, Counterfactual, Simulation},
    graph::CausalGraph,
    intervention::{predict_intervention, Intervention, Prediction},
    observation::Observation,
    scm::StructuralCausalModel,
};

/// High-level facade matching the `CausalReasoningEngine` interface from spec.txt.
///
/// Typical usage:
/// ```text
/// let mut engine = CausalReasoningEngine::new();
/// engine.add_relation(draft);         // from NeuralPatternLayer::extract_causal_relations
/// engine.add_observation(obs);        // numeric data for SCM fitting
/// engine.fit();                       // fit linear SCM from observations
/// let p = engine.predict_intervention(intervention, "outcome", &baseline)?;
/// ```
#[derive(Debug, Default)]
pub struct CausalReasoningEngine {
    builder: CausalGraphBuilder,
    graph: CausalGraph,
    scm: Option<StructuralCausalModel>,
    fitted: bool,
}

impl CausalReasoningEngine {
    pub fn new() -> Self {
        CausalReasoningEngine::default()
    }

    /// Add a causal relation (typically from `NeuralPatternLayer::extract_causal_relations`).
    /// Must be called before `fit()`.
    pub fn add_relation(&mut self, relation: CausalRelationDraft) {
        self.builder.add_relation(relation);
        self.fitted = false;
    }

    /// Add an observation for SCM fitting. Must be called before `fit()`.
    pub fn add_observation(&mut self, obs: Observation) {
        self.builder.add_observation(obs);
        self.fitted = false;
    }

    /// Build the causal graph and fit the linear SCM. Calling this is required before any
    /// prediction, counterfactual, or effect-estimation method. It is idempotent (re-fitting
    /// rebuilds the graph from scratch — safe to call again after adding more relations).
    pub fn fit(&mut self) {
        // Drain the builder and replace internal state
        let mut tmp = CausalGraphBuilder::new();
        std::mem::swap(&mut self.builder, &mut tmp);
        let (graph, scm) = tmp.build();
        self.graph = graph;
        self.scm = scm;
        self.fitted = true;
    }

    /// Read-only access to the underlying causal graph (after `fit()`).
    pub fn graph(&self) -> &CausalGraph {
        &self.graph
    }

    /// Predict the expected value of `outcome` under `do(intervention.variable = value)`.
    pub fn predict_intervention(
        &self,
        intervention: Intervention,
        outcome: &str,
        baseline: &HashMap<String, f64>,
    ) -> Result<Prediction, EveError> {
        let scm = self.require_scm()?;
        Ok(predict_intervention(
            &self.graph,
            scm,
            &intervention,
            outcome,
            baseline,
        ))
    }

    /// Simulate the counterfactual: "what would have happened if…?"
    pub fn simulate_counterfactual(
        &self,
        counterfactual: Counterfactual,
    ) -> Result<Simulation, EveError> {
        let scm = self.require_scm()?;
        Ok(simulate_counterfactual(&self.graph, scm, &counterfactual))
    }

    /// Identify confounders of `(treatment, outcome)` from the DAG structure alone.
    pub fn identify_confounders(&self, treatment: &str, outcome: &str) -> Vec<String> {
        identify_confounders(&self.graph, treatment, outcome)
    }

    /// Estimate the total causal effect of `treatment` on `outcome` via the fitted SCM.
    pub fn estimate_causal_effect(&self, treatment: &str, outcome: &str) -> Result<f64, EveError> {
        let scm = self.require_scm()?;
        estimate_causal_effect(&self.graph, scm, treatment, outcome).ok_or_else(|| {
            EveError::Causal(format!(
                "no directed path from '{treatment}' to '{outcome}'"
            ))
        })
    }

    fn require_scm(&self) -> Result<&StructuralCausalModel, EveError> {
        self.scm.as_ref().ok_or_else(|| {
            EveError::Causal(
                "SCM not fitted — call fit() with observations before making predictions".into(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_core::CausalRelationKind;

    fn study_engine() -> CausalReasoningEngine {
        let mut engine = CausalReasoningEngine::new();
        engine.add_relation(CausalRelationDraft {
            cause: "studyHours".into(),
            effect: "examScore".into(),
            relation: CausalRelationKind::Causes,
        });
        for (h, s) in [(3.0, 65.0), (5.0, 85.0), (7.0, 105.0)] {
            engine.add_observation(Observation::from_pairs([
                ("studyHours", h),
                ("examScore", s),
            ]));
        }
        engine.fit();
        engine
    }

    #[test]
    fn predict_intervention_example() {
        let engine = study_engine();
        let pred = engine
            .predict_intervention(
                Intervention::new("studyHours", 10.0),
                "examScore",
                &HashMap::new(),
            )
            .unwrap();
        assert!((pred.expected_value - 135.0).abs() < 1.0);
        assert!(pred.confidence > 0.0);
    }

    #[test]
    fn counterfactual_example() {
        let engine = study_engine();
        let mut actual = HashMap::new();
        actual.insert("studyHours".to_string(), 5.0);
        actual.insert("examScore".to_string(), 85.0);
        let mut changes = HashMap::new();
        changes.insert("studyHours".to_string(), 7.0);
        let sim = engine
            .simulate_counterfactual(Counterfactual::new(actual, changes))
            .unwrap();
        let cf_score = sim.counterfactual_world["examScore"];
        assert!((cf_score - 105.0).abs() < 2.0, "got {cf_score}");
    }

    #[test]
    fn causal_effect_via_engine() {
        let engine = study_engine();
        let effect = engine
            .estimate_causal_effect("studyHours", "examScore")
            .unwrap();
        assert!((effect - 10.0).abs() < 0.5, "got {effect}");
    }

    #[test]
    fn predict_requires_fit_with_observations() {
        let mut engine = CausalReasoningEngine::new();
        engine.add_relation(CausalRelationDraft {
            cause: "x".into(),
            effect: "y".into(),
            relation: CausalRelationKind::Causes,
        });
        // fit without observations → no SCM
        engine.fit();
        let result = engine.predict_intervention(Intervention::new("x", 1.0), "y", &HashMap::new());
        assert!(result.is_err());
    }
}
