use std::{
    error::Error,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use clap::{CommandFactory, Parser, Subcommand};
use ntro::{env, yaml};

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
    Env {
        /// Path to a yaml file.
        source_files: Vec<PathBuf>,

        /// Set the output directory, to where to save the env.d.ts file.
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

    let (output_path, content) = match cli.command {
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

            (output_path, content)
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ntro", &mut std::io::stdout());
            return Ok(());
        }
        Command::Env {
            source_files,
            output_dir,
        } => {
            let content = env::generate_typescript_types(&source_files)?;

            let output_path = output_dir.unwrap_or_default().join("env.d.ts");

            (output_path, content)
        }
    };

    match prettier(content.as_bytes(), &output_path) {
        Ok(content) => {
            let mut ofile = File::create(&output_path)?;
            ofile.write_all(&content)?;
        }
        Err(e) => {
            eprint!("couldn't prettify output: {e}");
        }
    }

    Ok(())
}

fn prettier(file: &[u8], file_name: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut prettier = process::Command::new("prettierd")
        .arg(file_name)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()?;

    let mut prettier_stdin = prettier.stdin.take().ok_or("failed to open stdin")?;

    prettier_stdin.write_all(file)?;

    drop(prettier_stdin);

    let output = prettier.wait_with_output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "{}",
            String::from_utf8_lossy(&[output.stdout, output.stderr].concat())
        )
        .into())
    }
}
