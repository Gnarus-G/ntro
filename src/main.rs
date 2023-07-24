use std::{fs::File, io::Write, path::PathBuf};

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use ntro::{dotenv, yaml};

mod prettify;
use prettify::prettify;

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
    /// Generate typescript types from .env files.
    Dotenv {
        /// Path to a yaml file.
        source_files: Vec<PathBuf>,

        /// Set the output directory, to where to save the env.d.ts file.
        #[arg(short)]
        output_dir: Option<PathBuf>,

        /// Generate a typescript module implementing a zod schema for env variables
        #[arg(short, long)]
        zod: bool,
    },
    /// Generate a completions file for a specified shell
    Completion {
        // The shell for which to generate completions
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Yaml {
            source_file,
            output_dir,
        } => {
            let content = yaml::generate_typescript_types(&source_file)?;

            let output_path = output_dir
                .unwrap_or_default()
                .join(source_file.with_extension("d.ts").file_name().expect(
                "the file path given should have had a filename for its yaml content to be parsed",
            ));

            write_output(output_path, content)?;
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ntro", &mut std::io::stdout());
        }
        Command::Dotenv {
            source_files,
            output_dir,
            zod,
        } => {
            if zod {
                let content = dotenv::zod::generate_zod_schema(&source_files)?;
                let output_path = output_dir.clone().unwrap_or_default().join("env.parsed.ts");

                write_output(output_path, content)?;

                if let Err(e) = dotenv::zod::npm_install() {
                    eprintln!("{e}");
                }
            }

            let content = dotenv::generate_typescript_types(&source_files)?;
            let output_path = output_dir.unwrap_or_default().join("env.d.ts");

            write_output(output_path, content)?;
        }
    };

    Ok(())
}

fn write_output(output_path: PathBuf, content: String) -> Result<()> {
    let content = prettify(content.as_bytes(), &output_path)?;

    let mut ofile = File::create(&output_path)?;
    ofile.write_all(&content)?;

    Ok(())
}
