use std::env;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::cache::GitPromptCache;
use crate::git::{self, PromptGitInfo};
use crate::prompt::segment::{PromptSegment, SegmentKind};
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
    let segments = segments_for_path(path, git);
    let path = segments[0]
        .value
        .as_deref()
        .expect("the path segment always has a value");
    let symbol = segments
        .last()
        .and_then(|segment| segment.value.as_deref())
        .expect("the symbol segment always has a value");

    match segments
        .get(1)
        .filter(|segment| segment.kind == SegmentKind::Git)
    {
        Some(git) => format!(
            "{path}  {} {symbol} ",
            git.value
                .as_deref()
                .expect("a visible Git segment has a value")
        ),
        None => format!("{path}{symbol} "),
    }
}

fn segments_for_path(path: &Path, git: Option<&PromptGitInfo>) -> Vec<PromptSegment> {
    let mut segments = vec![PromptSegment::new(
        SegmentKind::Path,
        path.display().to_string(),
        "path",
    )];

    let git_segment = PromptSegment::optional(
        SegmentKind::Git,
        git.map(|info| {
            let dirty_marker = if info.dirty { "*" } else { "" };
            format!("{}{dirty_marker}", info.branch)
        }),
        if git.is_some_and(|info| info.dirty) {
            "git_dirty"
        } else {
            "git_clean"
        },
    )
    .with_label("Git")
    .with_icon("git_branch", "git:");

    if git_segment.is_visible() {
        segments.push(git_segment);
    }

    segments.push(PromptSegment::new(SegmentKind::Symbol, ">", "symbol"));
    segments
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

    #[test]
    fn builds_typed_segments_without_ansi_codes() {
        let git = PromptGitInfo {
            branch: "main".to_string(),
            dirty: true,
        };

        let segments = segments_for_path(Path::new("workspace"), Some(&git));

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].kind, SegmentKind::Path);
        assert_eq!(segments[1].kind, SegmentKind::Git);
        assert_eq!(segments[1].style_key, "git_dirty");
        assert_eq!(segments[2].kind, SegmentKind::Symbol);
        assert!(segments.iter().all(|segment| {
            segment
                .value
                .as_ref()
                .is_none_or(|value| !value.contains("\u{1b}["))
        }));
    }
}
