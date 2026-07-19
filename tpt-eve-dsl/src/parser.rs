// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::ast::{Program, QueryDef, RuleDef, Statement, Term, Triple};
use crate::error::ParseError;
use crate::lexer::{Lexer, Token};

/// Recursive-descent parser for the Eve DSL.
pub struct Parser {
    tokens: Vec<(Token, usize, usize)>,
    position: usize,
}

impl Parser {
    pub fn new(lexer: &mut Lexer) -> Self {
        let mut tokens = Vec::new();
        loop {
            let line = lexer.line();
            let col = lexer.column();
            match lexer.next_token() {
                Ok(Token::Eof) | Err(_) => {
                    tokens.push((Token::Eof, line, col));
                    break;
                }
                Ok(tok) => tokens.push((tok, line, col)),
            }
        }
        Parser {
            tokens,
            position: 0,
        }
    }

    pub fn parse(source: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(source);
        let mut parser = Parser::new(&mut lexer);
        parser.parse_program()
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position].0
    }

    fn span(&self) -> (usize, usize) {
        (self.tokens[self.position].1, self.tokens[self.position].2)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        let (line, col) = self.span();
        if self.current() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!("Expected {:?}, found {:?}", expected, self.current()),
                line,
                col,
            ))
        }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while self.current() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current() {
            Token::Question => self.parse_query(),
            Token::Rule => self.parse_rule(),
            Token::Ident(_) | Token::Literal(_) | Token::Is | Token::Has => self.parse_fact(),
            _ => {
                let (line, col) = self.span();
                Err(ParseError::new(
                    format!("Unexpected token {:?}", self.current()),
                    line,
                    col,
                ))
            }
        }
    }

    fn parse_fact(&mut self) -> Result<Statement, ParseError> {
        let subject = self.parse_term(false)?;
        let predicate = self.parse_term(false)?;
        let object = self.parse_term(false)?;
        self.expect(Token::Period)?;
        Ok(Statement::Fact(Triple {
            subject,
            predicate,
            object,
        }))
    }

    fn parse_rule(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Rule)?;
        let name = match self.current() {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        };
        // Skip LBrace if present (optional for readability)
        if self.current() == &Token::LBrace {
            self.advance();
        }
        self.expect(Token::If)?;
        let mut conditions = Vec::new();
        loop {
            let triple = self.parse_triple(true)?;
            conditions.push(triple);
            match self.current() {
                Token::Comma => {
                    self.advance();
                }
                Token::Then => break,
                _ => break,
            }
        }
        self.expect(Token::Then)?;
        let conclusion = self.parse_triple(true)?;
        // Skip RBrace if present
        if self.current() == &Token::RBrace {
            self.advance();
        }
        // Skip optional trailing period after rules
        if self.current() == &Token::Period {
            self.advance();
        }
        Ok(Statement::Rule(RuleDef {
            name,
            conditions,
            conclusion,
        }))
    }

    fn parse_query(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Question)?;
        // In queries, only the object can be a variable (for binding)
        let subject = self.parse_term(false)?;
        let predicate = self.parse_term(false)?;
        let object = self.parse_term(true)?; // Allow variables in object position
        self.expect(Token::Period)?;
        Ok(Statement::Query(QueryDef {
            clauses: vec![Triple {
                subject,
                predicate,
                object,
            }],
        }))
    }

    fn parse_triple(&mut self, in_rule: bool) -> Result<Triple, ParseError> {
        let subject = self.parse_term(in_rule)?;
        let predicate = self.parse_term(in_rule)?;
        let object = self.parse_term(in_rule)?;
        Ok(Triple {
            subject,
            predicate,
            object,
        })
    }

    /// Parse a term, with context for whether we're in a rule (where uppercase = variable)
    fn parse_term(&mut self, in_rule: bool) -> Result<Term, ParseError> {
        match self.current().clone() {
            Token::Ident(s) => {
                self.advance();
                // Uppercase-leading identifiers are variables only in rule context
                if in_rule && s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    Ok(Term::Var(s))
                } else {
                    Ok(Term::Ident(s))
                }
            }
            Token::Literal(s) => {
                let s = s.clone();
                self.advance();
                Ok(Term::Literal(s))
            }
            Token::Is => {
                self.advance();
                Ok(Term::Ident("is".into()))
            }
            Token::Has => {
                self.advance();
                Ok(Term::Ident("has".into()))
            }
            _ => {
                let (line, col) = self.span();
                Err(ParseError::new(
                    format!("Expected term, found {:?}", self.current()),
                    line,
                    col,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_fact() {
        let source = r#"TypeScript is "programming language"."#;
        let program = Parser::parse(source).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Fact(t) => {
                assert_eq!(t.subject, Term::Ident("TypeScript".into()));
                assert_eq!(t.predicate, Term::Ident("is".into()));
                assert_eq!(t.object, Term::Literal("programming language".into()));
            }
            _ => panic!("Expected fact"),
        }
    }

    #[test]
    fn parses_rule() {
        let source = r#"rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}
"#;
        let program = Parser::parse(source).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Rule(r) => {
                assert_eq!(r.name, Some("transitive_has".into()));
                assert_eq!(r.conditions.len(), 2);
                assert_eq!(r.conditions[0].subject, Term::Var("X".into()));
                assert_eq!(r.conclusion.predicate, Term::Ident("has".into()));
            }
            _ => panic!("Expected rule"),
        }
    }

    #[test]
    fn parses_query() {
        let source = "? TypeScript has Z.";
        let program = Parser::parse(source).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Query(q) => {
                assert_eq!(q.clauses.len(), 1);
                assert_eq!(q.clauses[0].subject, Term::Ident("TypeScript".into()));
                assert_eq!(q.clauses[0].predicate, Term::Ident("has".into()));
                assert_eq!(q.clauses[0].object, Term::Var("Z".into()));
            }
            _ => panic!("Expected query"),
        }
    }

    #[test]
    fn parses_multiple_facts() {
        let source = r#"TypeScript is "programming language".
"programming language" has syntax."#;
        let program = Parser::parse(source).unwrap();
        assert_eq!(program.statements.len(), 2);
    }

    #[test]
    fn parses_mixed_statements() {
        let source = r#"TypeScript is "programming language".
rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}
"#;
        let program = Parser::parse(source).unwrap();
        assert_eq!(program.statements.len(), 2);
    }
}
