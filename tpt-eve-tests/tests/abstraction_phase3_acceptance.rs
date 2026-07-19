use tpt_eve_abstraction::{AbstractionEngine, AnalogyMapper, PatternMiner};
use tpt_eve_core::{Fact, Value};
use tpt_eve_neural::{MockNeuralLayer, NeuralPatternLayer};

/// Phase 3 acceptance tests: extract abstract principles, find cross-domain analogies,
/// transfer knowledge.  All tests run synchronously or with the mock backend (no live LLM).

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

/// Three programming-language examples sharing `X is programming_language → programming_language has syntax`.
fn programming_language_examples() -> Vec<Vec<Fact>> {
    ["TypeScript", "Python", "Rust"]
        .iter()
        .map(|name| {
            vec![
                fact(name, "is", "programming_language"),
                fact("programming_language", "has", "syntax"),
            ]
        })
        .collect()
}

fn mammal_facts() -> Vec<Fact> {
    vec![
        fact("cat", "is", "mammal"),
        fact("cat", "has", "fur"),
        fact("dog", "is", "mammal"),
        fact("dog", "has", "fur"),
        fact("mammal", "is_a", "animal"),
    ]
}

fn bird_facts() -> Vec<Fact> {
    vec![
        fact("eagle", "is", "bird"),
        fact("eagle", "has", "feathers"),
        fact("hawk", "is", "bird"),
        fact("hawk", "has", "feathers"),
        fact("bird", "is_a", "animal"),
    ]
}

// ---------------------------------------------------------------------------
// Success criterion 1: extract abstract principles from examples
// ---------------------------------------------------------------------------

/// The `is → has` chain must appear in all three language examples and be mined as a principle.
#[test]
fn extracts_is_has_principle_from_language_examples() {
    let examples = programming_language_examples();
    let miner = PatternMiner::new(2);
    let principles = miner.mine(&examples);

    assert!(
        !principles.is_empty(),
        "should produce at least one principle"
    );

    let has_is_has = principles
        .iter()
        .any(|p| p.predicate_chain == ["is", "has"]);
    assert!(
        has_is_has,
        "expected is → has principle; got: {principles:?}"
    );
}

/// Support and confidence must be consistent with the example count.
#[test]
fn principle_support_and_confidence_are_correct() {
    let examples = programming_language_examples();
    let miner = PatternMiner::new(1);
    let principles = miner.mine(&examples);

    let p = principles
        .iter()
        .find(|p| p.predicate_chain == ["is", "has"])
        .expect("is → has principle must exist");

    assert_eq!(p.support, 3, "support should equal number of examples");
    assert!(
        (p.confidence - 1.0).abs() < 1e-5,
        "confidence should be 1.0 when all examples match"
    );
}

/// Principles below min_support must be filtered out.
#[test]
fn min_support_filters_infrequent_patterns() {
    let mut examples = programming_language_examples();
    // Add a unique example that introduces a rare chain not shared with the others.
    examples.push(vec![fact("Haskell", "uses", "lazy_evaluation")]);

    let miner = PatternMiner::new(3);
    let principles = miner.mine(&examples);

    for p in &principles {
        assert!(
            p.support >= 3,
            "principle '{}' has support {} which is below min_support=3",
            p.name,
            p.support
        );
    }
}

// ---------------------------------------------------------------------------
// Success criterion 2: find cross-domain analogies
// ---------------------------------------------------------------------------

/// The analogy score between mammals and birds should be positive (they share predicates).
#[test]
fn finds_analogy_between_mammals_and_birds() {
    let mapper = AnalogyMapper::new();
    let analogy = mapper.map("mammals", &mammal_facts(), "birds", &bird_facts());

    assert!(
        analogy.score > 0.0,
        "analogy score must be positive for related domains; got {}",
        analogy.score
    );
    assert!(analogy.score <= 1.0, "score cannot exceed 1.0");
    assert!(
        !analogy.mapping.is_empty(),
        "mapping must not be empty for related domains"
    );
}

/// "mammal" should map to "bird" since they share both `is` (from their children) and `is_a`.
#[test]
fn mammal_maps_to_bird_in_analogy() {
    let mapper = AnalogyMapper::new();
    let analogy = mapper.map("mammals", &mammal_facts(), "birds", &bird_facts());

    assert_eq!(
        analogy.mapping.get("mammal").map(String::as_str),
        Some("bird"),
        "mammal should map to bird; full mapping: {:?}",
        analogy.mapping
    );
}

/// Unrelated domains produce a zero or near-zero score.
#[test]
fn unrelated_domains_have_low_analogy_score() {
    let mapper = AnalogyMapper::new();
    let domain_a = vec![fact("A", "unique_pred_alpha", "B")];
    let domain_b = vec![fact("X", "unique_pred_beta", "Y")];
    let analogy = mapper.map("domain_a", &domain_a, "domain_b", &domain_b);
    // No shared predicates → Jaccard = 0 for all pairs → score = 0
    assert_eq!(
        analogy.score, 0.0,
        "score should be 0 for unrelated domains"
    );
}

// ---------------------------------------------------------------------------
// Success criterion 3: transfer knowledge to new contexts
// ---------------------------------------------------------------------------

/// A new mammal fact should be projected into the bird domain via the analogy mapping.
#[test]
fn transfers_new_fact_to_target_domain() {
    let engine = AbstractionEngine::new();
    let analogy = engine.map_analogy("mammals", &mammal_facts(), "birds", &bird_facts());

    let new_mammal_fact = vec![fact("cat", "lives_in", "forest")];
    let transferred = engine.transfer(&analogy, &new_mammal_fact);

    assert_eq!(transferred.len(), 1);
    // The subject "cat" should have been remapped
    let expected_subject = analogy
        .mapping
        .get("cat")
        .map(String::as_str)
        .unwrap_or("cat");
    assert_eq!(
        transferred[0].subject.as_str(),
        expected_subject,
        "subject should be remapped via analogy"
    );
    // Predicate is preserved verbatim
    assert_eq!(transferred[0].predicate, Value::Symbol("lives_in".into()));
}

/// Confidence of transferred facts should be strictly less than the source (scaled by score < 1).
#[test]
fn transferred_confidence_reflects_analogy_uncertainty() {
    let engine = AbstractionEngine::new();
    let analogy = engine.map_analogy("mammals", &mammal_facts(), "birds", &bird_facts());

    // Analogy score < 1.0 means transferred confidence < source confidence.
    if analogy.score < 1.0 {
        let source = vec![Fact::asserted(
            Value::Symbol("cat".into()),
            Value::Symbol("is".into()),
            Value::Symbol("mammal".into()),
        )];
        let result = engine.transfer(&analogy, &source);
        assert!(
            result[0].confidence < 1.0,
            "transferred confidence ({}) should be < 1.0 when score < 1.0",
            result[0].confidence
        );
    }
}

/// Values absent from the mapping are kept verbatim in the transferred fact.
#[test]
fn unmapped_values_survive_transfer_verbatim() {
    use std::collections::HashMap;
    use tpt_eve_abstraction::Analogy;

    let analogy = Analogy {
        source_domain: "src".into(),
        target_domain: "tgt".into(),
        mapping: HashMap::new(),
        score: 0.5,
    };
    let engine = AbstractionEngine::new();
    let source = vec![fact("unknown_entity", "rel", "unknown_object")];
    let result = engine.transfer(&analogy, &source);
    assert_eq!(result[0].subject, Value::Symbol("unknown_entity".into()));
    assert_eq!(result[0].object, Value::Symbol("unknown_object".into()));
}

// ---------------------------------------------------------------------------
// AbstractionEngine facade tests
// ---------------------------------------------------------------------------

#[test]
fn engine_mines_principles_from_registered_examples() {
    let mut engine = AbstractionEngine::new();
    for (name, facts) in ["TypeScript", "Python", "Rust"]
        .iter()
        .zip(programming_language_examples())
    {
        engine.add_example(*name, facts);
    }
    let principles = engine.mine_principles(2);
    assert!(
        principles
            .iter()
            .any(|p| p.predicate_chain == ["is", "has"]),
        "engine should mine is → has from 3 language examples"
    );
}

#[test]
fn engine_example_names_are_tracked() {
    let mut engine = AbstractionEngine::new();
    engine.add_example("alpha", vec![]);
    engine.add_example("beta", vec![]);
    assert_eq!(engine.example_names(), vec!["alpha", "beta"]);
}

// ---------------------------------------------------------------------------
// Neural embed/find_similar: mock conformance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mock_embed_returns_fixed_length_vector() {
    let mock = MockNeuralLayer::new();
    let emb = mock
        .embed("programming languages share structure")
        .await
        .unwrap();
    assert!(!emb.0.is_empty(), "embedding must be non-empty");
    assert_eq!(emb.0.len(), 384, "mock returns 384-dim embedding");
}

#[tokio::test]
async fn mock_find_similar_returns_all_candidates_ranked() {
    let mock = MockNeuralLayer::new();
    let candidates = vec![
        "TypeScript".to_string(),
        "Python".to_string(),
        "Rust".to_string(),
    ];
    let results = mock
        .find_similar("programming language", candidates.clone())
        .await
        .unwrap();

    assert_eq!(
        results.len(),
        candidates.len(),
        "all candidates must be returned"
    );
    // Mock returns them with decreasing scores — first result should have highest score.
    assert!(
        results[0].score >= results[1].score,
        "results must be sorted descending by score"
    );
}
