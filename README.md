# TPT Eve

TPT Eve is a cognitive architecture that moves beyond statistical pattern matching to build
systems capable of genuine understanding, causal reasoning, and discovery. LLMs are excellent at
recognizing patterns in data, but on their own they don't understand *why* things happen, can't
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
│  CAUSAL REASONING ENGINE     — causal graphs, interventions  │   Phase 2
│  ABSTRACTION ENGINE          — cross-domain principles       │   Phase 3
│  SYMBOLIC REASONING LAYER    — working memory, rule engine   │ ← Phase 1
│  NEURAL PATTERN LAYER (LLM)  — fact & relation extraction    │ ← Phase 1
│  ACTIVE LEARNING LOOP        — hypotheses, experiments       │   Phase 4
└─────────────────────────────────────────────────────────────┘
```

Phase 1 (Foundation) is implemented now: a Neural Pattern Layer that extracts facts from text via
a pluggable LLM backend, and a Symbolic Reasoning Layer with a hand-written rule/query DSL,
embedded working memory, and forward-chaining inference.

## Quickstart

```bash
cargo build --workspace

# Runs the Phase 1 end-to-end demo against a deterministic mock LLM backend —
# no live model required.
cargo run -p eve-cli -- demo --backend mock
```

Other backends: `--backend spark` (local, via tpt-spark's headless JSON-RPC socket),
`--backend ollama` (local, via Ollama's HTTP API), `--backend anthropic|openai|openrouter`
(cloud, via the respective API key env var).

## Workspace Layout

| Crate | Purpose |
|---|---|
| `eve-core` | Shared types: `Fact`, `Pattern`, `Value`, `EveError` |
| `eve-dsl` | The rule/query DSL: lexer, parser, AST |
| `eve-symbolic` | Working memory, forward-chaining inference, consistency checking |
| `eve-neural` | `NeuralPatternLayer` trait + tpt-spark/Ollama/cloud/mock backends |
| `eve-cli` | The `eve` binary |
| `eve-tests` | Workspace integration tests |

See [CLAUDE.md](CLAUDE.md) for build commands and architectural detail.

## License

Apache-2.0 — see [LICENSE](LICENSE).
