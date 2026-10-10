use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::Duration;

use crate::cache::GitPromptCache;
use crate::config::{AppConfig, IconSetting};
use crate::git::{self, PromptGitInfo};
use crate::prompt::builder;
use crate::prompt::segment::{PromptSegment, SegmentKind};
use crate::render;
use crate::render::ansi;
use crate::render::icons::{self, IconMode};
use crate::render::palette::Palette;
use crate::theme::{self, ThemeError};

const GIT_CACHE_TTL: Duration = Duration::from_millis(250);

pub struct Prompt {
    git_cache: GitPromptCache,
    palette: Palette,
    icon_mode: IconMode,
    config: AppConfig,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            git_cache: GitPromptCache::new(GIT_CACHE_TTL),
            palette: Palette::default(),
            icon_mode: IconMode::from_environment(),
            config: AppConfig::default(),
        }
    }
}

impl Prompt {
    pub fn from_config(config: &AppConfig) -> Result<Self, ThemeError> {
        let theme = theme::load_builtin(&config.theme)?;
        let icon_setting = match config.prompt.icons {
            IconSetting::Auto => theme.icons,
            explicit => explicit,
        };
        let icon_mode = match icon_setting {
            IconSetting::Auto => IconMode::from_environment(),
            IconSetting::Always => IconMode::Always,
            IconSetting::Never => IconMode::Never,
        };

        Ok(Self {
            git_cache: GitPromptCache::new(GIT_CACHE_TTL),
            palette: theme.palette,
            icon_mode,
            config: config.clone(),
        })
    }

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
            &self.config,
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
    config: &AppConfig,
) -> String {
    let segments = builder::build(path, git, config);
    let mut output = String::new();

    for segment in &segments {
        if !output.is_empty() {
            if config.prompt.newline && segment.kind == SegmentKind::Symbol {
                output.push('\n');
            } else {
                output.push_str(&config.prompt.separator);
            }
        }
        output.push_str(&render_segment(segment, color, icons, palette));
    }

    output.push(' ');
    output
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
                &AppConfig::default(),
            ),
            "workspace  > "
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
                &AppConfig::default(),
            ),
            "project with spaces  git:main  > "
        );
        assert_eq!(
            render_for_path(
                Path::new("project"),
                Some(&dirty),
                false,
                false,
                &Palette::default(),
                &AppConfig::default(),
            ),
            "project  git:feature/prompt*  > "
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
                &AppConfig::default(),
            ),
            "\u{1b}[1;36mproject\u{1b}[0m  \u{1b}[32mgit:main\u{1b}[0m  \u{1b}[1;35m>\u{1b}[0m "
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
                &AppConfig::default(),
            ),
            "\u{f07b} project  \u{e0a0} main  > "
        );
    }

    #[test]
    fn builds_typed_segments_without_ansi_codes() {
        let git = PromptGitInfo {
            branch: "main".to_string(),
            dirty: true,
        };

        let segments = builder::build(Path::new("workspace"), Some(&git), &AppConfig::default());

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

    #[test]
    fn follows_custom_order_separator_and_newline() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec!["git".into(), "path".into(), "symbol".into()];
        config.prompt.separator = " | ".into();
        config.prompt.newline = true;
        let git = PromptGitInfo {
            branch: "main".into(),
            dirty: false,
        };

        assert_eq!(
            render_for_path(
                Path::new("project"),
                Some(&git),
                false,
                false,
                &Palette::default(),
                &config,
            ),
            "git:main | project\n> "
        );
    }

    #[test]
    fn applies_minimal_theme_defaults_and_explicit_icon_override() {
        let mut config = AppConfig {
            theme: "minimal".into(),
            ..AppConfig::default()
        };
        let prompt = Prompt::from_config(&config).expect("minimal theme should load");
        assert_eq!(prompt.icon_mode, IconMode::Never);
        assert_eq!(
            prompt.palette.color("primary"),
            Some(ansi::AnsiColor::White)
        );

        config.prompt.icons = IconSetting::Always;
        let prompt = Prompt::from_config(&config).expect("minimal theme should load");
        assert_eq!(prompt.icon_mode, IconMode::Always);
    }
}
