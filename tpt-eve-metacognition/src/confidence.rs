// SPDX-License-Identifier: MIT OR Apache-2.0
use serde::{Deserialize, Serialize};
use tpt_eve_core::{Fact, Value};

/// A single confidence record: the fact being tracked plus its confidence history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceRecord {
    pub fact: Fact,
    /// Confidence values in chronological order; the last entry is current.
    pub history: Vec<f32>,
}

impl ConfidenceRecord {
    fn new(fact: Fact) -> Self {
        let initial = fact.confidence;
        ConfidenceRecord {
            fact,
            history: vec![initial],
        }
    }

    /// Most-recent confidence value.
    pub fn current(&self) -> f32 {
        *self.history.last().expect("history is never empty")
    }

    /// Trend: positive = rising, negative = falling, zero = unchanged.
    pub fn trend(&self) -> f32 {
        if self.history.len() < 2 {
            return 0.0;
        }
        self.history.last().unwrap() - self.history[self.history.len() - 2]
    }
}

/// Tracks confidence over time for a collection of facts.
///
/// Facts are keyed by their (subject, predicate, object) triple so that
/// repeated calls to `track` for the same triple append to the history
/// rather than creating duplicates.
#[derive(Debug, Default)]
pub struct ConfidenceTracker {
    records: Vec<ConfidenceRecord>,
}

impl ConfidenceTracker {
    pub fn new() -> Self {
        ConfidenceTracker::default()
    }

    /// Begin (or continue) tracking `fact`.  If the triple already exists the
    /// fact's confidence is appended to the existing history.
    pub fn track(&mut self, fact: Fact) {
        if let Some(rec) = self.records.iter_mut().find(|r| r.fact.same_triple(&fact)) {
            rec.history.push(fact.confidence);
            rec.fact = fact; // update metadata (source, etc.)
        } else {
            self.records.push(ConfidenceRecord::new(fact));
        }
    }

    /// Explicitly update the confidence of an already-tracked fact.
    pub fn update(&mut self, fact: &Fact, new_confidence: f32) {
        let clamped = new_confidence.clamp(0.0, 1.0);
        if let Some(rec) = self.records.iter_mut().find(|r| r.fact.same_triple(fact)) {
            rec.history.push(clamped);
            rec.fact.confidence = clamped;
        }
    }

    /// All tracked records.
    pub fn records(&self) -> &[ConfidenceRecord] {
        &self.records
    }

    /// Records whose current confidence is below `threshold`.
    pub fn uncertain(&self, threshold: f32) -> Vec<&ConfidenceRecord> {
        self.records
            .iter()
            .filter(|r| r.current() < threshold)
            .collect()
    }

    /// Mean Expected Calibration Error (ECE) proxy: average absolute deviation
    /// of each fact's initial vs. current confidence, across all records with
    /// at least two data points.
    ///
    /// Returns 0.0 when no multi-point history exists (perfectly calibrated by
    /// default when there's nothing to measure).
    pub fn calibration_error(&self) -> f32 {
        let multi: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.history.len() >= 2)
            .collect();
        if multi.is_empty() {
            return 0.0;
        }
        let total: f32 = multi
            .iter()
            .map(|r| (r.history[0] - r.current()).abs())
            .sum();
        total / multi.len() as f32
    }

    /// All unique subjects that have at least one tracked fact.
    pub fn known_subjects(&self) -> Vec<&Value> {
        let mut subjects: Vec<&Value> = self.records.iter().map(|r| &r.fact.subject).collect();
        subjects.dedup_by(|a, b| a == b);
        subjects
    }

    /// Current confidence for the given triple, if tracked.
    pub fn confidence_of(&self, fact: &Fact) -> Option<f32> {
        self.records
            .iter()
            .find(|r| r.fact.same_triple(fact))
            .map(|r| r.current())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_eve_core::Value;

    fn fact(s: &str, p: &str, o: &str, conf: f32) -> Fact {
        let mut f = Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        );
        f.confidence = conf;
        f
    }

    #[test]
    fn tracking_adds_initial_confidence() {
        let mut tracker = ConfidenceTracker::new();
        tracker.track(fact("A", "is", "B", 0.7));
        assert_eq!(tracker.records().len(), 1);
        assert!((tracker.records()[0].current() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn tracking_same_triple_appends_history() {
        let mut tracker = ConfidenceTracker::new();
        tracker.track(fact("A", "is", "B", 0.5));
        tracker.track(fact("A", "is", "B", 0.8));
        assert_eq!(tracker.records().len(), 1);
        assert_eq!(tracker.records()[0].history.len(), 2);
        assert!((tracker.records()[0].current() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn update_appends_and_clamps() {
        let mut tracker = ConfidenceTracker::new();
        let f = fact("X", "has", "Y", 0.6);
        tracker.track(f.clone());
        tracker.update(&f, 1.5); // clamped to 1.0
        assert!((tracker.records()[0].current() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn uncertain_filters_below_threshold() {
        let mut tracker = ConfidenceTracker::new();
        tracker.track(fact("A", "is", "B", 0.9));
        tracker.track(fact("C", "is", "D", 0.3));
        let uncertain = tracker.uncertain(0.5);
        assert_eq!(uncertain.len(), 1);
        assert_eq!(uncertain[0].fact.subject, Value::Symbol("C".into()));
    }

    #[test]
    fn calibration_error_zero_for_single_observations() {
        let mut tracker = ConfidenceTracker::new();
        tracker.track(fact("A", "is", "B", 0.8));
        assert!((tracker.calibration_error()).abs() < 1e-6);
    }

    #[test]
    fn calibration_error_nonzero_after_update() {
        let mut tracker = ConfidenceTracker::new();
        let f = fact("A", "is", "B", 0.5);
        tracker.track(f.clone());
        tracker.update(&f, 0.9);
        assert!(tracker.calibration_error() > 0.0);
    }

    #[test]
    fn trend_positive_after_increase() {
        let mut tracker = ConfidenceTracker::new();
        let f = fact("A", "is", "B", 0.3);
        tracker.track(f.clone());
        tracker.update(&f, 0.8);
        assert!(tracker.records()[0].trend() > 0.0);
    }
}
