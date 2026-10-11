use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use jiff::Zoned;

use crate::core::ExecutionStatus;
use crate::git::PromptGitInfo;

#[derive(Debug, Clone)]
pub struct PromptContext {
    pub user: Option<String>,
    pub host: Option<String>,
    pub path: PathBuf,
    pub git: Option<PromptGitInfo>,
    pub status: Option<ExecutionStatus>,
    pub runtime: Option<String>,
    pub duration: Option<Duration>,
    pub time: String,
}

impl PromptContext {
    pub fn collect(
        path: PathBuf,
        git: Option<PromptGitInfo>,
        status: Option<ExecutionStatus>,
        duration: Option<Duration>,
    ) -> Self {
        Self {
            user: first_non_empty_env(&["USER", "USERNAME"]),
            host: sysinfo::System::host_name().filter(|value| !value.trim().is_empty()),
            runtime: detect_runtime(&path),
            path,
            git,
            status,
            duration,
            time: Zoned::now().strftime("%H:%M").to_string(),
        }
    }
}

fn first_non_empty_env(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}

fn detect_runtime(path: &Path) -> Option<String> {
    [
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js"),
        ("pyproject.toml", "Python"),
        ("requirements.txt", "Python"),
        ("go.mod", "Go"),
    ]
    .into_iter()
    .find_map(|(marker, runtime)| path.join(marker).is_file().then(|| runtime.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;
    use std::fs;

    #[test]
    fn runtime_detection_is_local_and_optional() {
        let directory = temp_path("prompt-context", "runtime");
        fs::create_dir(&directory).expect("runtime fixture should be created");
        assert_eq!(detect_runtime(&directory), None);

        fs::write(directory.join("Cargo.toml"), "[package]\nname='fixture'\n")
            .expect("runtime marker should be written");
        assert_eq!(detect_runtime(&directory).as_deref(), Some("Rust"));

        fs::remove_dir_all(directory).expect("runtime fixture should be removed");
    }
}
