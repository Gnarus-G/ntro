use anyhow::Context;
use std::{path::Path, process};

use which::which;

use super::pm::PackageManager;

pub fn prettier(file_extension: &str) -> anyhow::Result<process::Child> {
    let pm = PackageManager::from_current_project();

    // pretend file name, doesn't need to exist,
    // but its extension describe how prettier
    // should work
    let file_name = &Path::new("temp").with_extension(file_extension);

    let package_manager_executable = pm
        .map(PackageManager::into_executor_name)
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
