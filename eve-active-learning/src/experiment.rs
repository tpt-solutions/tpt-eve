// SPDX-License-Identifier: Apache-2.0
use eve_core::Fact;
use serde::{Deserialize, Serialize};

use crate::hypothesis::Hypothesis;

/// Describes a concrete experiment that can verify or refute a hypothesis.
///
/// The experiment provides a set of *test observations* — new facts gathered (or simulated)
/// specifically to check whether the hypothesis prediction holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDesign {
    /// Human-readable name for this experiment.
    pub name: String,
    /// The hypothesis being tested.
    pub hypothesis: Hypothesis,
    /// New observations to evaluate against the prediction.
    pub test_observations: Vec<Fact>,
}

impl ExperimentDesign {
    pub fn new(
        name: impl Into<String>,
        hypothesis: Hypothesis,
        test_observations: Vec<Fact>,
    ) -> Self {
        ExperimentDesign {
            name: name.into(),
            hypothesis,
            test_observations,
        }
    }
}

/// The outcome of running an `ExperimentDesign`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Whether the prediction was confirmed by the test observations.
    pub confirmed: bool,
    /// Facts from the test observations that directly support the prediction.
    pub supporting: Vec<Fact>,
    /// Facts that directly contradict the prediction (same subject+predicate, different object).
    pub conflicting: Vec<Fact>,
    /// Updated confidence for the hypothesis based on this result.
    pub updated_confidence: f32,
}

/// Runs an `ExperimentDesign` and produces an `ExperimentResult`.
///
/// Confirmation logic:
/// - If any test observation matches the predicted triple exactly → *confirmed*.
/// - If any test observation shares (subject, predicate) but has a different object → *conflicting*.
/// - Otherwise → inconclusive (not confirmed, no conflict).
pub struct ExperimentRunner;

impl ExperimentRunner {
    pub fn new() -> Self {
        ExperimentRunner
    }

    pub fn run(&self, design: &ExperimentDesign) -> ExperimentResult {
        let pred = &design.hypothesis.prediction;
        let prior = design.hypothesis.confidence;

        let supporting: Vec<Fact> = design
            .test_observations
            .iter()
            .filter(|obs| obs.same_triple(pred))
            .cloned()
            .collect();

        let conflicting: Vec<Fact> = design
            .test_observations
            .iter()
            .filter(|obs| {
                obs.subject == pred.subject
                    && obs.predicate == pred.predicate
                    && obs.object != pred.object
            })
            .cloned()
            .collect();

        let confirmed = !supporting.is_empty();
        let updated_confidence = if confirmed {
            bayesian_update(prior, true)
        } else if !conflicting.is_empty() {
            bayesian_update(prior, false)
        } else {
            // Inconclusive: slight decay toward the prior
            prior * 0.95
        };

        ExperimentResult {
            confirmed,
            supporting,
            conflicting,
            updated_confidence,
        }
    }
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        ExperimentRunner::new()
    }
}

/// Simple Naive-Bayes update with P(E|H)=0.9, P(E|¬H)=0.1.
fn bayesian_update(prior: f32, positive_evidence: bool) -> f32 {
    let (likelihood_h, likelihood_not_h) = if positive_evidence {
        (0.9_f32, 0.1_f32)
    } else {
        (0.1_f32, 0.9_f32)
    };
    let p_evidence = likelihood_h * prior + likelihood_not_h * (1.0 - prior);
    (likelihood_h * prior / p_evidence).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::Hypothesis;
    use eve_core::{Fact, FactSource, Value};

    fn sym(s: &str) -> Value {
        Value::Symbol(s.into())
    }

    fn make_hypothesis(s: &str, p: &str, o: &str) -> Hypothesis {
        Hypothesis {
            description: format!("{s} {p} {o}"),
            prediction: Fact {
                subject: sym(s),
                predicate: sym(p),
                object: sym(o),
                source: FactSource::Extracted { confidence: 0.6 },
                confidence: 0.6,
            },
            confidence: 0.6,
            supporting_facts: vec![],
        }
    }

    #[test]
    fn confirmed_when_observation_matches_prediction() {
        let h = make_hypothesis("Python", "has", "syntax");
        let obs = vec![Fact::asserted(sym("Python"), sym("has"), sym("syntax"))];
        let design = ExperimentDesign::new("test", h, obs);
        let result = ExperimentRunner::new().run(&design);
        assert!(result.confirmed);
        assert!(!result.supporting.is_empty());
        assert!(result.conflicting.is_empty());
        assert!(result.updated_confidence > 0.6);
    }

    #[test]
    fn conflicting_when_observation_contradicts_prediction() {
        let h = make_hypothesis("Python", "has", "syntax");
        let obs = vec![Fact::asserted(sym("Python"), sym("has"), sym("no_syntax"))];
        let design = ExperimentDesign::new("test", h, obs);
        let result = ExperimentRunner::new().run(&design);
        assert!(!result.confirmed);
        assert!(!result.conflicting.is_empty());
        assert!(result.updated_confidence < 0.6);
    }

    #[test]
    fn inconclusive_when_no_relevant_observation() {
        let h = make_hypothesis("Python", "has", "syntax");
        let obs = vec![Fact::asserted(sym("Java"), sym("is"), sym("language"))];
        let design = ExperimentDesign::new("test", h, obs);
        let result = ExperimentRunner::new().run(&design);
        assert!(!result.confirmed);
        assert!(result.conflicting.is_empty());
        // Confidence decays slightly but remains close to prior.
        assert!(result.updated_confidence < 0.6);
        assert!(result.updated_confidence > 0.5);
    }
}
