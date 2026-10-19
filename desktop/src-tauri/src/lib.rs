use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

mod git_runtime;
mod settings;

use git_runtime::GitRuntime;
use settings::{DesktopSettings, SettingsLoad, SettingsState};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
static GIT_RUNTIME: OnceLock<Result<Option<GitRuntime>, String>> = OnceLock::new();

#[derive(Default)]
struct PtyState {
    sessions: Mutex<HashMap<u64, PtySession>>,
}

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyOutput {
    session_id: u64,
    data: Vec<u8>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyExit {
    session_id: u64,
    success: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyError {
    session_id: u64,
    message: String,
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

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

#[tauri::command]
fn start_session(
    app: AppHandle,
    state: State<'_, PtyState>,
    cols: u16,
    rows: u16,
) -> Result<u64, String> {
    let git_runtime = bundled_git_runtime(&app)?;
    let pair = native_pty_system()
        .openpty(pty_size(cols, rows))
        .map_err(|error| format!("cannot open native PTY: {error}"))?;

    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate desktop executable: {error}"))?;
    let mut command = CommandBuilder::new(executable);
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");
    if let Some(runtime) = git_runtime.as_ref() {
        configure_git_environment(&mut command, runtime)?;
    }
    #[cfg(all(windows, debug_assertions))]
    if git_runtime.is_none() {
        configure_development_nano(&app, &mut command)?;
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("cannot start Terminal Rust: {error}"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("cannot open PTY output: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("cannot open PTY input: {error}"))?;
    let killer = child.clone_killer();
    let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);

    state
        .sessions
        .lock()
        .map_err(|_| "PTY session store is unavailable".to_string())?
        .insert(
            session_id,
            PtySession {
                master: pair.master,
                writer,
                killer,
            },
        );

    let output_app = app.clone();
    std::thread::spawn(move || forward_output(session_id, &mut reader, &output_app));

    std::thread::spawn(move || {
        let result = child.wait();
        if let Ok(mut sessions) = app.state::<PtyState>().sessions.lock() {
            sessions.remove(&session_id);
        }
        let payload = match result {
            Ok(status) => PtyExit {
                session_id,
                success: status.success(),
            },
            Err(error) => {
                let _ = app.emit(
                    "pty-error",
                    PtyError {
                        session_id,
                        message: format!("cannot wait for terminal process: {error}"),
                    },
                );
                PtyExit {
                    session_id,
                    success: false,
                }
            }
        };
        let _ = app.emit("pty-exit", payload);
    });

    Ok(session_id)
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

fn forward_output(session_id: u64, reader: &mut dyn Read, app: &AppHandle) {
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if app
                    .emit(
                        "pty-output",
                        PtyOutput {
                            session_id,
                            data: buffer[..count].to_vec(),
                        },
                    )
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                let _ = app.emit(
                    "pty-error",
                    PtyError {
                        session_id,
                        message: format!("cannot read terminal output: {error}"),
                    },
                );
                break;
            }
        }
    }
}

#[tauri::command]
fn write_session(state: State<'_, PtyState>, session_id: u64, data: Vec<u8>) -> Result<(), String> {
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY session store is unavailable".to_string())?;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "terminal session no longer exists".to_string())?;
    session
        .writer
        .write_all(&data)
        .and_then(|()| session.writer.flush())
        .map_err(|error| format!("cannot write terminal input: {error}"))
}

#[tauri::command]
fn resize_session(
    state: State<'_, PtyState>,
    session_id: u64,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY session store is unavailable".to_string())?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "terminal session no longer exists".to_string())?;
    session
        .master
        .resize(pty_size(cols, rows))
        .map_err(|error| format!("cannot resize terminal: {error}"))
}

#[tauri::command]
fn close_session(state: State<'_, PtyState>, session_id: u64) -> Result<(), String> {
    let mut session = state
        .sessions
        .lock()
        .map_err(|_| "PTY session store is unavailable".to_string())?
        .remove(&session_id);

    if let Some(session) = session.as_mut()
        && let Err(error) = session.killer.kill()
        && error.raw_os_error() != Some(0)
    {
        return Err(format!("cannot close terminal session: {error}"));
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PtyState::default())
        .setup(|app| {
            let config_directory = app.path().app_config_dir()?;
            app.manage(SettingsState::new(config_directory));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            is_terminal_rust_desktop,
            load_desktop_settings,
            save_desktop_settings,
            start_session,
            write_session,
            resize_session,
            close_session
        ])
        .run(tauri::generate_context!())
        .expect("desktop application should start");
}

#[cfg(test)]
mod tests {
    use super::{PtyExit, PtyOutput, pty_size};

    #[test]
    fn pty_dimensions_never_reach_the_native_backend_as_zero() {
        let size = pty_size(0, 0);

        assert_eq!(size.cols, 1);
        assert_eq!(size.rows, 1);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn pty_events_use_the_frontend_camel_case_contract() {
        let output = serde_json::to_value(PtyOutput {
            session_id: 7,
            data: vec![65, 10],
        })
        .expect("PTY output should serialize");
        let exit = serde_json::to_value(PtyExit {
            session_id: 7,
            success: true,
        })
        .expect("PTY exit should serialize");

        assert_eq!(output["sessionId"], 7);
        assert_eq!(output["data"], serde_json::json!([65, 10]));
        assert_eq!(exit["sessionId"], 7);
        assert_eq!(exit["success"], true);
    }
}
