use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandContribution {
    pub name: String,
    pub summary: String,
}

impl CommandContribution {
    pub(crate) fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= 64
            && self.name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            })
            && !self.summary.trim().is_empty()
            && !self.summary.chars().any(char::is_control)
    }
}
