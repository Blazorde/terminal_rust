use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    User,
    Host,
    Path,
    Git,
    Status,
    Runtime,
    Duration,
    Time,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentIcon {
    pub name: String,
    pub fallback: String,
}

impl SegmentIcon {
    pub fn new(name: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fallback: fallback.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSegment {
    pub kind: SegmentKind,
    pub label: Option<String>,
    pub value: Option<String>,
    pub style_key: String,
    pub icon: Option<SegmentIcon>,
}

impl PromptSegment {
    pub fn new(kind: SegmentKind, value: impl Into<String>, style_key: impl Into<String>) -> Self {
        Self {
            kind,
            label: None,
            value: Some(value.into()),
            style_key: style_key.into(),
            icon: None,
        }
    }

    pub fn optional(
        kind: SegmentKind,
        value: Option<impl Into<String>>,
        style_key: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            label: None,
            value: value.map(Into::into),
            style_key: style_key.into(),
            icon: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_icon(mut self, name: impl Into<String>, fallback: impl Into<String>) -> Self {
        self.icon = Some(SegmentIcon::new(name, fallback));
        self
    }

    pub fn is_visible(&self) -> bool {
        self.value.as_ref().is_some_and(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_keeps_data_separate_from_rendering() {
        let segment = PromptSegment::new(SegmentKind::Git, "main*", "git_dirty")
            .with_label("Git")
            .with_icon("git_branch", "git:");

        assert_eq!(segment.value.as_deref(), Some("main*"));
        assert_eq!(segment.style_key, "git_dirty");
        assert_eq!(
            segment.icon.as_ref().map(|icon| icon.name.as_str()),
            Some("git_branch")
        );
        assert!(!segment.style_key.contains("\u{1b}["));
    }

    #[test]
    fn optional_segment_is_hidden_without_a_value() {
        let segment = PromptSegment::optional(SegmentKind::Git, None::<String>, "git_clean");

        assert!(!segment.is_visible());
    }

    #[test]
    fn model_supports_multiple_segment_kinds() {
        for kind in [
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
            assert!(PromptSegment::new(kind, "value", "default").is_visible());
        }
    }
}
