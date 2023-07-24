use chumsky::prelude::*;
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, Context, Result};

use crate::pm::PackageManager;

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

pub fn npm_install() -> Result<()> {
    let package_info: Value = File::open("./package.json")
        .context("couldn't open package.json")
        .map(BufReader::new)
        .and_then(|reader| serde_json::from_reader(reader).context("failed to parse package.json"))
        .context("failed to read package.json")?;

    if package_info
        .get("dependencies")
        .and_then(|deps| deps.get("zod"))
        .is_some()
    {
        return Ok(());
    }

    eprintln!("Installing zod");
    let out = PackageManager::from_current_project()
        .ok_or(anyhow!("couldn't get package manager from current project"))
        .or(PackageManager::from_global())
        .map(|pm| match pm {
            PackageManager::Pnpm => ("pnpm", "add"),
            PackageManager::Yarn => ("yarn", "add"),
            PackageManager::Npm => ("npm", "i"),
        })
        .and_then(|(exe, arg)| {
            Command::new(exe)
                .arg(arg)
                .arg("zod")
                .output()
                .with_context(|| {
                    format!("failed to execute installation with package manager: {exe}")
                })
        })?;

    if !out.status.success() {
        return Err(anyhow!(
            "installation failed with exit code {:?}",
            out.status.code()
        ));
    }

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
