# tpt-eve-abstraction

[![crates.io](https://img.shields.io/crates/v/tpt-eve-abstraction.svg)](https://crates.io/crates/tpt-eve-abstraction)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-abstraction)](https://docs.rs/tpt-eve-abstraction)

Phase 3 abstraction layer for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): pattern mining,
analogical mapping, principle generalization, and cross-domain knowledge transfer.

- `PatternMiner` — mines `AbstractPrinciple`s from recurring structure in the fact base
- `AnalogyMapper` — builds `Analogy` mappings between a source and target domain
- `transfer_knowledge` — applies a mapped analogy to carry facts from one domain to another
- `AbstractionEngine` — ties the above together into the Phase 3 pipeline

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
