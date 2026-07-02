use clap::{Parser, Subcommand};
use eve_core::{Fact, Value};
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
