use anyhow::Context;
use which::which;

#[derive(Debug)]
pub enum PackageManager {
    Pnpm,
    Yarn,
    Npm,
}

impl PackageManager {
    pub fn from_current_project() -> Option<Self> {
        let Ok(dir) = std::env::current_dir() else {
            return None;
        };

        if dir.join("pnpm-lock.yaml").is_file() && which("pnpm").is_ok() {
            return Some(Self::Pnpm);
        }
        if dir.join("package-lock.json").is_file() && which("npm").is_ok() {
            return Some(Self::Npm);
        }
        if dir.join("yarn.lock").is_file() && which("yarn").is_ok() {
            return Some(Self::Yarn);
        }

        None
    }

    pub fn from_global() -> anyhow::Result<Self> {
        which("pnpm")
            .map(|_| Self::Pnpm)
            .or(which("npm").map(|_| Self::Npm))
            .or(which("yarn").map(|_| Self::Yarn))
            .context("failed to find either of one pnpm, npm, or yarn in the system")
    }

    pub fn into_executor_name(self) -> &'static str {
        match self {
            PackageManager::Pnpm => "pnpx",
            PackageManager::Yarn => "yarn",
            PackageManager::Npm => "npx",
        }
    }
}
