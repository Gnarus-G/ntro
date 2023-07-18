use anyhow::{anyhow, Context};
use std::{io::Write, path::Path, process};

use which::which;

pub fn prettify(file: &[u8], file_name: &Path) -> anyhow::Result<Vec<u8>> {
    let mut prettier = prettier(file_name)?;

    let mut prettier_stdin = prettier.stdin.take().ok_or(anyhow!(
        "failed to open stdin to pass file contents to prettier"
    ))?;

    prettier_stdin.write_all(file)?;

    // Finish (close file handle)
    drop(prettier_stdin);

    let output = prettier.wait_with_output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(anyhow!(
            "{}",
            String::from_utf8_lossy(&[output.stdout, output.stderr].concat())
        ))
    }
}

fn prettier(file_name: &Path) -> anyhow::Result<process::Child> {
    let pm = PackageManager::get();

    let package_manager_executable = pm
        .map(PackageManager::into_exe_name)
        .context("no package manager detected, neither pnpm, npm, nor yarn")
        .map(|exe| (exe, vec!["prettier", "--stdin-filepath"]));

    let (exe, args) = which("prettierd")
        .map(|_| ("prettierd", vec![]))
        .or(package_manager_executable.context("failed to find prettierd in the path, and couldn't use a package manager to execute prettier with"))?;

    let child = process::Command::new(exe)
        .args(&args)
        .arg(file_name)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn prettier with command: {} {}",
                exe,
                [args, vec![&file_name.to_string_lossy()]]
                    .concat()
                    .join(" ")
            )
        })?;

    Ok(child)
}

#[derive(Debug)]
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
