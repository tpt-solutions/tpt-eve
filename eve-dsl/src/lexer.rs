use crate::error::ParseError;

/// Token types for the DSL lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// An identifier (e.g., "TypeScript", "has")
    Ident(String),
    /// A quoted string literal (e.g., "programming language")
    Literal(String),
    /// The "is" keyword (common predicate)
    Is,
    /// The "rule" keyword (rule definition)
    Rule,
    /// The "if" keyword (rule condition)
    If,
    /// The "then" keyword (rule conclusion)
    Then,
    /// The "has" keyword (common predicate)
    Has,
    /// The question mark (query start)
    Question,
    /// A period (statement terminator)
    Period,
    /// A comma (unused in DSL v1, but reserved)
    Comma,
    /// Left curly brace (rule body)
    LBrace,
    /// Right curly brace (rule body end)
    RBrace,
    /// End of input
    Eof,
}

/// Lexer state tracking position in the source.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            chars: source.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self, first_char: char) -> String {
        let mut ident = String::new();
        ident.push(first_char);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_literal(&mut self) -> Result<String, ParseError> {
        let mut literal = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance(); // consume closing quote
                return Ok(literal);
            } else if c == '\\' {
                self.advance();
                match self.peek() {
                    Some('n') => {
                        literal.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        literal.push('\t');
                        self.advance();
                    }
                    Some('"') => {
                        literal.push('"');
                        self.advance();
                    }
                    Some('\\') => {
                        literal.push('\\');
                        self.advance();
                    }
                    Some(c) => {
                        literal.push(c);
                        self.advance();
                    }
                    None => {
                        return Err(ParseError::new(
                            "Unterminated string literal".to_string(),
                            self.line,
                            self.column,
                        ));
                    }
                }
            } else {
                literal.push(c);
                self.advance();
            }
        }
        Err(ParseError::new(
            "Unterminated string literal".to_string(),
            self.line,
            self.column,
        ))
    }

    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();
        let start_line = self.line;
        let start_col = self.column;

        match self.advance() {
            None => Ok(Token::Eof),
            Some(c) => {
                match c {
                    '.' => Ok(Token::Period),
                    ',' => Ok(Token::Comma),
                    '{' => Ok(Token::LBrace),
                    '}' => Ok(Token::RBrace),
                    '?' => Ok(Token::Question),
                    '"' => {
                        let lit = self.read_literal()?;
                        Ok(Token::Literal(lit))
                    }
                    _ if c.is_alphabetic() || c == '_' => {
                        let ident = self.read_ident(c);
                        let lower = ident.to_lowercase();
                        match lower.as_str() {
                            "is" => Ok(Token::Is),
                            "has" => Ok(Token::Has),
                            "rule" => Ok(Token::Rule),
                            "if" => Ok(Token::If),
                            "then" => Ok(Token::Then),
                            _ => Ok(Token::Ident(ident)),
                        }
                    }
                    _ => Err(ParseError::new(
                        format!("Unexpected character: '{}'", c),
                        start_line,
                        start_col,
                    )),
                }
            }
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_simple_fact() {
        let source = r#"TypeScript is "programming language"."#;
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("TypeScript".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Is);
        assert_eq!(lexer.next_token().unwrap(), Token::Literal("programming language".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Period);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn lexes_rule_definition() {
        let source = r#"rule transitive_has {
  if X is Y, Y has Z
  then X has Z
}
"#;
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next_token().unwrap(), Token::Rule);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("transitive_has".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::LBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::If);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("X".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Is);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("Y".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Comma);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("Y".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Has);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("Z".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Then);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("X".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Has);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("Z".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::RBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn lexes_braces() {
        let source = "{}";
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next_token().unwrap(), Token::LBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::RBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn lexes_query() {
        let source = "? TypeScript has Z.";
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.next_token().unwrap(), Token::Question);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("TypeScript".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Has);
        assert_eq!(lexer.next_token().unwrap(), Token::Ident("Z".into()));
        assert_eq!(lexer.next_token().unwrap(), Token::Period);
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn reports_unterminated_literal() {
        let source = r#"TypeScript is "programming language"#;
        let mut lexer = Lexer::new(source);
        lexer.next_token(); // TypeScript
        lexer.next_token(); // is
        let result = lexer.next_token();
        assert!(matches!(result, Err(ParseError { .. })));
    }
}