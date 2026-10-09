use std::env;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::cache::GitPromptCache;
use crate::git::{self, PromptGitInfo};
use crate::render;

const GIT_CACHE_TTL: Duration = Duration::from_millis(250);

pub struct Prompt {
    git_cache: GitPromptCache,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            git_cache: GitPromptCache::new(GIT_CACHE_TTL),
        }
    }
}

impl Prompt {
    pub fn render(&mut self) -> io::Result<String> {
        let current_dir = env::current_dir()?;
        let git = self
            .git_cache
            .get_or_refresh(&current_dir, git::prompt_info);
        Ok(render_for_path(&current_dir, git.as_ref()))
    }

    pub fn print(&mut self) -> io::Result<()> {
        render::stdout(self.render()?)
    }
}

fn render_for_path(path: &Path, git: Option<&PromptGitInfo>) -> String {
    match git {
        Some(info) => {
            let dirty_marker = if info.dirty { "*" } else { "" };
            format!("{}  {}{} > ", path.display(), info.branch, dirty_marker)
        }
        None => format!("{}> ", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_simple_prompt_without_git() {
        assert_eq!(render_for_path(Path::new("workspace"), None), "workspace> ");
    }

    #[test]
    fn renders_clean_and_dirty_git_segments() {
        let clean = PromptGitInfo {
            branch: "main".to_string(),
            dirty: false,
        };
        let dirty = PromptGitInfo {
            branch: "feature/prompt".to_string(),
            dirty: true,
        };

        assert_eq!(
            render_for_path(Path::new("project with spaces"), Some(&clean)),
            "project with spaces  main > "
        );
        assert_eq!(
            render_for_path(Path::new("project"), Some(&dirty)),
            "project  feature/prompt* > "
        );
    }
}
