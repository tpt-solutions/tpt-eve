# tpt-eve-symbolic

[![crates.io](https://img.shields.io/crates/v/tpt-eve-symbolic.svg)](https://crates.io/crates/tpt-eve-symbolic)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-symbolic)](https://docs.rs/tpt-eve-symbolic)

Phase 1 symbolic reasoning layer for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): working
memory, forward-chaining inference, consistency checking, and question answering, built on
`tpt-eve-core` and `tpt-eve-dsl`.

- `WorkingMemory` — indexed fact storage (by-subject/predicate/object `HashMap`s over a `Vec`) with
  exact-triple deduplication, keeping the highest-confidence fact on conflict
- `InferenceEngine` — naive fixpoint forward-chaining, rerunning every rule each pass until no new
  facts are derived or an iteration cap is hit
- `ConsistencyChecker` — flags structural contradictions (same subject+predicate, differing object,
  on an exclusive predicate like `is`)
- `WorkingMemoryStore` — embedded `redb`-backed persistence, no external service required
- `qa` — question answering over asserted and inferred facts

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
