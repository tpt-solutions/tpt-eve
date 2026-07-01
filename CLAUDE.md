# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build --workspace

# Lint (must pass clean — -D warnings is enforced)
cargo clippy --workspace -- -D warnings

# Test all crates
cargo test --workspace

# Test a single crate
cargo test -p eve-symbolic

# Run a single test by name
cargo test -p eve-symbolic inference::tests::forward_chain_derives_transitive_fact

# Format
cargo fmt --all

# Format check only
cargo fmt --all -- --check

# Run the CLI demo (no live LLM required — uses the mock backend)
cargo run -p eve-cli -- demo --backend mock
```

Rustfmt config (`rustfmt.toml`): max_width 100, 4-space tabs, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`.

## Architecture

TPT Eve augments LLM pattern recognition with symbolic reasoning, causal inference, abstraction,
active learning, and meta-cognition. Phase 1 (Foundation) implements the first two layers as a
Cargo workspace of sibling crates, leaving room for `eve-causal`, `eve-abstraction`,
`eve-active-learning`, and `eve-metacognition` to be added later without restructuring.

```
eve-core      — Shared types: Value, Fact, Pattern, EveError, CausalRelationDraft (no internal deps)
eve-dsl       — Rule/query DSL: lexer, parser, AST (depends on eve-core)
eve-symbolic  — Working memory, forward-chaining inference, consistency checking, QA (depends on eve-core, eve-dsl)
eve-neural    — NeuralPatternLayer trait + tpt-spark/Ollama/cloud/mock backends (depends on eve-core)
eve-cli       — `eve` binary: extract / demo / ask subcommands (depends on all of the above)
eve-tests     — Workspace integration tests, publish = false
```

Dependency direction is a strict DAG: `eve-core` → `eve-dsl` → `eve-symbolic`; `eve-core` → `eve-neural`
(a sibling of `eve-symbolic`, not coupled to it — `eve-neural` produces `Fact`s, `eve-symbolic` consumes
them, wired together only in `eve-cli`/`eve-tests`).

### `eve-core`

Shared vocabulary every other crate builds on: `Value` (`Symbol`/`Text`), `Fact` (subject-predicate-object
triple with `FactSource` provenance and a confidence score), `Pattern` (a triple where any slot may be
`Bound` or a `Variable`), `EveError` (shared error type), and `CausalRelationDraft` (a Phase 2 type
defined now so `eve-neural`'s trait surface matches the spec from day one).

### `eve-dsl` — the rule/query DSL

A small Datalog-like language for facts, rules, and queries:

```
TypeScript is "programming language".
"programming language" has syntax.

rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}

? TypeScript has Z.
```

Hand-written recursive-descent lexer (`lexer.rs`) and parser (`parser.rs`) — no parser-combinator
dependency (pest/nom/chumsky); the grammar is small and stable, and no other tpt-* repo uses one, so
this keeps the dependency surface flat. `ast.rs` defines the syntactic types (`Term`, `Triple`,
`RuleDef`, `QueryDef`, `Statement`, `Program`); `eve-dsl` has no notion of working memory or
inference — that lowering happens in `eve-symbolic`.

### `eve-symbolic` — working memory and inference

- **Working memory is attended via indexed lookup, not a generic graph DB.** `WorkingMemory` holds
  facts in a `Vec` plus by-subject/predicate/object `HashMap` indexes for O(1) candidate narrowing
  before pattern unification — a minimal "index + scan" Datalog, not a full RETE network.
- **Deduplication is exact-triple.** `add_fact` keeps the highest-confidence fact when the same
  (subject, predicate, object) triple is added twice.
- **Forward-chaining is naive fixpoint recomputation, not RETE-incremental.** `InferenceEngine::forward_chain`
  reruns every rule against the full fact set each pass until no new facts are derived or an iteration
  cap (default 100) is hit. This is a known Phase 1 scaling limit — fine at hundreds-to-low-thousands
  of facts, worth revisiting if rule/fact counts grow significantly.
- **Consistency checking is structural, not graph-asserted.** `ConsistencyChecker` flags contradictions
  by triple shape (same subject+predicate, differing object, on an "exclusive" predicate like `is`).
  This differs from tpt-anima's `CausalGraph::validate_causal_integrity()`, which detects contradictions
  via an explicitly-asserted `Contradicts` edge type — that approach isn't reusable here since Phase 1
  has no causal graph yet (Phase 2's `eve-causal` is where tpt-anima's DFS cycle-detection pattern is
  worth revisiting as a template).
- **Backtracking is a Phase 1 stub.** `backtrack()` matches the spec's `SymbolicReasoningLayer` interface
  but returns `EveError::NotSupported` — real backtracking belongs with Phase 2's intervention/
  counterfactual machinery.
- **Persistence is embedded, not a server.** `WorkingMemoryStore` uses `redb` (pure-Rust, single-file,
  no external service) at `~/.tpt/eve/working_memory.redb` by default, matching tpt-spark's `~/.tpt/`
  convention and its no-daemon philosophy. Indexes are never persisted, only facts — indexes are
  rebuilt on load.

### `eve-neural` — pluggable LLM integration

`NeuralPatternLayer` is an `async_trait` with `extract_facts`, `extract_causal_relations`, and an
`embed` method that defaults to `NotSupported`. Backends:

- **`SparkNeuralLayer`** — local, talks to tpt-spark's headless JSON-RPC socket
  (`\\.\pipe\tpt-spark` on Windows / unix socket elsewhere), driving `spark_infer`. Does not support
  `embed()` — tpt-spark has no embeddings endpoint as of this writing.
- **`OllamaNeuralLayer`** — local, talks to Ollama's HTTP API (`http://localhost:11434` by default,
  override via `OLLAMA_HOST`). Supports `embed()` via `/api/embeddings` since Ollama natively serves
  embedding models (e.g. `nomic-embed-text`) — the most readily-available local embeddings path today.
- **`CloudNeuralLayer`** — Anthropic Messages API, OpenAI Chat Completions, or OpenRouter's
  OpenAI-compatible gateway, selected via `CloudProvider`. OpenAI and OpenRouter support `embed()`;
  Anthropic does not.
- **`MockNeuralLayer`** — deterministic fixture backend returning spec.txt's exact worked-example
  facts. Used by `eve demo --backend mock` (the default) and by `eve-tests` so CI needs neither a live
  LLM process nor an API key.

**Embeddings are deliberately not a Phase 1 dependency for the local Spark path.** Phase 1's success
criteria (extract facts, apply rules, answer "what is" questions) need zero embedding/similarity
search. When Phase 3 (Abstraction) needs embeddings more heavily, prefer adding a `spark_embed` method
to tpt-spark over standing up a Python sentence-transformers subprocess — it keeps the whole stack in
the existing Rust/Spark process model. Ollama and OpenAI/OpenRouter are working alternatives until then.

### `eve-cli`

`eve extract <text>`, `eve demo`, `eve ask "<question>"`, with `--backend mock|spark|ollama|anthropic|
openai|openrouter`. `eve demo` is the literal Phase 1 acceptance test: extract facts from spec.txt's
TypeScript paragraph, load a `.eve` fixture with the transitive-`has` rule, forward-chain, check
consistency, and answer "What does TypeScript have?" using both asserted and inferred facts.

## Key Cross-Layer Relationships

- **`eve-neural` → `eve-symbolic`**: `NeuralPatternLayer::extract_facts` produces `eve_core::Fact`s that
  `WorkingMemory::add_fact` consumes directly — no adapter layer needed since both depend on the same
  `eve-core` types.
- **`eve-dsl` → `eve-symbolic`**: `eve-symbolic/src/lowering.rs` converts `eve_dsl::ast` types
  (`Term`, `RuleDef`, `QueryDef`) into `eve_core`/`eve_symbolic` runtime types (`Value`, `Pattern`,
  `Rule`) — `eve-dsl` stays purely syntactic and has no knowledge of working memory.
- **`tpt-spark` is an external integration, not a workspace member.** `eve-neural`'s `spark_backend.rs`
  is the only place that speaks tpt-spark's JSON-RPC protocol; see tpt-spark's `HEADLESS_API.md` for
  the wire contract.
- **`tpt-library` is intentionally not depended upon.** It's currently empty; tpt-eve was deliberately
  not chosen to seed it yet (decided when evaluating reuse of tpt-anima's `CausalGraph`, which turned
  out not to be a drop-in fit for Phase 1). Revisit when `eve-causal` lands in Phase 2.
