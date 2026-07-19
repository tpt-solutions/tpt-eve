use tpt_eve_core::Value;
use tpt_eve_neural::{MockNeuralLayer, NeuralPatternLayer};
use tpt_eve_symbolic::{Rule, SymbolicReasoningLayer};

/// End-to-end test: extract facts from TypeScript text, apply transitive rule,
/// and answer "What does TypeScript have?"
///
/// This is the Phase 1 acceptance test using MockNeuralLayer.
/// MockNeuralLayer returns objects as Value::Text (extracted strings), so
/// the supporting "has" fact and assertions use the same type.
#[tokio::test]
async fn full_phase1_pipeline() {
    // 1. Extract facts via neural layer
    let mock = MockNeuralLayer::new();
    let text = "TypeScript is a programming language. Programming languages have syntax.";
    let extracted_facts = mock.extract_facts(text).await.unwrap();
    assert!(
        !extracted_facts.is_empty(),
        "Should extract at least one fact"
    );

    // 2. Load extracted facts into the symbolic layer
    let mut layer = SymbolicReasoningLayer::new();
    for fact in extracted_facts {
        layer.add_fact(fact);
    }
    // MockNeuralLayer gives us: TypeScript is Text("programming language")
    // Add the supporting "has" fact using the same Text type so Y unifies correctly
    layer.add_fact(tpt_eve_core::Fact::asserted(
        Value::Text("programming language".into()),
        Value::Symbol("has".into()),
        Value::Symbol("syntax".into()),
    ));

    // 3. Add the transitive rule: if X is Y, Y has Z then X has Z
    layer.add_rule(Rule::new(
        Some("transitive_has".into()),
        vec![
            (
                tpt_eve_core::PatternTerm::Variable("X".into()),
                tpt_eve_core::PatternTerm::Bound(Value::Symbol("is".into())),
                tpt_eve_core::PatternTerm::Variable("Y".into()),
            ),
            (
                tpt_eve_core::PatternTerm::Variable("Y".into()),
                tpt_eve_core::PatternTerm::Bound(Value::Symbol("has".into())),
                tpt_eve_core::PatternTerm::Variable("Z".into()),
            ),
        ],
        (
            tpt_eve_core::PatternTerm::Variable("X".into()),
            tpt_eve_core::PatternTerm::Bound(Value::Symbol("has".into())),
            tpt_eve_core::PatternTerm::Variable("Z".into()),
        ),
    ));

    // 4. Run forward chaining inference
    layer.infer();

    // 5. Verify no contradictions
    let contradictions = layer.check_consistency();
    assert!(
        contradictions.is_empty(),
        "Should have no contradictions in simple example"
    );

    // 6. Answer "What does TypeScript have?" — should derive TypeScript has syntax
    let answer = layer.ask("What does TypeScript have?");
    assert!(
        answer.is_some(),
        "Should find an answer for 'What does TypeScript have?'"
    );
    let answer = answer.unwrap();
    assert_eq!(answer.subject, Value::Symbol("TypeScript".into()));
    assert_eq!(answer.predicate, Value::Symbol("has".into()));
    assert_eq!(answer.object, Value::Symbol("syntax".into()));
}
