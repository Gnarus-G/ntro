use chumsky::prelude::*;
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};

use super::parse::{parser_with_type_hint, Variable};

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

    let next_public_vars = vars.iter().filter(|v| v.is_public()).collect::<Vec<_>>();
    let other_vars = vars.iter().filter(|v| !v.is_public()).collect::<Vec<_>>();

    let to_field_schema = |var: &&Variable| -> String {
        format!(
            r#"    {}: {},"#,
            var.key,
            match &var.type_hint {
                Some(th) => match th {
                    super::typehint_parser::TypeHint::String => "z.coerce.string()".to_string(),
                    super::typehint_parser::TypeHint::Number => "z.coerce.number()".to_string(),
                    super::typehint_parser::TypeHint::Boolean => "z.coerce.boolean()".to_string(),
                    super::typehint_parser::TypeHint::Union(values) =>
                        format!("z.enum([{}])", values.join(",")),
                },
                None => "z.string()".to_string(),
            }
        )
    };

    let js_code = include_str!("module.ts");

    let js_import_line: &str = js_code
        .lines()
        .next()
        .expect("should have an import line at the top of the js implementation");

    let js_impl = js_code
        .lines()
        .skip_while(|line| !line.contains("/* --- MAIN IMPLEMENTATION BELOW --- */"))
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");

    let output = format!(
        r#"
{js_import_line}

const clientEnvSchemas = {{
{}
}}

const serverEnvSchemas = {{
    ...clientEnvSchemas,
{}
}}

{js_impl}

const processEnv = {{
{}
}}
               "#,
        next_public_vars
            .iter()
            .map(to_field_schema)
            .collect::<Vec<_>>()
            .join("\n"),
        other_vars
            .iter()
            .map(to_field_schema)
            .collect::<Vec<_>>()
            .join("\n"),
        vars.iter()
            .map(|var| format!("   {}: process.env.{},", var.key, var.key))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    Ok(output)
}

pub fn add_tsconfig_path<P: AsRef<Path>>(path: P) -> Result<()> {
    let mut ts_config: Value = File::open("./tsconfig.json")
        .context("couldn't open tsconfig.json")
        .map(BufReader::new)
        .and_then(|reader| serde_json::from_reader(reader).context("failed to parse tsconfig.json"))
        .context("failed to read tsconfig.json")?;

    ts_config
        .get_mut("compilerOptions")
        .context("couldn't find compilerOptions in tsconfig.json")
        .and_then(|paths| {
            paths
                .get_mut("paths")
                .and_then(|node| node.as_object_mut())
                .map(|paths| {
                    paths.insert(
                        "$env".to_string(),
                        Value::Array(vec![Value::String(
                            path.as_ref().to_string_lossy().to_string(),
                        )]),
                    )
                })
                .ok_or(anyhow!("failed to add $env as a path on tsconfig.json"))
        })?;

    File::options()
        .write(true)
        .open("./tsconfig.json")
        .map(BufWriter::new)
        .and_then(|mut w| {
            let new_content = serde_json::to_string_pretty::<Value>(&ts_config)?;
            w.write_all(new_content.as_bytes())?;
            w.flush()
        })
        .context("failed to flush updated tsconfig.json contents")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use insta::assert_display_snapshot;

    use crate::dotenv::zod::generate_zod_schema;

    #[test]
    fn zod_schema_gen() {
        let output = generate_zod_schema(&[
            PathBuf::from("src/dotenv/.env.test"),
            PathBuf::from("src/dotenv/.env.test2"),
        ])
        .unwrap();
        assert_display_snapshot!(output);
    }
}
