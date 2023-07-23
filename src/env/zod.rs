use chumsky::prelude::*;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};

use super::parse::parser_with_type_hint;

pub fn generate_zod_schema(files: &[PathBuf]) -> Result<String> {
    let parse = |text, file_name| {
        parser_with_type_hint().parse(text).map_err(|err| {
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
        .collect::<Vec<_>>();

    let output = format!(
        r#"
import z from "zod";

const envSchema = z.object({{
{}
}})

export const env = envSchema.parse({{
{}
}})
               "#,
        vars.iter()
            .map(|var| format!(
                r#"    {}: {},"#,
                var.key,
                match &var.type_hint {
                    Some(th) => match th {
                        super::typehint_parser::TypeHint::String => "z.string()".to_string(),
                        super::typehint_parser::TypeHint::Number => "z.number()".to_string(),
                        super::typehint_parser::TypeHint::Boolean => "z.boolean()".to_string(),
                        super::typehint_parser::TypeHint::Union(values) =>
                            format!("z.enum([{}])", values.join(",")),
                    },
                    None => "z.string()".to_string(),
                }
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        vars.iter()
            .map(|var| format!("   {}: process.env.{},", var.key, var.key))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use insta::assert_display_snapshot;

    use crate::env::zod::generate_zod_schema;

    #[test]
    fn zod_schema_gen() {
        let output = generate_zod_schema(&[
            PathBuf::from("src/.env.test"),
            PathBuf::from("src/.env.test2"),
        ])
        .unwrap();
        assert_display_snapshot!(output);
    }
}
