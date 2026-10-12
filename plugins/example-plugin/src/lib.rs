#![forbid(unsafe_code)]

use terminal_rust_plugin_sdk::{
    CommandRequest, CommandResponse, CommandStatus, OutputChunk, OutputStream, Plugin, PluginError,
};

#[derive(Default)]
pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn execute(&mut self, request: CommandRequest) -> Result<CommandResponse, PluginError> {
        if request.contribution_id != "example.hello/hello" {
            return Err(PluginError {
                code: "unknown_contribution".into(),
                message: format!("unknown contribution '{}'", request.contribution_id),
                retryable: false,
            });
        }

        let recipient = request.arguments.first().map_or("world", String::as_str);
        Ok(CommandResponse {
            status: CommandStatus::Success,
            output: vec![OutputChunk {
                stream: OutputStream::Stdout,
                text: format!("Hello, {recipient}!\n"),
                semantic_style: Some("success".into()),
            }],
            directory_change: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use terminal_rust_plugin_sdk::PluginContext;

    #[test]
    fn hello_command_returns_typed_output() {
        let response = HelloPlugin
            .execute(CommandRequest {
                request_id: "test".into(),
                contribution_id: "example.hello/hello".into(),
                arguments: vec!["developer".into()],
                context: PluginContext {
                    working_directory: None,
                    last_status: None,
                    config: BTreeMap::new(),
                },
            })
            .unwrap();

        assert_eq!(response.status, CommandStatus::Success);
        assert_eq!(response.output[0].text, "Hello, developer!\n");
    }
}
