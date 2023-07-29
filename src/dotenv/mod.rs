use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use self::parse::parse_variables;

mod parse;

mod typehint_parser;
pub mod zod;

pub fn generate_typescript_types(files: &[PathBuf]) -> Result<String> {
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
                .map(|text| {
                    parse_variables(&text)
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                })
        })
        .filter_map(|result| {
            if let Err(e) = &result {
                log::error!("{e:?}");
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
            .map(|var| format!(
                r#"
         {}?: string"#,
                var
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use insta::assert_display_snapshot;

    use crate::dotenv::generate_typescript_types;

    #[test]
    fn introspect_typescript_types_gen() {
        let output = generate_typescript_types(&[
            PathBuf::from("src/dotenv/.env.test"),
            PathBuf::from("src/dotenv/.env.test2"),
        ])
        .unwrap();
        assert_display_snapshot!(output);
    }
}
