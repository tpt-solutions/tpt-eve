# TPT Eve

TPT Eve is a cognitive architecture that moves beyond statistical pattern matching to build
systems capable of genuine understanding, causal reasoning, and discovery. LLMs are excellent at
recognising patterns in data, but on their own they don't understand *why* things happen, can't
reliably extract abstract principles, and can't discover genuinely new insights.

TPT Eve doesn't replace the statistical engine — it augments it. LLMs handle language
understanding and pattern recognition; on top of that, TPT Eve layers symbolic reasoning, causal
inference, cross-domain abstraction, active experimentation, and meta-cognition.

See [spec.txt](spec.txt) for the full design document and [todo.md](todo.md) for the project's
current build status.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      TPT EVE ARCHITECTURE                    │
├─────────────────────────────────────────────────────────────┤
│  META-COGNITIVE LAYER        — confidence, knowledge gaps    │   Phase 5
│  ACTIVE LEARNING LOOP        — hypotheses, experiments       │   Phase 4
│  ABSTRACTION ENGINE          — cross-domain principles       │   Phase 3
│  CAUSAL REASONING ENGINE     — causal graphs, interventions  │   Phase 2
│  SYMBOLIC REASONING LAYER    — working memory, rule engine   │   Phase 1
│  NEURAL PATTERN LAYER (LLM)  — fact & relation extraction    │   Phase 1
└─────────────────────────────────────────────────────────────┘
```

All five phases are implemented. Each phase builds on the previous and is independently
accessible via the `eve` CLI.

## Quickstart

```bash
cargo build --workspace

# Phase 1 — extract facts, apply rules, answer questions (mock LLM, no key needed)
cargo run -p eve-cli -- demo --backend mock

# Phase 2 — causal reasoning: interventions, counterfactuals, effect estimation
cargo run -p eve-cli -- whatif --backend mock

# Phase 3 — abstraction: pattern mining, analogical mapping, knowledge transfer
cargo run -p eve-cli -- abstract

# Phase 4 — active learning: hypothesis generation, experiment, belief updating
cargo run -p eve-cli -- learn

# Phase 5 — meta-cognition: confidence tracking, gap detection, learning agenda
cargo run -p eve-cli -- reflect
```

Other backends for `demo`, `extract`, and `whatif`:

| Flag | Backend | Requires |
|------|---------|---------|
| `--backend mock` | Deterministic fixture (default) | Nothing |
| `--backend spark` | tpt-spark headless JSON-RPC | tpt-spark running |
| `--backend ollama` | Ollama HTTP API | Ollama + a model |
| `--backend anthropic` | Anthropic Messages API | `ANTHROPIC_API_KEY` |
| `--backend openai` | OpenAI Chat Completions | `OPENAI_API_KEY` |
| `--backend openrouter` | OpenRouter (OpenAI-compatible) | `OPENROUTER_API_KEY` |

### Environment variables

| Variable | Purpose |
|---|---|
| `OLLAMA_HOST` | Ollama base URL (default: `http://localhost:11434`) |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `OPENROUTER_API_KEY` | OpenRouter API key |

## CLI Reference

```
eve extract <text> [--backend <b>] [--model <id>] [--format text|json] [--save <file>]
                                     Extract facts; optionally save session to redb file
eve ask "<question>" [--load <file>] Query the symbolic layer (load session or demo KB)
eve explain "<question>" [--load <file>]
                                     Answer with full inference provenance chain
eve demo [--backend <b>] [--model <id>]
                                     Phase 1 end-to-end: extract → infer → ask
eve whatif [--backend <b>] [--model <id>] [--graph]
                                     Phase 2 causal: intervention + counterfactual
                                     --graph emits DOT digraph on stdout
eve abstract [--backend <b>]         Phase 3 abstraction demo
eve learn                            Phase 4 active learning demo
eve reflect                          Phase 5 meta-cognition demo
eve repl                             Interactive REPL: type facts/rules/queries line by line
eve serve [--backend <b>] [--port <n>]
                                     HTTP REST API on port 3737 (default)
                                     POST /extract  {"text":"..."}  → [facts]
                                     POST /ask      {"question":"..."} → {"answer":...}
                                     GET  /facts    → [all asserted facts]
```

## Workspace Layout

| Crate | Phase | Purpose |
|---|---|---|
| `eve-core` | — | Shared types: `Fact`, `Pattern`, `Value`, `EveError` |
| `eve-dsl` | — | Rule/query DSL: lexer, parser, AST |
| `eve-symbolic` | 1 | Working memory, forward-chaining inference, consistency checking |
| `eve-neural` | 1 | `NeuralPatternLayer` trait + tpt-spark/Ollama/cloud/mock backends |
| `eve-causal` | 2 | Causal DAG construction, SCM, intervention calculus, counterfactuals |
| `eve-abstraction` | 3 | Pattern mining, analogical reasoning, knowledge transfer |
| `eve-active-learning` | 4 | Hypothesis generation, experiment design, Bayesian belief updating |
| `eve-metacognition` | 5 | Confidence tracking, knowledge-gap detection, learning prioritisation |
| `eve-cli` | — | The `eve` binary |
| `eve-tests` | — | Workspace integration and acceptance tests |

See [CLAUDE.md](CLAUDE.md) for build commands and architectural detail.

## License

Apache-2.0 — see [LICENSE](LICENSE).
