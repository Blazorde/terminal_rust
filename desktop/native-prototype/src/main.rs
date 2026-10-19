use std::io::{Read, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use eframe::egui;
use egui_tty::{Terminal, TerminalStyle, Tty, TtyStream};
use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

struct PtyConnection {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    exited: Arc<AtomicBool>,
}

impl Tty for PtyConnection {
    fn write(&self, bytes: &[u8]) -> egui_tty::Result<()> {
        let mut writer = self.writer.lock().map_err(egui_tty::Error::msg)?;
        writer.write_all(bytes).map_err(egui_tty::Error::msg)?;
        writer.flush().map_err(egui_tty::Error::msg)
    }

    fn resize(&self, cols: u16, rows: u16) -> egui_tty::Result<()> {
        self.master
            .lock()
            .map_err(egui_tty::Error::msg)?
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(egui_tty::Error::msg)
    }

    fn has_exited(&self) -> bool {
        self.exited.load(Ordering::Acquire)
    }
}

impl Drop for PtyConnection {
    fn drop(&mut self) {
        if let Ok(killer) = self.killer.get_mut() {
            let _ = killer.kill();
        }
    }
}

struct NativeTerminalApp {
    terminal: Terminal,
    last_copied_selection: Option<String>,
    clipboard_error: Option<String>,
}

impl NativeTerminalApp {
    fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let terminal = Terminal::new(spawn_shell()?)?.with_label("Terminal Rust");
        Ok(Self {
            terminal,
            last_copied_selection: None,
            clipboard_error: None,
        })
    }

    fn paste_from_clipboard(&mut self) {
        let result = arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.get_text())
            .map_err(|error| format!("Impossible de lire le presse-papiers : {error}"))
            .and_then(|text| {
                self.terminal
                    .send(&bracketed_paste(&text))
                    .map_err(|error| format!("Impossible d’envoyer le texte au terminal : {error}"))
            });

        self.clipboard_error = result.err();
    }
}

impl eframe::App for NativeTerminalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let clipboard_error = self.clipboard_error.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(16, 20, 28)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Terminal Rust");
                    ui.label("Prototype natif — session PTY réelle");
                });
                if let Some(error) = &clipboard_error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                ui.separator();
                let response = self
                    .terminal
                    .ui(ui, &TerminalStyle::from_visuals(ui.visuals()));
                if response.secondary_clicked() {
                    self.paste_from_clipboard();
                }
            });

        if let Some(selection) = self.terminal.selected_text() {
            if self.last_copied_selection.as_ref() != Some(&selection) {
                ui.ctx().copy_text(selection.clone());
                self.last_copied_selection = Some(selection);
            }
        } else {
            self.last_copied_selection = None;
        }
    }
}

fn spawn_shell() -> Result<TtyStream, Box<dyn std::error::Error + Send + Sync>> {
    let pair = native_pty_system().openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let executable = std::env::current_exe()?;
    let mut command = CommandBuilder::new(executable);
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");
    command.env("TERMINAL_RUST_FASTFETCH", "0");

    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);

    let mut cleanup_killer = child.clone_killer();
    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = cleanup_killer.kill();
            return Err(Box::new(std::io::Error::other(error.to_string())));
        }
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = cleanup_killer.kill();
            return Err(Box::new(std::io::Error::other(error.to_string())));
        }
    };
    let killer = child.clone_killer();
    let exited = Arc::new(AtomicBool::new(false));
    let child_exited = Arc::clone(&exited);
    if let Err(error) = std::thread::Builder::new()
        .name("native-terminal-child".into())
        .spawn(move || {
            wait_for_child(&mut child, child_exited);
        })
    {
        let _ = cleanup_killer.kill();
        return Err(Box::new(error));
    }

    let (sender, output) = mpsc::channel();
    if let Err(error) = std::thread::Builder::new()
        .name("native-terminal-output".into())
        .spawn(move || {
            let mut buffer = vec![0_u8; 8 * 1024];
            while let Ok(count) = reader.read(&mut buffer) {
                if count == 0 || sender.send(buffer[..count].to_vec()).is_err() {
                    break;
                }
            }
        })
    {
        let _ = cleanup_killer.kill();
        return Err(Box::new(error));
    }

    Ok(TtyStream {
        output,
        tty: Arc::new(PtyConnection {
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            killer: Mutex::new(killer),
            exited,
        }),
    })
}

fn wait_for_child(child: &mut Box<dyn Child + Send + Sync>, exited: Arc<AtomicBool>) {
    let _ = child.wait();
    exited.store(true, Ordering::Release);
}

fn bracketed_paste(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend(
        text.chars()
            .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
            .flat_map(|character| {
                let mut encoded = [0; 4];
                character.encode_utf8(&mut encoded).as_bytes().to_vec()
            }),
    );
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

fn run_native_app() -> eframe::Result {
    eframe::run_native(
        "Terminal Rust — prototype natif",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 640.0])
                .with_min_inner_size([560.0, 400.0]),
            ..Default::default()
        },
        Box::new(|_context| {
            NativeTerminalApp::new().map(|app| Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--pty-shell") {
        return terminal_rust::app::run_interactive();
    }

    match run_native_app() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cannot start native Terminal Rust prototype: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::bracketed_paste;

    #[test]
    fn paste_uses_bracketed_mode_and_strips_terminal_control_characters() {
        assert_eq!(
            bracketed_paste("hello\u{1b}[31m\n"),
            b"\x1b[200~hello[31m\n\x1b[201~"
        );
    }
}
