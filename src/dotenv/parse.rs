use super::typehint_parser::{ParseTyeHint, TypeHint};

type WithLineNumber<T> = (T, usize);

#[derive(Debug)]
pub struct Variable {
    pub type_hint: Option<WithLineNumber<TypeHint>>,
    pub key: String,
}

impl Variable {
    pub fn is_public(&self) -> bool {
        self.key.starts_with("NEXT_PUBLIC_")
    }
}

pub fn parse_variables(source: &str) -> Vec<&str> {
    return source
        .lines()
        .filter_map(|line| {
            if line.starts_with('#') {
                return None;
            }
            return match line.split('=').collect::<Vec<_>>()[..] {
                [ident, ..] if !ident.is_empty() => Some(ident.trim()),
                _ => None,
            };
        })
        .collect::<Vec<_>>();
}

pub fn parse_variables_with_type_hints(source: &str) -> Vec<Variable> {
    enum Token<'source> {
        LineComment(&'source str, usize),
        Ident(&'source str, usize),
    }

    let mut tokens = source
        .lines()
        .enumerate()
        .filter_map(|(l_num, line)| {
            if line.starts_with('#') {
                return Some(Token::LineComment(line, l_num));
            }
            return match line.split('=').collect::<Vec<_>>()[..] {
                [ident, ..] if !ident.is_empty() => Some(Token::Ident(ident.trim(), l_num)),
                _ => None,
            };
        })
        .peekable();

    let mut vars = Vec::new();

    loop {
        match tokens.next() {
            None => break,
            Some(token) => match (token, tokens.peek()) {
                (Token::LineComment(comment, l_num), Some(Token::Ident(ident, _))) => {
                    let var = Variable {
                        type_hint: comment.into_type_hint().map(|th| (th, l_num)),
                        key: ident.to_string(),
                    };
                    vars.push(var);
                    tokens.next();
                }
                (Token::Ident(ident, _), _) => {
                    vars.push(Variable {
                        type_hint: None,
                        key: ident.to_string(),
                    });
                }

                _ => {}
            },
        };
    }

    vars
}
