mod yaml;

use std::{error::Error, fs::File, io::Write, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};

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
            let content = yaml::generate_typescript_types(&source_file)?;

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
