# tpt-eve-core

[![crates.io](https://img.shields.io/crates/v/tpt-eve-core.svg)](https://crates.io/crates/tpt-eve-core)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-core)](https://docs.rs/tpt-eve-core)

Shared core types for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): `Fact`, `Pattern`,
`Value`, `EveError`, and `CausalRelationDraft`. Every other `tpt-eve-*` crate depends on this one;
it has no internal dependencies of its own.

- `Value` — a `Symbol` or `Text` term
- `Fact` — a subject-predicate-object triple with `FactSource` provenance and a confidence score
- `Pattern` — a triple where any slot may be bound or a variable, used for rule matching
- `EveError` — the shared error type returned across the workspace
- `CausalRelationDraft` — a causal-relation type consumed by `tpt-eve-causal` and produced by
  `tpt-eve-neural`'s extraction backends

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
