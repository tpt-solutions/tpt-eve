# tpt-eve-dsl

[![crates.io](https://img.shields.io/crates/v/tpt-eve-dsl.svg)](https://crates.io/crates/tpt-eve-dsl)
[![docs.rs](https://img.shields.io/docsrs/tpt-eve-dsl)](https://docs.rs/tpt-eve-dsl)

A small Datalog-like rule/query DSL used by [TPT Eve](https://github.com/tpt-solutions/tpt-eve):
a hand-written recursive-descent lexer and parser producing a syntactic AST (`Term`, `Triple`,
`RuleDef`, `QueryDef`, `Statement`, `Program`). Purely syntactic — no notion of working memory or
inference; that lowering happens in `tpt-eve-symbolic`.

## DSL Tour

### Facts

Facts are triples in the form `Subject predicate "object".`:

```
TypeScript is "programming language".
"programming language" has syntax.
TypeScript provides "compile-time type checking".
```

Subject and predicate are bare symbols; the object is a quoted string.
Facts end with a period.

### Rules

Rules define inference patterns using `if`/`then` with variables (`X`, `Y`, `Z`…):

```
rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}
```

Variables are uppercase identifiers. The `if` clause is a comma-separated list of
triples where any slot can be a variable. The `then` clause produces a single derived
triple.

### Queries

Queries ask the working memory for matching bindings:

```
? TypeScript has Z.
```

The `?` prefix marks a query. Unbound variables in the query return all matching
values from working memory after forward chaining.

### Full example

```
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

## Related crates

This isn't a general-purpose Datalog implementation — for that, see
[crepe](https://crates.io/crates/crepe) or [ascent](https://crates.io/crates/ascent), both more
mature and macro-driven. `tpt-eve-dsl` is intentionally narrow: it's the syntax layer feeding
`tpt-eve-symbolic`'s fact/provenance/confidence model, not a drop-in replacement for either.

## License

Licensed under either of [MIT](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-eve/blob/master/LICENSE-APACHE) at your option.
