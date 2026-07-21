use tpt_eve_core::{Fact, Value};
use tpt_eve_metacognition::{
    ConfidenceTracker, GapType, KnowledgeGapDetector, LearningPrioritizer, MetaCognitionEngine,
    UnderstandingLevel, UnderstandingMonitor,
};

// Phase 5 acceptance tests: confidence tracking, knowledge-gap identification,
// learning prioritization, and confidence calibration.
// All tests run without a live LLM.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fact(s: &str, p: &str, o: &str) -> Fact {
    Fact::asserted(
        Value::Symbol(s.into()),
        Value::Symbol(p.into()),
        Value::Symbol(o.into()),
    )
}

fn conf_fact(s: &str, p: &str, o: &str, conf: f32) -> Fact {
    let mut f = fact(s, p, o);
    f.confidence = conf;
    f
}

/// Standard set: TypeScript + Python are programming languages; TypeScript has
/// a type system at low confidence; syntax is unexplained (only appears as object).
fn base_facts() -> Vec<Fact> {
    vec![
        conf_fact("TypeScript", "is", "programming_language", 0.95),
        conf_fact("Python", "is", "programming_language", 0.90),
        conf_fact("programming_language", "has", "syntax", 0.85),
        conf_fact("TypeScript", "has", "type_system", 0.35),
    ]
}

// ---------------------------------------------------------------------------
// Success criterion 1: confidence tracking accuracy
// ---------------------------------------------------------------------------

/// All tracked facts appear in the records at their initial confidence.
#[test]
fn all_facts_are_tracked() {
    let mut tracker = ConfidenceTracker::new();
    for f in base_facts() {
        tracker.track(f);
    }
    assert_eq!(
        tracker.records().len(),
        4,
        "all four facts should be tracked"
    );
}

/// Re-tracking the same triple appends to history without creating a duplicate.
#[test]
fn re_tracking_same_triple_appends_history() {
    let mut tracker = ConfidenceTracker::new();
    tracker.track(conf_fact("A", "is", "B", 0.5));
    tracker.track(conf_fact("A", "is", "B", 0.8));
    assert_eq!(tracker.records().len(), 1, "must not create duplicate");
    assert_eq!(
        tracker.records()[0].history.len(),
        2,
        "history must have two entries"
    );
    assert!((tracker.records()[0].current() - 0.8).abs() < 1e-6);
}

/// `update` appends the new value and clamps it to [0, 1].
#[test]
fn explicit_update_clamps_and_appends() {
    let mut tracker = ConfidenceTracker::new();
    let f = conf_fact("A", "is", "B", 0.5);
    tracker.track(f.clone());
    tracker.update(&f, 2.0); // out-of-range → clamped to 1.0
    assert!((tracker.records()[0].current() - 1.0).abs() < 1e-6);
}

/// `uncertain` returns only records below the threshold.
#[test]
fn uncertain_returns_only_low_confidence_records() {
    let mut tracker = ConfidenceTracker::new();
    for f in base_facts() {
        tracker.track(f);
    }
    let uncertain = tracker.uncertain(0.5);
    assert_eq!(
        uncertain.len(),
        1,
        "only TypeScript-has-type_system (0.35) should be below 0.5"
    );
    assert_eq!(uncertain[0].fact.predicate, Value::Symbol("has".into()));
}

/// Trend is positive after a confidence increase.
#[test]
fn trend_is_positive_after_increase() {
    let mut tracker = ConfidenceTracker::new();
    let f = conf_fact("A", "is", "B", 0.3);
    tracker.track(f.clone());
    tracker.update(&f, 0.9);
    assert!(tracker.records()[0].trend() > 0.0);
}

// ---------------------------------------------------------------------------
// Success criterion 2: knowledge-gap identification
// ---------------------------------------------------------------------------

/// Low-confidence facts are surfaced as `LowConfidenceFact` gaps.
#[test]
fn low_confidence_fact_detected_as_gap() {
    let gaps = KnowledgeGapDetector::new().detect(&base_facts(), 0.5);
    assert!(
        gaps.iter()
            .any(|g| g.gap_type == GapType::LowConfidenceFact),
        "TypeScript-has-type_system (conf 0.35) must be flagged; got: {gaps:?}"
    );
}

/// Entities appearing only as objects are flagged as `UnexplainedEntity`.
#[test]
fn unexplained_entity_detected() {
    let facts = vec![
        fact("TypeScript", "has", "syntax"),
        fact("TypeScript", "has", "type_system"),
    ];
    let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
    let unexplained: Vec<_> = gaps
        .iter()
        .filter(|g| g.gap_type == GapType::UnexplainedEntity)
        .collect();
    assert!(
        !unexplained.is_empty(),
        "objects without outgoing facts must be flagged; got: {gaps:?}"
    );
}

/// Category peers missing an attribute are flagged as `MissingAttribute`.
#[test]
fn missing_attribute_detected_for_category_peer() {
    let facts = vec![
        fact("TypeScript", "is", "programming_language"),
        fact("Python", "is", "programming_language"),
        fact("TypeScript", "has", "type_system"),
    ];
    let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
    let missing = gaps.iter().find(|g| {
        g.gap_type == GapType::MissingAttribute && g.subject == Value::Symbol("Python".into())
    });
    assert!(
        missing.is_some(),
        "Python missing 'has type_system' must be detected; got: {gaps:?}"
    );
}

/// Fully-described, high-confidence facts produce no low-confidence gaps.
#[test]
fn no_low_confidence_gaps_for_high_confidence_facts() {
    let facts = vec![
        conf_fact("A", "describes", "B", 0.95),
        conf_fact("B", "describes", "C", 0.90),
        conf_fact("C", "describes", "A", 0.92),
    ];
    let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
    let lc: Vec<_> = gaps
        .iter()
        .filter(|g| g.gap_type == GapType::LowConfidenceFact)
        .collect();
    assert!(
        lc.is_empty(),
        "no low-confidence gaps expected; got: {lc:?}"
    );
}

// ---------------------------------------------------------------------------
// Success criterion 3: learning prioritization
// ---------------------------------------------------------------------------

/// Low-confidence gaps are ranked before missing-attribute gaps.
#[test]
fn low_confidence_gap_outranks_missing_attribute() {
    let facts = vec![
        conf_fact("A", "is", "category", 0.2), // low confidence
        fact("B", "is", "category"),
        fact("A", "has", "prop"),
        // B missing 'has prop' → MissingAttribute gap
    ];
    let gaps = KnowledgeGapDetector::new().detect(&facts, 0.5);
    let agenda = LearningPrioritizer::new().prioritize(&gaps);
    assert!(!agenda.is_empty(), "must produce a learning agenda");
    assert_eq!(
        agenda[0].gap.gap_type,
        GapType::LowConfidenceFact,
        "LowConfidenceFact must rank first; got {:?}",
        agenda[0].gap.gap_type
    );
}

/// Ranks are 1-based and contiguous.
#[test]
fn ranks_are_one_based_and_contiguous() {
    let gaps = KnowledgeGapDetector::new().detect(&base_facts(), 0.5);
    let agenda = LearningPrioritizer::new().prioritize(&gaps);
    for (i, item) in agenda.iter().enumerate() {
        assert_eq!(item.rank, i + 1, "rank must equal index+1");
    }
}

/// Empty gap list produces empty agenda.
#[test]
fn empty_gaps_produce_empty_agenda() {
    assert!(LearningPrioritizer::new().prioritize(&[]).is_empty());
}

// ---------------------------------------------------------------------------
// Success criterion 4: confidence calibration
// ---------------------------------------------------------------------------

/// Calibration error is 0 when only a single observation per fact exists.
#[test]
fn calibration_error_zero_with_single_observations() {
    let mut tracker = ConfidenceTracker::new();
    for f in base_facts() {
        tracker.track(f);
    }
    assert!(
        tracker.calibration_error() < 1e-6,
        "calibration error must be 0 with no updates"
    );
}

/// Calibration error is nonzero after updating a fact's confidence.
#[test]
fn calibration_error_nonzero_after_update() {
    let mut tracker = ConfidenceTracker::new();
    let f = conf_fact("A", "is", "B", 0.5);
    tracker.track(f.clone());
    tracker.update(&f, 0.95);
    assert!(
        tracker.calibration_error() > 0.0,
        "calibration error should be nonzero after update"
    );
}

/// Calibration error equals the mean |initial − current| across updated facts.
#[test]
fn calibration_error_matches_mean_absolute_deviation() {
    let mut tracker = ConfidenceTracker::new();
    let f1 = conf_fact("A", "is", "B", 0.5);
    let f2 = conf_fact("C", "is", "D", 0.4);
    tracker.track(f1.clone());
    tracker.track(f2.clone());
    tracker.update(&f1, 0.9); // deviation 0.4
    tracker.update(&f2, 0.8); // deviation 0.4
    let err = tracker.calibration_error();
    assert!(
        (err - 0.4).abs() < 1e-5,
        "expected calibration error 0.4; got {err}"
    );
}

// ---------------------------------------------------------------------------
// End-to-end via MetaCognitionEngine facade
// ---------------------------------------------------------------------------

/// Full meta-cognition cycle through the engine facade.
#[test]
fn full_metacognition_cycle() {
    let mut engine = MetaCognitionEngine::new();
    for f in base_facts() {
        engine.add_fact(f);
    }

    // Confidence starts tracked
    assert_eq!(engine.records().len(), 4);

    // Update one low-confidence fact
    let low_f = conf_fact("TypeScript", "has", "type_system", 0.35);
    engine.update_confidence(&low_f, 0.88);
    assert!(engine.calibration_error() > 0.0);

    // Gap detection
    let gaps = engine.detect_gaps(0.6);
    assert!(!gaps.is_empty(), "gaps must exist before update propagates");

    // Learning agenda
    let agenda = engine.prioritize_learning(&gaps);
    assert!(!agenda.is_empty());
    for (i, item) in agenda.iter().enumerate() {
        assert_eq!(item.rank, i + 1);
    }

    // Understanding level is deterministic
    let level = engine.assess_understanding();
    assert!(
        matches!(
            level,
            UnderstandingLevel::High | UnderstandingLevel::Medium | UnderstandingLevel::Low
        ),
        "level must be one of the three variants"
    );

    // Clarification requests are capped
    let qs = engine.request_clarifications(2);
    assert!(qs.len() <= 2);
}

/// Understanding is Low when no facts are tracked.
#[test]
fn understanding_low_with_no_facts() {
    let engine = MetaCognitionEngine::new();
    assert_eq!(engine.assess_understanding(), UnderstandingLevel::Low);
}

/// Understanding is High with many high-confidence facts and few gaps.
#[test]
fn understanding_high_with_strong_knowledge_base() {
    let mut engine = MetaCognitionEngine::new();
    // Fully described chain — no unexplained objects.
    for (s, p, o) in &[
        ("A", "describes", "B"),
        ("B", "describes", "C"),
        ("C", "describes", "A"),
    ] {
        engine.add_fact(conf_fact(s, p, o, 0.95));
    }
    assert_eq!(engine.assess_understanding(), UnderstandingLevel::High);
}

/// Monitor generates clarification questions tailored to gap type.
#[test]
fn monitor_clarification_questions_are_nonempty() {
    let mut engine = MetaCognitionEngine::new();
    engine.add_fact(conf_fact("A", "is", "B", 0.2));
    let qs = engine.request_clarifications(5);
    assert!(
        !qs.is_empty(),
        "should produce at least one clarification question"
    );
    for q in &qs {
        assert!(
            !q.question.is_empty(),
            "clarification question must not be empty"
        );
    }
}

/// `UnderstandingMonitor::assess` returns Low for empty facts, regardless of gaps.
#[test]
fn monitor_low_for_empty_facts() {
    let level = UnderstandingMonitor::new().assess(&[], &[]);
    assert_eq!(level, UnderstandingLevel::Low);
}
