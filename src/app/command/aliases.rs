use std::collections::{BTreeMap, HashSet};

use super::{CommandResult, ShellState, command_names, find_command, map_prefixed_error};
use crate::commands::user;
use crate::config::{self, UserCommandConfig, UserCommandKind};
use crate::render;

pub(super) fn run(args: &[String], state: &mut ShellState) -> CommandResult {
    let persistent = args.first().is_some_and(|arg| arg == "--save");
    let args = if persistent { &args[1..] } else { args };
    if args.is_empty() || args == ["--list"] {
        return map_prefixed_error(
            "alias",
            render::stdout(state.user_commands.render_aliases()),
        );
    }
    if args.len() < 2 {
        return CommandResult::Error(
            "alias: expected [--save] <name> <command> [arguments...]".into(),
        );
    }

    let name = &args[0];
    let target = &args[1];
    let command = UserCommandConfig {
        kind: if find_command(target).is_some() {
            UserCommandKind::Builtin
        } else {
            UserCommandKind::Process
        },
        target: target.clone(),
        args: args[2..].to_vec(),
        append_args: true,
        description: String::new(),
    };
    let native_names = reserved_names(state);
    if persistent {
        let mut aliases = state.user_commands.persistent_aliases().clone();
        aliases.insert(name.clone(), command);
        if let Err(error) = validate_aliases(&aliases, &native_names, &state.user_commands) {
            return CommandResult::Error(format!("alias: {error}"));
        }
        if let Err(error) = config::save_aliases(aliases.clone()) {
            return CommandResult::Error(format!("alias: {error}"));
        }
        if let Err(error) = state
            .user_commands
            .set_persistent_aliases(aliases, &native_names)
        {
            return CommandResult::Error(format!("alias: {error}"));
        }
    } else if let Err(error) =
        state
            .user_commands
            .set_session_alias(name.clone(), command, &native_names)
    {
        return CommandResult::Error(format!("alias: {error}"));
    }
    map_prefixed_error(
        "alias",
        render::stdout(format!(
            "Alias '{name}' saved{}.\n",
            if persistent {
                " permanently"
            } else {
                " for this session"
            }
        )),
    )
}

pub(super) fn remove(args: &[String], state: &mut ShellState) -> CommandResult {
    let persistent = args.first().is_some_and(|arg| arg == "--save");
    let name = if persistent {
        args.get(1)
    } else {
        args.first()
    };
    let Some(name) = name else {
        return CommandResult::Error("unalias: expected [--save] <name>".into());
    };
    if args.len() != if persistent { 2 } else { 1 } {
        return CommandResult::Error("unalias: expected [--save] <name>".into());
    }

    if persistent {
        let mut aliases = state.user_commands.persistent_aliases().clone();
        if !aliases.contains_key(name) {
            return CommandResult::Error(format!("unalias: saved alias '{name}' does not exist"));
        }
        aliases.remove(name);
        if let Err(error) = config::save_aliases(aliases.clone()) {
            return CommandResult::Error(format!("unalias: {error}"));
        }
        let native_names = reserved_names(state);
        if let Err(error) = state
            .user_commands
            .set_persistent_aliases(aliases, &native_names)
        {
            return CommandResult::Error(format!("unalias: {error}"));
        }
        state.user_commands.remove_session_alias(name);
    } else if let Err(error) = state.user_commands.remove_alias(name, false) {
        return CommandResult::Error(format!("unalias: {error}"));
    }
    map_prefixed_error(
        "unalias",
        render::stdout(format!("Alias '{name}' removed.\n")),
    )
}

fn validate_aliases(
    aliases: &BTreeMap<String, UserCommandConfig>,
    native_names: &HashSet<String>,
    commands: &user::UserCommandRegistry,
) -> Result<(), String> {
    for (name, command) in aliases {
        if commands.configured_command_exists(name) {
            return Err(format!(
                "alias '{name}' conflicts with a configured command"
            ));
        }
        if native_names.contains(name) {
            return Err(format!("alias '{name}' conflicts with an existing command"));
        }
        if !crate::config::is_valid_user_command_name(name) {
            return Err(format!("invalid alias name '{name}'"));
        }
        if command.target.trim().is_empty() || command.target.contains('\0') {
            return Err(format!("alias '{name}' has an invalid target"));
        }
        if command.kind == UserCommandKind::Builtin && !native_names.contains(&command.target) {
            return Err(format!(
                "alias '{name}' targets unknown built-in command '{}'",
                command.target
            ));
        }
        if command.args.iter().any(|argument| argument.contains('\0')) {
            return Err(format!("alias '{name}' contains a NUL argument"));
        }
    }
    Ok(())
}

fn reserved_names(state: &ShellState) -> HashSet<String> {
    command_names()
        .map(str::to_string)
        .chain(state.plugin_command_names().map(str::to_string))
        .collect()
}
