use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

mod settings;

use settings::{DesktopSettings, SettingsLoad, SettingsState};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

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
    let pair = native_pty_system()
        .openpty(pty_size(cols, rows))
        .map_err(|error| format!("cannot open native PTY: {error}"))?;

    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate desktop executable: {error}"))?;
    let mut command = CommandBuilder::new(executable);
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");

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
