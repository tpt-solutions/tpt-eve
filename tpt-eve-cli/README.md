# tpt-eve-cli

[![crates.io](https://img.shields.io/crates/v/tpt-eve-cli.svg)](https://crates.io/crates/tpt-eve-cli)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-cli)](https://docs.rs/tpt-eve-cli)

The `tpt-eve` binary: the command-line entry point for [TPT Eve](https://github.com/tpt-solutions/tpt-eve),
wiring together all five layers (`tpt-eve-neural`, `tpt-eve-symbolic`, `tpt-eve-causal`,
`tpt-eve-abstraction`, `tpt-eve-active-learning`, `tpt-eve-metacognition`).

```bash
cargo install tpt-eve-cli

tpt-eve demo --backend mock      # Phase 1: extract → infer → ask
tpt-eve whatif --backend mock    # Phase 2: causal intervention + counterfactual
tpt-eve abstract                 # Phase 3: pattern mining, analogical mapping
tpt-eve learn                    # Phase 4: hypothesis, experiment, belief updating
tpt-eve reflect                  # Phase 5: confidence tracking, gap detection
tpt-eve repl                     # interactive REPL
tpt-eve serve                    # HTTP REST API
```

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full CLI
reference and backend/environment-variable table.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
