use std::{
    error::Error,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use clap::{CommandFactory, Parser, Subcommand};
use ntro::{env, yaml};
use which::which;

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

    match prettify(content.as_bytes(), &output_path) {
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

fn prettify(file: &[u8], file_name: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut prettier = prettier(file_name)?;

    let mut prettier_stdin = prettier.stdin.take().ok_or("failed to open stdin")?;

    prettier_stdin.write_all(file)?;

    // Finish (close file handle)
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

fn prettier(file_name: &Path) -> Result<process::Child, Box<dyn Error>> {
    let pm = PackageManager::get();

    let package_manager_executable = pm
        .map(PackageManager::into_exe_name)
        .map(|exe| (exe, vec!["prettier", "--stdin-filepath"]))
        .ok_or::<String>("no executable found with which to run prettier".into());

    let (exe, args) = which("prettierd")
        .map(|_| ("prettierd", vec![]))
        .or(package_manager_executable)?;

    let child = process::Command::new(exe)
        .args(args)
        .arg(file_name)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()?;

    Ok(child)
}

enum PackageManager {
    Pnpm,
    Yarn,
    Npm,
}

impl PackageManager {
    fn get() -> Option<Self> {
        let Ok(dir) = std::env::current_dir() else {
            return None;
        };

        if dir.join("pnpm-lock.yaml").is_file() {
            return Some(Self::Pnpm);
        }
        if dir.join("yarn.lock").is_file() {
            return Some(Self::Yarn);
        }
        if dir.join("package-lock.json").is_file() {
            return Some(Self::Npm);
        }

        None
    }

    fn into_exe_name(self) -> &'static str {
        match self {
            PackageManager::Pnpm => "pnpx",
            PackageManager::Yarn => "yarn",
            PackageManager::Npm => "npx",
        }
    }
}
