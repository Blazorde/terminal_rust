use std::collections::{BTreeMap, HashSet};
use std::io;
use std::process::Command;

use crate::config::{UserCommandConfig, UserCommandKind};
use crate::render;

#[derive(Debug, Clone, Default)]
pub struct UserCommandRegistry {
    commands: BTreeMap<String, UserCommandConfig>,
}

impl UserCommandRegistry {
    pub fn new<'a>(
        commands: &BTreeMap<String, UserCommandConfig>,
        native_names: impl Iterator<Item = &'a str>,
    ) -> Result<Self, String> {
        let native_names: HashSet<_> = native_names.collect();
        for (name, command) in commands {
            if native_names.contains(name.as_str()) {
                return Err(format!(
                    "custom command '{name}' conflicts with a built-in command"
                ));
            }
            if command.kind == UserCommandKind::Builtin
                && !native_names.contains(command.target.as_str())
            {
                return Err(format!(
                    "custom command '{name}' targets unknown built-in command '{}'",
                    command.target
                ));
            }
        }
        Ok(Self {
            commands: commands.clone(),
        })
    }

    pub fn get(&self, name: &str) -> Option<&UserCommandConfig> {
        self.commands.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }

    pub fn render_list(&self) -> String {
        if self.commands.is_empty() {
            return "No custom commands configured.\n".into();
        }
        let mut output = format!("Custom commands ({}):\n", self.commands.len());
        for (name, command) in &self.commands {
            let kind = match command.kind {
                UserCommandKind::Builtin => "builtin",
                UserCommandKind::Process => "process",
            };
            let description = if command.description.is_empty() {
                "custom command"
            } else {
                &command.description
            };
            output.push_str(&format!(
                "  {name:<20} [{kind}] {} - {description}\n",
                command.target
            ));
        }
        output
    }
}

pub fn run_process(
    name: &str,
    command: &UserCommandConfig,
    invocation_args: &[String],
) -> Result<(), String> {
    let mut process = Command::new(&command.target);
    process.args(&command.args);
    if command.append_args {
        process.args(invocation_args);
    } else if !invocation_args.is_empty() {
        return Err(format!(
            "{name}: this custom command does not accept arguments"
        ));
    }

    let output = process
        .output()
        .map_err(|error| format!("{name}: cannot start '{}': {error}", command.target))?;
    write_process_output(&output.stdout, &output.stderr)
        .map_err(|error| format!("{name}: cannot write process output: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{name}: process exited with status {}",
            output.status
        ))
    }
}

fn write_process_output(stdout: &[u8], stderr: &[u8]) -> io::Result<()> {
    if !stdout.is_empty() {
        render::stdout(String::from_utf8_lossy(stdout))?;
    }
    if !stderr.is_empty() {
        render::stderr(String::from_utf8_lossy(stderr))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(kind: UserCommandKind, target: &str) -> UserCommandConfig {
        UserCommandConfig {
            kind,
            target: target.into(),
            description: "fixture".into(),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_builtin_conflicts_and_unknown_targets() {
        let mut commands = BTreeMap::new();
        commands.insert("pwd".into(), command(UserCommandKind::Builtin, "help"));
        assert!(
            UserCommandRegistry::new(&commands, ["pwd", "help"].into_iter())
                .unwrap_err()
                .contains("conflicts")
        );

        commands.clear();
        commands.insert(
            "shortcut".into(),
            command(UserCommandKind::Builtin, "missing"),
        );
        assert!(
            UserCommandRegistry::new(&commands, ["pwd", "help"].into_iter())
                .unwrap_err()
                .contains("unknown built-in")
        );
    }

    #[test]
    fn missing_process_has_an_actionable_error() {
        let command = command(
            UserCommandKind::Process,
            "terminal-rust-executable-that-does-not-exist",
        );
        let error = run_process("missing-tool", &command, &[]).unwrap_err();
        assert!(error.contains("missing-tool: cannot start"));
        assert!(error.contains(&command.target));
    }
}
