// SPDX-License-Identifier: MIT OR Apache-2.0
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use tpt_eve_abstraction::AbstractionEngine;
use tpt_eve_active_learning::{ActiveLearningEngine, ExperimentDesign};
use tpt_eve_causal::{CausalReasoningEngine, Counterfactual, Intervention, Observation};
use tpt_eve_core::{CausalRelationDraft, CausalRelationKind, EveError, Fact, Value};
use tpt_eve_dsl::{parse, Statement};
use tpt_eve_metacognition::MetaCognitionEngine;
use tpt_eve_neural::{
    CloudNeuralLayer, CloudProvider, MockNeuralLayer, NeuralPatternLayer, OllamaNeuralLayer,
    SparkNeuralLayer,
};
use tpt_eve_symbolic::{
    query_to_patterns, rule_to_runtime, triple_to_fact, Rule, SymbolicReasoningLayer,
    WorkingMemory, WorkingMemoryStore,
};

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser)]
#[command(name = "tpt-eve")]
#[command(version, about = "TPT Eve cognitive architecture CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract facts from text using the neural layer
    Extract {
        /// Text to extract facts from
        text: String,
        /// Backend: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
        /// Override the model used by cloud/ollama backends
        #[arg(long)]
        model: Option<String>,
        /// Save extracted facts to a session file (redb) for later use with `ask --load`
        #[arg(long)]
        save: Option<std::path::PathBuf>,
        /// Output format: text (default) or json
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
    /// Run the Phase 1 demo pipeline (extract → infer → ask)
    Demo {
        /// Backend: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
        /// Override the model used by cloud/ollama backends
        #[arg(long)]
        model: Option<String>,
    },
    /// Ask a question; optionally load facts from a saved session
    Ask {
        /// Question to ask (e.g. "What does TypeScript have?")
        question: String,
        /// Load facts from a session file saved with `extract --save`
        #[arg(long)]
        load: Option<std::path::PathBuf>,
    },
    /// Show the inference chain that produced an answer (Phase 1)
    Explain {
        /// Question to explain (e.g. "What does TypeScript have?")
        question: String,
        /// Load facts from a session file saved with `extract --save`
        #[arg(long)]
        load: Option<std::path::PathBuf>,
    },
    /// Interactive REPL: type facts and questions in real time
    Repl,
    /// Answer a "what if" question using causal reasoning (Phase 2 demo)
    Whatif {
        /// Backend: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
        /// Override the model used by cloud/ollama backends
        #[arg(long)]
        model: Option<String>,
        /// Print the causal graph in DOT format instead of running the demo
        #[arg(long)]
        graph: bool,
    },
    /// Demonstrate Phase 3 abstraction: pattern mining, analogical mapping, knowledge transfer
    Abstract {
        /// Backend: mock, spark, ollama, anthropic, openai, openrouter (for embed-based similarity)
        #[arg(short, long, default_value = "mock")]
        backend: String,
    },
    /// Demonstrate Phase 4 active learning: hypothesis generation, experiment, belief updating
    Learn,
    /// Demonstrate Phase 5 meta-cognition: confidence tracking, gap detection, learning agenda
    Reflect,
    /// Start an HTTP REST API server exposing the cognitive pipeline
    Serve {
        /// Backend for /extract: mock, spark, ollama, anthropic, openai, openrouter
        #[arg(short, long, default_value = "mock")]
        backend: String,
        /// Port to listen on
        #[arg(short, long, default_value = "3737")]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            text,
            backend,
            model,
            save,
            format,
        } => {
            let neural = build_backend(&backend, model.as_deref());
            match with_spinner_and_retry("Extracting facts…", || neural.extract_facts(&text)).await
            {
                Ok(facts) => {
                    print_facts(&facts, &format);
                    if let Some(path) = save {
                        let mut wm = WorkingMemory::new();
                        for f in facts {
                            wm.add_fact(f);
                        }
                        match WorkingMemoryStore::new(path.clone()) {
                            Ok(store) => match store.save(&wm) {
                                Ok(()) => {
                                    eprintln!("Saved {} fact(s) to {}", wm.len(), path.display())
                                }
                                Err(e) => eprintln!("Save failed: {e}"),
                            },
                            Err(e) => eprintln!("Could not open session file: {e}"),
                        }
                    }
                }
                Err(e) => eprintln!("Error extracting facts: {e}"),
            }
        }

        Commands::Demo { backend, model } => {
            run_demo(&backend, model.as_deref()).await;
        }

        Commands::Whatif {
            backend,
            model,
            graph,
        } => {
            if graph {
                run_whatif_dot(&backend, model.as_deref()).await;
            } else {
                run_whatif(&backend, model.as_deref()).await;
            }
        }

        Commands::Abstract { backend } => {
            run_abstract(&backend);
        }

        Commands::Learn => run_learn(),
        Commands::Reflect => run_reflect(),
        Commands::Serve { backend, port } => {
            run_serve(&backend, port).await;
        }

        Commands::Ask { question, load } => {
            let mut layer = load_layer_for_ask(load);
            layer.infer();
            match layer.ask(&question) {
                Some(fact) => println!("{} {} {}", fact.subject, fact.predicate, fact.object),
                None => println!("No answer found."),
            }
        }

        Commands::Explain { question, load } => {
            run_explain(&question, load);
        }

        Commands::Repl => run_repl(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn print_facts(facts: &[Fact], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            let arr: Vec<serde_json::Value> = facts
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "subject":   f.subject.as_str(),
                        "predicate": f.predicate.as_str(),
                        "object":    f.object.as_str(),
                        "confidence": f.confidence,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&arr).unwrap_or_default());
        }
        OutputFormat::Text => {
            for f in facts {
                println!(
                    "{} {} {}  [{:.0}%]",
                    f.subject,
                    f.predicate,
                    f.object,
                    f.confidence * 100.0
                );
            }
        }
    }
}

/// Build a SymbolicReasoningLayer pre-loaded with the demo TypeScript knowledge base,
/// or with facts from a saved session file when `--load <path>` is provided.
fn load_layer_for_ask(load: Option<std::path::PathBuf>) -> SymbolicReasoningLayer {
    let mut layer = SymbolicReasoningLayer::new();

    if let Some(path) = load {
        match WorkingMemoryStore::new(path.clone()) {
            Ok(store) => match store.load() {
                Ok(wm) => {
                    eprintln!("Loaded {} fact(s) from {}", wm.len(), path.display());
                    for fact in wm.facts() {
                        layer.add_fact(fact.clone());
                    }
                    return layer;
                }
                Err(e) => eprintln!("Could not load session: {e}"),
            },
            Err(e) => eprintln!("Could not open session file: {e}"),
        }
    }

    // Fallback: built-in demo knowledge base
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
    layer
}

fn build_backend(backend: &str, model: Option<&str>) -> Box<dyn NeuralPatternLayer> {
    let layer: Box<dyn NeuralPatternLayer> = match backend {
        "spark" => Box::new(SparkNeuralLayer::new()),
        "ollama" => {
            let mut b = OllamaNeuralLayer::new();
            if let Some(m) = model {
                b = b.with_model(m);
            }
            Box::new(b)
        }
        "anthropic" => {
            let mut b = CloudNeuralLayer::from_env(CloudProvider::Anthropic);
            if let Some(m) = model {
                b = b.with_model(m);
            }
            Box::new(b)
        }
        "openai" => {
            let mut b = CloudNeuralLayer::from_env(CloudProvider::OpenAi);
            if let Some(m) = model {
                b = b.with_model(m);
            }
            Box::new(b)
        }
        "openrouter" => {
            let mut b = CloudNeuralLayer::from_env(CloudProvider::OpenRouter);
            if let Some(m) = model {
                b = b.with_model(m);
            }
            Box::new(b)
        }
        _ => Box::new(MockNeuralLayer::new()),
    };
    layer
}

// ── Spinner + retry ───────────────────────────────────────────────────────────

async fn with_spinner_and_retry<F, Fut, T>(msg: &str, mut f: F) -> Result<T, EveError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, EveError>>,
{
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
    pb.set_message(msg.to_owned());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    for delay_secs in [1u64, 2] {
        match f().await {
            Err(EveError::Transport(e)) => {
                pb.println(format!("Network error ({e}), retrying in {delay_secs}s…"));
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            }
            result => {
                pb.finish_and_clear();
                return result;
            }
        }
    }
    let result = f().await;
    pb.finish_and_clear();
    result
}

// ── eve explain ───────────────────────────────────────────────────────────────

fn run_explain(question: &str, load: Option<std::path::PathBuf>) {
    let mut layer = load_layer_for_ask(load);
    let inferences = layer.infer();

    match layer.ask(question) {
        None => println!("No answer found for: {question}"),
        Some(fact) => {
            println!(
                "Answer: {} {} {}",
                fact.subject, fact.predicate, fact.object
            );
            let derived = inferences.iter().find(|inf| inf.derived.same_triple(&fact));
            match derived {
                Some(inf) => {
                    let rule_name = inf.rule_name.as_deref().unwrap_or("<unnamed rule>");
                    println!("\nDerived by rule: {rule_name}");
                    println!("Supporting asserted facts:");
                    for f in layer
                        .facts()
                        .iter()
                        .filter(|f| matches!(f.source, tpt_eve_core::FactSource::Asserted))
                    {
                        println!("  {} {} {}", f.subject, f.predicate, f.object);
                    }
                }
                None => println!("\n(Directly asserted — no inference chain)"),
            }
        }
    }
}

// ── eve repl ──────────────────────────────────────────────────────────────────

fn run_repl() {
    use std::io::{BufRead, Write};

    let mut layer = SymbolicReasoningLayer::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    println!("TPT Eve REPL — type facts ('X is Y.'), questions ('?What is X?'), or 'exit'.");

    loop {
        print!("tpt-eve> ");
        stdout.flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        let input = line.trim();
        if input == "exit" || input == "quit" {
            break;
        }
        if input.is_empty() {
            continue;
        }

        if let Some(question) = input.strip_prefix('?') {
            layer.infer();
            match layer.ask(question.trim()) {
                Some(f) => println!("→ {} {} {}", f.subject, f.predicate, f.object),
                None => println!("→ No answer found."),
            }
        } else {
            match parse(input) {
                Ok(program) => {
                    let mut added = 0usize;
                    for stmt in program.statements {
                        match stmt {
                            Statement::Fact(triple) => match triple_to_fact(&triple) {
                                Ok(fact) => {
                                    layer.add_fact(fact);
                                    added += 1;
                                }
                                Err(e) => eprintln!("Error: {e}"),
                            },
                            Statement::Rule(rule_def) => {
                                let (conditions, conclusion) = rule_to_runtime(&rule_def);
                                layer.add_rule(Rule::new(rule_def.name, conditions, conclusion));
                                added += 1;
                            }
                            Statement::Query(query_def) => {
                                layer.infer();
                                let patterns = query_to_patterns(&query_def);
                                let results = layer.working_memory().query_conjunctive(&patterns);
                                if results.is_empty() {
                                    println!("→ (no results)");
                                } else {
                                    for bindings in &results {
                                        let mut parts: Vec<_> = bindings.iter().collect();
                                        parts.sort_by_key(|(k, _)| k.as_str());
                                        let s: Vec<String> = parts
                                            .iter()
                                            .map(|(k, v)| format!("{k} = {v}"))
                                            .collect();
                                        println!("→ {}", s.join(", "));
                                    }
                                }
                            }
                        }
                    }
                    if added > 0 {
                        println!("OK ({added} item(s) added)");
                    }
                }
                Err(e) => eprintln!("Parse error: {e}"),
            }
        }
    }
    println!("Goodbye.");
}

// ── Phase 2: causal reasoning ─────────────────────────────────────────────────

async fn run_whatif(backend: &str, model: Option<&str>) {
    let neural = build_backend(backend, model);

    println!("=== Phase 2: Causal Reasoning Demo ===\n");

    let text = "Studying more hours causes higher exam scores. \
                Exam scores are preceded by adequate sleep. \
                Stress prevents good performance.";

    let relations: Vec<CausalRelationDraft> =
        match with_spinner_and_retry("Extracting causal relations…", || {
            neural.extract_causal_relations(text)
        })
        .await
        {
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

    let mut engine = CausalReasoningEngine::new();
    for rel in relations {
        println!(
            "  Relation: {} --[{:?}]--> {}",
            rel.cause, rel.relation, rel.effect
        );
        engine.add_relation(rel);
    }
    println!();

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

    let baseline = std::collections::HashMap::from([("sleep".to_string(), 7.5_f64)]);
    match engine.predict_intervention(Intervention::new("studyHours", 10.0), "examScore", &baseline) {
        Ok(pred) => println!(
            "Intervention — do(studyHours = 10):\n  Expected examScore: {:.1}  (confidence {:.0}%)\n",
            pred.expected_value,
            pred.confidence * 100.0
        ),
        Err(e) => eprintln!("Intervention prediction failed: {e}"),
    }

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

    match engine.estimate_causal_effect("studyHours", "examScore") {
        Ok(effect) => println!("Total causal effect of studyHours → examScore: {effect:.2}"),
        Err(e) => eprintln!("Effect estimation failed: {e}"),
    }

    let conf = engine.identify_confounders("studyHours", "examScore");
    if conf.is_empty() {
        println!("No confounders identified for (studyHours, examScore).");
    } else {
        println!("Confounders: {}", conf.join(", "));
    }
}

async fn run_whatif_dot(backend: &str, model: Option<&str>) {
    let neural = build_backend(backend, model);

    let text = "Studying more hours causes higher exam scores. \
                Exam scores are preceded by adequate sleep. \
                Stress prevents good performance.";

    let relations: Vec<CausalRelationDraft> =
        match with_spinner_and_retry("Extracting causal relations…", || {
            neural.extract_causal_relations(text)
        })
        .await
        {
            Ok(r) if !r.is_empty() => r,
            _ => vec![
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
                CausalRelationDraft {
                    cause: "stress".into(),
                    effect: "examScore".into(),
                    relation: CausalRelationKind::Prevents,
                },
            ],
        };

    let mut engine = CausalReasoningEngine::new();
    for rel in relations {
        engine.add_relation(rel);
    }
    engine.fit();

    let graph = engine.graph();
    println!("digraph causal {{");
    println!("  rankdir=LR;");
    for node in &graph.nodes {
        println!("  \"{node}\";");
    }
    for edge in &graph.edges {
        println!(
            "  \"{}\" -> \"{}\" [label=\"{:?}  {:.2}\"];",
            edge.from, edge.to, edge.relation, edge.strength
        );
    }
    println!("}}");
}

// ── Phase 3: abstraction ─────────────────────────────────────────────────────

fn run_abstract(backend: &str) {
    let _ = backend; // backend flag available for future embedding-based similarity

    println!("=== Phase 3: Abstraction Demo ===\n");

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

// ── Phase 4: active learning ─────────────────────────────────────────────────

fn run_learn() {
    println!("=== Phase 4: Active Learning Demo ===\n");

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

    let hypotheses = engine.generate_hypotheses();
    println!(
        "Hypotheses generated from known facts ({} total):",
        hypotheses.len()
    );
    for h in &hypotheses {
        println!("  [{:.0}% conf] {}", h.confidence * 100.0, h.description);
    }
    println!();

    if let Some(hyp) = hypotheses.first() {
        println!("Testing hypothesis: \"{}\"", hyp.description);
        println!(
            "  Prediction: {} {} {}",
            hyp.prediction.subject, hyp.prediction.predicate, hyp.prediction.object
        );

        let mut design = engine.design_experiment(hyp);
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

// ── Phase 5: meta-cognition ───────────────────────────────────────────────────

fn run_reflect() {
    println!("=== Phase 5: Meta-Cognition Demo ===\n");

    let mut engine = MetaCognitionEngine::new();

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
    engine.add_fact(low("TypeScript", "has", "type_system", 0.4));
    engine.add_fact(low("Python", "supports", "dynamic_typing", 0.35));

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

    let ts_type = low("TypeScript", "has", "type_system", 0.4);
    engine.update_confidence(&ts_type, 0.85);
    println!(
        "After new evidence for TypeScript-has-type_system: calibration error = {:.3}\n",
        engine.calibration_error()
    );

    let gaps = engine.detect_gaps(0.6);
    println!("Knowledge gaps detected ({} total):", gaps.len());
    for g in &gaps {
        println!(
            "  [{:?}  priority={:.2}] {}",
            g.gap_type, g.priority, g.description
        );
    }
    println!();

    let agenda = engine.prioritize_learning(&gaps);
    println!("Learning agenda (ranked):");
    for item in &agenda {
        println!("  {}. {}", item.rank, item.rationale);
    }
    println!();

    let level = engine.assess_understanding();
    println!("Overall understanding level: {level}\n");

    let questions = engine.request_clarifications(3);
    println!("Top clarification questions:");
    for q in &questions {
        println!("  [{:.0}% urgency] {}", q.priority * 100.0, q.question);
    }
}

// ── eve serve ─────────────────────────────────────────────────────────────────

async fn run_serve(backend: &str, port: u16) {
    use axum::{
        extract::State,
        http::StatusCode,
        routing::{get, post},
        Json, Router,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct AppState {
        wm: Arc<Mutex<WorkingMemory>>,
        backend: Arc<String>,
    }

    #[derive(Deserialize)]
    struct ExtractReq {
        text: String,
    }

    #[derive(Serialize)]
    struct FactJson {
        subject: String,
        predicate: String,
        object: String,
        confidence: f32,
    }

    #[derive(Deserialize)]
    struct AskReq {
        question: String,
    }

    #[derive(Serialize)]
    struct AskResp {
        answer: Option<FactJson>,
    }

    fn to_json(f: &Fact) -> FactJson {
        FactJson {
            subject: f.subject.as_str().to_string(),
            predicate: f.predicate.as_str().to_string(),
            object: f.object.as_str().to_string(),
            confidence: f.confidence,
        }
    }

    async fn route_extract(
        State(state): State<AppState>,
        Json(req): Json<ExtractReq>,
    ) -> Result<Json<Vec<FactJson>>, (StatusCode, String)> {
        let neural = build_backend(&state.backend, None);
        let facts = neural
            .extract_facts(&req.text)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
        let mut wm = state.wm.lock().await;
        for f in &facts {
            wm.add_fact(f.clone());
        }
        Ok(Json(facts.iter().map(to_json).collect()))
    }

    async fn route_ask(State(state): State<AppState>, Json(req): Json<AskReq>) -> Json<AskResp> {
        let mut layer = SymbolicReasoningLayer::new();
        {
            let wm = state.wm.lock().await;
            for f in wm.facts() {
                layer.add_fact(f.clone());
            }
        }
        layer.infer();
        let answer = layer.ask(&req.question).map(|f| to_json(&f));
        Json(AskResp { answer })
    }

    async fn route_facts(State(state): State<AppState>) -> Json<Vec<FactJson>> {
        let wm = state.wm.lock().await;
        Json(wm.facts().iter().map(to_json).collect())
    }

    let state = AppState {
        wm: Arc::new(Mutex::new(WorkingMemory::new())),
        backend: Arc::new(backend.to_string()),
    };

    let app = Router::new()
        .route("/extract", post(route_extract))
        .route("/ask", post(route_ask))
        .route("/facts", get(route_facts))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    println!("TPT Eve HTTP server listening on http://{addr}");
    println!("  POST /extract  {{\"text\": \"...\"}}       → [facts]");
    println!("  POST /ask      {{\"question\": \"...\"}}   → {{\"answer\": ...}}");
    println!("  GET  /facts                             → [all asserted facts]");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener");
    axum::serve(listener, app).await.expect("Server error");
}

// ── Phase 1: demo pipeline ────────────────────────────────────────────────────

async fn run_demo(backend: &str, model: Option<&str>) {
    let neural = build_backend(backend, model);
    let mut layer = SymbolicReasoningLayer::new();

    let text = "TypeScript is a programming language that provides compile-time type checking. \
                It compiles to JavaScript. Programming languages have syntax.";

    match with_spinner_and_retry("Extracting facts…", || neural.extract_facts(text)).await {
        Ok(facts) => {
            for fact in facts {
                layer.add_fact(fact);
            }
        }
        Err(e) => eprintln!("Warning: fact extraction failed ({e}), continuing with fixture only"),
    }

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
