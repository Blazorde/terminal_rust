use std::collections::{BTreeMap, HashSet};
use std::io;
use std::process::Command;

use crate::config::{UserCommandConfig, UserCommandKind};
use crate::render;

#[derive(Debug, Clone, Default)]
pub struct UserCommandRegistry {
    commands: BTreeMap<String, UserCommandConfig>,
    persistent_aliases: BTreeMap<String, UserCommandConfig>,
    session_aliases: BTreeMap<String, UserCommandConfig>,
    hidden_aliases: HashSet<String>,
}

impl UserCommandRegistry {
    pub fn new<'a>(
        commands: &BTreeMap<String, UserCommandConfig>,
        aliases: &BTreeMap<String, UserCommandConfig>,
        native_names: impl Iterator<Item = &'a str>,
    ) -> Result<Self, String> {
        let native_names: HashSet<_> = native_names.collect();
        for (name, command) in commands.iter().chain(aliases) {
            if native_names.contains(name.as_str()) {
                return Err(format!(
                    "custom command '{name}' conflicts with a built-in command"
                ));
            }
            if aliases.contains_key(name) && commands.contains_key(name) {
                return Err(format!(
                    "alias '{name}' conflicts with a configured command"
                ));
            }
            if !crate::config::is_valid_user_command_name(name) {
                return Err(format!("custom command has invalid name '{name}'"));
            }
            if command.target.trim().is_empty()
                || command.target.contains('\0')
                || command.args.iter().any(|argument| argument.contains('\0'))
            {
                return Err(format!(
                    "custom command '{name}' has an invalid target or argument"
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
            persistent_aliases: aliases.clone(),
            session_aliases: BTreeMap::new(),
            hidden_aliases: HashSet::new(),
        })
    }

    pub fn get(&self, name: &str) -> Option<&UserCommandConfig> {
        self.session_aliases
            .get(name)
            .or_else(|| {
                if self.hidden_aliases.contains(name) {
                    None
                } else {
                    self.persistent_aliases.get(name)
                }
            })
            .or_else(|| self.commands.get(name))
    }

    pub fn names(&self) -> Vec<String> {
        self.commands
            .keys()
            .chain(self.persistent_aliases.keys())
            .chain(self.session_aliases.keys())
            .filter(|name| !self.hidden_aliases.contains(*name))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn persistent_aliases(&self) -> &BTreeMap<String, UserCommandConfig> {
        &self.persistent_aliases
    }

    pub fn configured_command_exists(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn aliases(&self) -> Vec<(String, UserCommandConfig, bool)> {
        let mut aliases = self
            .persistent_aliases
            .iter()
            .filter(|(name, _)| !self.hidden_aliases.contains(*name))
            .map(|(name, command)| (name.clone(), (command.clone(), true)))
            .collect::<BTreeMap<_, _>>();
        for (name, command) in &self.session_aliases {
            aliases.insert(name.clone(), (command.clone(), false));
        }
        aliases
            .into_iter()
            .map(|(name, (command, persistent))| (name, command, persistent))
            .collect()
    }

    pub fn set_session_alias(
        &mut self,
        name: String,
        command: UserCommandConfig,
        native_names: &HashSet<String>,
    ) -> Result<(), String> {
        self.validate_alias(&name, &command, native_names)?;
        self.hidden_aliases.remove(&name);
        self.session_aliases.insert(name, command);
        Ok(())
    }

    pub fn set_persistent_aliases(
        &mut self,
        aliases: BTreeMap<String, UserCommandConfig>,
        native_names: &HashSet<String>,
    ) -> Result<(), String> {
        for (name, command) in &aliases {
            self.validate_alias(name, command, native_names)?;
            if self.commands.contains_key(name) {
                return Err(format!(
                    "alias '{name}' conflicts with a configured command"
                ));
            }
        }
        self.persistent_aliases = aliases;
        for name in self.persistent_aliases.keys() {
            self.session_aliases.remove(name);
            self.hidden_aliases.remove(name);
        }
        Ok(())
    }

    pub fn render_aliases(&self) -> String {
        let aliases = self.aliases();
        if aliases.is_empty() {
            return "No aliases configured.\nUse 'alias <name> <command> [arguments...]' to add one.\n".into();
        }
        let mut output = format!("Aliases ({}):\n", aliases.len());
        for (name, command, persistent) in aliases {
            let arguments = if command.args.is_empty() {
                String::new()
            } else {
                format!(" {}", command.args.join(" "))
            };
            output.push_str(&format!(
                "  {name:<20} {}{} [{}]\n",
                command.target,
                arguments,
                if persistent { "saved" } else { "session" }
            ));
        }
        output
    }

    pub fn remove_alias(&mut self, name: &str, permanent: bool) -> Result<(), String> {
        if self.commands.contains_key(name) {
            return Err(format!("'{name}' is a configured command, not an alias"));
        }
        let existed = self.session_aliases.remove(name).is_some()
            || self.persistent_aliases.contains_key(name);
        if !existed {
            return Err(format!("alias '{name}' does not exist"));
        }
        if permanent {
            self.persistent_aliases.remove(name);
            self.hidden_aliases.remove(name);
        } else if self.persistent_aliases.contains_key(name) {
            self.hidden_aliases.insert(name.to_string());
        }
        Ok(())
    }

    pub fn remove_session_alias(&mut self, name: &str) {
        self.session_aliases.remove(name);
    }

    pub fn render_list(&self) -> String {
        if self.commands.is_empty()
            && self.persistent_aliases.is_empty()
            && self.session_aliases.is_empty()
        {
            return "No custom commands or aliases configured.\n".into();
        }
        let mut output = String::new();
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
        for (name, command, persistent) in self.aliases() {
            output.push_str(&format!(
                "  {name:<20} [alias, {}] {} {}\n",
                if persistent { "saved" } else { "session" },
                command.target,
                command.args.join(" ")
            ));
        }
        output
    }

    fn validate_alias(
        &self,
        name: &str,
        command: &UserCommandConfig,
        native_names: &HashSet<String>,
    ) -> Result<(), String> {
        if !crate::config::is_valid_user_command_name(name) {
            return Err(format!(
                "invalid alias name '{name}' (use 1-64 lowercase letters, digits, '-' or '_')"
            ));
        }
        if native_names.contains(name) || self.commands.contains_key(name) {
            return Err(format!("alias '{name}' conflicts with an existing command"));
        }
        if command.target.trim().is_empty()
            || command.target.contains('\0')
            || command.args.iter().any(|argument| argument.contains('\0'))
        {
            return Err("alias target and arguments must not be empty or contain NUL".into());
        }
        if command.kind == UserCommandKind::Builtin && !native_names.contains(&command.target) {
            return Err(format!(
                "alias '{name}' targets unknown built-in command '{}'",
                command.target
            ));
        }
        Ok(())
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
            UserCommandRegistry::new(&commands, &BTreeMap::new(), ["pwd", "help"].into_iter())
                .unwrap_err()
                .contains("conflicts")
        );

        commands.clear();
        commands.insert(
            "shortcut".into(),
            command(UserCommandKind::Builtin, "missing"),
        );
        assert!(
            UserCommandRegistry::new(&commands, &BTreeMap::new(), ["pwd", "help"].into_iter())
                .unwrap_err()
                .contains("unknown built-in")
        );
    }

    #[test]
    fn session_alias_can_hide_a_persistent_alias_without_deleting_it() {
        let native_names = ["pwd".to_string(), "help".to_string()]
            .into_iter()
            .collect::<HashSet<_>>();
        let mut aliases = BTreeMap::new();
        aliases.insert("ll".into(), command(UserCommandKind::Builtin, "help"));
        let mut registry =
            UserCommandRegistry::new(&BTreeMap::new(), &aliases, ["pwd", "help"].into_iter())
                .expect("persistent aliases should load");

        registry
            .set_session_alias(
                "ll".into(),
                command(UserCommandKind::Builtin, "pwd"),
                &native_names,
            )
            .expect("session alias should override a persistent alias");
        assert_eq!(registry.get("ll").unwrap().target, "pwd");
        registry
            .remove_alias("ll", false)
            .expect("session alias should be removable");

        assert!(registry.get("ll").is_none());
        assert_eq!(registry.persistent_aliases()["ll"].target, "help");
        registry
            .remove_alias("ll", true)
            .expect("persistent alias should be removable");
        assert!(registry.get("ll").is_none());
        assert!(registry.persistent_aliases().is_empty());
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
