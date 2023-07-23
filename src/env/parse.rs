use chumsky::prelude::*;

use super::typehint_parser::{ParseTyeHint, TypeHint};

pub fn parser() -> impl Parser<char, Vec<String>, Error = Simple<char>> {
    let comment = just('#')
        .then(take_until(text::newline()))
        .map(|(_, (chars, ..))| String::from_iter(chars));

    let ident = text::ident();

    let value = take_until(text::newline()).map(|(chars, ..)| chars.iter().collect::<String>());

    let line = ident.then(just('=')).then(value).map(|((key, _), _)| key);

    line.padded().padded_by(comment.repeated()).repeated()
}

#[derive(Debug)]
pub struct Variable {
    pub type_hint: Option<TypeHint>,
    pub key: String,
}

pub fn parser_with_type_hint() -> impl Parser<char, Vec<Variable>, Error = Simple<char>> {
    let comment = just('#')
        .then(take_until(text::newline()))
        .map(|(_, (chars, ..))| String::from_iter(chars));

    let ident = text::ident();

    let value = take_until(text::newline()).map(|(chars, ..)| chars.iter().collect::<String>());

    let line = ident.then(just('=')).then(value).map(|((key, _), _)| key);

    comment
        .repeated()
        .then(line.padded())
        .map(|(comments, key)| Variable {
            key,
            type_hint: comments.last().and_then(|c| c.into_type_hint()),
        })
        .repeated()
}
