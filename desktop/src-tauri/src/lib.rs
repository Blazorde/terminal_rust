use std::env;
use std::sync::OnceLock;

use portable_pty::CommandBuilder;
use tauri::{AppHandle, Manager, State};

mod git_runtime;
mod pty;
mod settings;

use git_runtime::GitRuntime;
use settings::{DesktopSettings, SettingsLoad, SettingsState};

static GIT_RUNTIME: OnceLock<Result<Option<GitRuntime>, String>> = OnceLock::new();

#[tauri::command]
fn is_terminal_rust_desktop() -> bool {
    true
}

#[tauri::command]
fn load_desktop_settings(state: State<'_, SettingsState>) -> SettingsLoad {
    settings::load(&state)
}

#[tauri::command]
fn save_desktop_settings(
    state: State<'_, SettingsState>,
    settings: DesktopSettings,
) -> Result<(), String> {
    settings::save(&state, &settings)
}

fn bundled_git_runtime(app: &AppHandle) -> Result<Option<GitRuntime>, String> {
    GIT_RUNTIME
        .get_or_init(|| {
            let resources = app
                .path()
                .resource_dir()
                .map_err(|error| format!("cannot locate application resources: {error}"))?;
            let app_data = app
                .path()
                .app_local_data_dir()
                .map_err(|error| format!("cannot locate application data directory: {error}"))?;
            GitRuntime::prepare(&resources.join("resources"), &app_data)
        })
        .clone()
}

fn configure_git_environment(
    command: &mut CommandBuilder,
    runtime: &GitRuntime,
) -> Result<(), String> {
    command.env("TERMINAL_RUST_GIT", &runtime.executable);
    command.env("TERMINAL_RUST_GIT_EXEC_PATH", &runtime.exec_path);
    command.env("TERMINAL_RUST_NANO", &runtime.nano);
    command.env("TERMINAL_RUST_NVIM", &runtime.nvim);
    if let Some(template_dir) = runtime.template_dir.as_ref() {
        command.env("TERMINAL_RUST_GIT_TEMPLATE_DIR", template_dir);
    }
    if let Some(ca_bundle) = runtime.ca_bundle.as_ref() {
        command.env("TERMINAL_RUST_GIT_CA_BUNDLE", ca_bundle);
    }

    let mut path_entries = runtime.path_entries.clone();
    path_entries.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let path = env::join_paths(path_entries)
        .map_err(|error| format!("cannot configure bundled Git search path: {error}"))?;
    command.env("PATH", path);
    Ok(())
}

#[cfg(all(windows, debug_assertions))]
fn configure_development_nano(app: &AppHandle, command: &mut CommandBuilder) -> Result<(), String> {
    let resources = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot locate application resources: {error}"))?
        .join("resources");
    let source = resources.join("nano-win64.exe");
    if !source.is_file() {
        return Ok(());
    }

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("cannot locate application data directory: {error}"))?;
    let bin = app_data.join("development-tools").join("bin");
    std::fs::create_dir_all(&bin)
        .map_err(|error| format!("cannot create development tools directory: {error}"))?;
    let nano = bin.join("nano.exe");
    std::fs::copy(&source, &nano)
        .map_err(|error| format!("cannot install development Nano: {error}"))?;

    command.env("TERMINAL_RUST_NANO", &nano);
    let path = env::join_paths(
        std::iter::once(bin).chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .map_err(|error| format!("cannot configure development editor path: {error}"))?;
    command.env("PATH", path);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(pty::PtyState::default())
        .setup(|app| {
            let config_directory = app.path().app_config_dir()?;
            app.manage(SettingsState::new(config_directory));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            is_terminal_rust_desktop,
            load_desktop_settings,
            save_desktop_settings,
            pty::start_session,
            pty::write_session,
            pty::resize_session,
            pty::close_session
        ])
        .run(tauri::generate_context!())
        .expect("desktop application should start");
}
