// SPDX-License-Identifier: MIT OR Apache-2.0
//! Demonstrates working memory, forward-chaining inference, and question answering.

use tpt_eve_core::{PatternTerm, Value};
use tpt_eve_symbolic::{Rule, SymbolicReasoningLayer};

fn main() {
    let mut layer = SymbolicReasoningLayer::new();

    // Assert some facts
    layer.add_fact(tpt_eve_core::Fact::asserted(
        Value::Symbol("TypeScript".into()),
        Value::Symbol("is".into()),
        Value::Symbol("programming language".into()),
    ));
    layer.add_fact(tpt_eve_core::Fact::asserted(
        Value::Symbol("programming language".into()),
        Value::Symbol("has".into()),
        Value::Symbol("syntax".into()),
    ));
    layer.add_fact(tpt_eve_core::Fact::asserted(
        Value::Symbol("TypeScript".into()),
        Value::Symbol("provides".into()),
        Value::Text("compile-time type checking".into()),
    ));

    // Add a transitive rule: if X is Y and Y has Z, then X has Z
    layer.add_rule(Rule::new(
        Some("transitive_has".into()),
        vec![
            (
                PatternTerm::Variable("X".into()),
                PatternTerm::Bound(Value::Symbol("is".into())),
                PatternTerm::Variable("Y".into()),
            ),
            (
                PatternTerm::Variable("Y".into()),
                PatternTerm::Bound(Value::Symbol("has".into())),
                PatternTerm::Variable("Z".into()),
            ),
        ],
        (
            PatternTerm::Variable("X".into()),
            PatternTerm::Bound(Value::Symbol("has".into())),
            PatternTerm::Variable("Z".into()),
        ),
    ));

    // Forward-chain to derive new facts
    let inferences = layer.infer();
    println!("Derived {} new fact(s):", inferences.len());
    for inf in &inferences {
        println!(
            "  {} {} {} (by {})",
            inf.derived.subject,
            inf.derived.predicate,
            inf.derived.object,
            inf.rule_name.as_deref().unwrap_or("<unnamed>")
        );
    }

    // Ask a question
    println!();
    match layer.ask("What does TypeScript have?") {
        Some(fact) => println!(
            "Answer: {} {} {}",
            fact.subject, fact.predicate, fact.object
        ),
        None => println!("No answer found."),
    }

    // Check consistency
    let contradictions = layer.check_consistency();
    if contradictions.is_empty() {
        println!("\nNo contradictions detected.");
    } else {
        println!("\nContradictions found:");
        for c in &contradictions {
            println!(
                "  {} {} has conflicting values: {:?}",
                c.subject, c.predicate, c.conflicting_objects
            );
        }
    }

    println!("\nTotal facts: {}", layer.facts().len());
}
