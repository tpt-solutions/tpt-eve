# tpt-eve-metacognition

[![crates.io](https://img.shields.io/crates/v/tpt-eve-metacognition.svg)](https://crates.io/crates/tpt-eve-metacognition)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-metacognition)](https://docs.rs/tpt-eve-metacognition)

Phase 5 meta-cognition layer for [TPT Eve](https://github.com/tpt-solutions/tpt-eve): confidence
tracking, knowledge-gap detection, and strategic learning prioritization.

- `ConfidenceTracker` — tracks per-fact `ConfidenceRecord` history and calibration error
- `KnowledgeGapDetector` — finds `KnowledgeGap`s (low-confidence facts, missing attributes,
  unexplained entities) by `GapType`
- `LearningPrioritizer` — ranks knowledge gaps into a `LearningPriority` agenda
- `UnderstandingMonitor` — reports `UnderstandingLevel` and raises `ClarificationRequest`s
- `MetaCognitionEngine` — the Phase 5 pipeline tying the above together

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
