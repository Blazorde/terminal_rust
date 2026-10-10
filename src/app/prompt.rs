use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::Duration;

use crate::cache::GitPromptCache;
use crate::git::{self, PromptGitInfo};
use crate::prompt::segment::{PromptSegment, SegmentKind};
use crate::render;
use crate::render::ansi;

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
        let color = ansi::color_enabled(io::stdout().is_terminal());
        Ok(render_for_path(&current_dir, git.as_ref(), color))
    }

    pub fn print(&mut self) -> io::Result<()> {
        render::stdout(self.render()?)
    }
}

fn render_for_path(path: &Path, git: Option<&PromptGitInfo>, color: bool) -> String {
    let segments = segments_for_path(path, git);
    let path = render_segment(&segments[0], color);
    let symbol = render_segment(
        segments.last().expect("the symbol segment always exists"),
        color,
    );

    match segments
        .get(1)
        .filter(|segment| segment.kind == SegmentKind::Git)
    {
        Some(git) => format!("{path}  {} {symbol} ", render_segment(git, color)),
        None => format!("{path}{symbol} "),
    }
}

fn render_segment(segment: &PromptSegment, color: bool) -> String {
    let value = segment.value.as_deref().unwrap_or_default();
    ansi::paint(
        value,
        ansi::style_for_key(segment.style_key.as_str()),
        color,
    )
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
        assert_eq!(
            render_for_path(Path::new("workspace"), None, false),
            "workspace> "
        );
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
            render_for_path(Path::new("project with spaces"), Some(&clean), false),
            "project with spaces  main > "
        );
        assert_eq!(
            render_for_path(Path::new("project"), Some(&dirty), false),
            "project  feature/prompt* > "
        );
    }

    #[test]
    fn renders_each_colored_segment_with_its_own_reset() {
        let git = PromptGitInfo {
            branch: "main".to_string(),
            dirty: false,
        };

        assert_eq!(
            render_for_path(Path::new("project"), Some(&git), true),
            "\u{1b}[1;36mproject\u{1b}[0m  \u{1b}[32mmain\u{1b}[0m \u{1b}[1;35m>\u{1b}[0m "
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
