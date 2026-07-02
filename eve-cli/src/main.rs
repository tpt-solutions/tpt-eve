use clap::{Parser, Subcommand};
use eve_causal::{CausalReasoningEngine, Counterfactual, Intervention, Observation};
use eve_core::{CausalRelationDraft, CausalRelationKind, Fact, Value};
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

    // Load facts and rules from fixture
    let fixture_path = std::path::Path::new("fixtures/typescript_example.eve");
    if fixture_path.exists() {
        let fixture_content = std::fs::read_to_string(fixture_path).unwrap_or_default();
        match parse(&fixture_content) {
            Ok(program) => {
                for stmt in program.statements {
                    match stmt {
                        Statement::Fact(triple) => layer.add_fact(triple_to_fact(&triple)),
                        Statement::Rule(rule_def) => {
                            let (conditions, conclusion) = rule_to_runtime(&rule_def);
                            layer.add_rule(Rule::new(rule_def.name, conditions, conclusion));
                        }
                        Statement::Query(_) => {}
                    }
                }
            }
            Err(e) => eprintln!("Warning: fixture parse error: {}", e),
        }
    } else {
        // Fallback: assert the base facts using Text objects to match MockNeuralLayer's output
        layer.add_fact(Fact::asserted(
            Value::Symbol("TypeScript".into()),
            Value::Symbol("is".into()),
            Value::Text("programming language".into()),
        ));
        layer.add_fact(Fact::asserted(
            Value::Text("programming language".into()),
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
