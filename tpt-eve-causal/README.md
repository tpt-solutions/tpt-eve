# tpt-eve-causal

[![crates.io](https://img.shields.io/crates/v/tpt-eve-causal.svg)](https://crates.io/crates/tpt-eve-causal)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-causal)](https://docs.rs/tpt-eve-causal)

Phase 2 causal reasoning engine for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): DAG
construction, structural causal models, intervention calculus, and counterfactual simulation.

- `CausalGraphBuilder` / `CausalGraph` — builds a causal DAG from observations, rejecting cycles
- `StructuralCausalModel` — the underlying SCM backing intervention and counterfactual queries
- `predict_intervention` — "what if we intervened on X?" effect prediction
- `simulate_counterfactual` — "what would have happened if X had been different?" simulation
- `identify_confounders` / `estimate_causal_effect` — confounder detection and effect estimation

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## Related crates

For standalone, stats/ML-driven causal discovery (NOTEARS, PC/GES, causal forests, do-calculus over
tabular data), see [deep_causality](https://crates.io/crates/deep_causality) or
[causal-hub](https://crates.io/crates/causal-hub) — both more general than this crate.
`tpt-eve-causal` is narrower by design: it builds and reasons over DAGs assembled from
`tpt-eve-core::CausalRelationDraft`s produced by LLM extraction, not from raw observational datasets.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
