use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandContribution {
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub response: Option<String>,
}

impl CommandContribution {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.name.is_empty()
            || self.name.len() > 64
            || !self.name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            })
        {
            return Err("name must contain 1-64 lowercase ASCII letters, digits, '-' or '_'");
        }
        if self.summary.trim().is_empty() || self.summary.chars().any(char::is_control) {
            return Err("summary must be non-empty visible text");
        }
        if let Some(response) = &self.response
            && (response.is_empty()
                || response.len() > 4096
                || response
                    .chars()
                    .any(|character| character == '\0' || character == '\u{1b}'))
        {
            return Err("response must contain 1-4096 bytes without NUL or escape characters");
        }
        Ok(())
    }
}
