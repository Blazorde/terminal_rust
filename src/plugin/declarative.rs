use std::collections::{BTreeMap, HashSet};

use crate::plugin::registry::{PluginStatus, RegistryEntry};
use crate::plugin::{Permission, PermissionSet};

#[derive(Debug, Clone)]
pub(crate) struct DeclarativeCommand {
    pub(crate) plugin_id: String,
    pub(crate) response: String,
}

#[derive(Debug, Default)]
pub(crate) struct DeclarativeCommandRegistry {
    blocked: BTreeMap<String, String>,
    commands: BTreeMap<String, DeclarativeCommand>,
}

impl DeclarativeCommandRegistry {
    pub(crate) fn from_entries(
        entries: &[RegistryEntry],
        grants: &BTreeMap<String, Vec<String>>,
        reserved_names: impl IntoIterator<Item = String>,
    ) -> Self {
        let reserved = reserved_names.into_iter().collect::<HashSet<_>>();
        let mut candidates = BTreeMap::<String, Vec<(DeclarativeCommand, Option<String>)>>::new();

        for entry in entries {
            let RegistryEntry::Installed {
                compatibility,
                manifest,
                status: PluginStatus::Enabled,
                ..
            } = entry
            else {
                continue;
            };
            if !compatibility.is_compatible() {
                continue;
            }

            let granted = grants
                .get(&manifest.id)
                .into_iter()
                .flatten()
                .filter_map(|permission| Permission::from_name(permission))
                .collect::<Vec<_>>();
            let permissions = PermissionSet::for_manifest(manifest, &granted);
            let denial = permissions
                .authorize(Permission::CommandRegister)
                .and_then(|_| permissions.authorize(Permission::OutputWrite))
                .err()
                .map(|error| error.to_string());

            for contribution in &manifest.commands {
                let Some(response) = &contribution.response else {
                    continue;
                };
                candidates
                    .entry(contribution.name.clone())
                    .or_default()
                    .push((
                        DeclarativeCommand {
                            plugin_id: manifest.id.clone(),
                            response: response.clone(),
                        },
                        denial.clone(),
                    ));
            }
        }

        let mut blocked = BTreeMap::new();
        let mut accepted = BTreeMap::new();
        for (name, mut commands) in candidates {
            if reserved.contains(&name) {
                continue;
            }
            if commands.len() != 1 {
                blocked.insert(
                    name.clone(),
                    format!("plugin command '{name}' is blocked by a name conflict"),
                );
                continue;
            }
            if let Some((command, denial)) = commands.pop() {
                if let Some(denial) = denial {
                    blocked.insert(name, denial);
                } else {
                    accepted.insert(name, command);
                }
            }
        }
        Self {
            blocked,
            commands: accepted,
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&DeclarativeCommand> {
        self.commands.get(name)
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }

    pub(crate) fn blocked_reason(&self, name: &str) -> Option<&str> {
        self.blocked.get(name).map(String::as_str)
    }
}
