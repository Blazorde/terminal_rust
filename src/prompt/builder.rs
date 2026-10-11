use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::config::AppConfig;
use crate::core::ExecutionStatus;
use crate::git::PromptGitInfo;

use super::context::PromptContext;
use super::segment::{PromptSegment, SegmentKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PromptLayout {
    pub left: Vec<PromptSegment>,
    pub right: Vec<PromptSegment>,
}

#[cfg(test)]
pub fn build(path: &Path, git: Option<&PromptGitInfo>, config: &AppConfig) -> Vec<PromptSegment> {
    let context = PromptContext {
        user: None,
        host: None,
        path: path.to_path_buf(),
        git: git.cloned(),
        status: None,
        runtime: None,
        duration: None,
        time: String::new(),
    };
    build_layout(&context, config).left
}

pub fn build_layout(context: &PromptContext, config: &AppConfig) -> PromptLayout {
    PromptLayout {
        left: build_position(&config.prompt.segments, context, config),
        right: build_position(&config.prompt.right_segments, context, config),
    }
}

fn build_position(
    names: &[String],
    context: &PromptContext,
    config: &AppConfig,
) -> Vec<PromptSegment> {
    names
        .iter()
        .filter_map(|name| segment(name, context, config))
        .collect()
}

fn segment(name: &str, context: &PromptContext, config: &AppConfig) -> Option<PromptSegment> {
    match name {
        "user" => optional_text(
            SegmentKind::User,
            context.user.as_deref(),
            "muted",
            "Utilisateur",
            "user",
            "user:",
        ),
        "host" => optional_text(
            SegmentKind::Host,
            context.host.as_deref(),
            "muted",
            "Hôte",
            "host",
            "host:",
        ),
        "path" => Some(path_segment(&context.path, config)),
        "git" => git_segment(context.git.as_ref(), config),
        "status" => status_segment(context.status),
        "runtime" => optional_text(
            SegmentKind::Runtime,
            context.runtime.as_deref(),
            "secondary",
            "Runtime",
            "runtime",
            "runtime:",
        ),
        "duration" => context.duration.map(duration_segment),
        "time" => optional_text(
            SegmentKind::Time,
            Some(&context.time),
            "muted",
            "Heure",
            "time",
            "time:",
        ),
        "symbol" => Some(PromptSegment::new(
            SegmentKind::Symbol,
            &config.segments.symbol.value,
            "secondary",
        )),
        _ => None,
    }
}

fn optional_text(
    kind: SegmentKind,
    value: Option<&str>,
    style: &str,
    label: &str,
    icon: &str,
    fallback: &str,
) -> Option<PromptSegment> {
    let segment = PromptSegment::optional(kind, value, style)
        .with_label(label)
        .with_icon(icon, fallback);
    segment.is_visible().then_some(segment)
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

    PromptSegment::new(SegmentKind::Path, value, "primary")
        .with_label("Chemin")
        .with_icon("folder", "")
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

fn status_segment(status: Option<ExecutionStatus>) -> Option<PromptSegment> {
    status.map(|status| {
        let (value, style) = match status {
            ExecutionStatus::Continue => ("0", "success"),
            ExecutionStatus::Exit => ("exit", "muted"),
            ExecutionStatus::Failed => ("1", "error"),
        };
        PromptSegment::new(SegmentKind::Status, value, style)
            .with_label("Statut")
            .with_icon("status", "status:")
    })
}

fn duration_segment(duration: Duration) -> PromptSegment {
    let value = if duration.as_millis() < 1 {
        "<1 ms".to_string()
    } else if duration.as_secs_f64() < 1.0 {
        format!("{} ms", duration.as_millis())
    } else {
        format!("{:.1} s", duration.as_secs_f64())
    };
    PromptSegment::new(SegmentKind::Duration, value, "muted")
        .with_label("Durée")
        .with_icon("duration", "duration:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn context() -> PromptContext {
        PromptContext {
            user: Some("ada".into()),
            host: Some("atelier".into()),
            path: PathBuf::from("parent").join("project"),
            git: Some(PromptGitInfo {
                branch: "main".into(),
                dirty: true,
            }),
            status: Some(ExecutionStatus::Failed),
            runtime: Some("Rust".into()),
            duration: Some(Duration::from_millis(42)),
            time: "09:41".into(),
        }
    }

    #[test]
    fn follows_configured_order_position_and_visibility() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec!["host".into(), "path".into()];
        config.prompt.right_segments = vec!["time".into(), "status".into()];

        let layout = build_layout(&context(), &config);

        assert_eq!(
            layout
                .left
                .iter()
                .map(|segment| segment.kind)
                .collect::<Vec<_>>(),
            [SegmentKind::Host, SegmentKind::Path]
        );
        assert_eq!(
            layout
                .right
                .iter()
                .map(|segment| segment.kind)
                .collect::<Vec<_>>(),
            [SegmentKind::Time, SegmentKind::Status]
        );
    }

    #[test]
    fn exposes_every_gui_segment_as_typed_rust_data() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec![
            "user".into(),
            "host".into(),
            "path".into(),
            "git".into(),
            "symbol".into(),
        ];
        let layout = build_layout(&context(), &config);
        let kinds = layout
            .left
            .iter()
            .chain(&layout.right)
            .map(|segment| segment.kind)
            .collect::<Vec<_>>();

        for expected in [
            SegmentKind::User,
            SegmentKind::Host,
            SegmentKind::Path,
            SegmentKind::Git,
            SegmentKind::Status,
            SegmentKind::Runtime,
            SegmentKind::Duration,
            SegmentKind::Time,
            SegmentKind::Symbol,
        ] {
            assert!(kinds.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn unavailable_git_and_runtime_segments_are_omitted() {
        let mut context = context();
        context.git = None;
        context.runtime = None;

        let layout = build_layout(&context, &AppConfig::default());

        assert!(
            layout
                .left
                .iter()
                .chain(&layout.right)
                .all(|segment| !matches!(segment.kind, SegmentKind::Git | SegmentKind::Runtime))
        );
    }

    #[test]
    fn applies_folder_and_git_status_options() {
        let mut config = AppConfig::default();
        config.segments.path.style = "folder".into();
        config.segments.git.show_status = false;

        let layout = build_layout(&context(), &config);
        let path = layout
            .left
            .iter()
            .find(|segment| segment.kind == SegmentKind::Path)
            .expect("path segment should be present");
        let git = layout
            .left
            .iter()
            .find(|segment| segment.kind == SegmentKind::Git)
            .expect("git segment should be present");

        assert_eq!(path.value.as_deref(), Some("project"));
        assert_eq!(git.value.as_deref(), Some("main"));
    }
}
