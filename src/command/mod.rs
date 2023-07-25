mod pm;
mod prettier;

use std::{
    fs::File,
    io::{BufReader, Write},
    path::Path,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::command::pm::PackageManager;

use self::prettier::prettier;

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
        )
        .context("exited prettier execution with a fail status"))
    }
}

pub fn npm_install() -> Result<()> {
    let package_info: Value = File::open("./package.json")
        .context("couldn't open package.json")
        .map(BufReader::new)
        .and_then(|reader| serde_json::from_reader(reader).context("failed to parse package.json"))
        .context("failed to read package.json")?;

    if package_info
        .get("dependencies")
        .and_then(|deps| deps.get("zod"))
        .is_some()
    {
        return Ok(());
    }

    eprintln!("Installing zod");
    let out = PackageManager::from_current_project()
        .ok_or(anyhow!("couldn't get package manager from current project"))
        .or(PackageManager::from_global())
        .map(|pm| match pm {
            PackageManager::Pnpm => ("pnpm", "add"),
            PackageManager::Yarn => ("yarn", "add"),
            PackageManager::Npm => ("npm", "i"),
        })
        .and_then(|(exe, arg)| {
            Command::new(exe)
                .arg(arg)
                .arg("zod")
                .output()
                .with_context(|| {
                    format!("failed to execute installation with package manager: {exe}")
                })
        })?;

    if !out.status.success() {
        return Err(anyhow!(
            "installation failed with exit code {:?}",
            out.status.code()
        ));
    }

    Ok(())
}
