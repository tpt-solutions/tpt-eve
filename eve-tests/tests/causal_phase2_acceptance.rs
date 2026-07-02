use std::collections::HashMap;

use eve_causal::{CausalReasoningEngine, Counterfactual, Intervention, Observation};
use eve_core::{CausalRelationDraft, CausalRelationKind};
use eve_neural::{MockNeuralLayer, NeuralPatternLayer};

/// Phase 2 acceptance tests: build causal graphs, predict interventions, answer "what if".
///
/// Uses the study-hours → exam-score domain from spec.txt.

fn build_study_engine() -> CausalReasoningEngine {
    let mut engine = CausalReasoningEngine::new();
    engine.add_relation(CausalRelationDraft {
        cause: "studyHours".into(),
        effect: "examScore".into(),
        relation: CausalRelationKind::Causes,
    });
    engine.add_relation(CausalRelationDraft {
        cause: "sleep".into(),
        effect: "examScore".into(),
        relation: CausalRelationKind::Enables,
    });
    // True model: examScore = 29 + 10*studyHours + 1*sleep
    // Observations with independent variation in both parents so multivariate OLS can
    // identify each coefficient separately (avoids the collinearity trap).
    let data: &[(f64, f64, f64)] = &[
        (3.0, 6.0, 65.0),  // 29+30+6
        (3.0, 9.0, 68.0),  // 29+30+9
        (7.0, 6.0, 105.0), // 29+70+6
        (7.0, 9.0, 108.0), // 29+70+9
        (5.0, 7.5, 86.5),  // 29+50+7.5
    ];
    for &(h, s, score) in data {
        engine.add_observation(Observation::from_pairs([
            ("studyHours", h),
            ("sleep", s),
            ("examScore", score),
        ]));
    }
    engine.fit();
    engine
}

/// Success criterion 1: can build causal graphs from data.
#[test]
fn builds_causal_graph_from_observations() {
    let engine = build_study_engine();
    let graph = engine.graph();
    assert!(
        graph.nodes.contains("studyHours"),
        "studyHours node must exist"
    );
    assert!(
        graph.nodes.contains("examScore"),
        "examScore node must exist"
    );
    assert!(graph.nodes.contains("sleep"), "sleep node must exist");
    assert_eq!(graph.edges.len(), 2, "should have exactly 2 edges");
    assert!(!graph.has_cycle(), "causal graph must be a DAG");

    // Parents of examScore should include studyHours and sleep
    let parents: Vec<_> = graph
        .parents_of("examScore")
        .iter()
        .map(|e| e.from.as_str())
        .collect();
    assert!(parents.contains(&"studyHours"));
    assert!(parents.contains(&"sleep"));
}

/// Success criterion 2: can predict effects of interventions.
#[test]
fn predicts_intervention_effects() {
    let engine = build_study_engine();
    let baseline = HashMap::from([("sleep".to_string(), 7.5_f64)]);

    // Model: examScore = 29 + 10*studyHours + 1*sleep
    // do(studyHours=10), sleep=7.5 → 29 + 100 + 7.5 = 136.5
    let pred = engine
        .predict_intervention(
            Intervention::new("studyHours", 10.0),
            "examScore",
            &baseline,
        )
        .expect("prediction should succeed");

    assert_eq!(pred.outcome_variable, "examScore");
    assert!(
        (pred.expected_value - 136.5).abs() < 2.0,
        "expected ~136.5, got {}",
        pred.expected_value
    );
    assert!(pred.confidence > 0.0, "confidence must be positive");
    assert!(pred.confidence <= 1.0, "confidence must be ≤ 1.0");
}

/// Success criterion 3a: can answer "what if" questions via counterfactual simulation.
#[test]
fn answers_what_if_questions() {
    let engine = build_study_engine();

    // What if we had studied 7h instead of 5h (actual: 5h, sleep=7.5, score=86.5)?
    // Model: 29 + 10*studyHours + 1*sleep → noise for this individual ≈ 0
    // CF: 29 + 10*7 + 1*7.5 + 0 = 29 + 70 + 7.5 = 106.5
    let actual = HashMap::from([
        ("studyHours".to_string(), 5.0_f64),
        ("sleep".to_string(), 7.5_f64),
        ("examScore".to_string(), 86.5_f64),
    ]);
    let changes = HashMap::from([("studyHours".to_string(), 7.0_f64)]);
    let sim = engine
        .simulate_counterfactual(Counterfactual::new(actual, changes))
        .expect("simulation should succeed");

    let cf_score = sim.counterfactual_world["examScore"];
    assert!(
        (cf_score - 106.5).abs() < 3.0,
        "expected ~106.5, got {cf_score}"
    );
}

/// Success criterion 3b: cycle-introducing edges are rejected.
#[test]
fn causal_graph_rejects_cycles() {
    use eve_causal::{CausalEdge, CausalGraph};
    let mut graph = CausalGraph::new();
    graph
        .add_edge(CausalEdge {
            from: "A".into(),
            to: "B".into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        })
        .unwrap();
    graph
        .add_edge(CausalEdge {
            from: "B".into(),
            to: "C".into(),
            relation: CausalRelationKind::Causes,
            strength: 1.0,
        })
        .unwrap();
    let result = graph.add_edge(CausalEdge {
        from: "C".into(),
        to: "A".into(),
        relation: CausalRelationKind::Causes,
        strength: 1.0,
    });
    assert!(result.is_err(), "cycle should be rejected");
    assert!(!graph.has_cycle());
}

/// Causal effect estimation: direct effect of studyHours on examScore should be ≈ 10.
#[test]
fn estimates_causal_effect() {
    let engine = build_study_engine();
    let effect = engine
        .estimate_causal_effect("studyHours", "examScore")
        .expect("effect must be estimable");
    // True model: examScore = 29 + 10*studyHours + 1*sleep → coefficient is 10
    assert!(
        (effect - 10.0).abs() < 1.0,
        "expected effect ~10, got {effect}"
    );
}

/// Confounder identification: when there's no common ancestor, no confounders.
#[test]
fn identifies_no_confounders_in_simple_chain() {
    let engine = build_study_engine();
    let conf = engine.identify_confounders("studyHours", "examScore");
    // studyHours and sleep are siblings with no shared ancestor — no confounders
    assert!(conf.is_empty(), "expected no confounders, got {conf:?}");
}

/// MockNeuralLayer extract_causal_relations hook: the trait's default returns NotSupported,
/// but the mock overrides embed/find_similar; test that extract_causal_relations can be
/// handled gracefully (the mock falls back to the default NotSupported).
#[tokio::test]
async fn mock_neural_extract_causal_returns_not_supported() {
    let mock = MockNeuralLayer::new();
    let result = mock.extract_causal_relations("test text").await;
    // Default impl returns NotSupported — CLI handles this gracefully with fixture data
    assert!(
        result.is_err(),
        "mock should return NotSupported for causal relations"
    );
}
