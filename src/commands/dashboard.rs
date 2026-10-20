use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use sysinfo::{Disks, MINIMUM_CPU_UPDATE_INTERVAL, Networks, System};
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

use crate::git;
use crate::render;

mod view;

use view::{DashboardStyle, render_dashboard, terminal_content_width};

const CONTENT_WIDTH: usize = 68;
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const MIN_LIVE_WIDTH: u16 = (CONTENT_WIDTH + 2) as u16;
const MIN_LIVE_HEIGHT: u16 = 24;

pub fn dashboard() -> io::Result<()> {
    let style = DashboardStyle::detect();
    if should_run_live_dashboard() {
        run_live_dashboard(style)
    } else {
        let snapshot = SystemProbe::new()?.snapshot()?;
        render::stdout(render_dashboard(&snapshot, style))
    }
}

#[derive(Clone, Debug)]
struct SystemSnapshot {
    os: String,
    kernel: String,
    host: String,
    uptime_seconds: u64,
    cpu_name: String,
    cpu_usage: f32,
    logical_cores: usize,
    physical_cores: Option<usize>,
    memory_used: u64,
    memory_total: u64,
    swap_used: u64,
    swap_total: u64,
    process_count: usize,
    disk: Option<DiskSnapshot>,
    network_received: u64,
    network_transmitted: u64,
    current_dir: PathBuf,
    inside_git_repository: bool,
}

#[derive(Clone, Debug)]
struct DiskSnapshot {
    mount_point: PathBuf,
    used: u64,
    total: u64,
}

struct SystemProbe {
    system: System,
    disks: Disks,
    networks: Networks,
}

impl SystemProbe {
    fn new() -> io::Result<Self> {
        let mut system = System::new_all();

        // CPU usage requires two measurements. This is the only explicit wait;
        // later live samples use the dashboard refresh interval as their gap.
        thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();

        Ok(Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        })
    }

    fn snapshot(&mut self) -> io::Result<SystemSnapshot> {
        let current_dir = env::current_dir()?;
        self.system.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
        let (network_received, network_transmitted) =
            self.networks
                .iter()
                .fold((0_u64, 0_u64), |(received, transmitted), (_, data)| {
                    (
                        received.saturating_add(data.total_received()),
                        transmitted.saturating_add(data.total_transmitted()),
                    )
                });

        let cpu_name = self
            .system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim())
            .filter(|name| !name.is_empty())
            .unwrap_or("Unknown CPU")
            .to_string();

        Ok(SystemSnapshot {
            os: System::long_os_version().unwrap_or_else(|| env::consts::OS.to_string()),
            kernel: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
            host: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            uptime_seconds: System::uptime(),
            cpu_name,
            cpu_usage: self.system.global_cpu_usage(),
            logical_cores: self.system.cpus().len(),
            physical_cores: System::physical_core_count(),
            memory_used: self.system.used_memory(),
            memory_total: self.system.total_memory(),
            swap_used: self.system.used_swap(),
            swap_total: self.system.total_swap(),
            process_count: self.system.processes().len(),
            disk: current_disk(&self.disks, &current_dir),
            network_received,
            network_transmitted,
            inside_git_repository: git::is_inside_work_tree(&current_dir),
            current_dir,
        })
    }
}

fn should_run_live_dashboard() -> bool {
    let stdin_terminal = io::stdin().is_terminal();
    let stdout_terminal = io::stdout().is_terminal();
    let desktop_pty = env::var_os("TERMINAL_RUST_GUI").is_some();
    let dumb_terminal = env::var("TERM").is_ok_and(|term| term == "dumb");
    let size = terminal::size().ok();

    live_dashboard_supported(
        stdin_terminal,
        stdout_terminal,
        desktop_pty,
        dumb_terminal,
        size,
    )
}

fn live_dashboard_supported(
    stdin_terminal: bool,
    stdout_terminal: bool,
    desktop_pty: bool,
    dumb_terminal: bool,
    size: Option<(u16, u16)>,
) -> bool {
    let terminal_pair = (stdin_terminal && stdout_terminal) || desktop_pty;
    let enough_space = size
        .map(|(width, height)| width >= MIN_LIVE_WIDTH && height >= MIN_LIVE_HEIGHT)
        .unwrap_or(desktop_pty);

    terminal_pair && !dumb_terminal && enough_space
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardAction {
    Refresh,
    Exit,
}

trait LiveDashboardRuntime {
    fn snapshot(&mut self) -> io::Result<SystemSnapshot>;
    fn draw(&mut self, frame: &str) -> io::Result<()>;
    fn wait_for_action(&mut self, timeout: Duration) -> io::Result<DashboardAction>;
}

struct CrosstermDashboardRuntime {
    probe: SystemProbe,
}

impl CrosstermDashboardRuntime {
    fn new() -> io::Result<Self> {
        Ok(Self {
            probe: SystemProbe::new()?,
        })
    }
}

impl LiveDashboardRuntime for CrosstermDashboardRuntime {
    fn snapshot(&mut self) -> io::Result<SystemSnapshot> {
        self.probe.snapshot()
    }

    fn draw(&mut self, frame: &str) -> io::Result<()> {
        let mut stdout: io::StdoutLock<'_> = io::stdout().lock();
        execute!(
            stdout,
            MoveTo(0, 0),
            Clear(ClearType::All),
            Print(raw_mode_frame(frame))
        )?;
        stdout.flush()
    }

    fn wait_for_action(&mut self, timeout: Duration) -> io::Result<DashboardAction> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || !event::poll(remaining)? {
                return Ok(DashboardAction::Refresh);
            }
            if let Some(action) = dashboard_action(&event::read()?) {
                return Ok(action);
            }
        }
    }
}

fn raw_mode_frame(frame: &str) -> String {
    frame.replace('\n', "\r\n")
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout().lock();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout().lock();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = stdout.flush();
    }
}

fn run_live_dashboard(style: DashboardStyle) -> io::Result<()> {
    let _session = TerminalSession::enter()?;
    let mut runtime = CrosstermDashboardRuntime::new()?;
    run_live_dashboard_with(&mut runtime, style)
}

fn run_live_dashboard_with(
    runtime: &mut impl LiveDashboardRuntime,
    style: DashboardStyle,
) -> io::Result<()> {
    loop {
        let snapshot = runtime.snapshot()?;
        let mut frame_style = style;
        frame_style.content_width =
            terminal_content_width(terminal::size().ok(), style.content_width);
        let mut frame = render_dashboard(&snapshot, frame_style);
        frame.push_str(" Live refresh: 1s  |  q / Esc / Ctrl+C: close\n");
        runtime.draw(&frame)?;

        if runtime.wait_for_action(LIVE_REFRESH_INTERVAL)? == DashboardAction::Exit {
            return Ok(());
        }
    }
}

fn dashboard_action(event: &Event) -> Option<DashboardAction> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => match (key.code, key.modifiers) {
            (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                Some(DashboardAction::Exit)
            }
            (KeyCode::Char('q') | KeyCode::Esc, _) => Some(DashboardAction::Exit),
            _ => None,
        },
        Event::Resize(_, _) => Some(DashboardAction::Refresh),
        _ => None,
    }
}

fn current_disk(disks: &Disks, current_dir: &Path) -> Option<DiskSnapshot> {
    let disk = disks
        .list()
        .iter()
        .filter(|disk| current_dir.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .or_else(|| disks.list().first())?;

    let total = disk.total_space();
    Some(DiskSnapshot {
        mount_point: disk.mount_point().to_path_buf(),
        used: total.saturating_sub(disk.available_space()),
        total,
    })
}

pub use view::format_duration;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use view::{BAR_WIDTH, format_bytes, progress_bar, truncate};

    fn snapshot() -> SystemSnapshot {
        SystemSnapshot {
            os: "Test OS".to_string(),
            kernel: "1.0".to_string(),
            host: "workstation".to_string(),
            uptime_seconds: 90_061,
            cpu_name: "Test CPU".to_string(),
            cpu_usage: 42.0,
            logical_cores: 8,
            physical_cores: Some(4),
            memory_used: 8 * 1024_u64.pow(3),
            memory_total: 16 * 1024_u64.pow(3),
            swap_used: 0,
            swap_total: 0,
            process_count: 123,
            disk: Some(DiskSnapshot {
                mount_point: PathBuf::from("/"),
                used: 50 * 1024_u64.pow(3),
                total: 100 * 1024_u64.pow(3),
            }),
            network_received: 1024,
            network_transmitted: 2048,
            current_dir: PathBuf::from("/project"),
            inside_git_repository: true,
        }
    }

    #[test]
    fn renders_plain_dashboard_with_important_sections() {
        let output = render_dashboard(
            &snapshot(),
            DashboardStyle {
                color: false,
                unicode: false,
                content_width: CONTENT_WIDTH,
            },
        );

        assert!(output.contains("SYSTEM DASHBOARD"));
        assert!(output.contains("PERFORMANCE"));
        assert!(output.contains("CPU usage:"));
        assert!(output.contains("Memory:"));
        assert!(output.contains("Disk usage:"));
        assert!(output.contains("repository detected"));
        assert!(!output.contains("\u{1b}["));
    }

    #[test]
    fn formats_bytes_using_binary_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(2 * 1024_u64.pow(3)), "2.0 GiB");
    }

    #[test]
    fn clamps_progress_bar_percentage() {
        assert_eq!(progress_bar(-1.0, false, BAR_WIDTH), ".".repeat(BAR_WIDTH));
        assert_eq!(progress_bar(101.0, false, BAR_WIDTH), "#".repeat(BAR_WIDTH));
    }

    #[test]
    fn truncates_long_values_to_dashboard_width() {
        let value = "a".repeat(80);
        let truncated = truncate(&value, 10);

        assert_eq!(truncated, "aaaaaaa...");
        assert_eq!(truncated.chars().count(), 10);
        assert_eq!(truncate("東京東京東京", 7), "東京...");
    }

    #[test]
    fn live_dashboard_requires_two_terminals_and_enough_space() {
        assert!(live_dashboard_supported(
            true,
            true,
            false,
            false,
            Some((80, 24))
        ));
        assert!(!live_dashboard_supported(
            false,
            true,
            false,
            false,
            Some((80, 24))
        ));
        assert!(!live_dashboard_supported(
            true,
            false,
            false,
            false,
            Some((80, 24))
        ));
        assert!(!live_dashboard_supported(
            true,
            true,
            false,
            true,
            Some((80, 24))
        ));
        assert!(!live_dashboard_supported(
            true,
            true,
            false,
            false,
            Some((69, 24))
        ));
        assert!(!live_dashboard_supported(
            true,
            true,
            false,
            false,
            Some((80, 23))
        ));
        assert!(!live_dashboard_supported(true, true, false, false, None));
        assert!(live_dashboard_supported(false, false, true, false, None));
        assert!(!live_dashboard_supported(
            false,
            false,
            true,
            false,
            Some((60, 20))
        ));
    }

    #[test]
    fn dashboard_content_width_tracks_available_terminal_columns() {
        assert_eq!(terminal_content_width(Some((69, 24)), CONTENT_WIDTH), 67);
        assert_eq!(
            terminal_content_width(Some((120, 40)), CONTENT_WIDTH),
            CONTENT_WIDTH
        );
        assert_eq!(terminal_content_width(None, CONTENT_WIDTH), CONTENT_WIDTH);
    }

    #[test]
    fn quit_keys_and_resize_map_to_live_actions() {
        let press = |code, modifiers| Event::Key(crossterm::event::KeyEvent::new(code, modifiers));

        assert_eq!(
            dashboard_action(&press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(DashboardAction::Exit)
        );
        assert_eq!(
            dashboard_action(&press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(DashboardAction::Exit)
        );
        assert_eq!(
            dashboard_action(&press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(DashboardAction::Exit)
        );
        assert_eq!(
            dashboard_action(&Event::Resize(100, 30)),
            Some(DashboardAction::Refresh)
        );
        assert_eq!(
            dashboard_action(&press(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
    }

    struct FakeRuntime {
        actions: VecDeque<DashboardAction>,
        frames: Vec<String>,
        snapshot_count: usize,
        timeouts: Vec<Duration>,
    }

    impl LiveDashboardRuntime for FakeRuntime {
        fn snapshot(&mut self) -> io::Result<SystemSnapshot> {
            self.snapshot_count += 1;
            let mut next = snapshot();
            next.cpu_usage = self.snapshot_count as f32;
            Ok(next)
        }

        fn draw(&mut self, frame: &str) -> io::Result<()> {
            self.frames.push(frame.to_string());
            Ok(())
        }

        fn wait_for_action(&mut self, timeout: Duration) -> io::Result<DashboardAction> {
            self.timeouts.push(timeout);
            Ok(self.actions.pop_front().unwrap_or(DashboardAction::Exit))
        }
    }

    #[test]
    fn live_dashboard_refreshes_deterministically_until_exit() {
        let mut runtime = FakeRuntime {
            actions: VecDeque::from([DashboardAction::Refresh, DashboardAction::Exit]),
            frames: Vec::new(),
            snapshot_count: 0,
            timeouts: Vec::new(),
        };

        run_live_dashboard_with(
            &mut runtime,
            DashboardStyle {
                color: false,
                unicode: false,
                content_width: CONTENT_WIDTH,
            },
        )
        .expect("fake live dashboard should exit cleanly");

        assert_eq!(runtime.snapshot_count, 2);
        assert_eq!(runtime.frames.len(), 2);
        assert!(
            runtime
                .frames
                .iter()
                .all(|frame| frame.contains("q / Esc / Ctrl+C"))
        );
        assert_ne!(runtime.frames[0], runtime.frames[1]);
        assert_eq!(runtime.timeouts, vec![LIVE_REFRESH_INTERVAL; 2]);
    }

    #[test]
    fn dashboard_lines_share_the_same_width() {
        let output = render_dashboard(
            &snapshot(),
            DashboardStyle {
                color: false,
                unicode: false,
                content_width: CONTENT_WIDTH,
            },
        );

        let expected_width = UnicodeWidthStr::width(output.lines().next().unwrap());

        for line in output.lines() {
            assert_eq!(
                UnicodeWidthStr::width(line),
                expected_width,
                "misaligned line: {line}"
            );
        }
    }

    #[test]
    fn dashboard_fits_narrow_terminals_and_unicode_paths() {
        let mut data = snapshot();
        data.current_dir = PathBuf::from("/projets/東京");
        for content_width in [38, 18] {
            let output = render_dashboard(
                &data,
                DashboardStyle {
                    color: false,
                    unicode: true,
                    content_width,
                },
            );

            for line in output.lines() {
                assert_eq!(
                    UnicodeWidthStr::width(line),
                    content_width + 2,
                    "line exceeds narrow terminal width: {line}"
                );
            }
            if content_width == 18 {
                assert!(output.contains("42.0%"));
            }
        }
    }

    #[test]
    fn raw_mode_frame_returns_to_line_start() {
        assert_eq!(raw_mode_frame("a\nb\n"), "a\r\nb\r\n");
    }
}
