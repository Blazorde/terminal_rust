use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::Duration;

use crate::cache::GitPromptCache;
use crate::git::{self, PromptGitInfo};
use crate::prompt::segment::{PromptSegment, SegmentKind};
use crate::render;
use crate::render::ansi;
use crate::render::icons::{self, IconMode};
use crate::render::palette::Palette;

const GIT_CACHE_TTL: Duration = Duration::from_millis(250);

pub struct Prompt {
    git_cache: GitPromptCache,
    palette: Palette,
    icon_mode: IconMode,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            git_cache: GitPromptCache::new(GIT_CACHE_TTL),
            palette: Palette::default(),
            icon_mode: IconMode::from_environment(),
        }
    }
}

impl Prompt {
    pub fn render(&mut self) -> io::Result<String> {
        let current_dir = env::current_dir()?;
        let git = self
            .git_cache
            .get_or_refresh(&current_dir, git::prompt_info);
        let is_terminal = io::stdout().is_terminal();
        let color = ansi::color_enabled(is_terminal);
        let icons = self.icon_mode.enabled(is_terminal);
        Ok(render_for_path(
            &current_dir,
            git.as_ref(),
            color,
            icons,
            &self.palette,
        ))
    }

    pub fn print(&mut self) -> io::Result<()> {
        render::stdout(self.render()?)
    }
}

fn render_for_path(
    path: &Path,
    git: Option<&PromptGitInfo>,
    color: bool,
    icons: bool,
    palette: &Palette,
) -> String {
    let segments = segments_for_path(path, git);
    let path = render_segment(&segments[0], color, icons, palette);
    let symbol = render_segment(
        segments.last().expect("the symbol segment always exists"),
        color,
        icons,
        palette,
    );

    match segments
        .get(1)
        .filter(|segment| segment.kind == SegmentKind::Git)
    {
        Some(git) => format!(
            "{path}  {} {symbol} ",
            render_segment(git, color, icons, palette)
        ),
        None => format!("{path}{symbol} "),
    }
}

fn render_segment(
    segment: &PromptSegment,
    color: bool,
    icons_enabled: bool,
    palette: &Palette,
) -> String {
    let value = segment.value.as_deref().unwrap_or_default();
    let decorated = icons::decorate(value, segment.icon.as_ref(), icons_enabled);
    ansi::paint(
        &decorated,
        ansi::style_for_key(segment.style_key.as_str(), palette),
        color,
    )
}

fn segments_for_path(path: &Path, git: Option<&PromptGitInfo>) -> Vec<PromptSegment> {
    let mut segments = vec![
        PromptSegment::new(SegmentKind::Path, path.display().to_string(), "primary")
            .with_icon("folder", ""),
    ];

    let git_segment = PromptSegment::optional(
        SegmentKind::Git,
        git.map(|info| {
            let dirty_marker = if info.dirty { "*" } else { "" };
            format!("{}{dirty_marker}", info.branch)
        }),
        if git.is_some_and(|info| info.dirty) {
            "warning"
        } else {
            "success"
        },
    )
    .with_label("Git")
    .with_icon("git_branch", "git:");

    if git_segment.is_visible() {
        segments.push(git_segment);
    }

    segments.push(PromptSegment::new(SegmentKind::Symbol, ">", "secondary"));
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_simple_prompt_without_git() {
        assert_eq!(
            render_for_path(
                Path::new("workspace"),
                None,
                false,
                false,
                &Palette::default(),
            ),
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
            render_for_path(
                Path::new("project with spaces"),
                Some(&clean),
                false,
                false,
                &Palette::default(),
            ),
            "project with spaces  git:main > "
        );
        assert_eq!(
            render_for_path(
                Path::new("project"),
                Some(&dirty),
                false,
                false,
                &Palette::default(),
            ),
            "project  git:feature/prompt* > "
        );
    }

    #[test]
    fn renders_each_colored_segment_with_its_own_reset() {
        let git = PromptGitInfo {
            branch: "main".to_string(),
            dirty: false,
        };

        assert_eq!(
            render_for_path(
                Path::new("project"),
                Some(&git),
                true,
                false,
                &Palette::default(),
            ),
            "\u{1b}[1;36mproject\u{1b}[0m  \u{1b}[32mgit:main\u{1b}[0m \u{1b}[1;35m>\u{1b}[0m "
        );
    }

    #[test]
    fn renders_nerd_font_icons_without_losing_segment_values() {
        let git = PromptGitInfo {
            branch: "main".to_string(),
            dirty: false,
        };

        assert_eq!(
            render_for_path(
                Path::new("project"),
                Some(&git),
                false,
                true,
                &Palette::default(),
            ),
            "\u{f07b} project  \u{e0a0} main > "
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
        assert_eq!(segments[1].style_key, "warning");
        assert_eq!(segments[2].kind, SegmentKind::Symbol);
        assert!(segments.iter().all(|segment| {
            segment
                .value
                .as_ref()
                .is_none_or(|value| !value.contains("\u{1b}["))
        }));
    }
}
