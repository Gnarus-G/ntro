use anyhow::{anyhow, Context, Result};
use chumsky::prelude::*;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

pub fn generate_typescript_types(files: &[PathBuf]) -> Result<String> {
    let parse = |text, file_name| {
        parser().parse(text).map_err(|err| {
            anyhow!(
                "failed to parse {:?}: {}",
                file_name,
                err.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
    };

    let vars = files
        .iter()
        .map(|file| {
            File::open(file)
                .map(BufReader::new)
                .and_then(|mut rdr| {
                    let mut buf = String::new();
                    rdr.read_to_string(&mut buf).map(|_| buf)
                })
                .context(format!("failed read {file:?}"))
                .and_then(|text| parse(text, file))
        })
        .filter_map(|result| {
            if let Err(e) = &result {
                eprintln!("{e:?}");
            }
            result.ok()
        })
        .flatten()
        .collect::<BTreeSet<_>>();

    let output = format!(
        r#"
declare namespace NodeJS {{
    interface ProcessEnv {{
        {}
    }}
}}
               "#,
        vars.iter()
            .map(|v| format!(
                r#"
         {}: string | undefined"#,
                v.key
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(output)
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Variable {
    key: String,
}

fn parser() -> impl Parser<char, Vec<Variable>, Error = Simple<char>> {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use insta::assert_display_snapshot;

    use crate::env::generate_typescript_types;

    #[test]
    fn introspect_typescript_types_gen() {
        let output = generate_typescript_types(&[
            PathBuf::from("src/.env.test"),
            PathBuf::from("src/.env.test2"),
        ])
        .unwrap();
        assert_display_snapshot!(output);
    }
}
