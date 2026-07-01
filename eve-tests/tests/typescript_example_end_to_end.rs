use eve_core::{Fact, FactSource, Value};
use eve_neural::{MockNeuralLayer, NeuralPatternLayer};
use eve_symbolic::{Rule, SymbolicReasoningLayer};

/// End-to-end test: extract facts from TypeScript text, apply transitive rule,
/// and answer "What does TypeScript have?"
/// 
/// This is the Phase 1 acceptance test using MockNeuralLayer.
#[tokio::test]
async fn full_phase1_pipeline() {
    // 1. Extract facts using neural layer
    let mock = MockNeuralLayer::new();
    let text = "TypeScript is a programming language. Programming languages have syntax.";
    let extracted_facts = mock.extract_facts(text).await.unwrap();
    
    // Verify we got facts
    assert!(!extracted_facts.is_empty(), "Should extract at least one fact");
    
    // 2. Build symbolic layer and add extracted facts
    let mut layer = SymbolicReasoningLayer::new();
    for fact in extracted_facts {
        layer.add_fact(fact);
    }
    
    // Add a fact to enable transitive reasoning: TypeScript is programming language
    layer.add_fact(Fact::asserted(
        Value::Symbol("TypeScript".into()),
        Value::Symbol("is".into()),
        Value::Symbol("programming language".into()),
    ));
    
    // 3. Add the transitive rule: if X is Y, Y has Z then X has Z
    layer.add_rule(Rule::new(
        Some("transitive_has".into()),
        vec![
            (eve_core::PatternTerm::Variable("X".into()), 
             eve_core::PatternTerm::Bound(Value::Symbol("is".into())), 
             eve_core::PatternTerm::Variable("Y".into())),
            (eve_core::PatternTerm::Variable("Y".into()), 
             eve_core::PatternTerm::Bound(Value::Symbol("has".into())), 
             eve_core::PatternTerm::Variable("Z".into())),
        ],
        (eve_core::PatternTerm::Variable("X".into()), 
         eve_core::PatternTerm::Bound(Value::Symbol("has".into())), 
         eve_core::PatternTerm::Variable("Z".into())),
    ));
    
    // 4. Add the programming language has syntax fact
    layer.add_fact(Fact::asserted(
        Value::Symbol("programming language".into()),
        Value::Symbol("has".into()),
        Value::Symbol("syntax".into()),
    ));
    
    // 5. Run forward chaining inference
    let inferences = layer.infer();
    
    // 6. Check consistency
    let contradictions = layer.check_consistency();
    assert!(contradictions.is_empty(), "Should have no contradictions in simple example");
    
    // 7. Answer "What does TypeScript have?"
    let answer = layer.ask("What does TypeScript have?");
    
    // 8. Verify we found the inferred fact: TypeScript has syntax
    assert!(answer.is_some(), "Should find an answer for 'What does TypeScript have?'");
    let answer = answer.unwrap();
    assert_eq!(answer.subject, Value::Symbol("TypeScript".into()));
    assert_eq!(answer.predicate, Value::Symbol("has".into()));
    assert_eq!(answer.object, Value::Symbol("syntax".into()));
}