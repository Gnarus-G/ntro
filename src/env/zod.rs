use chumsky::prelude::*;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
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

    let output = format!(
        r#"
import z from "zod";

const clientEnvSchemas = {{
{}
}}

const serverEnvSchemas = {{
{}
}}

{}

const processEnv = {{
{}
}}

class BadEnvError extends Error {{
    constructor(public message: string, public cause: unknown){{
        super(message)
        if (cause instanceof Error) {{
          this.message = [message, cause].join("\n ↳ ");
        }}
    }}
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
        JS_PROXY_DEFS,
        vars.iter()
            .map(|var| format!("   {}: process.env.{},", var.key, var.key))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    Ok(output)
}

const JS_PROXY_DEFS: &str = r#"
export const clientEnv: z.infer<z.ZodObject<typeof clientEnvSchemas>> =
  new Proxy({} as any, {
    get(_, prop: string) {
      try {
        if (prop in clientEnvSchemas) {
          return clientEnvSchemas[prop as keyof typeof clientEnvSchemas].parse(
            processEnv[prop as keyof typeof processEnv],
            { path: [prop] }
          );
        }
        throw new Error(
          `${prop} is not defined for client side environment variables.`
        );
      } catch (e) {
        throw new BadEnvError(`failed to read ${prop} from proccess.env`, e);
      }
    },
  });

export const serverEnv: z.infer<z.ZodObject<typeof serverEnvSchemas>> =
  new Proxy({} as any, {
    get(_, prop: string) {
      try {
        if (prop in serverEnvSchemas) {
          return serverEnvSchemas[prop as keyof typeof serverEnvSchemas].parse(
            processEnv[prop as keyof typeof processEnv],
            { path: [prop] }
          );
        }
        throw new Error(
          `${prop} is not defined for server side environment variables.`
        );
      } catch (e) {
        throw new BadEnvError(`failed to read ${prop} from proccess.env`, e);
      }
    },
  });
"#;

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
