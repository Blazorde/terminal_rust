use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub theme: String,
    pub prompt: PromptConfig,
    pub segments: SegmentsConfig,
    pub plugins: PluginsConfig,
    pub commands: BTreeMap<String, UserCommandConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            theme: "default".to_string(),
            prompt: PromptConfig::default(),
            segments: SegmentsConfig::default(),
            plugins: PluginsConfig::default(),
            commands: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct UserCommandConfig {
    pub kind: UserCommandKind,
    pub target: String,
    pub args: Vec<String>,
    pub append_args: bool,
    pub description: String,
}

impl Default for UserCommandConfig {
    fn default() -> Self {
        Self {
            kind: UserCommandKind::Builtin,
            target: String::new(),
            args: Vec::new(),
            append_args: true,
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserCommandKind {
    #[default]
    Builtin,
    Process,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct PluginsConfig {
    pub enabled: Vec<String>,
    #[serde(flatten)]
    pub settings: BTreeMap<String, toml::Table>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PromptConfig {
    pub segments: Vec<String>,
    pub right_segments: Vec<String>,
    pub icons: IconSetting,
    pub separator: String,
    pub newline: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            segments: vec![
                "user".to_string(),
                "host".to_string(),
                "path".to_string(),
                "git".to_string(),
                "symbol".to_string(),
            ],
            right_segments: vec![
                "status".to_string(),
                "runtime".to_string(),
                "duration".to_string(),
                "time".to_string(),
            ],
            icons: IconSetting::Auto,
            separator: "  ".to_string(),
            newline: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IconSetting {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SegmentsConfig {
    pub path: PathSegmentConfig,
    pub git: GitSegmentConfig,
    pub symbol: SymbolSegmentConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PathSegmentConfig {
    pub style: String,
}

impl Default for PathSegmentConfig {
    fn default() -> Self {
        Self {
            style: "full".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct GitSegmentConfig {
    pub show_status: bool,
}

impl Default for GitSegmentConfig {
    fn default() -> Self {
        Self { show_status: true }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SymbolSegmentConfig {
    pub value: String,
}

impl Default for SymbolSegmentConfig {
    fn default() -> Self {
        Self {
            value: ">".to_string(),
        }
    }
}
