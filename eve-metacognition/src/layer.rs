// SPDX-License-Identifier: Apache-2.0
use eve_core::Fact;

use crate::{
    confidence::{ConfidenceRecord, ConfidenceTracker},
    knowledge_gap::{KnowledgeGap, KnowledgeGapDetector},
    learning_priority::{LearningPrioritizer, LearningPriority},
    monitoring::{ClarificationRequest, UnderstandingLevel, UnderstandingMonitor},
};

/// High-level facade for the Phase 5 Meta-Cognition layer.
///
/// Typical usage:
/// ```text
/// let mut engine = MetaCognitionEngine::new();
/// engine.add_fact(known_fact);
/// engine.update_confidence(&some_fact, 0.9);
/// let gaps = engine.detect_gaps(0.6);
/// let agenda = engine.prioritize_learning(&gaps);
/// let level  = engine.assess_understanding();
/// let qs     = engine.request_clarifications(3);
/// ```
#[derive(Debug, Default)]
pub struct MetaCognitionEngine {
    tracker: ConfidenceTracker,
    /// Threshold below which a fact is treated as low-confidence.
    pub confidence_threshold: f32,
}

impl MetaCognitionEngine {
    pub fn new() -> Self {
        MetaCognitionEngine {
            tracker: ConfidenceTracker::new(),
            confidence_threshold: 0.6,
        }
    }

    /// Register a fact for tracking.  If the same triple was already tracked,
    /// the new confidence value is appended to its history.
    pub fn add_fact(&mut self, fact: Fact) {
        self.tracker.track(fact);
    }

    /// Explicitly revise the tracked confidence of a fact (e.g., after new
    /// evidence arrives from the active-learning layer).
    pub fn update_confidence(&mut self, fact: &Fact, new_confidence: f32) {
        self.tracker.update(fact, new_confidence);
    }

    /// Return all tracked confidence records.
    pub fn records(&self) -> &[ConfidenceRecord] {
        self.tracker.records()
    }

    /// Mean Expected Calibration Error across all multi-observation facts.
    pub fn calibration_error(&self) -> f32 {
        self.tracker.calibration_error()
    }

    /// Detect knowledge gaps at `self.confidence_threshold`.
    pub fn detect_gaps(&self, threshold: f32) -> Vec<KnowledgeGap> {
        let facts: Vec<Fact> = self
            .tracker
            .records()
            .iter()
            .map(|r| {
                let mut f = r.fact.clone();
                f.confidence = r.current();
                f
            })
            .collect();
        KnowledgeGapDetector::new().detect(&facts, threshold)
    }

    /// Rank the supplied gaps into a learning agenda.
    pub fn prioritize_learning(&self, gaps: &[KnowledgeGap]) -> Vec<LearningPriority> {
        LearningPrioritizer::new().prioritize(gaps)
    }

    /// Assess the overall understanding level of the current knowledge base.
    pub fn assess_understanding(&self) -> UnderstandingLevel {
        let facts: Vec<Fact> = self
            .tracker
            .records()
            .iter()
            .map(|r| {
                let mut f = r.fact.clone();
                f.confidence = r.current();
                f
            })
            .collect();
        let gaps = KnowledgeGapDetector::new().detect(&facts, self.confidence_threshold);
        UnderstandingMonitor::new().assess(&facts, &gaps)
    }

    /// Generate clarification questions for the top `max_requests` knowledge
    /// gaps ordered by urgency.
    pub fn request_clarifications(&self, max_requests: usize) -> Vec<ClarificationRequest> {
        let gaps = self.detect_gaps(self.confidence_threshold);
        UnderstandingMonitor::new().clarifications(&gaps, max_requests)
    }

    /// Facts whose current confidence is below `threshold`.
    pub fn uncertain_facts(&self, threshold: f32) -> Vec<&ConfidenceRecord> {
        self.tracker.uncertain(threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_core::Value;

    fn fact(s: &str, p: &str, o: &str) -> Fact {
        Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        )
    }

    fn low_conf(s: &str, p: &str, o: &str, conf: f32) -> Fact {
        let mut f = fact(s, p, o);
        f.confidence = conf;
        f
    }

    #[test]
    fn add_fact_is_tracked() {
        let mut engine = MetaCognitionEngine::new();
        engine.add_fact(fact("A", "is", "B"));
        assert_eq!(engine.records().len(), 1);
    }

    #[test]
    fn update_confidence_changes_current() {
        let mut engine = MetaCognitionEngine::new();
        let f = fact("A", "is", "B");
        engine.add_fact(f.clone());
        engine.update_confidence(&f, 0.95);
        assert!((engine.records()[0].current() - 0.95).abs() < 1e-6);
    }

    #[test]
    fn detect_gaps_surfaces_low_confidence() {
        let mut engine = MetaCognitionEngine::new();
        engine.add_fact(low_conf("A", "is", "B", 0.2));
        let gaps = engine.detect_gaps(0.5);
        assert!(!gaps.is_empty(), "should detect low-confidence gap");
    }

    #[test]
    fn prioritize_learning_returns_ranked_list() {
        let mut engine = MetaCognitionEngine::new();
        engine.add_fact(low_conf("A", "is", "B", 0.2));
        engine.add_fact(fact("TypeScript", "is", "programming_language"));
        engine.add_fact(fact("Python", "is", "programming_language"));
        engine.add_fact(fact("TypeScript", "has", "syntax"));
        let gaps = engine.detect_gaps(0.5);
        let agenda = engine.prioritize_learning(&gaps);
        // Ranks must be 1, 2, 3, ...
        for (i, p) in agenda.iter().enumerate() {
            assert_eq!(p.rank, i + 1);
        }
    }

    #[test]
    fn assess_understanding_low_when_empty() {
        let engine = MetaCognitionEngine::new();
        assert_eq!(engine.assess_understanding(), UnderstandingLevel::Low);
    }

    #[test]
    fn assess_understanding_high_with_good_facts() {
        let mut engine = MetaCognitionEngine::new();
        // Three fully-described entities — no unexplained objects.
        let mut f1 = fact("A", "describes", "B");
        f1.confidence = 0.95;
        let mut f2 = fact("B", "describes", "C");
        f2.confidence = 0.90;
        let mut f3 = fact("C", "describes", "A");
        f3.confidence = 0.92;
        engine.add_fact(f1);
        engine.add_fact(f2);
        engine.add_fact(f3);
        assert_eq!(engine.assess_understanding(), UnderstandingLevel::High);
    }

    #[test]
    fn request_clarifications_is_capped() {
        let mut engine = MetaCognitionEngine::new();
        // Ten low-confidence facts each generating a gap.
        for i in 0..10_u32 {
            engine.add_fact(low_conf(&format!("E{i}"), "is", "category", 0.1));
        }
        let qs = engine.request_clarifications(3);
        assert!(qs.len() <= 3);
    }

    #[test]
    fn calibration_error_increases_after_update() {
        let mut engine = MetaCognitionEngine::new();
        let f = fact("A", "is", "B");
        engine.add_fact(f.clone());
        let before = engine.calibration_error();
        engine.update_confidence(&f, 0.99);
        let after = engine.calibration_error();
        assert!(
            after > before,
            "calibration error should increase after update: {before} → {after}"
        );
    }
}
