use std::{fs::File, io::Write, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::{CommandFactory, Parser, Subcommand};
use ntro::{dotenv, yaml};
use simple_logger::SimpleLogger;

mod command;
mod watch;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Disable logs.
    #[arg(short, long, global = true)]
    quiet: bool,

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
        /// Path(s) to some .env files.
        source_files: Vec<PathBuf>,

        /// Set the output directory, to where to save the env.d.ts file.
        #[arg(short)]
        output_dir: Option<PathBuf>,

        /// Wath for changes in the source files and rerun.
        #[arg(short, long)]
        watch: bool,

        /// Generate a typescript module implementing a zod schema for env variables
        #[arg(short, long)]
        zod: bool,

        /// Update the project's tsconfig.json to include a path alias to the env.parsed.ts module that
        /// holds the zod schemas.
        #[arg(short = 'p', long, requires("zod"))]
        set_ts_config_path_alias: bool,

        /// For node project; will install and use dotenv to pull in the .env files into
        /// process.env
        #[arg(long, requires("zod"))]
        node: bool,
    },
    /// Generate a completions file for a specified shell
    Completion {
        // The shell for which to generate completions
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.quiet {
        SimpleLogger::new().init().unwrap();
    }

    run(cli)?;

    Ok(())
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Yaml {
            source_file,
            output_dir,
        } => {
            log::info!(
                "starting to generate a declaration file for {:?}",
                source_file
            );
            let content = yaml::generate_typescript_types(&source_file)?;

            let output_path = output_dir
                .unwrap_or_default()
                .join(source_file.with_extension("d.ts").file_name().expect(
                "the file path given should have had a filename for its yaml content to be parsed",
            ));

            write_output(&output_path, content)?;

            log::info!(
                "successfully generated a declaration file for {:?} to {:?}",
                source_file,
                output_path
            );
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ntro", &mut std::io::stdout());
        }
        Command::Dotenv {
            source_files,
            output_dir,
            zod,
            set_ts_config_path_alias,
            watch,
            node,
        } => {
            let work = || -> anyhow::Result<()> {
                if zod {
                    log::info!("starting to generate zod schema for {:?}", source_files);
                    let content = dotenv::zod::generate_zod_schema(&source_files, node)?;
                    let output_path = output_dir.clone().unwrap_or_default().join("env.parsed.ts");

                    write_output(&output_path, content)?;

                    if node {
                        if let Err(e) = command::npm_install("dotenv") {
                            log::error!("{e:#}");
                        }
                    }

                    if let Err(e) = command::npm_install("zod") {
                        log::error!("{e:#}");
                    }

                    if set_ts_config_path_alias {
                        if let Err(e) = dotenv::zod::add_tsconfig_path(&output_path) {
                            log::error!("{e:#}");
                        }
                    }

                    log::info!(
                        "successfully generated a declaration file for {:?} to {:?}",
                        source_files,
                        output_path
                    );
                }

                log::info!(
                    "starting to generate typescript declaration files for {:?}",
                    source_files
                );
                let content = dotenv::generate_typescript_types(&source_files)?;
                let output_path = output_dir.clone().unwrap_or_default().join("env.d.ts");

                write_output(&output_path, content)?;

                log::info!(
                    "successfully generated a declaration file for {:?} to {:?}",
                    source_files,
                    output_path
                );

                Ok(())
            };

            if watch {
                let work_logging_errors = || {
                    if let Err(e) = work() {
                        log::error!("{e:#}");
                    }
                };

                work_logging_errors();

                watch::watch(&source_files, work_logging_errors)?;
            } else {
                work()?;
            }
        }
    };

    Ok(())
}

fn write_output(output_path: &PathBuf, content: String) -> Result<()> {
    let content = command::prettify(
        content.as_bytes(),
        output_path
            .extension()
            .ok_or(anyhow!("output_path given doesn't have an extension"))?
            .to_string_lossy(),
    )?;

    let mut ofile = File::create(output_path)?;
    ofile.write_all(&content)?;

    Ok(())
}
