use std::env;
use std::io::{self, IsTerminal};
use std::time::Duration;

#[cfg(test)]
use std::path::Path;

use crate::cache::GitPromptCache;
use crate::config::{AppConfig, IconSetting};
use crate::core::ExecutionStatus;
use crate::git;
#[cfg(test)]
use crate::git::PromptGitInfo;
use crate::prompt::builder;
use crate::prompt::context::PromptContext;
use crate::render;
use crate::render::ansi;
use crate::render::icons::IconMode;
use crate::render::palette::Palette;
use crate::render::separators::Separator;
use crate::theme::{self, ThemeError};

const GIT_CACHE_TTL: Duration = Duration::from_millis(250);
const GUI_PROMPT_ENV: &str = "TERMINAL_RUST_GUI";
const GUI_PROMPT_OSC: u16 = 777;

pub struct Prompt {
    git_cache: GitPromptCache,
    palette: Palette,
    icon_mode: IconMode,
    separator: Option<Separator>,
    config: AppConfig,
    last_status: Option<ExecutionStatus>,
    last_duration: Option<Duration>,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            git_cache: GitPromptCache::new(GIT_CACHE_TTL),
            palette: Palette::default(),
            icon_mode: IconMode::from_environment(),
            separator: None,
            config: AppConfig::default(),
            last_status: None,
            last_duration: None,
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
            separator: theme.separator,
            config: config.clone(),
            last_status: None,
            last_duration: None,
        })
    }

    pub fn record_execution(&mut self, status: ExecutionStatus, duration: Duration) {
        self.last_status = Some(status);
        self.last_duration = Some(duration);
    }

    pub fn render(&mut self) -> io::Result<String> {
        let current_dir = env::current_dir()?;
        let git = self
            .git_cache
            .get_or_refresh(&current_dir, git::prompt_info);
        let is_terminal = io::stdout().is_terminal();
        let color = ansi::color_enabled(is_terminal);
        let icons = self.icon_mode.enabled(is_terminal);
        let context = PromptContext::collect(
            current_dir.clone(),
            git.clone(),
            self.last_status,
            self.last_duration,
        );
        let layout = builder::build_layout(&context, &self.config);
        let rendered = render_segments(
            &layout.left,
            color,
            icons,
            self.separator.as_ref(),
            &self.palette,
            &self.config,
        );

        if env::var_os(GUI_PROMPT_ENV).is_some() {
            Ok(format!("{}{}", gui_metadata(&layout)?, rendered))
        } else {
            Ok(rendered)
        }
    }

    pub fn print(&mut self) -> io::Result<()> {
        render::stdout(self.render()?)
    }
}

#[cfg(test)]
fn render_for_path(
    path: &Path,
    git: Option<&PromptGitInfo>,
    color: bool,
    icons: bool,
    separator: Option<&Separator>,
    palette: &Palette,
    config: &AppConfig,
) -> String {
    let segments = builder::build(path, git, config);
    render_segments(&segments, color, icons, separator, palette, config)
}

fn render_segments(
    segments: &[crate::prompt::segment::PromptSegment],
    color: bool,
    icons: bool,
    separator: Option<&Separator>,
    palette: &Palette,
    config: &AppConfig,
) -> String {
    render::prompt_segments(
        segments,
        color,
        icons,
        separator,
        &config.prompt.separator,
        config.prompt.newline,
        palette,
    )
}

fn gui_metadata(layout: &builder::PromptLayout) -> io::Result<String> {
    let payload = serde_json::to_string(layout).map_err(io::Error::other)?;
    Ok(format!("\u{1b}]{GUI_PROMPT_OSC};{payload}\u{7}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::segment::SegmentKind;

    #[test]
    fn renders_a_simple_prompt_without_git() {
        assert_eq!(
            render_for_path(
                Path::new("workspace"),
                None,
                false,
                false,
                None,
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
                None,
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
                None,
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
                None,
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
                None,
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
                None,
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

    #[test]
    fn renders_powerline_separator_or_ascii_fallback() {
        let separator = Separator::new("\u{e0b0}", " | ");
        let config = AppConfig::default();

        assert_eq!(
            render_for_path(
                Path::new("project"),
                None,
                false,
                true,
                Some(&separator),
                &Palette::default(),
                &config,
            ),
            "\u{f07b} project\u{e0b0}> "
        );
        assert_eq!(
            render_for_path(
                Path::new("project"),
                None,
                false,
                false,
                Some(&separator),
                &Palette::default(),
                &config,
            ),
            "project | > "
        );
    }
}
