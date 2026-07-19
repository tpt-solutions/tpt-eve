# tpt-eve-active-learning

[![crates.io](https://img.shields.io/crates/v/tpt-eve-active-learning.svg)](https://crates.io/crates/tpt-eve-active-learning)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-active-learning)](https://docs.rs/tpt-eve-active-learning)

Phase 4 active learning layer for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): hypothesis
generation, experiment design, and Bayesian belief updating.

- `HypothesisGenerator` — proposes `Hypothesis`es from gaps in the current knowledge base
- `ExperimentRunner` / `ExperimentDesign` — designs and runs experiments to test a hypothesis,
  producing an `ExperimentResult`
- `BeliefUpdater` — updates `BeliefState` confidence via Bayesian updating from experiment outcomes
- `ActiveLearningEngine` — the Phase 4 pipeline: hypothesize → experiment → update belief

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
