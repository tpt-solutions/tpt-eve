# tpt-eve-neural

[![crates.io](https://img.shields.io/crates/v/tpt-eve-neural.svg)](https://crates.io/crates/tpt-eve-neural)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-neural)](https://docs.rs/tpt-eve-neural)

Pluggable LLM integration for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): the
`NeuralPatternLayer` trait (`extract_facts`, `extract_causal_relations`, `embed`) plus backends:

| Backend | Transport | `embed()` |
|---|---|---|
| `SparkNeuralLayer` | tpt-spark headless JSON-RPC (local) | not supported |
| `OllamaNeuralLayer` | Ollama HTTP API (local) | supported via `/api/embeddings` |
| `CloudNeuralLayer` | Anthropic Messages / OpenAI / OpenRouter | OpenAI & OpenRouter only |
| `MockNeuralLayer` | deterministic fixtures, no network | not supported |

`MockNeuralLayer` is what `tpt-eve-cli demo --backend mock` and the workspace test suite use, so
neither needs a live LLM process or an API key.

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## Related crates

Not a general-purpose multi-provider chat client — see [llm](https://crates.io/crates/llm),
[rllm](https://crates.io/crates/rllm), or [llm-connector](https://crates.io/crates/llm-connector)
for that (10+ providers vs. the four backends here). `tpt-eve-neural`'s differentiator is the
`NeuralPatternLayer` trait's extraction-specific shape (`extract_facts`, `extract_causal_relations`
returning `tpt-eve-core` types directly) and the `SparkNeuralLayer` backend, which talks to
tpt-spark's local JSON-RPC socket — no other crate integrates with tpt-spark.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
