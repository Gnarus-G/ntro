use chumsky::prelude::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable {
    pub key: String,
}

pub fn parser() -> impl Parser<char, Vec<Variable>, Error = Simple<char>> {
    let comment = just('#')
        .then(take_until(text::newline()))
        .map(|(_, (chars, ..))| String::from_iter(chars));

    let ident = text::ident();

    let value = take_until(text::newline()).map(|(chars, ..)| chars.iter().collect::<String>());

    let line = ident.then(just('=')).then(value).map(|((key, _), _)| {
        return Variable { key };
    });

    line.padded().padded_by(comment.repeated()).repeated()
}
