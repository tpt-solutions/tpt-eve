# tpt-eve-dsl

[![crates.io](https://img.shields.io/crates/v/tpt-eve-dsl.svg)](https://crates.io/crates/tpt-eve-dsl)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-dsl)](https://docs.rs/tpt-eve-dsl)

A small Datalog-like rule/query DSL used by [TPT Eve](https://github.com/tpt-solutions/tpt-eve):
a hand-written recursive-descent lexer and parser producing a syntactic AST (`Term`, `Triple`,
`RuleDef`, `QueryDef`, `Statement`, `Program`). Purely syntactic — no notion of working memory or
inference; that lowering happens in `tpt-eve-symbolic`.

```text
TypeScript is "programming language".
"programming language" has syntax.

rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}

? TypeScript has Z.
```

```rust
let program = tpt_eve_dsl::parse(source)?;
```

See the [workspace README](https://github.com/tpt-solutions/tpt-eve#readme) for the full
architecture and CLI.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
