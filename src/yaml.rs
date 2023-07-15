use std::{error::Error, fs::File, io::BufReader, path::Path};

use serde::Deserialize;
use serde_yaml::Value;

pub fn generate_typescript_types(file: &Path) -> Result<String, Box<dyn Error>> {
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
        Parsed::Many(documents) => {
            let type_strings = documents
                .into_iter()
                .map(introspect_typescript_types)
                .collect::<Vec<_>>();

            let number_of_types = type_strings.len();

            Ok(format!(
                "declare namespace {} {{ {:#};\n export type All = [{:#}] }}",
                file_name_to_type_name(
                    file.file_stem()
                        .expect("couldn't parse a filename from input")
                        .to_str()
                        .expect("path given should be in utf-8")
                ),
                type_strings
                    .into_iter()
                    .enumerate()
                    .map(|(idx, text)| format!("export type Document{idx} = {text}"))
                    .collect::<Vec<_>>()
                    .join(";\n"),
                (0..number_of_types)
                    .map(|idx| format!("Document{idx}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        }
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

    use super::{file_name_to_type_name, generate_typescript_types};

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
