use clap::{Parser, Subcommand};
use eve_core::{Fact, Value};
use eve_dsl::{parse, Statement};
use eve_neural::{MockNeuralLayer, NeuralPatternLayer};
use eve_symbolic::{Rule, SymbolicReasoningLayer};

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
        Commands::Extract { text } => {
            let mock = MockNeuralLayer::new();
            match mock.extract_facts(&text).await {
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
            // Simple demo that uses in-memory symbolic layer
            let mut layer = SymbolicReasoningLayer::new();
            
            // Load some sample facts
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

            // Add transitive rule
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

            layer.infer();

            match layer.ask(&question) {
                Some(fact) => {
                    println!("{} {} {}", fact.subject, fact.predicate, fact.object);
                }
                None => {
                    println!("No answer found.");
                }
            }
        }
    }
}

async fn run_demo(backend: &str) {
    // Load facts from neural layer
    let mut layer = SymbolicReasoningLayer::new();
    
    // Use mock backend for now (respects --backend flag)
    let text = "TypeScript is a programming language that provides compile-time type checking. \
                It compiles to JavaScript. Programming languages have syntax.";
    
    let facts = if backend == "mock" {
        let mock = MockNeuralLayer::new();
        mock.extract_facts(text).await.unwrap_or_default()
    } else {
        // For other backends, show placeholder
        println!("Backend '{}' not implemented yet, using mock", backend);
        let mock = MockNeuralLayer::new();
        mock.extract_facts(text).await.unwrap_or_default()
    };

    for fact in &facts {
        layer.add_fact(fact.clone());
    }

    // Load rules from fixture if available
    let fixture_path = std::path::Path::new("fixtures/typescript_example.eve");
    if fixture_path.exists() {
        let fixture_content = std::fs::read_to_string(fixture_path).unwrap_or_default();
        if let Ok(program) = parse(&fixture_content) {
            for stmt in program.statements {
                if let Statement::Rule(rule_def) = stmt {
                    let (conditions, conclusion) = eve_symbolic::rule_to_runtime(&rule_def);
                    layer.add_rule(Rule::new(rule_def.name, conditions, conclusion));
                }
            }
        }
    } else {
        // Add default transitive rule
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
    }

    // Run inference
    let inferences = layer.infer();
    
    // Check consistency
    let contradictions = layer.check_consistency();
    if !contradictions.is_empty() {
        println!("Contradictions found:");
        for c in &contradictions {
            println!("  {} {} has conflicting values", c.subject, c.predicate);
        }
    }

    // Answer question
    let answer = layer.ask("What does TypeScript have?");
    match answer {
        Some(fact) => {
            println!("TypeScript has {}", fact.object);
        }
        None => {
            println!("No answer found for 'What does TypeScript have?'");
        }
    }
}