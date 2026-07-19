// SPDX-License-Identifier: MIT OR Apache-2.0
mod ast;
mod error;
mod lexer;
mod parser;

pub use ast::{Program, QueryDef, RuleDef, Statement, Term, Triple};
pub use error::ParseError;

/// Parse a DSL source string into a Program.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    parser::Parser::parse(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fact_and_rule() {
        let source = r#"TypeScript is "programming language".
rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}
"#;
        let program = parse(source).unwrap();
        assert_eq!(program.statements.len(), 2);
    }
}
