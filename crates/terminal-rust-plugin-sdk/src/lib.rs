#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const HOST_API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 0 };

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginContext {
    pub working_directory: Option<ResourceToken>,
    pub last_status: Option<i32>,
    #[serde(default)]
    pub config: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceToken(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRequest {
    pub request_id: String,
    pub contribution_id: String,
    pub arguments: Vec<String>,
    pub context: PluginContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandResponse {
    pub status: CommandStatus,
    #[serde(default)]
    pub output: Vec<OutputChunk>,
    pub directory_change: Option<ResourceToken>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Success,
    Failure,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputChunk {
    pub stream: OutputStream,
    pub text: String,
    pub semantic_style: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

pub trait Plugin {
    fn execute(&mut self, request: CommandRequest) -> Result<CommandResponse, PluginError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_messages_round_trip_through_json() {
        let request = CommandRequest {
            request_id: "request-1".into(),
            contribution_id: "example.hello/hello".into(),
            arguments: vec!["Terminal Rust".into()],
            context: PluginContext {
                working_directory: Some(ResourceToken("cwd:opaque".into())),
                last_status: Some(0),
                config: BTreeMap::new(),
            },
        };

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: CommandRequest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, request);
    }
}
