use tpt_eve_active_learning::{
    ActiveLearningEngine, BeliefState, BeliefUpdater, ExperimentDesign, ExperimentRunner,
    Hypothesis, HypothesisGenerator,
};
use tpt_eve_core::{Fact, FactSource, Value};

// Phase 4 acceptance tests: generate testable hypotheses, run experiments, discover insights.
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

fn hypothesis_for(s: &str, p: &str, o: &str) -> Hypothesis {
    Hypothesis {
        description: format!("{s} {p} {o}"),
        prediction: Fact {
            subject: Value::Symbol(s.into()),
            predicate: Value::Symbol(p.into()),
            object: Value::Symbol(o.into()),
            source: FactSource::Extracted { confidence: 0.6 },
            confidence: 0.6,
        },
        confidence: 0.6,
        supporting_facts: vec![],
    }
}

/// Programming-language facts: TypeScript + Python are both programming_languages.
/// TypeScript has syntax.  Python's "has syntax" should be hypothesised.
fn lang_facts() -> Vec<Fact> {
    vec![
        fact("TypeScript", "is", "programming_language"),
        fact("Python", "is", "programming_language"),
        fact("programming_language", "has", "syntax"),
    ]
}

// ---------------------------------------------------------------------------
// Success criterion 1: generate testable hypotheses
// ---------------------------------------------------------------------------

/// Chain completion: TypeScript -is-> programming_language -has-> syntax
/// → hypothesis TypeScript -has-> syntax.
#[test]
fn generates_chain_completion_hypothesis() {
    let gen = HypothesisGenerator::new();
    let hs = gen.generate(&lang_facts());
    assert!(
        !hs.is_empty(),
        "should produce at least one hypothesis from lang_facts"
    );
    let has_chain = hs.iter().any(|h| {
        h.prediction.subject == Value::Symbol("TypeScript".into())
            && h.prediction.predicate == Value::Symbol("has".into())
            && h.prediction.object == Value::Symbol("syntax".into())
    });
    assert!(
        has_chain,
        "chain completion TypeScript-has-syntax must be generated; got: {hs:?}"
    );
}

/// Category generalisation: both TypeScript and Python are programming_language,
/// TypeScript has syntax → hypothesis Python -has-> syntax.
#[test]
fn generates_category_generalisation_hypothesis() {
    // Add TypeScript has syntax explicitly so the category generaliser can fire.
    let mut facts = lang_facts();
    facts.push(fact("TypeScript", "has", "syntax"));

    let gen = HypothesisGenerator::new();
    let hs = gen.generate(&facts);
    let has_cat = hs.iter().any(|h| {
        h.prediction.subject == Value::Symbol("Python".into())
            && h.prediction.predicate == Value::Symbol("has".into())
            && h.prediction.object == Value::Symbol("syntax".into())
    });
    assert!(
        has_cat,
        "category generalisation Python-has-syntax must be generated; got: {hs:?}"
    );
}

/// Hypotheses must not suggest facts that already exist in the knowledge base.
#[test]
fn hypotheses_do_not_duplicate_known_facts() {
    let mut facts = lang_facts();
    facts.push(fact("TypeScript", "has", "syntax")); // already asserted

    let gen = HypothesisGenerator::new();
    let hs = gen.generate(&facts);
    for h in &hs {
        assert!(
            !facts.iter().any(|f| f.same_triple(&h.prediction)),
            "hypothesis {:?} duplicates an existing fact",
            h.prediction
        );
    }
}

/// All generated hypotheses start with a confidence in (0, 1).
#[test]
fn hypothesis_confidence_is_in_unit_interval() {
    let gen = HypothesisGenerator::new();
    for h in gen.generate(&lang_facts()) {
        assert!(
            h.confidence > 0.0 && h.confidence < 1.0,
            "confidence {} is out of (0,1) for hypothesis '{}'",
            h.confidence,
            h.description
        );
    }
}

// ---------------------------------------------------------------------------
// Success criterion 2: design and run experiments
// ---------------------------------------------------------------------------

/// A confirmed experiment (observation matches prediction) sets `confirmed = true`.
#[test]
fn experiment_confirmed_when_observation_matches_prediction() {
    let h = hypothesis_for("Python", "has", "syntax");
    let observation = fact("Python", "has", "syntax");
    let design = ExperimentDesign::new("test-python-has-syntax", h, vec![observation]);
    let result = ExperimentRunner::new().run(&design);

    assert!(result.confirmed, "experiment must be confirmed");
    assert!(
        !result.supporting.is_empty(),
        "must have supporting evidence"
    );
    assert!(
        result.conflicting.is_empty(),
        "must have no conflicting evidence"
    );
    assert!(
        result.updated_confidence > 0.6,
        "confidence must increase after confirmation; got {}",
        result.updated_confidence
    );
}

/// A conflicting experiment (same subject+predicate, different object) sets `confirmed = false`
/// and lowers confidence.
#[test]
fn experiment_refuted_when_observation_contradicts_prediction() {
    let h = hypothesis_for("Python", "has", "syntax");
    let observation = fact("Python", "has", "no_syntax");
    let design = ExperimentDesign::new("test-python-no-syntax", h, vec![observation]);
    let result = ExperimentRunner::new().run(&design);

    assert!(!result.confirmed, "experiment must be refuted");
    assert!(
        !result.conflicting.is_empty(),
        "must have conflicting evidence"
    );
    assert!(
        result.updated_confidence < 0.6,
        "confidence must decrease after refutation; got {}",
        result.updated_confidence
    );
}

/// An inconclusive experiment (irrelevant observation) leaves confidence close to prior.
#[test]
fn experiment_inconclusive_when_observation_is_unrelated() {
    let h = hypothesis_for("Python", "has", "syntax");
    let observation = fact("Rust", "is", "language");
    let design = ExperimentDesign::new("test-unrelated", h, vec![observation]);
    let result = ExperimentRunner::new().run(&design);

    assert!(!result.confirmed);
    assert!(result.conflicting.is_empty());
    // Confidence must remain reasonably close to prior (0.6).
    assert!(
        result.updated_confidence > 0.4 && result.updated_confidence <= 0.6,
        "inconclusive confidence should stay near prior 0.6; got {}",
        result.updated_confidence
    );
}

// ---------------------------------------------------------------------------
// Bayesian belief updating
// ---------------------------------------------------------------------------

/// Positive evidence raises belief.
#[test]
fn positive_evidence_raises_belief() {
    let updater = BeliefUpdater::new();
    let b = BeliefState::new(fact("Python", "has", "syntax"), 0.5);
    let after = updater.update_positive(&b);
    assert!(
        after.belief > b.belief,
        "positive evidence should raise belief: {} → {}",
        b.belief,
        after.belief
    );
}

/// Negative evidence lowers belief.
#[test]
fn negative_evidence_lowers_belief() {
    let updater = BeliefUpdater::new();
    let b = BeliefState::new(fact("Python", "has", "syntax"), 0.5);
    let after = updater.update_negative(&b);
    assert!(
        after.belief < b.belief,
        "negative evidence should lower belief: {} → {}",
        b.belief,
        after.belief
    );
}

/// Ten rounds of positive evidence converges to high confidence.
#[test]
fn repeated_positive_evidence_converges_above_95_percent() {
    let updater = BeliefUpdater::new();
    let initial = BeliefState::new(fact("Python", "has", "syntax"), 0.5);
    let evidence = vec![true; 10];
    let final_state = updater.update_sequence(&initial, &evidence);
    assert!(
        final_state.belief > 0.95,
        "10 positive updates should push belief above 0.95; got {}",
        final_state.belief
    );
}

/// Ten rounds of negative evidence converges to very low confidence.
#[test]
fn repeated_negative_evidence_converges_below_5_percent() {
    let updater = BeliefUpdater::new();
    let initial = BeliefState::new(fact("Python", "has", "syntax"), 0.5);
    let evidence = vec![false; 10];
    let final_state = updater.update_sequence(&initial, &evidence);
    assert!(
        final_state.belief < 0.05,
        "10 negative updates should push belief below 0.05; got {}",
        final_state.belief
    );
}

// ---------------------------------------------------------------------------
// Success criterion 3: discover new insights via ActiveLearningEngine
// ---------------------------------------------------------------------------

/// Full cycle: add facts → generate → design → run → update → discover.
#[test]
fn full_active_learning_cycle_discovers_insight() {
    let mut engine = ActiveLearningEngine::new();
    for f in lang_facts() {
        engine.add_fact(f);
    }

    let hypotheses = engine.generate_hypotheses();
    assert!(
        !hypotheses.is_empty(),
        "engine must generate at least one hypothesis"
    );

    // Find the TypeScript-has-syntax chain hypothesis and confirm it.
    let ts_has = hypotheses.iter().find(|h| {
        h.prediction.subject == Value::Symbol("TypeScript".into())
            && h.prediction.predicate == Value::Symbol("has".into())
            && h.prediction.object == Value::Symbol("syntax".into())
    });
    assert!(
        ts_has.is_some(),
        "chain hypothesis TypeScript-has-syntax must exist"
    );

    let mut h = ts_has.unwrap().clone();
    let before_confidence = h.confidence;

    let design = engine.design_experiment(&h);
    let result = engine.run_experiment(design);
    engine.update_belief(&mut h, &result);

    assert!(
        h.confidence > before_confidence,
        "confirmed experiment must raise confidence: {} → {}",
        before_confidence,
        h.confidence
    );
}

/// Insights are not returned when all hypotheses are below the threshold.
#[test]
fn no_insights_returned_when_below_threshold() {
    let mut engine = ActiveLearningEngine::new();
    for f in lang_facts() {
        engine.add_fact(f);
    }
    // Initial hypothesis confidence is < 0.9 → nothing at threshold 0.9.
    let insights = engine.discover_insights(0.9);
    assert!(
        insights.is_empty(),
        "no insights at threshold 0.9 when all hypotheses start below that"
    );
}

/// Insights are returned at a permissive threshold.
#[test]
fn insights_returned_at_permissive_threshold() {
    let mut engine = ActiveLearningEngine::new();
    for f in lang_facts() {
        engine.add_fact(f);
    }
    let insights = engine.discover_insights(0.5);
    assert!(
        !insights.is_empty(),
        "insights at threshold 0.5 should not be empty given lang_facts"
    );
}

/// Engine integrates with `ActiveLearningEngine::update_belief` correctly.
#[test]
fn engine_update_belief_raises_confidence_on_confirmed_result() {
    let mut engine = ActiveLearningEngine::new();
    for f in lang_facts() {
        engine.add_fact(f);
    }

    let hs = engine.generate_hypotheses();
    let mut h = hs[0].clone();
    let prior = h.confidence;

    let design = engine.design_experiment(&h);
    let result = engine.run_experiment(design);
    engine.update_belief(&mut h, &result);

    assert!(
        h.confidence > prior,
        "update_belief after confirmed result must raise confidence: {prior} → {}",
        h.confidence
    );
}
