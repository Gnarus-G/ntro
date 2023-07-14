use std::{
    error::Error,
    fs::File,
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use clap::{CommandFactory, Parser, Subcommand};
use serde::Deserialize;
use serde_yaml::Value;

#[derive(Parser, Debug)]
#[clap(author)]
/// Generate types from configuration files
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Generate typescript types from yaml files.
    Yaml {
        /// Path to a yaml file.
        source_file: PathBuf,

        /// Set the output directory, to where to save the *.d.ts file.
        #[arg(short)]
        output_dir: Option<PathBuf>,
    },
    /// Generate a completions file for a specified shell
    Completion {
        // The shell for which to generate completions
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Yaml {
            source_file,
            output_dir,
        } => {
            let content = generate_typescript_types(&source_file)?;

            let output_path = output_dir
                .unwrap_or_default()
                .join(source_file.with_extension("d.ts").file_name().expect(
                "the file path given should have had a filename for its yaml content to be parsed",
            ));

            let mut ofile = File::create(output_path)?;

            ofile.write_all(content.as_bytes())?;
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ntro", &mut std::io::stdout())
        }
    };

    Ok(())
}

fn generate_typescript_types(file: &Path) -> Result<String, Box<dyn Error>> {
    match parse_yaml(file)? {
        Parsed::One(document) => Ok(format!(
            "declare type {} = {:#}",
            file_name_to_type_name(
                file.file_stem()
                    .expect("couldn't parse a filename from input")
                    .to_str()
                    .expect("path given should be in utf-8")
            ),
            introspect_typescript_types(document)
        )),
        Parsed::Many(documents) => Ok(format!(
            "declare namespace {} {{ {:#} }}",
            file_name_to_type_name(
                file.file_stem()
                    .expect("couldn't parse a filename from input")
                    .to_str()
                    .expect("path given should be in utf-8")
            ),
            documents
                .into_iter()
                .map(introspect_typescript_types)
                .enumerate()
                .map(|(idx, text)| format!("export type Document{idx} = {text}"))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

enum Parsed {
    One(Value),
    Many(Vec<Value>),
}

fn parse_yaml(file: &Path) -> Result<Parsed, Box<dyn Error>> {
    let rdr = BufReader::new(File::open(file)?);
    let mut values = vec![];

    for doc in serde_yaml::Deserializer::from_reader(rdr) {
        let value = Value::deserialize(doc)?;
        values.push(value);
    }

    if values.len() == 1 {
        return Ok(Parsed::One(values[0].clone()));
    }

    Ok(Parsed::Many(values))
}

fn introspect_typescript_types(value: Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{s}'"),
        Value::Sequence(s) => {
            let mut buf = String::new();
            buf.push('[');

            let elements: Vec<_> = s.into_iter().map(introspect_typescript_types).collect();

            buf.push_str(&elements.join(","));

            buf.push(']');
            buf
        }
        Value::Mapping(m) => {
            let mut buf = String::new();
            buf.push('{');

            let kvs: Vec<_> = m
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}: {}",
                        &introspect_typescript_types(key),
                        &introspect_typescript_types(value)
                    )
                })
                .collect();

            buf.push_str(&kvs.join(","));

            buf.push('}');
            buf
        }
        Value::Tagged(tv) => introspect_typescript_types(tv.value),
    }
}

fn file_name_to_type_name(fname: &str) -> String {
    fname
        .split(['-', '.'])
        .map(to_first_uppercase)
        .collect::<Vec<_>>()
        .join("")
}

fn to_first_uppercase(n: &str) -> String {
    let mut buf = n.to_owned();
    let fc = buf.get(0..1).unwrap_or_default().to_owned().to_uppercase();
    buf.replace_range(0..1, &fc);
    buf
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use insta::assert_display_snapshot;

    use crate::{file_name_to_type_name, generate_typescript_types};

    #[test]
    fn file_name_to_type_name_conversion() {
        assert_eq!(file_name_to_type_name("test"), "Test".to_string());
        assert_eq!(
            file_name_to_type_name("test.config"),
            "TestConfig".to_string()
        );
        assert_eq!(
            file_name_to_type_name("test-config"),
            "TestConfig".to_string()
        );
        assert_eq!(
            file_name_to_type_name("test-config-tee.prod"),
            "TestConfigTeeProd".to_string()
        );
    }

    #[test]
    fn introspect_typescript_types_gen() {
        let output = generate_typescript_types(Path::new("src/test.yaml")).unwrap();
        assert_display_snapshot!(output);

        let output = generate_typescript_types(Path::new("src/test.multiple.yaml")).unwrap();
        assert_display_snapshot!(output)
    }
}
