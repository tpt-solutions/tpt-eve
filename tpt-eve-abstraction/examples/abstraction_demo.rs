// SPDX-License-Identifier: MIT OR Apache-2.0
//! Demonstrates pattern mining, analogical mapping, and knowledge transfer.

use tpt_eve_abstraction::AbstractionEngine;
use tpt_eve_core::{Fact, Value};

fn main() {
    let mut engine = AbstractionEngine::new();

    // Register programming language examples
    let lang_facts = |name: &str| -> Vec<Fact> {
        vec![
            Fact::asserted(
                Value::Symbol(name.into()),
                Value::Symbol("is".into()),
                Value::Symbol("programming_language".into()),
            ),
            Fact::asserted(
                Value::Symbol("programming_language".into()),
                Value::Symbol("has".into()),
                Value::Symbol("syntax".into()),
            ),
        ]
    };

    engine.add_example("typescript", lang_facts("TypeScript"));
    engine.add_example("python", lang_facts("Python"));
    engine.add_example("rust", lang_facts("Rust"));

    // Mine abstract principles
    let principles = engine.mine_principles(2);
    println!("Abstract principles from 3 language examples:");
    for p in &principles {
        println!(
            "  [support={}/3  conf={:.0}%] {}",
            p.support,
            p.confidence * 100.0,
            p.name
        );
    }

    // Analogy: mammals ↔ birds
    let mammals = vec![
        Fact::asserted(
            Value::Symbol("cat".into()),
            Value::Symbol("is".into()),
            Value::Symbol("mammal".into()),
        ),
        Fact::asserted(
            Value::Symbol("cat".into()),
            Value::Symbol("has".into()),
            Value::Symbol("fur".into()),
        ),
        Fact::asserted(
            Value::Symbol("dog".into()),
            Value::Symbol("is".into()),
            Value::Symbol("mammal".into()),
        ),
        Fact::asserted(
            Value::Symbol("dog".into()),
            Value::Symbol("has".into()),
            Value::Symbol("fur".into()),
        ),
        Fact::asserted(
            Value::Symbol("mammal".into()),
            Value::Symbol("is_a".into()),
            Value::Symbol("animal".into()),
        ),
    ];
    let birds = vec![
        Fact::asserted(
            Value::Symbol("eagle".into()),
            Value::Symbol("is".into()),
            Value::Symbol("bird".into()),
        ),
        Fact::asserted(
            Value::Symbol("eagle".into()),
            Value::Symbol("has".into()),
            Value::Symbol("feathers".into()),
        ),
        Fact::asserted(
            Value::Symbol("hawk".into()),
            Value::Symbol("is".into()),
            Value::Symbol("bird".into()),
        ),
        Fact::asserted(
            Value::Symbol("hawk".into()),
            Value::Symbol("has".into()),
            Value::Symbol("feathers".into()),
        ),
        Fact::asserted(
            Value::Symbol("bird".into()),
            Value::Symbol("is_a".into()),
            Value::Symbol("animal".into()),
        ),
    ];

    let analogy = engine.map_analogy("mammals", &mammals, "birds", &birds);
    println!(
        "\nAnalogy: {} → {} (score {:.2})",
        analogy.source_domain, analogy.target_domain, analogy.score
    );
    let mut pairs: Vec<_> = analogy.mapping.iter().collect();
    pairs.sort_by_key(|(k, _)| k.as_str());
    for (src, tgt) in pairs {
        println!("  {src} → {tgt}");
    }

    // Transfer: cat lives_in forest → bird domain
    let new_fact = vec![Fact::asserted(
        Value::Symbol("cat".into()),
        Value::Symbol("lives_in".into()),
        Value::Symbol("forest".into()),
    )];
    let transferred = engine.transfer(&analogy, &new_fact);
    println!("\nKnowledge transfer:");
    for f in &transferred {
        println!(
            "  {} {} {} (conf {:.2})",
            f.subject, f.predicate, f.object, f.confidence
        );
    }
}
