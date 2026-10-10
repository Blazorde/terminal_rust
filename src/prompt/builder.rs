use std::path::Path;

use crate::config::AppConfig;
use crate::git::PromptGitInfo;

use super::segment::{PromptSegment, SegmentKind};

pub fn build(path: &Path, git: Option<&PromptGitInfo>, config: &AppConfig) -> Vec<PromptSegment> {
    config
        .prompt
        .segments
        .iter()
        .filter_map(|name| match name.as_str() {
            "path" => Some(path_segment(path, config)),
            "git" => git_segment(git, config),
            "symbol" => Some(PromptSegment::new(
                SegmentKind::Symbol,
                &config.segments.symbol.value,
                "secondary",
            )),
            _ => None,
        })
        .collect()
}

fn path_segment(path: &Path, config: &AppConfig) -> PromptSegment {
    let value = if config.segments.path.style == "folder" {
        path.file_name()
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string())
    } else {
        path.display().to_string()
    };

    PromptSegment::new(SegmentKind::Path, value, "primary").with_icon("folder", "")
}

fn git_segment(git: Option<&PromptGitInfo>, config: &AppConfig) -> Option<PromptSegment> {
    let dirty = git.is_some_and(|info| info.dirty);
    let segment = PromptSegment::optional(
        SegmentKind::Git,
        git.map(|info| {
            let dirty_marker = if config.segments.git.show_status && info.dirty {
                "*"
            } else {
                ""
            };
            format!("{}{dirty_marker}", info.branch)
        }),
        if dirty { "warning" } else { "success" },
    )
    .with_label("Git")
    .with_icon("git_branch", "git:");

    segment.is_visible().then_some(segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_configured_order_and_omits_unselected_segments() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec!["symbol".into(), "path".into()];

        let segments = build(Path::new("workspace"), None, &config);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].kind, SegmentKind::Symbol);
        assert_eq!(segments[1].kind, SegmentKind::Path);
    }

    #[test]
    fn applies_folder_and_git_status_options() {
        let mut config = AppConfig::default();
        config.segments.path.style = "folder".into();
        config.segments.git.show_status = false;
        let git = PromptGitInfo {
            branch: "main".into(),
            dirty: true,
        };

        let segments = build(
            Path::new("parent").join("project").as_path(),
            Some(&git),
            &config,
        );

        assert_eq!(segments[0].value.as_deref(), Some("project"));
        assert_eq!(segments[1].value.as_deref(), Some("main"));
    }
}
