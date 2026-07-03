# tpt-eve — Master Task Checklist

Full project roadmap per [spec.txt](spec.txt). Phase 1 is being built now end-to-end; Phases 2-5
are tracked here at milestone granularity for future work. See [CLAUDE.md](CLAUDE.md) for
architecture detail.

## Phase 0 — Project Scaffold

- [x] `git init`
- [x] `LICENSE` (Apache-2.0, copied from tpt-anima)
- [x] `rustfmt.toml` (copied from tpt-anima)
- [x] `.gitignore` (copied from tpt-anima)
- [x] Workspace root `Cargo.toml` with 6 crate members + `[workspace.dependencies]`
- [x] Scaffold `eve-core/` (`Cargo.toml` + `src/`)
- [x] Scaffold `eve-dsl/` (`Cargo.toml` + `src/`)
- [x] Scaffold `eve-symbolic/` (`Cargo.toml` + `src/`)
- [x] Scaffold `eve-neural/` (`Cargo.toml` + `src/`)
- [x] Scaffold `eve-cli/` (`Cargo.toml` + `src/main.rs`)
- [x] Scaffold `eve-tests/` (`Cargo.toml`, `publish = false`)
- [x] Verify `cargo build --workspace` passes clean
- [x] Verify `cargo clippy --workspace -- -D warnings` passes clean
- [x] Write `CLAUDE.md`
- [x] Write `README.md`

## Phase 1 — Foundation (build now)

Goal: Neural Pattern Layer + basic Symbolic Reasoning Layer, working memory, forward chaining.
Success criteria (from spec.txt): can extract facts from text, can apply simple logical rules,
can answer "what is" questions.

### 1.1 Core Types (`eve-core`)
- [x] `fact.rs` — `Value`, `Fact`, `FactSource`
- [x] `pattern.rs` — `PatternTerm`, `Pattern`
- [x] `error.rs` — `EveError`
- [x] `causal_draft.rs` — `CausalRelationDraft` (Phase 2 stub type)
- [x] Unit tests: Value/Fact/Pattern equality, hashing, serde round-trip

### 1.2 Rule DSL (`eve-dsl`)
- [x] `lexer.rs` — `Token`, `Lexer`
- [x] `ast.rs` — `Term`, `Triple`, `RuleDef`, `QueryDef`, `Statement`, `Program`
- [x] `parser.rs` — recursive-descent parser
- [x] `error.rs` — `ParseError` with line/col spans
- [x] `lib.rs` — `pub fn parse(source: &str) -> Result<Program, ParseError>`
- [x] Unit tests: lex/parse facts, rules, multi-clause queries, error cases
- [x] `eve-cli/fixtures/typescript_example.eve` — DSL fixture for the demo

### 1.3 Working Memory (`eve-symbolic`)
- [x] `working_memory.rs` — `WorkingMemory`, indexes, `add_fact`, `query`, `query_conjunctive`
- [x] `lowering.rs` — DSL AST → `eve_core`/`eve_symbolic` runtime types
- [x] `persistence.rs` — redb-backed `WorkingMemoryStore` (save/load snapshot)
- [x] Unit tests: add_fact dedup, query unification, conjunctive join, persistence round-trip

### 1.4 Inference Engine (`eve-symbolic`)
- [x] `rule.rs` — `Rule`
- [x] `inference.rs` — `InferenceEngine::forward_chain`, `Inference`
- [x] `consistency.rs` — `ConsistencyChecker`, `Contradiction`
- [x] `backtrack.rs` — stub matching spec interface (`NotSupported` in Phase 1)
- [x] `qa.rs` — "what is"/attribute question → `Pattern` translation
- [x] `layer.rs` — `SymbolicReasoningLayer` facade
- [x] Unit tests: forward-chain fixpoint + transitive-rule scenario, consistency true/false
      positives, QA translation

### 1.5 Neural Pattern Layer (`eve-neural`)
- [x] `lib.rs` — `NeuralPatternLayer` trait (`async_trait`)
- [x] `jsonrpc.rs` — newline-delimited JSON-RPC client over named pipe (Windows) / unix socket
- [x] `spark_backend.rs` — `SparkNeuralLayer` (extract_facts, extract_causal_relations via spark_infer)
- [x] `ollama_backend.rs` — `OllamaNeuralLayer` (HTTP, supports `embed()`)
- [x] `cloud_backend.rs` — `CloudNeuralLayer` (Anthropic, OpenAI, OpenRouter)
- [x] `mock_backend.rs` — `MockNeuralLayer` deterministic fixture backend
- [x] `prompts.rs` — fact-extraction + causal-extraction prompt templates
- [x] Unit tests: prompt formatting, response JSON parsing (clean/prose-wrapped/malformed),
      JSON-RPC framing via `tokio::io::duplex`, `MockNeuralLayer` conformance

### 1.6 CLI / Vertical Slice (`eve-cli`)
- [x] `main.rs` — clap subcommands: `extract`, `demo`, `ask`
- [x] `--backend mock|spark|ollama|anthropic|openai|openrouter` flag
- [x] Wire `eve demo` end-to-end pipeline (extract → assert → parse rule → forward_chain →
      consistency check → ask)

### 1.7 Integration Test (`eve-tests`)
- [x] `tests/typescript_example_end_to_end.rs` — full Phase 1 acceptance test using `MockNeuralLayer`

### 1.8 Phase 1 Exit Criteria
- [x] `cargo test --workspace` green
- [x] `cargo clippy --workspace -- -D warnings` green
- [x] `cargo fmt --all -- --check` green
- [ ] Manual run: `eve demo --backend spark` against a real loaded tpt-spark model
- [ ] Manual spot-check: `eve demo --backend ollama` and `eve demo --backend openrouter`
- [ ] Commit

## Phase 2 — Causal Reasoning (Months 4-6) ✓ COMPLETE

Goal: causal graph construction, intervention prediction, basic counterfactual simulation.
Success criteria: can build causal graphs from data, predict effects of interventions, answer
"what if" questions.

- [x] New crate `eve-causal`: causal graph (DAG) construction from observations
- [ ] Revisit: extract a shared `causal-graph` core into `tpt-library`, evaluate back-porting
      tpt-anima's `CausalGraph` onto it (DFS cycle-detection pattern was used here as a template;
      full back-port deferred — Phase 1 notes on tpt-library still apply)
- [x] Structural causal models / intervention calculus (`do()` operator) — linear SCM with
      multivariate OLS, do-calculus via mutilated graph
- [x] Counterfactual simulation — Pearl's abduction-action-prediction procedure
- [x] Confounder identification, causal effect estimation
- [x] Wire `NeuralPatternLayer::extract_causal_relations` into real causal graph construction
      (via `CausalGraphBuilder`, demonstrated in `eve demo whatif`)
- [ ] Evaluate adding `spark_embed` to tpt-spark if embeddings are needed earlier than Phase 3
- [x] Success-criteria tests: build causal graph from data, predict intervention effects, answer
      "what if" questions (`eve-tests/tests/causal_phase2_acceptance.rs`, 7 tests green)

## Phase 3 — Abstraction (Months 7-9) ✓ COMPLETE

Goal: cross-domain pattern extraction, analogical reasoning, principle generalization.
Success criteria: can extract abstract principles from examples, find analogies between domains,
transfer knowledge to new contexts.

- [x] New crate `eve-abstraction`: pattern mining across multiple examples/domains
      (`PatternMiner`, `AbstractPrinciple`, length-2 predicate-chain mining with min_support)
- [x] Analogical mapping between domains (`AnalogyMapper`, Jaccard-signature greedy bijection)
- [x] Principle generalization (`AbstractionEngine` facade wiring all three capabilities)
- [ ] Add `spark_embed` to tpt-spark (preferred over a Python sentence-transformers fallback);
      Ollama/OpenAI/OpenRouter `embed()` already work as a stopgap — deferred (external repo)
- [x] Real `NeuralPatternLayer::embed` / `find_similar` implementation + tests
      (`OllamaNeuralLayer::find_similar` via embed + cosine similarity; `MockNeuralLayer`
      deterministic fixture; `eve-cli abstract` demo subcommand)
- [x] Success-criteria tests: extract abstract principles, find cross-domain analogies, transfer
      knowledge (`eve-tests/tests/abstraction_phase3_acceptance.rs`, 13 tests green)

## Phase 4 — Active Learning (Months 10-12) ✓ COMPLETE

Goal: hypothesis generation, experiment design and execution, model updating.
Success criteria: can generate testable hypotheses, design and run experiments, discover new
insights.

- [x] New crate `eve-active-learning`: hypothesis generation (`HypothesisGenerator` — chain
      completion + category generalisation strategies)
- [x] Experiment design + execution environment (`ExperimentDesign`, `ExperimentRunner` —
      sandboxed logical test environment; confirms/refutes/inconclusive classification)
- [x] Bayesian belief updating (`BeliefState`, `BeliefUpdater` — Naive-Bayes posterior with
      configurable likelihood parameters; `update_sequence` for multi-round evidence)
- [x] `ActiveLearningEngine` facade: `generate_hypotheses`, `design_experiment`,
      `run_experiment`, `update_belief`, `discover_insights`
- [x] `eve learn` CLI subcommand demonstrating full cycle end-to-end
- [x] Success-criteria tests: generate testable hypotheses, run experiments, discover insights
      (`eve-tests/tests/active_learning_phase4_acceptance.rs`, 13 tests green)

## Phase 5 — Meta-Cognition (Months 13-15) ✓ COMPLETE

Goal: confidence tracking, knowledge gap detection, strategic learning decisions.
Success criteria: can track confidence in beliefs, identify what it doesn't know, prioritize
learning.

- [x] New crate `eve-metacognition`: confidence tracking across `Fact`/`Inference`/belief types
      (`ConfidenceTracker`, `ConfidenceRecord` — per-triple history, trend, calibration error)
- [x] Knowledge gap detection (`KnowledgeGapDetector` — low-confidence facts, missing attributes
      in category peers, unexplained entities; priority scoring)
- [x] Strategic learning-priority decisions (`LearningPrioritizer` — tier-ranked agenda:
      LowConfidence → MissingAttribute → IncompletePeerCoverage → UnexplainedEntity)
- [x] Understanding-level monitoring / clarification requests (`UnderstandingMonitor` —
      High/Medium/Low classification; `ClarificationRequest` generation keyed by gap type)
- [x] `MetaCognitionEngine` facade: `add_fact`, `update_confidence`, `detect_gaps`,
      `prioritize_learning`, `assess_understanding`, `request_clarifications`, `calibration_error`
- [x] `eve reflect` CLI subcommand demonstrating full cycle end-to-end
- [x] Success-criteria tests: confidence tracking accuracy, knowledge-gap identification, learning
      prioritization, confidence calibration
      (`eve-tests/tests/metacognition_phase5_acceptance.rs`, 20 tests green)
