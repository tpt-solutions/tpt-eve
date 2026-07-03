// SPDX-License-Identifier: Apache-2.0
use clap::{Parser, Subcommand};
use eve_abstraction::AbstractionEngine;
use eve_active_learning::{ActiveLearningEngine, ExperimentDesign};
use eve_causal::{CausalReasoningEngine, Counterfactual, Intervention, Observation};
use eve_core::{CausalRelationDraft, CausalRelationKind, Fact, Value};
use eve_metacognition::MetaCognitionEngine;
use eve_dsl::{parse, Statement};
use eve_neural::{
    CloudNeuralLayer, CloudProvider, MockNeuralLayer, NeuralPatternLayer, OllamaNeuralLayer,
    SparkNeuralLayer,
};
use eve_symbolic::{rule_to_runtime, triple_to_fact, Rule, SymbolicReasoningLayer};

#[derive(Parser)]
#[command(name = "eve")]
#[command(version, about = "TPT Eve cognitive architecture CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract facts from text using neural layer
    Extract {
        /// Text to extract facts from
        text: String,
        /// Backend to use: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
    },
    /// Run the demo pipeline (extract -> infer -> ask)
    Demo {
        /// Backend to use: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
    },
    /// Ask a question about the knowledge base
    Ask {
        /// Question to ask
        question: String,
    },
    /// Answer a "what if" question using causal reasoning (Phase 2 demo)
    Whatif {
        /// Backend to use: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
    },
    /// Demonstrate Phase 3 abstraction: pattern mining, analogical mapping, knowledge transfer
    Abstract,
    /// Demonstrate Phase 4 active learning: hypothesis generation, experiment, belief updating
    Learn,
    /// Demonstrate Phase 5 meta-cognition: confidence tracking, gap detection, learning agenda
    Reflect,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { text, backend } => {
            let neural = build_backend(&backend);
            match neural.extract_facts(&text).await {
                Ok(facts) => {
                    for fact in facts {
                        println!("{} {} {}", fact.subject, fact.predicate, fact.object);
                    }
                }
                Err(e) => eprintln!("Error extracting facts: {}", e),
            }
        }
        Commands::Demo { backend } => {
            run_demo(&backend).await;
        }
        Commands::Whatif { backend } => {
            run_whatif(&backend).await;
        }
        Commands::Abstract => {
            run_abstract();
        }
        Commands::Learn => {
            run_learn();
        }
        Commands::Reflect => {
            run_reflect();
        }
        Commands::Ask { question } => {
            let mut layer = SymbolicReasoningLayer::new();

            layer.add_fact(Fact::asserted(
                Value::Symbol("TypeScript".into()),
                Value::Symbol("is".into()),
                Value::Symbol("programming language".into()),
            ));
            layer.add_fact(Fact::asserted(
                Value::Symbol("programming language".into()),
                Value::Symbol("has".into()),
                Value::Symbol("syntax".into()),
            ));

            layer.add_rule(Rule::new(
                Some("transitive_has".into()),
                vec![
                    (
                        eve_core::PatternTerm::Variable("X".into()),
                        eve_core::PatternTerm::Bound(Value::Symbol("is".into())),
                        eve_core::PatternTerm::Variable("Y".into()),
                    ),
                    (
                        eve_core::PatternTerm::Variable("Y".into()),
                        eve_core::PatternTerm::Bound(Value::Symbol("has".into())),
                        eve_core::PatternTerm::Variable("Z".into()),
                    ),
                ],
                (
                    eve_core::PatternTerm::Variable("X".into()),
                    eve_core::PatternTerm::Bound(Value::Symbol("has".into())),
                    eve_core::PatternTerm::Variable("Z".into()),
                ),
            ));

            layer.infer();

            match layer.ask(&question) {
                Some(fact) => println!("{} {} {}", fact.subject, fact.predicate, fact.object),
                None => println!("No answer found."),
            }
        }
    }
}

async fn run_whatif(backend: &str) {
    let neural = build_backend(backend);

    // Phase 2 demo: study-hours → exam-score causal graph from spec.txt example
    println!("=== Phase 2: Causal Reasoning Demo ===\n");

    // 1. Extract causal relations from text (falls back to mock relations if not supported)
    let text = "Studying more hours causes higher exam scores. \
                Exam scores are preceded by adequate sleep. \
                Stress prevents good performance.";

    let relations: Vec<CausalRelationDraft> = match neural.extract_causal_relations(text).await {
        Ok(r) if !r.is_empty() => {
            println!("Extracted {} causal relation(s) from text.", r.len());
            r
        }
        _ => {
            println!("(Neural backend did not return causal relations — using fixture data.)");
            vec![
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
            ]
        }
    };

    // 2. Build causal engine and add observations
    let mut engine = CausalReasoningEngine::new();
    for rel in relations {
        println!(
            "  Relation: {} --[{:?}]--> {}",
            rel.cause, rel.relation, rel.effect
        );
        engine.add_relation(rel);
    }
    println!();

    // Observed data: independent variation so OLS can identify both coefficients.
    // True model (approx): examScore = 29 + 10*studyHours + 1*sleep
    let observations = vec![
        Observation::from_pairs([("studyHours", 3.0), ("sleep", 6.0), ("examScore", 65.0)]),
        Observation::from_pairs([("studyHours", 3.0), ("sleep", 9.0), ("examScore", 68.0)]),
        Observation::from_pairs([("studyHours", 7.0), ("sleep", 6.0), ("examScore", 105.0)]),
        Observation::from_pairs([("studyHours", 7.0), ("sleep", 9.0), ("examScore", 108.0)]),
        Observation::from_pairs([("studyHours", 5.0), ("sleep", 7.5), ("examScore", 86.5)]),
    ];
    for obs in observations {
        engine.add_observation(obs);
    }
    engine.fit();

    // 3. Intervention: what if studyHours = 10?
    let baseline = std::collections::HashMap::from([("sleep".to_string(), 7.5_f64)]);
    match engine.predict_intervention(Intervention::new("studyHours", 10.0), "examScore", &baseline) {
        Ok(pred) => println!(
            "Intervention — do(studyHours = 10):\n  Expected examScore: {:.1}  (confidence {:.0}%)\n",
            pred.expected_value,
            pred.confidence * 100.0
        ),
        Err(e) => eprintln!("Intervention prediction failed: {e}"),
    }

    // 4. Counterfactual: what if we had studied 7h instead of 5h?
    let actual = std::collections::HashMap::from([
        ("studyHours".to_string(), 5.0_f64),
        ("sleep".to_string(), 7.5_f64),
        ("examScore".to_string(), 86.5_f64),
    ]);
    let changes = std::collections::HashMap::from([("studyHours".to_string(), 7.0_f64)]);
    match engine.simulate_counterfactual(Counterfactual::new(actual, changes)) {
        Ok(sim) => {
            let cf_score = sim
                .counterfactual_world
                .get("examScore")
                .copied()
                .unwrap_or(0.0);
            println!(
                "Counterfactual — if studyHours had been 7 instead of 5:\n  Expected examScore: {cf_score:.1}\n"
            );
        }
        Err(e) => eprintln!("Counterfactual simulation failed: {e}"),
    }

    // 5. Causal effect
    match engine.estimate_causal_effect("studyHours", "examScore") {
        Ok(effect) => println!("Total causal effect of studyHours → examScore: {effect:.2}"),
        Err(e) => eprintln!("Effect estimation failed: {e}"),
    }

    // 6. Confounders
    let conf = engine.identify_confounders("studyHours", "examScore");
    if conf.is_empty() {
        println!("No confounders identified for (studyHours, examScore).");
    } else {
        println!("Confounders: {}", conf.join(", "));
    }
}

fn run_abstract() {
    println!("=== Phase 3: Abstraction Demo ===\n");

    // Three programming-language examples sharing the same relational structure.
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

    let mut engine = AbstractionEngine::new();
    engine.add_example("typescript", lang_facts("TypeScript"));
    engine.add_example("python", lang_facts("Python"));
    engine.add_example("rust", lang_facts("Rust"));

    // 1. Mine abstract principles (min_support = 2: must appear in ≥ 2 examples).
    let principles = engine.mine_principles(2);
    println!("Abstract principles mined from 3 programming-language examples:");
    for p in &principles {
        println!(
            "  [support={}/{}  conf={:.0}%] {}",
            p.support,
            3,
            p.confidence * 100.0,
            p.name
        );
    }
    println!();

    // 2. Analogical mapping: mammals → birds.
    let mammals: Vec<Fact> = vec![
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
    let birds: Vec<Fact> = vec![
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
        "Analogy: {} → {}  (score {:.2})",
        analogy.source_domain, analogy.target_domain, analogy.score
    );
    let mut pairs: Vec<(&String, &String)> = analogy.mapping.iter().collect();
    pairs.sort_by_key(|(k, _)| k.as_str());
    for (src, tgt) in pairs {
        println!("  {src} → {tgt}");
    }
    println!();

    // 3. Transfer knowledge: new mammal fact → bird domain.
    let new_mammal_fact = vec![Fact::asserted(
        Value::Symbol("cat".into()),
        Value::Symbol("lives_in".into()),
        Value::Symbol("forest".into()),
    )];
    let transferred = engine.transfer(&analogy, &new_mammal_fact);
    println!("Knowledge transfer (cat lives_in forest → bird domain):");
    for f in &transferred {
        println!(
            "  {} {} {}  (confidence {:.2})",
            f.subject, f.predicate, f.object, f.confidence
        );
    }
}

fn run_learn() {
    println!("=== Phase 4: Active Learning Demo ===\n");

    // Seed the engine with a handful of programming-language facts.
    let mut engine = ActiveLearningEngine::new();
    let lang_facts = [
        ("TypeScript", "is", "programming_language"),
        ("Python", "is", "programming_language"),
        ("programming_language", "has", "syntax"),
        ("TypeScript", "has", "type_system"),
    ];
    for (s, p, o) in lang_facts {
        engine.add_fact(Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        ));
    }

    // 1. Generate hypotheses.
    let hypotheses = engine.generate_hypotheses();
    println!(
        "Hypotheses generated from known facts ({} total):",
        hypotheses.len()
    );
    for h in &hypotheses {
        println!("  [{:.0}% conf] {}", h.confidence * 100.0, h.description);
    }
    println!();

    // 2. Pick the first hypothesis, design and run an experiment that confirms it.
    if let Some(hyp) = hypotheses.first() {
        println!("Testing hypothesis: \"{}\"", hyp.description);
        println!(
            "  Prediction: {} {} {}",
            hyp.prediction.subject, hyp.prediction.predicate, hyp.prediction.object
        );

        // Supply an observation that matches the prediction (simulates a real experiment).
        let mut design = engine.design_experiment(hyp);
        // Replace the default observation with the actual predicted fact (confirmed).
        design = ExperimentDesign::new(
            design.name.clone(),
            design.hypothesis.clone(),
            vec![hyp.prediction.clone()],
        );
        let result = engine.run_experiment(design);

        let mut hyp_mut = hyp.clone();
        engine.update_belief(&mut hyp_mut, &result);
        println!(
            "  Experiment: {} (confidence {:.0}% → {:.0}%)\n",
            if result.confirmed {
                "CONFIRMED"
            } else {
                "REFUTED/INCONCLUSIVE"
            },
            hyp.confidence * 100.0,
            hyp_mut.confidence * 100.0,
        );
    }

    // 3. Discover insights at a low threshold to surface chain-completion results.
    let insights = engine.discover_insights(0.5);
    println!("Insights discovered (confidence ≥ 50%):");
    if insights.is_empty() {
        println!("  (none)");
    } else {
        for f in &insights {
            println!(
                "  {} {} {}  (confidence {:.0}%)",
                f.subject,
                f.predicate,
                f.object,
                f.confidence * 100.0
            );
        }
    }
}

fn run_reflect() {
    println!("=== Phase 5: Meta-Cognition Demo ===\n");

    let mut engine = MetaCognitionEngine::new();

    // Seed with programming-language facts at varying confidence levels.
    let high = |s: &str, p: &str, o: &str| -> Fact {
        let mut f = Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        );
        f.confidence = 0.95;
        f
    };
    let low = |s: &str, p: &str, o: &str, conf: f32| -> Fact {
        let mut f = Fact::asserted(
            Value::Symbol(s.into()),
            Value::Symbol(p.into()),
            Value::Symbol(o.into()),
        );
        f.confidence = conf;
        f
    };

    engine.add_fact(high("TypeScript", "is", "programming_language"));
    engine.add_fact(high("Python", "is", "programming_language"));
    engine.add_fact(high("programming_language", "has", "syntax"));
    engine.add_fact(low("TypeScript", "has", "type_system", 0.4)); // uncertain
    engine.add_fact(low("Python", "supports", "dynamic_typing", 0.35)); // uncertain

    // 1. Confidence tracking
    println!("Tracked facts ({} total):", engine.records().len());
    for rec in engine.records() {
        println!(
            "  [{:.0}% conf] {} {} {}",
            rec.current() * 100.0,
            rec.fact.subject,
            rec.fact.predicate,
            rec.fact.object
        );
    }
    println!(
        "\nCalibration error (initial vs. updated): {:.3}\n",
        engine.calibration_error()
    );

    // Update one fact's confidence to simulate new evidence arriving.
    let ts_type = low("TypeScript", "has", "type_system", 0.4);
    engine.update_confidence(&ts_type, 0.85);
    println!(
        "After new evidence for TypeScript-has-type_system: calibration error = {:.3}\n",
        engine.calibration_error()
    );

    // 2. Knowledge gap detection
    let gaps = engine.detect_gaps(0.6);
    println!("Knowledge gaps detected ({} total):", gaps.len());
    for g in &gaps {
        println!("  [{:?}  priority={:.2}] {}", g.gap_type, g.priority, g.description);
    }
    println!();

    // 3. Learning agenda
    let agenda = engine.prioritize_learning(&gaps);
    println!("Learning agenda (ranked):");
    for item in &agenda {
        println!("  {}. {}", item.rank, item.rationale);
    }
    println!();

    // 4. Understanding level
    let level = engine.assess_understanding();
    println!("Overall understanding level: {level}\n");

    // 5. Clarification requests
    let questions = engine.request_clarifications(3);
    println!("Top clarification questions:");
    for q in &questions {
        println!("  [{:.0}% urgency] {}", q.priority * 100.0, q.question);
    }
}

fn build_backend(backend: &str) -> Box<dyn NeuralPatternLayer> {
    match backend {
        "spark" => Box::new(SparkNeuralLayer::new()),
        "ollama" => Box::new(OllamaNeuralLayer::new()),
        "anthropic" => Box::new(CloudNeuralLayer::from_env(CloudProvider::Anthropic)),
        "openai" => Box::new(CloudNeuralLayer::from_env(CloudProvider::OpenAi)),
        "openrouter" => Box::new(CloudNeuralLayer::from_env(CloudProvider::OpenRouter)),
        _ => Box::new(MockNeuralLayer::new()),
    }
}

async fn run_demo(backend: &str) {
    let neural = build_backend(backend);
    let mut layer = SymbolicReasoningLayer::new();

    let text = "TypeScript is a programming language that provides compile-time type checking. \
                It compiles to JavaScript. Programming languages have syntax.";

    match neural.extract_facts(text).await {
        Ok(facts) => {
            for fact in facts {
                layer.add_fact(fact);
            }
        }
        Err(e) => eprintln!(
            "Warning: fact extraction failed ({}), continuing with fixture only",
            e
        ),
    }

    // Load facts and rules from the embedded fixture (compiled in at build time).
    const FIXTURE: &str = include_str!("../fixtures/typescript_example.eve");
    match parse(FIXTURE) {
        Ok(program) => {
            for stmt in program.statements {
                match stmt {
                    Statement::Fact(triple) => match triple_to_fact(&triple) {
                        Ok(fact) => layer.add_fact(fact),
                        Err(e) => eprintln!("Warning: invalid fact in fixture: {e}"),
                    },
                    Statement::Rule(rule_def) => {
                        let (conditions, conclusion) = rule_to_runtime(&rule_def);
                        layer.add_rule(Rule::new(rule_def.name, conditions, conclusion));
                    }
                    Statement::Query(_) => {}
                }
            }
        }
        Err(e) => eprintln!("Warning: fixture parse error: {e}"),
    }

    layer.infer();

    let contradictions = layer.check_consistency();
    if !contradictions.is_empty() {
        println!("Contradictions found:");
        for c in &contradictions {
            println!("  {} {} has conflicting values", c.subject, c.predicate);
        }
    }

    match layer.ask("What does TypeScript have?") {
        Some(fact) => println!("TypeScript has {}", fact.object),
        None => println!("No answer found for 'What does TypeScript have?'"),
    }
}
