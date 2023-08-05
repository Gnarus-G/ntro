use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeHint {
    String,
    Number,
    Boolean,
    Union(Box<[Box<str>]>),
}

impl Display for TypeHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TypeHint::Union(values) => values
                .iter()
                .map(|a| a.as_ref())
                .collect::<Vec<_>>()
                .join(" | "),
            tk => format!("{tk:?}").to_lowercase(),
        };

        f.write_str(&s)
    }
}

pub trait ParseTyeHint {
    fn into_type_hint(self) -> Option<TypeHint>;
}

impl ParseTyeHint for &str {
    fn into_type_hint(self) -> Option<TypeHint> {
        Parser::new(self).parse().ok()
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum TokenKind {
    Keyword,
    Pound,
    StringType,
    NumberType,
    BooleanType,
    StringLiteral,
    Pipe,
    Eof,
    Illegal,
}

#[derive(Debug, Clone, Copy)]
struct Token<'source> {
    kind: TokenKind,
    text: &'source str,
}

struct Lexer<'source> {
    source: &'source str,
    position: usize,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            position: 0,
            source,
        }
    }

    fn char(&self) -> Option<&'source u8> {
        self.source.as_bytes().get(self.position)
    }

    fn char_skipping_whitespace(&mut self) -> Option<&'source u8> {
        while self
            .char()
            .map(|&c| c.is_ascii_whitespace())
            .unwrap_or(false)
        {
            self.step();
        }

        return self.char();
    }

    fn step(&mut self) {
        self.position += 1;
    }

    pub fn next_token(&mut self) -> Token<'source> {
        let Some(ch) = self.char_skipping_whitespace() else {
            return Token{
                kind: TokenKind::Eof,
                text: ""
            }
        };

        let token = match ch {
            b'@' => self.lex_keyword("@type"),
            b'\'' => self.lex_string_literal(),
            b'|' => Token {
                kind: TokenKind::Pipe,
                text: "|",
            },
            c if c.is_ascii_alphabetic() => self.lex_type(),
            b'#' => Token {
                kind: TokenKind::Pound,
                text: "#",
            },
            _ => Token {
                kind: TokenKind::Illegal,
                text: &self.source[self.position..self.position + 1],
            },
        };

        self.step();

        token
    }

    fn lex_type(&mut self) -> Token<'source> {
        let start = self.position;

        while self
            .char()
            .map(|&c| c.is_ascii_lowercase())
            .unwrap_or(false)
        {
            self.step();
        }

        let s = &self.source[start..self.position];

        match s {
            "string" => Token {
                kind: TokenKind::StringType,
                text: s,
            },
            "number" => Token {
                kind: TokenKind::NumberType,
                text: s,
            },
            "boolean" => Token {
                kind: TokenKind::BooleanType,
                text: s,
            },
            _ => Token {
                kind: TokenKind::Illegal,
                text: s,
            },
        }
    }

    fn lex_keyword(&mut self, keyword: &str) -> Token<'source> {
        let start = self.position;

        self.step();

        while self
            .char()
            .map(|&c| c.is_ascii_lowercase())
            .unwrap_or(false)
        {
            self.step();
        }

        let s = &self.source[start..self.position];

        if s == keyword {
            return Token {
                kind: TokenKind::Keyword,
                text: s,
            };
        }

        return Token {
            kind: TokenKind::Illegal,
            text: s,
        };
    }

    fn lex_string_literal(&mut self) -> Token<'source> {
        let start = self.position;

        self.step();

        while self.char().map(|&c| c != b'\'').unwrap_or(false) {
            self.step();
        }

        let Some(b'\'') = self.char() else {
            let s = &self.source[start..self.position];
            return Token {
                kind: TokenKind::Illegal,
                text: s,
            };
        };

        self.step();

        let s = &self.source[start..self.position];

        return Token {
            kind: TokenKind::StringLiteral,
            text: s,
        };
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Token<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();

        if token.kind == TokenKind::Eof {
            return None;
        }

        return Some(token);
    }
}

use thiserror::Error;

#[derive(Debug, Error)]
enum ParseError<'source> {
    #[error("expected to find {expected:?} but found {found:?}")]
    ExpectedToken {
        expected: TokenKind,
        found: Token<'source>,
    },
    #[error("unexpected end of input")]
    UnexpectedEnd,
    #[error("unexpected token found: {found:?}")]
    IllegalToken { found: Token<'source> },
}

struct Parser<'source> {
    lexer: Lexer<'source>,
    token: Token<'source>,
    peeked: Option<Token<'source>>,
}

impl<'source> Parser<'source> {
    pub fn new(code: &'source str) -> Self {
        let mut lexer = Lexer::new(code);
        Self {
            peeked: None,
            token: lexer.next_token(),
            lexer,
        }
    }

    fn next_token(&mut self) -> &Token<'source> {
        self.token = match self.peeked.take() {
            Some(t) => t,
            None => self.lexer.next_token(),
        };

        &self.token
    }

    // fn peek_token(&mut self) -> &Token<'source> {
    //     self.peeked.get_or_insert_with(|| self.lexer.next_token())
    // }

    pub fn parse(&mut self) -> Result<TypeHint, ParseError<'source>> {
        if self.token.kind == TokenKind::Pound {
            self.next_token();
        }

        self.expect(TokenKind::Keyword)?;

        self.next_token();

        match self.token.kind {
            TokenKind::StringType => return Ok(TypeHint::String),
            TokenKind::NumberType => return Ok(TypeHint::Number),
            TokenKind::BooleanType => return Ok(TypeHint::Boolean),
            TokenKind::StringLiteral => {
                let mut union: Vec<Box<str>> = vec![self.token.text.into()];

                while self.next_token().kind == TokenKind::Pipe && self.token.kind != TokenKind::Eof
                {
                    // just ignore any bunch of consecutive pipes
                    while self.next_token().kind == TokenKind::Pipe {}
                    union.push(self.token.text.into());
                }

                return Ok(TypeHint::Union(union.into()));
            }
            TokenKind::Eof => return Err(ParseError::UnexpectedEnd),
            TokenKind::Pipe | TokenKind::Illegal | TokenKind::Keyword | TokenKind::Pound => {
                return Err(ParseError::IllegalToken { found: self.token })
            }
        }
    }

    fn expect(&self, kind: TokenKind) -> Result<(), ParseError<'source>> {
        if self.token.kind != kind {
            return Err(ParseError::ExpectedToken {
                expected: kind,
                found: self.token,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use crate::dotenv::typehint_parser::{Lexer, Parser};

    #[test]
    fn lexing_type_hints() {
        assert_debug_snapshot!(Lexer::new("@type string").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("# @type string").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type number").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("# @type number").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type boolean").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type 'qa' | 'dev' | 'prod'").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("# @type 'qa' | 'dev' | 'prod'").collect::<Vec<_>>());
    }

    #[test]
    fn parse_type_hints() {
        assert_debug_snapshot!(Parser::new("@type string").parse());
        assert_debug_snapshot!(Parser::new("# @type string").parse());
        assert_debug_snapshot!(Parser::new("@type number").parse());
        assert_debug_snapshot!(Parser::new("@type boolean").parse());
        assert_debug_snapshot!(Parser::new("@type 'qa' | 'dev' | 'prod'").parse());
        assert_debug_snapshot!(Parser::new("@type 'qa' || 'dev' ||| | 'prod' | || 'test'").parse());
    }
}
