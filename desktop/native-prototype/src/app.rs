use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui;
use egui_tty::{ColorScheme, Terminal, TerminalStyle};
use terminal_rust::appearance::{
    self, AppearanceSettings, AppearanceTheme, appearance_settings_path,
};

use crate::NativeResult;
use crate::launch::{LaunchOverrides, theme_label, theme_preference};
use crate::pty::{bracketed_paste, spawn_shell};

const SETTINGS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const MIN_WINDOW_WIDTH: f32 = 800.0;

pub struct NativeTerminalApp {
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
                ctx.set_theme(theme_preference(self.settings.theme));
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
        let (cols, rows) = self.terminal.size();
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
                    ui.separator();
                    ui.small(format!("{cols} × {rows}"));
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

pub fn run(overrides: LaunchOverrides) -> eframe::Result {
    eframe::run_native(
        "Terminal Rust — prototype natif",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 640.0])
                .with_min_inner_size([MIN_WINDOW_WIDTH, 400.0]),
            ..Default::default()
        },
        Box::new(move |context| {
            let system_dark_mode = context
                .egui_ctx
                .system_theme()
                .is_none_or(|theme| theme == egui::Theme::Dark);
            let system_theme = if system_dark_mode { "dark" } else { "light" };
            NativeTerminalApp::new(overrides, system_theme, system_dark_mode).map(|app| {
                context
                    .egui_ctx
                    .set_theme(theme_preference(app.settings.theme));
                Box::new(app) as Box<dyn eframe::App>
            })
        }),
    )
}

fn file_modified(path: Option<&std::path::Path>) -> Option<SystemTime> {
    path.and_then(|path| path.metadata().ok())
        .and_then(|metadata| metadata.modified().ok())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};

    use egui_tty::{Terminal, Tty, TtyStream};

    struct TestTty;

    impl Tty for TestTty {
        fn write(&self, _bytes: &[u8]) -> egui_tty::Result<()> {
            Ok(())
        }

        fn resize(&self, _cols: u16, _rows: u16) -> egui_tty::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn fastfetch_gpu_summary_fits_the_default_eighty_column_grid() {
        let gpu_line = "GPU 2: NVIDIA GeForce RTX 4070 Laptop GPU @ 3.10 GHz (7.76 GiB) [Discrete]";
        assert_eq!(gpu_line.chars().count(), 74);

        let (sender, output) = mpsc::channel();
        sender
            .send(format!("{gpu_line}\r\n").into_bytes())
            .expect("test terminal should receive Fastfetch output");
        let mut terminal = Terminal::new(TtyStream {
            output,
            tty: Arc::new(TestTty),
        })
        .expect("test terminal should initialize");

        terminal.poll();
        let visible_text = terminal
            .visible_text()
            .expect("test terminal should render visible rows");

        assert!(
            visible_text.lines().any(|line| line.trim_end() == gpu_line),
            "the GPU line should stay on one row in an 80-column grid: {visible_text:?}"
        );
        assert_eq!(terminal.size().0, 80);
    }
}
