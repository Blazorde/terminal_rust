use super::schema::AppConfig;
use crate::plugin::{Permission, is_valid_id};
use std::collections::HashSet;

pub fn validate(config: &AppConfig) -> Result<(), String> {
    if config.version != 1 {
        return Err(format!(
            "unsupported configuration version {} (expected 1)",
            config.version
        ));
    }
    if config.theme.trim().is_empty() {
        return Err("theme must not be empty".to_string());
    }
    if config.prompt.segments.is_empty() && config.prompt.right_segments.is_empty() {
        return Err("prompt layout must contain at least one segment".to_string());
    }
    let mut seen = HashSet::new();
    for (property, segments) in [
        ("prompt.segments", &config.prompt.segments),
        ("prompt.right_segments", &config.prompt.right_segments),
    ] {
        for segment in segments {
            if !matches!(
                segment.as_str(),
                "user"
                    | "host"
                    | "path"
                    | "git"
                    | "status"
                    | "runtime"
                    | "duration"
                    | "time"
                    | "symbol"
            ) {
                return Err(format!("{property} contains unknown segment '{segment}'"));
            }
            if !seen.insert(segment) {
                return Err(format!(
                    "prompt layout contains duplicate segment '{segment}'"
                ));
            }
        }
    }
    if config.prompt.separator.chars().any(char::is_control) {
        return Err("prompt.separator must not contain control characters".to_string());
    }
    if config.segments.path.style != "full" && config.segments.path.style != "folder" {
        return Err("segments.path.style must be 'full' or 'folder'".to_string());
    }
    if config.segments.symbol.value.is_empty()
        || config.segments.symbol.value.chars().any(char::is_control)
    {
        return Err("segments.symbol.value must be visible text".to_string());
    }

    let mut enabled_plugins = HashSet::new();
    for plugin_id in &config.plugins.enabled {
        validate_plugin_id(plugin_id, "plugins.enabled")?;
        if !enabled_plugins.insert(plugin_id) {
            return Err(format!(
                "plugins.enabled contains duplicate plugin '{plugin_id}'"
            ));
        }
    }
    for plugin_id in config.plugins.settings.keys() {
        validate_plugin_id(plugin_id, "plugins")?;
    }
    for (plugin_id, permissions) in &config.plugins.grants {
        validate_plugin_id(plugin_id, "plugins.grants")?;
        let mut granted = HashSet::new();
        for permission in permissions {
            if Permission::from_name(permission).is_none() {
                return Err(format!(
                    "plugins.grants.{plugin_id} contains unknown permission '{permission}'"
                ));
            }
            if !granted.insert(permission) {
                return Err(format!(
                    "plugins.grants.{plugin_id} contains duplicate permission '{permission}'"
                ));
            }
        }
    }
    for (name, command) in &config.commands {
        if !is_valid_user_command_name(name) {
            return Err(format!("commands contains invalid command name '{name}'"));
        }
        if command.target.trim().is_empty()
            || command.target.chars().any(|character| character == '\0')
        {
            return Err(format!(
                "commands.{name}.target must not be empty or contain NUL"
            ));
        }
        if command.args.iter().any(|argument| argument.contains('\0')) {
            return Err(format!("commands.{name}.args must not contain NUL"));
        }
        if command.description.chars().any(char::is_control) {
            return Err(format!(
                "commands.{name}.description must not contain control characters"
            ));
        }
    }

    let _ = config.segments.git.show_status;
    let _ = config.prompt.newline;
    Ok(())
}

fn is_valid_user_command_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && name
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

fn validate_plugin_id(plugin_id: &str, property: &str) -> Result<(), String> {
    if is_valid_id(plugin_id) {
        Ok(())
    } else {
        Err(format!(
            "{property} contains invalid plugin id '{plugin_id}' (expected 1-64 lowercase ASCII letters, digits, '.', '_' or '-', starting and ending with a letter or digit)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_supported_segments_across_both_positions() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec![
            "user".into(),
            "host".into(),
            "path".into(),
            "git".into(),
            "symbol".into(),
        ];
        config.prompt.right_segments = vec![
            "status".into(),
            "runtime".into(),
            "duration".into(),
            "time".into(),
        ];

        assert_eq!(validate(&config), Ok(()));
    }

    #[test]
    fn rejects_duplicates_across_positions() {
        let mut config = AppConfig::default();
        config.prompt.right_segments.push("path".into());

        assert_eq!(
            validate(&config),
            Err("prompt layout contains duplicate segment 'path'".into())
        );
    }

    #[test]
    fn rejects_duplicate_enabled_plugins() {
        let mut config = AppConfig::default();
        config.plugins.enabled = vec!["git".into(), "git".into()];

        assert_eq!(
            validate(&config),
            Err("plugins.enabled contains duplicate plugin 'git'".into())
        );
    }

    #[test]
    fn rejects_invalid_plugin_identifiers() {
        let mut config = AppConfig::default();
        config.plugins.enabled = vec!["Git Plugin".into()];

        let error = validate(&config).expect_err("invalid plugin id should fail");
        assert!(error.contains("plugins.enabled contains invalid plugin id 'Git Plugin'"));
    }

    #[test]
    fn rejects_unknown_and_duplicate_plugin_grants() {
        let mut config = AppConfig::default();
        config
            .plugins
            .grants
            .insert("example.plugin".into(), vec!["unknown".into()]);
        assert!(
            validate(&config)
                .unwrap_err()
                .contains("unknown permission")
        );

        config.plugins.grants.insert(
            "example.plugin".into(),
            vec!["output.write".into(), "output.write".into()],
        );
        assert!(
            validate(&config)
                .unwrap_err()
                .contains("duplicate permission")
        );
    }

    #[test]
    fn rejects_invalid_user_command_names_and_nul_arguments() {
        let mut config = AppConfig::default();
        config.commands.insert(
            "Bad Command".into(),
            super::super::schema::UserCommandConfig {
                target: "pwd".into(),
                ..Default::default()
            },
        );
        assert!(
            validate(&config)
                .unwrap_err()
                .contains("invalid command name")
        );

        config.commands.clear();
        config.commands.insert(
            "bad-arg".into(),
            super::super::schema::UserCommandConfig {
                target: "pwd".into(),
                args: vec!["bad\0argument".into()],
                ..Default::default()
            },
        );
        assert!(
            validate(&config)
                .unwrap_err()
                .contains("must not contain NUL")
        );
    }
}
