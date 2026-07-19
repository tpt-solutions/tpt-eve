// SPDX-License-Identifier: MIT OR Apache-2.0
use serde::{Deserialize, Serialize};
use tpt_eve_core::Fact;

/// A fact paired with a mutable belief strength (the agent's current confidence
/// that the fact is true, independently of the fact's own `confidence` field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefState {
    pub fact: Fact,
    /// Probability that the fact is true, in [0.0, 1.0].
    pub belief: f32,
}

impl BeliefState {
    pub fn new(fact: Fact, initial_belief: f32) -> Self {
        BeliefState {
            fact,
            belief: initial_belief.clamp(0.0, 1.0),
        }
    }
}

/// Updates `BeliefState` values using Bayesian inference.
///
/// The updater uses fixed likelihood parameters that can be overridden:
/// - `p_evidence_given_true`:  P(E | H is true),  default 0.9
/// - `p_evidence_given_false`: P(E | H is false), default 0.1
///
/// These represent the reliability of the evidence source.  For a highly
/// reliable experiment lower `p_evidence_given_false`; for a noisy source
/// raise it.
#[derive(Debug, Clone)]
pub struct BeliefUpdater {
    pub p_evidence_given_true: f32,
    pub p_evidence_given_false: f32,
}

impl BeliefUpdater {
    pub fn new() -> Self {
        BeliefUpdater {
            p_evidence_given_true: 0.9,
            p_evidence_given_false: 0.1,
        }
    }

    /// Return a new `BeliefState` after incorporating *positive* evidence
    /// (an experiment or observation that supports the belief).
    pub fn update_positive(&self, state: &BeliefState) -> BeliefState {
        BeliefState {
            fact: state.fact.clone(),
            belief: self.posterior(state.belief, true),
        }
    }

    /// Return a new `BeliefState` after incorporating *negative* evidence
    /// (an experiment or observation that contradicts the belief).
    pub fn update_negative(&self, state: &BeliefState) -> BeliefState {
        BeliefState {
            fact: state.fact.clone(),
            belief: self.posterior(state.belief, false),
        }
    }

    /// Apply multiple rounds of evidence in sequence.
    ///
    /// `evidence` is a slice of `bool`s: `true` = positive, `false` = negative.
    pub fn update_sequence(&self, state: &BeliefState, evidence: &[bool]) -> BeliefState {
        evidence.iter().fold(state.clone(), |s, &positive| {
            if positive {
                self.update_positive(&s)
            } else {
                self.update_negative(&s)
            }
        })
    }

    // P(H | E) = P(E | H) * P(H)  /  [P(E|H)*P(H) + P(E|¬H)*(1-P(H))]
    fn posterior(&self, prior: f32, positive: bool) -> f32 {
        let (lh, lnh) = if positive {
            (self.p_evidence_given_true, self.p_evidence_given_false)
        } else {
            (
                1.0 - self.p_evidence_given_true,
                1.0 - self.p_evidence_given_false,
            )
        };
        let p_e = lh * prior + lnh * (1.0 - prior);
        (lh * prior / p_e).clamp(0.0, 1.0)
    }
}

impl Default for BeliefUpdater {
    fn default() -> Self {
        BeliefUpdater::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_eve_core::{Fact, Value};

    fn belief(confidence: f32) -> BeliefState {
        BeliefState::new(
            Fact::asserted(
                Value::Symbol("A".into()),
                Value::Symbol("has".into()),
                Value::Symbol("B".into()),
            ),
            confidence,
        )
    }

    #[test]
    fn positive_evidence_raises_belief() {
        let updater = BeliefUpdater::new();
        let before = belief(0.5);
        let after = updater.update_positive(&before);
        assert!(
            after.belief > before.belief,
            "positive evidence should raise belief: {} → {}",
            before.belief,
            after.belief
        );
    }

    #[test]
    fn negative_evidence_lowers_belief() {
        let updater = BeliefUpdater::new();
        let before = belief(0.5);
        let after = updater.update_negative(&before);
        assert!(
            after.belief < before.belief,
            "negative evidence should lower belief: {} → {}",
            before.belief,
            after.belief
        );
    }

    #[test]
    fn repeated_positive_evidence_converges_to_high_confidence() {
        let updater = BeliefUpdater::new();
        let initial = belief(0.5);
        let evidence: Vec<bool> = vec![true; 10];
        let final_state = updater.update_sequence(&initial, &evidence);
        assert!(
            final_state.belief > 0.95,
            "10 positive updates from 0.5 should exceed 0.95; got {}",
            final_state.belief
        );
    }

    #[test]
    fn repeated_negative_evidence_converges_to_low_confidence() {
        let updater = BeliefUpdater::new();
        let initial = belief(0.5);
        let evidence: Vec<bool> = vec![false; 10];
        let final_state = updater.update_sequence(&initial, &evidence);
        assert!(
            final_state.belief < 0.05,
            "10 negative updates from 0.5 should fall below 0.05; got {}",
            final_state.belief
        );
    }

    #[test]
    fn belief_stays_in_unit_interval() {
        let updater = BeliefUpdater::new();
        for &start in &[0.0_f32, 0.01, 0.5, 0.99, 1.0] {
            let b = belief(start);
            let pos = updater.update_positive(&b);
            let neg = updater.update_negative(&b);
            assert!((0.0..=1.0).contains(&pos.belief));
            assert!((0.0..=1.0).contains(&neg.belief));
        }
    }

    #[test]
    fn high_reliability_updater_converges_faster() {
        let standard = BeliefUpdater::new();
        let precise = BeliefUpdater {
            p_evidence_given_true: 0.99,
            p_evidence_given_false: 0.01,
        };
        let b = belief(0.5);
        assert!(precise.update_positive(&b).belief > standard.update_positive(&b).belief);
    }
}
