mod loader;
mod schema;
mod validation;

pub use loader::load_user_config;
pub use schema::{AppConfig, IconSetting, UserCommandConfig, UserCommandKind};
