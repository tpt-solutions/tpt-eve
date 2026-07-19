// SPDX-License-Identifier: MIT OR Apache-2.0
use tpt_eve_core::{Fact, FactSource};

use crate::{
    belief::{BeliefState, BeliefUpdater},
    experiment::{ExperimentDesign, ExperimentResult, ExperimentRunner},
    hypothesis::{Hypothesis, HypothesisGenerator},
};

/// High-level facade for the Phase 4 Active Learning layer.
///
/// Typical usage:
/// ```text
/// let mut engine = ActiveLearningEngine::new();
/// engine.add_fact(known_fact);
/// let hypotheses = engine.generate_hypotheses();
/// let design    = engine.design_experiment(&hypotheses[0]);
/// let result    = engine.run_experiment(design);
/// engine.update_belief(&mut hypotheses[0], &result);
/// let insights  = engine.discover_insights(0.8);
/// ```
#[derive(Debug, Default)]
pub struct ActiveLearningEngine {
    known_facts: Vec<Fact>,
    belief_updater: BeliefUpdater,
}

impl ActiveLearningEngine {
    pub fn new() -> Self {
        ActiveLearningEngine::default()
    }

    /// Register a known fact as background knowledge.
    pub fn add_fact(&mut self, fact: Fact) {
        self.known_facts.push(fact);
    }

    /// Generate testable hypotheses from all currently known facts.
    pub fn generate_hypotheses(&self) -> Vec<Hypothesis> {
        HypothesisGenerator::new().generate(&self.known_facts)
    }

    /// Create an experiment design that can be used to test `hypothesis`.
    ///
    /// The design ships the hypothesis prediction as the *only* test observation
    /// so that the caller can substitute real observations before running.
    pub fn design_experiment(&self, hypothesis: &Hypothesis) -> ExperimentDesign {
        ExperimentDesign::new(
            format!("verify: {}", hypothesis.description),
            hypothesis.clone(),
            vec![hypothesis.prediction.clone()],
        )
    }

    /// Run an experiment and return the result.
    pub fn run_experiment(&self, design: ExperimentDesign) -> ExperimentResult {
        ExperimentRunner::new().run(&design)
    }

    /// Apply the experiment result to update a hypothesis's confidence.
    pub fn update_belief(&self, hypothesis: &mut Hypothesis, result: &ExperimentResult) {
        let state = BeliefState::new(hypothesis.prediction.clone(), hypothesis.confidence);
        let updated = if result.confirmed {
            self.belief_updater.update_positive(&state)
        } else if !result.conflicting.is_empty() {
            self.belief_updater.update_negative(&state)
        } else {
            // Inconclusive: use the confidence already computed by the runner.
            BeliefState::new(hypothesis.prediction.clone(), result.updated_confidence)
        };
        hypothesis.confidence = updated.belief;
        hypothesis.prediction.confidence = updated.belief;
    }

    /// Return facts derived from high-confidence hypotheses (confidence ≥ `threshold`).
    ///
    /// These are the "new insights" the active learning cycle has discovered: predictions
    /// that have been confirmed strongly enough to be treated as facts.
    pub fn discover_insights(&self, threshold: f32) -> Vec<Fact> {
        HypothesisGenerator::new()
            .generate(&self.known_facts)
            .into_iter()
            .filter(|h| h.confidence >= threshold)
            .map(|h| Fact {
                source: FactSource::Extracted {
                    confidence: h.confidence,
                },
                confidence: h.confidence,
                ..h.prediction
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_eve_core::Value;

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    #[test]
    fn engine_generates_hypotheses_from_known_facts() {
        let mut engine = ActiveLearningEngine::new();
        engine.add_fact(fact("TypeScript", "is", "programming_language"));
        engine.add_fact(fact("programming_language", "has", "syntax"));
        let hs = engine.generate_hypotheses();
        assert!(!hs.is_empty(), "should produce at least one hypothesis");
    }

    #[test]
    fn engine_designs_experiment_for_hypothesis() {
        let mut engine = ActiveLearningEngine::new();
        engine.add_fact(fact("TypeScript", "is", "programming_language"));
        engine.add_fact(fact("programming_language", "has", "syntax"));
        let hs = engine.generate_hypotheses();
        let design = engine.design_experiment(&hs[0]);
        assert_eq!(design.hypothesis.description, hs[0].description);
    }

    #[test]
    fn positive_experiment_increases_hypothesis_confidence() {
        let mut engine = ActiveLearningEngine::new();
        engine.add_fact(fact("TypeScript", "is", "programming_language"));
        engine.add_fact(fact("programming_language", "has", "syntax"));
        let hs = engine.generate_hypotheses();
        let mut h = hs
            .into_iter()
            .find(|h| h.prediction.subject == Value::Symbol("TypeScript".into()))
            .expect("chain hypothesis must exist");
        let before = h.confidence;
        let design = engine.design_experiment(&h);
        let result = engine.run_experiment(design);
        engine.update_belief(&mut h, &result);
        assert!(
            h.confidence > before,
            "confirmed experiment should raise confidence: {} → {}",
            before,
            h.confidence
        );
    }

    #[test]
    fn discover_insights_respects_threshold() {
        let mut engine = ActiveLearningEngine::new();
        engine.add_fact(fact("TypeScript", "is", "programming_language"));
        engine.add_fact(fact("programming_language", "has", "syntax"));
        // Threshold above initial hypothesis confidence (0.6) → no insights yet.
        let insights = engine.discover_insights(0.9);
        assert!(
            insights.is_empty(),
            "threshold 0.9 should return no insights when all hypotheses are at 0.6"
        );
        // Lower threshold → insights returned.
        let insights = engine.discover_insights(0.5);
        assert!(
            !insights.is_empty(),
            "threshold 0.5 should return chain-completion insights"
        );
    }
}
