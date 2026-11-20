mod appearance;
mod loader;
mod schema;
mod shell_state;
mod validation;

pub use appearance::{
    AppearanceSettings, AppearanceTheme, appearance_ansi_sequences,
    appearance_ansi_sequences_for_system, appearance_palette, appearance_settings_path,
    apply_appearance_environment, load_appearance_settings, save_appearance_settings,
    validate_appearance_settings, validate_hex_color,
};
pub use loader::load_user_config;
pub use schema::{AppConfig, IconSetting, UserCommandConfig, UserCommandKind};
pub(crate) use shell_state::{load_persistent_shell_state, save_aliases, save_startup_script};
pub(crate) use validation::is_valid_user_command_name;
