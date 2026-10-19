use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime};

use eframe::egui;
use egui_tty::{ColorScheme, Terminal, TerminalStyle, Tty, TtyStream};
use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use terminal_rust::appearance::{
    self, AppearanceSettings, AppearanceTheme, appearance_settings_path,
};

const SETTINGS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const USAGE: &str = "Usage: terminal-rust-native-prototype [--theme system|dark|light] [--foreground #RRGGBB] [--background #RRGGBB] [--color INDEX=#RRGGBB]";

type NativeResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type TerminalStream = (TtyStream, mpsc::Sender<Vec<u8>>);

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
    settings_error: Option<String>,
    settings: AppearanceSettings,
    overrides: LaunchOverrides,
    terminal_output: mpsc::Sender<Vec<u8>>,
    settings_path: Option<PathBuf>,
    settings_modified: Option<SystemTime>,
    last_settings_check: Instant,
    last_system_dark_mode: Option<bool>,
}

impl NativeTerminalApp {
    fn new(
        overrides: LaunchOverrides,
        system_theme: &'static str,
        system_dark_mode: bool,
    ) -> NativeResult<Self> {
        let settings_path = appearance_settings_path();
        let mut settings = appearance::load_appearance_settings()?;
        overrides.apply(&mut settings);
        appearance::validate_appearance_settings(&settings)?;
        let settings_modified = file_modified(settings_path.as_deref());
        let (stream, terminal_output) = spawn_shell(&overrides, system_theme)?;
        let terminal = Terminal::new(stream)?.with_label("Terminal Rust");
        let mut terminal = terminal;
        terminal.request_focus();
        Ok(Self {
            terminal,
            last_copied_selection: None,
            clipboard_error: None,
            settings_error: None,
            settings,
            overrides,
            terminal_output,
            settings_path,
            settings_modified,
            last_settings_check: Instant::now(),
            last_system_dark_mode: Some(system_dark_mode),
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

    fn refresh_settings(&mut self, ctx: &egui::Context, system_dark_mode: bool) {
        if self.last_settings_check.elapsed() < SETTINGS_REFRESH_INTERVAL {
            return;
        }
        self.last_settings_check = Instant::now();

        let modified = file_modified(self.settings_path.as_deref());
        if modified == self.settings_modified {
            return;
        }
        self.settings_modified = modified;

        match appearance::load_appearance_settings() {
            Ok(mut settings) => {
                self.overrides.apply(&mut settings);
                self.settings = settings;
                self.settings_error = None;
                ctx.set_theme(egui_theme(self.settings.theme));
                self.apply_terminal_appearance(system_dark_mode);
            }
            Err(error) => self.settings_error = Some(error),
        }
    }

    fn apply_terminal_appearance(&mut self, system_dark_mode: bool) {
        let system_theme = if system_dark_mode {
            AppearanceTheme::Dark
        } else {
            AppearanceTheme::Light
        };
        let colors = appearance::appearance_ansi_sequences_for_system(&self.settings, system_theme);
        if let Err(error) = self.terminal_output.send(colors.into_bytes()) {
            self.settings_error = Some(format!("Impossible d’appliquer la palette : {error}"));
        }
    }
}

impl eframe::App for NativeTerminalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let system_dark_mode = ui.visuals().dark_mode;
        self.refresh_settings(ui.ctx(), system_dark_mode);
        if self.last_system_dark_mode != Some(system_dark_mode) {
            self.last_system_dark_mode = Some(system_dark_mode);
            self.apply_terminal_appearance(system_dark_mode);
        }
        let visuals = ui.visuals().clone();
        egui::containers::Panel::top("terminal_header")
            .frame(
                egui::Frame::new()
                    .fill(visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(18, 12))
                    .stroke(egui::Stroke::new(
                        1.0,
                        visuals.widgets.noninteractive.bg_stroke.color,
                    )),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Terminal Rust");
                    ui.separator();
                    ui.label("Session shell PTY");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small(theme_label(self.settings.theme));
                    });
                });
                if let Some(error) = &self.clipboard_error {
                    ui.colored_label(visuals.error_fg_color, error);
                }
                if let Some(error) = &self.settings_error {
                    ui.colored_label(visuals.error_fg_color, format!("Réglages : {error}"));
                }
            });

        egui::Frame::new()
            .fill(visuals.extreme_bg_color)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                let mut style = TerminalStyle::from_visuals(ui.visuals());
                style.scheme = ColorScheme::Dark;
                style.font.size = 14.0;
                style.padding = 12.0;
                let response = self.terminal.ui(ui, &style);
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

fn spawn_shell(overrides: &LaunchOverrides, system_theme: &str) -> NativeResult<TerminalStream> {
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
    command.env("TERMINAL_RUST_NATIVE_UI", "1");
    command.env("TERMINAL_RUST_APPEARANCE_SYSTEM_RESOLVED", system_theme);
    overrides.apply_to_command(&mut command);

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
    let terminal_output = sender.clone();
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

    Ok((
        TtyStream {
            output,
            tty: Arc::new(PtyConnection {
                writer: Mutex::new(writer),
                master: Mutex::new(pair.master),
                killer: Mutex::new(killer),
                exited,
            }),
        },
        terminal_output,
    ))
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

fn run_native_app(overrides: LaunchOverrides) -> eframe::Result {
    eframe::run_native(
        "Terminal Rust — prototype natif",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 640.0])
                .with_min_inner_size([560.0, 400.0]),
            ..Default::default()
        },
        Box::new(move |context| {
            let system_dark_mode = context
                .egui_ctx
                .system_theme()
                .is_none_or(|theme| theme == egui::Theme::Dark);
            let system_theme = if system_dark_mode { "dark" } else { "light" };
            NativeTerminalApp::new(overrides, system_theme, system_dark_mode).map(|app| {
                context.egui_ctx.set_theme(egui_theme(app.settings.theme));
                Box::new(app) as Box<dyn eframe::App>
            })
        }),
    )
}

#[derive(Default, Clone)]
struct LaunchOverrides {
    theme: Option<AppearanceTheme>,
    foreground: Option<String>,
    background: Option<String>,
    palette: BTreeMap<u8, String>,
}

impl LaunchOverrides {
    fn apply(&self, settings: &mut AppearanceSettings) {
        if let Some(theme) = self.theme {
            settings.theme = theme;
        }
        if let Some(foreground) = &self.foreground {
            settings.foreground = Some(foreground.clone());
        }
        if let Some(background) = &self.background {
            settings.background = Some(background.clone());
        }
        settings.palette.extend(self.palette.clone());
    }

    fn apply_to_command(&self, command: &mut CommandBuilder) {
        if let Some(theme) = self.theme {
            command.env("TERMINAL_RUST_APPEARANCE_THEME", theme_label(theme));
        }
        if let Some(foreground) = &self.foreground {
            command.env("TERMINAL_RUST_APPEARANCE_FOREGROUND", foreground);
        }
        if let Some(background) = &self.background {
            command.env("TERMINAL_RUST_APPEARANCE_BACKGROUND", background);
        }
        for (index, color) in &self.palette {
            command.env(format!("TERMINAL_RUST_APPEARANCE_COLOR_{index}"), color);
        }
    }
}

fn parse_launch_options(arguments: &[String]) -> Result<LaunchOverrides, String> {
    let mut options = LaunchOverrides::default();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--theme" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                options.theme = Some(parse_theme(value)?);
            }
            "--foreground" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                appearance::validate_hex_color(value)?;
                options.foreground = Some(value.clone());
            }
            "--background" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                appearance::validate_hex_color(value)?;
                options.background = Some(value.clone());
            }
            "--color" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                let (index, color) = value
                    .split_once('=')
                    .ok_or_else(|| "expected --color INDEX=#RRGGBB".to_string())?;
                let index = index
                    .parse::<u8>()
                    .map_err(|_| "ANSI color index must be between 0 and 15".to_string())?;
                if index > 15 {
                    return Err("ANSI color index must be between 0 and 15".into());
                }
                appearance::validate_hex_color(color)?;
                options.palette.insert(index, color.to_string());
            }
            _ => return Err(format!("unknown option '{argument}'\n{USAGE}")),
        }
    }
    Ok(options)
}

fn parse_theme(value: &str) -> Result<AppearanceTheme, String> {
    match value {
        "system" => Ok(AppearanceTheme::System),
        "dark" => Ok(AppearanceTheme::Dark),
        "light" => Ok(AppearanceTheme::Light),
        _ => Err(format!(
            "invalid theme '{value}'; expected system, dark, or light"
        )),
    }
}

fn egui_theme(theme: AppearanceTheme) -> egui::ThemePreference {
    match theme {
        AppearanceTheme::System => egui::ThemePreference::System,
        AppearanceTheme::Dark => egui::ThemePreference::Dark,
        AppearanceTheme::Light => egui::ThemePreference::Light,
    }
}

fn theme_label(theme: AppearanceTheme) -> &'static str {
    match theme {
        AppearanceTheme::System => "system",
        AppearanceTheme::Dark => "dark",
        AppearanceTheme::Light => "light",
    }
}

fn file_modified(path: Option<&std::path::Path>) -> Option<SystemTime> {
    path.and_then(|path| path.metadata().ok())
        .and_then(|metadata| metadata.modified().ok())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.as_slice() == ["--pty-shell"] {
        return terminal_rust::app::run_interactive();
    }
    if matches!(arguments.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let overrides = match parse_launch_options(&arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match run_native_app(overrides) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cannot start native Terminal Rust prototype: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LaunchOverrides, bracketed_paste, parse_launch_options};
    use terminal_rust::appearance::{AppearanceSettings, AppearanceTheme};

    #[test]
    fn paste_uses_bracketed_mode_and_strips_terminal_control_characters() {
        assert_eq!(
            bracketed_paste("hello\u{1b}[31m\n"),
            b"\x1b[200~hello[31m\n\x1b[201~"
        );
    }

    #[test]
    fn launch_options_validate_and_collect_custom_palette_entries() {
        let options = parse_launch_options(&[
            "--theme".into(),
            "light".into(),
            "--color".into(),
            "1=#FF1020".into(),
        ])
        .expect("valid launch options should parse");
        assert_eq!(options.theme, Some(AppearanceTheme::Light));
        assert_eq!(options.palette.get(&1).map(String::as_str), Some("#FF1020"));
    }

    #[test]
    fn launch_options_reject_invalid_palette_values() {
        assert!(parse_launch_options(&["--color".into(), "16=#FFFFFF".into()]).is_err());
        assert!(parse_launch_options(&["--foreground".into(), "red".into()]).is_err());
    }

    #[test]
    fn launch_overrides_do_not_mutate_saved_appearance_settings() {
        let options = LaunchOverrides {
            theme: Some(AppearanceTheme::Dark),
            ..Default::default()
        };
        let saved = AppearanceSettings::default();
        let mut effective = saved.clone();
        options.apply(&mut effective);
        assert_eq!(saved.theme, AppearanceTheme::System);
        assert_eq!(effective.theme, AppearanceTheme::Dark);
    }
}
