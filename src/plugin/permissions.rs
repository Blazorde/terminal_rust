use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use super::{Permission, PluginManifest};

#[derive(Debug, Clone)]
pub(crate) struct PermissionSet {
    plugin_id: String,
    requested: BTreeSet<Permission>,
    granted: BTreeSet<Permission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionDenial {
    NotDeclared,
    NotGranted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionError {
    plugin_id: String,
    permission: Permission,
    denial: PermissionDenial,
}

impl PermissionSet {
    pub(crate) fn for_manifest(manifest: &PluginManifest, granted: &[Permission]) -> Self {
        Self {
            plugin_id: manifest.id.clone(),
            requested: manifest.permissions.iter().copied().collect(),
            granted: granted.iter().copied().collect(),
        }
    }

    pub(crate) fn authorize(&self, permission: Permission) -> Result<(), PermissionError> {
        let denial = if !self.requested.contains(&permission) {
            Some(PermissionDenial::NotDeclared)
        } else if !self.granted.contains(&permission) {
            Some(PermissionDenial::NotGranted)
        } else {
            None
        };

        denial.map_or(Ok(()), |denial| {
            Err(PermissionError {
                plugin_id: self.plugin_id.clone(),
                permission,
                denial,
            })
        })
    }
}

impl fmt::Display for PermissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self.denial {
            PermissionDenial::NotDeclared => "is not declared in its manifest",
            PermissionDenial::NotGranted => "has not been granted by the user",
        };
        write!(
            formatter,
            "plugin '{}' cannot use permission '{}': {reason}",
            self.plugin_id, self.permission
        )
    }
}

impl Error for PermissionError {}

#[cfg(test)]
mod tests {
    use semver::{Version, VersionReq};

    use super::*;

    fn manifest(permissions: Vec<Permission>) -> PluginManifest {
        PluginManifest {
            schema_version: 1,
            id: "example.permissions".into(),
            name: "Permission fixture".into(),
            version: Version::new(1, 0, 0),
            description: "Fixture".into(),
            host_api: VersionReq::parse("^1").unwrap(),
            authors: Vec::new(),
            permissions,
            commands: Vec::new(),
            prompt_segments: Vec::new(),
        }
    }

    #[test]
    fn authorizes_only_requested_and_granted_permissions() {
        let manifest = manifest(vec![Permission::FilesystemRead]);
        let permissions = PermissionSet::for_manifest(&manifest, &[Permission::FilesystemRead]);

        assert!(permissions.authorize(Permission::FilesystemRead).is_ok());
    }

    #[test]
    fn blocks_an_action_missing_from_the_manifest() {
        let manifest = manifest(Vec::new());
        let permissions = PermissionSet::for_manifest(&manifest, &[Permission::ProcessSpawn]);

        let error = permissions.authorize(Permission::ProcessSpawn).unwrap_err();

        assert_eq!(error.denial, PermissionDenial::NotDeclared);
        assert_eq!(
            error.to_string(),
            "plugin 'example.permissions' cannot use permission 'process.spawn': is not declared in its manifest"
        );
    }

    #[test]
    fn blocks_a_requested_permission_without_user_consent() {
        let manifest = manifest(vec![Permission::NetworkConnect]);
        let permissions = PermissionSet::for_manifest(&manifest, &[]);

        let error = permissions
            .authorize(Permission::NetworkConnect)
            .unwrap_err();

        assert_eq!(error.denial, PermissionDenial::NotGranted);
        assert!(
            error
                .to_string()
                .contains("has not been granted by the user")
        );
    }
}
