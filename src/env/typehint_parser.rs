#[derive(Debug)]
pub enum TypeHint {
    String,
    Number,
    Boolean,
    Union(Vec<String>),
}

pub trait ParseTyeHint {
    fn into_type_hint(self) -> Option<TypeHint>;
}

impl ParseTyeHint for &str {
    fn into_type_hint(self) -> Option<TypeHint> {
        Parser::new(self).parse()
    }
}

#[derive(PartialEq, Debug)]
enum TokenKind {
    Keyword,
    StringType,
    NumberType,
    BooleanType,
    StringLiteral,
    Pipe,
    Eof,
    Illegal,
}

#[derive(Debug)]
struct Token<'source> {
    kind: TokenKind,
    start: usize,
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
                start: self.position,
                text: ""
            }
        };

        let token = match ch {
            b'@' => self.lex_keyword("@type"),
            b'\'' => self.lex_string_literal(),
            b'|' => Token {
                kind: TokenKind::Pipe,
                start: self.position,
                text: "|",
            },
            c if c.is_ascii_alphabetic() => self.lex_type(),
            _ => Token {
                kind: TokenKind::Illegal,
                start: self.position,
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
                start,
                text: s,
            },
            "number" => Token {
                kind: TokenKind::NumberType,
                start,
                text: s,
            },
            "boolean" => Token {
                kind: TokenKind::BooleanType,
                start,
                text: s,
            },
            _ => Token {
                kind: TokenKind::Illegal,
                start,
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
                start,
                text: s,
            };
        }

        return Token {
            kind: TokenKind::Illegal,
            start,
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
                start,
                text: s,
            };
        };

        self.step();

        let s = &self.source[start..self.position];

        return Token {
            kind: TokenKind::StringLiteral,
            start,
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

    pub fn parse(&mut self) -> Option<TypeHint> {
        if self.token.kind != TokenKind::Keyword {
            return None;
        }

        while self.token.kind != TokenKind::Eof {
            match self.token.kind {
                TokenKind::StringType => return Some(TypeHint::String),
                TokenKind::NumberType => return Some(TypeHint::Number),
                TokenKind::BooleanType => return Some(TypeHint::Boolean),
                TokenKind::StringLiteral => {
                    let mut union = vec![self.token.text.to_string()];

                    while self.next_token().kind == TokenKind::Pipe
                        && self.token.kind != TokenKind::Eof
                    {
                        // just ignore any bunch of consecutive pipes
                        while self.next_token().kind == TokenKind::Pipe {}
                        union.push(self.token.text.to_string());
                    }

                    return Some(TypeHint::Union(union));
                }
                TokenKind::Eof => return None,
                TokenKind::Pipe => {}
                TokenKind::Illegal => {}
                TokenKind::Keyword => {}
            }

            self.next_token();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use crate::env::typehint_parser::{Lexer, Parser};

    #[test]
    fn lexing_type_hints() {
        assert_debug_snapshot!(Lexer::new("@type string").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type number").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type boolean").collect::<Vec<_>>());
        assert_debug_snapshot!(Lexer::new("@type 'qa' | 'dev' | 'prod'").collect::<Vec<_>>());
    }

    #[test]
    fn parse_type_hints() {
        assert_debug_snapshot!(Parser::new("@type string").parse());
        assert_debug_snapshot!(Parser::new("@type number").parse());
        assert_debug_snapshot!(Parser::new("@type boolean").parse());
        assert_debug_snapshot!(Parser::new("@type 'qa' | 'dev' | 'prod'").parse());
        assert_debug_snapshot!(Parser::new("@type 'qa' || 'dev' ||| | 'prod' | || 'test'").parse());
    }
}
