use colored::Colorize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};

use crate::command::prettify;

use super::{
    parse::{get_texts, parse_variables_with_type_hints, Variable},
    typehint_parser::TypeHint,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("\n{a}\n\nconflicts with:\n\n{b}\n")]
    ConflictingTypes { a: TypeHintAt, b: TypeHintAt },
}

#[derive(Debug)]
pub struct TypeHintAt {
    pub th: TypeHint,
    pub line: usize,
    pub meta: Metadata,
}

impl Display for TypeHintAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = self.meta.source.lines().skip(self.line);

        let curr_line = lines
            .next()
            .expect("assumption that the type hint was parsed along with its line number, failed");
        let next_line = lines
            .next()
            .expect("assumption that the type hint was parsed along with the variable it was decorating, failed");

        writeln!(f, "{}", self.meta.path.to_string_lossy().dimmed())?;
        writeln!(
            f,
            "  {}| {}",
            self.line + 1,
            curr_line
                .replace(
                    self.th.to_string().as_str(),
                    self.th.to_string().green().to_string().as_str()
                )
                .bold()
        )?;
        write!(f, "  {}| {}", self.line + 2, next_line)?;

        Ok(())
    }
}

impl From<(&Metadata, &(TypeHint, usize))> for TypeHintAt {
    fn from(value: (&Metadata, &(TypeHint, usize))) -> Self {
        Self {
            th: value.1 .0.clone(),
            meta: value.0.clone(),
            line: value.1 .1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    source: Arc<str>,
    path: Arc<Path>,
}

pub fn generate_zod_schema(files: &[PathBuf]) -> Result<String> {
    let text_and_file_names = get_texts(files);

    let sources = text_and_file_names.iter().map(|(source, path)| Metadata {
        source: source.as_str().into(),
        path: path.as_path().into(),
    });

    generate_zod_schema_from_texts(sources)
}

pub fn generate_zod_schema_from_texts(sources: impl Iterator<Item = Metadata>) -> Result<String> {
    let mut map: BTreeMap<String, (Variable, Metadata)> = BTreeMap::new();

    let variables = sources.flat_map(|meta| -> Vec<(Variable, Metadata)> {
        let vars = parse_variables_with_type_hints(meta.source.deref())
            .into_iter()
            .map(|var| (var, meta.clone()))
            .collect();

        return vars;
    });

    for (var, meta) in variables {
        if let Some((v, o_meta)) = map.get(&var.key) {
            if let (Some(lt), Some(rt)) = (&v.type_hint, &var.type_hint) {
                if lt.0 != rt.0 {
                    return Err(ParseError::ConflictingTypes {
                        a: (o_meta, lt).into(),
                        b: (&meta, rt).into(),
                    })
                    .context(
                        "found some conflicting types while parsing variables with type hints",
                    );
                }
            }
        }

        map.insert(var.key.clone(), (var, meta));
    }

    let vars = map.into_values().map(|value| value.0).collect::<Vec<_>>();

    let next_public_vars = vars.iter().filter(|&v| v.is_public()).collect::<Vec<_>>();
    let other_vars = vars.iter().filter(|v| !v.is_public()).collect::<Vec<_>>();

    let to_field_schema = |var: &&Variable| -> String {
        format!(
            r#"    {}: {},"#,
            var.key,
            match &var.type_hint {
                Some(th) => match &th.0 {
                    super::typehint_parser::TypeHint::String => "z.string()".to_string(),
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

    File::create("./tsconfig.json")
        .context("failed to open tsconfig.json")
        .map(BufWriter::new)
        .and_then(|mut w| {
            let new_content = serde_json::to_string::<Value>(&ts_config)?;
            let pretty = prettify(new_content.as_bytes(), ".json")?;
            w.write_all(&pretty)
                .context("failed to write to tsconfig.json")?;
            w.flush()
                .context("failed to flush updated tsconfig.json contents")
        })
        .context("failed to update tsconfig.json contents")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use insta::{assert_debug_snapshot, assert_display_snapshot};

    use crate::dotenv::zod::{generate_zod_schema, generate_zod_schema_from_texts};

    #[test]
    fn zod_schema_gen() {
        let output = generate_zod_schema(&[
            PathBuf::from("src/dotenv/.env.test"),
            PathBuf::from("src/dotenv/.env.test2"),
        ])
        .unwrap();
        assert_display_snapshot!(output);
    }

    #[test]
    fn zod_schema_gen_with_type_conflicts() {
        let case = |s: &str| {
            format!(
                r#"
# @type {}
KEY=
            "#,
                s
            )
        };

        fn generate(sources: &[String]) -> Result<String, anyhow::Error> {
            let sources = sources.iter().cloned().enumerate().map(|(i, source)| {
                crate::dotenv::zod::Metadata {
                    source: source.as_str().into(),
                    path: Path::new(&format!("src/dotenv/.env.test.{}", i)).into(),
                }
            });

            generate_zod_schema_from_texts(sources)
        }

        fn gen_err(sources: &[String]) {
            let output = generate(sources).unwrap_err();
            assert_debug_snapshot!(output);
        }

        gen_err(&[case("string"), case("number")]);
        gen_err(&[case("number"), case("boolean")]);
        gen_err(&[case("string"), case("boolean")]);
        gen_err(&[case("'a' | 'b'"), case("number")]);

        gen_err(&[case("string"), case("boolean"), case("number")]);

        // This is not a conflict
        generate(&[case("string"), case("string")]).unwrap();
    }
}
