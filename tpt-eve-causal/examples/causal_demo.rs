// SPDX-License-Identifier: MIT OR Apache-2.0
//! Demonstrates causal graph construction, intervention prediction,
//! and counterfactual simulation.

use tpt_eve_causal::{CausalReasoningEngine, Counterfactual, Intervention, Observation};
use tpt_eve_core::{CausalRelationDraft, CausalRelationKind};

fn main() {
    let mut engine = CausalReasoningEngine::new();

    // Define causal relationships
    let relations = vec![
        CausalRelationDraft {
            cause: "studyHours".into(),
            effect: "examScore".into(),
            relation: CausalRelationKind::Causes,
        },
        CausalRelationDraft {
            cause: "sleep".into(),
            effect: "examScore".into(),
            relation: CausalRelationKind::Enables,
        },
    ];

    for rel in &relations {
        println!("  {} --[{:?}]--> {}", rel.cause, rel.relation, rel.effect);
        engine.add_relation(rel.clone());
    }

    // Add observations
    let observations = vec![
        Observation::from_pairs([("studyHours", 3.0), ("sleep", 6.0), ("examScore", 65.0)]),
        Observation::from_pairs([("studyHours", 3.0), ("sleep", 9.0), ("examScore", 68.0)]),
        Observation::from_pairs([("studyHours", 7.0), ("sleep", 6.0), ("examScore", 105.0)]),
        Observation::from_pairs([("studyHours", 7.0), ("sleep", 9.0), ("examScore", 108.0)]),
    ];
    for obs in observations {
        engine.add_observation(obs);
    }
    engine.fit();

    // Predict intervention: do(studyHours = 10)
    let baseline = std::collections::HashMap::from([("sleep".to_string(), 7.5_f64)]);
    match engine.predict_intervention(
        Intervention::new("studyHours", 10.0),
        "examScore",
        &baseline,
    ) {
        Ok(pred) => println!(
            "\nIntervention do(studyHours=10): expected examScore = {:.1} (conf {:.0}%)",
            pred.expected_value,
            pred.confidence * 100.0
        ),
        Err(e) => eprintln!("Intervention failed: {e}"),
    }

    // Counterfactual: what if studyHours had been 7 instead of 5?
    let actual = std::collections::HashMap::from([
        ("studyHours".to_string(), 5.0_f64),
        ("sleep".to_string(), 7.5_f64),
        ("examScore".to_string(), 86.5_f64),
    ]);
    let changes = std::collections::HashMap::from([("studyHours".to_string(), 7.0_f64)]);
    match engine.simulate_counterfactual(Counterfactual::new(actual, changes)) {
        Ok(sim) => {
            let cf = sim
                .counterfactual_world
                .get("examScore")
                .copied()
                .unwrap_or(0.0);
            println!("Counterfactual: if studyHours had been 7 → examScore = {cf:.1}");
        }
        Err(e) => eprintln!("Counterfactual failed: {e}"),
    }

    // Causal effect estimation
    match engine.estimate_causal_effect("studyHours", "examScore") {
        Ok(effect) => println!("\nCausal effect studyHours → examScore: {effect:.2}"),
        Err(e) => eprintln!("Effect estimation failed: {e}"),
    }

    // Confounder identification
    let conf = engine.identify_confounders("studyHours", "examScore");
    if conf.is_empty() {
        println!("No confounders found.");
    } else {
        println!("Confounders: {}", conf.join(", "));
    }
}
