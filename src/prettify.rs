use std::{error::Error, io::Write, path::Path, process};

use which::which;

pub fn prettify(file: &[u8], file_name: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
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
