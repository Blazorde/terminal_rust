mod command;
mod manifest;
mod permissions;
mod prompt_segment;
pub(crate) mod registry;

pub(crate) use manifest::{Compatibility, Permission, PluginManifest, is_valid_id};
pub(crate) use permissions::PermissionSet;
