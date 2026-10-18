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

use crate::git;
use crate::render;

const CONTENT_WIDTH: usize = 68;
const BAR_WIDTH: usize = 18;
const CYAN: &str = "\u{1b}[36m";
const BOLD: &str = "\u{1b}[1m";
const RESET: &str = "\u{1b}[0m";
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
        let mut frame = render_dashboard(&snapshot, style);
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

#[derive(Clone, Copy)]
struct DashboardStyle {
    color: bool,
    unicode: bool,
}

impl DashboardStyle {
    fn detect() -> Self {
        let terminal = io::stdout().is_terminal() || env::var_os("TERMINAL_RUST_GUI").is_some();
        let dumb_terminal = env::var("TERM").is_ok_and(|term| term == "dumb");

        Self {
            color: terminal && env::var_os("NO_COLOR").is_none() && !dumb_terminal,
            unicode: terminal && !dumb_terminal,
        }
    }
}

fn render_dashboard(snapshot: &SystemSnapshot, style: DashboardStyle) -> String {
    let border = if style.unicode {
        Border::unicode()
    } else {
        Border::ascii()
    };
    let mut output = String::new();

    output.push_str(&border.top("TERMINAL RUST / SYSTEM DASHBOARD"));
    output.push_str(&section("SYSTEM", &border, style.color));
    output.push_str(&row("Host", &snapshot.host, &border));
    output.push_str(&row("OS", &snapshot.os, &border));
    output.push_str(&row("Kernel", &snapshot.kernel, &border));
    output.push_str(&row(
        "Uptime",
        &format_duration(snapshot.uptime_seconds),
        &border,
    ));

    output.push_str(&section("PERFORMANCE", &border, style.color));
    output.push_str(&row("CPU", &snapshot.cpu_name, &border));
    let core_count = match snapshot.physical_cores {
        Some(physical) => format!("{physical} physical / {} logical", snapshot.logical_cores),
        None => format!("{} logical", snapshot.logical_cores),
    };
    output.push_str(&row("Cores", &core_count, &border));
    output.push_str(&meter_row(
        "CPU usage",
        f64::from(snapshot.cpu_usage),
        None,
        &border,
        style.unicode,
    ));
    output.push_str(&meter_row(
        "Memory",
        percentage(snapshot.memory_used, snapshot.memory_total),
        Some(format!(
            "{} / {}",
            format_bytes(snapshot.memory_used),
            format_bytes(snapshot.memory_total)
        )),
        &border,
        style.unicode,
    ));
    if snapshot.swap_total > 0 {
        output.push_str(&meter_row(
            "Swap",
            percentage(snapshot.swap_used, snapshot.swap_total),
            Some(format!(
                "{} / {}",
                format_bytes(snapshot.swap_used),
                format_bytes(snapshot.swap_total)
            )),
            &border,
            style.unicode,
        ));
    }
    output.push_str(&row(
        "Processes",
        &snapshot.process_count.to_string(),
        &border,
    ));

    output.push_str(&section("STORAGE & NETWORK", &border, style.color));
    if let Some(disk) = &snapshot.disk {
        output.push_str(&row(
            "Disk",
            &disk.mount_point.display().to_string(),
            &border,
        ));
        output.push_str(&meter_row(
            "Disk usage",
            percentage(disk.used, disk.total),
            Some(format!(
                "{} / {}",
                format_bytes(disk.used),
                format_bytes(disk.total)
            )),
            &border,
            style.unicode,
        ));
    } else {
        output.push_str(&row("Disk", "unavailable", &border));
    }
    output.push_str(&row(
        "Network in",
        &format!("{} total", format_bytes(snapshot.network_received)),
        &border,
    ));
    output.push_str(&row(
        "Network out",
        &format!("{} total", format_bytes(snapshot.network_transmitted)),
        &border,
    ));

    output.push_str(&section("CONTEXT", &border, style.color));
    output.push_str(&row(
        "Directory",
        &snapshot.current_dir.display().to_string(),
        &border,
    ));
    output.push_str(&row(
        "Git",
        if snapshot.inside_git_repository {
            "repository detected"
        } else {
            "outside repository or Git unavailable"
        },
        &border,
    ));
    output.push_str(&border.bottom());

    output
}

struct Border {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
    left_join: char,
    right_join: char,
}

impl Border {
    const fn unicode() -> Self {
        Self {
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
            horizontal: '─',
            vertical: '│',
            left_join: '├',
            right_join: '┤',
        }
    }

    const fn ascii() -> Self {
        Self {
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
            horizontal: '-',
            vertical: '|',
            left_join: '+',
            right_join: '+',
        }
    }

    fn top(&self, title: &str) -> String {
        let title = format!(" {title} ");
        let remaining = CONTENT_WIDTH.saturating_sub(title.chars().count());
        format!(
            "{}{}{}{}\n",
            self.top_left,
            title,
            self.horizontal.to_string().repeat(remaining),
            self.top_right
        )
    }

    fn bottom(&self) -> String {
        format!(
            "{}{}{}\n",
            self.bottom_left,
            self.horizontal.to_string().repeat(CONTENT_WIDTH),
            self.bottom_right
        )
    }
}

fn section(title: &str, border: &Border, color: bool) -> String {
    let content = format!(" {} ", title);
    let remaining = CONTENT_WIDTH.saturating_sub(content.chars().count());
    let line = format!(
        "{}{}{}{}",
        border.left_join,
        content,
        border.horizontal.to_string().repeat(remaining),
        border.right_join
    );

    if color {
        format!("{BOLD}{CYAN}{line}{RESET}\n")
    } else {
        format!("{line}\n")
    }
}

fn row(label: &str, value: &str, border: &Border) -> String {
    let label_width = 13;
    let value_width = CONTENT_WIDTH.saturating_sub(label_width + 3);
    let value = truncate(value, value_width);

    format!(
        "{} {:<label_width$} {:<value_width$} {}\n",
        border.vertical,
        border_label(label),
        value,
        border.vertical
    )
}

fn meter_row(
    label: &str,
    percent: f64,
    detail: Option<String>,
    border: &Border,
    unicode: bool,
) -> String {
    let percent = percent.clamp(0.0, 100.0);
    let bar = progress_bar(percent, unicode);
    let detail = detail.map_or_else(String::new, |detail| format!("  {detail}"));
    row(label, &format!("{bar} {percent:>5.1}%{detail}"), border)
}

fn border_label(label: &str) -> String {
    format!("{label}:")
}

fn progress_bar(percent: f64, unicode: bool) -> String {
    let filled = ((percent.clamp(0.0, 100.0) / 100.0) * BAR_WIDTH as f64).round() as usize;
    let (full, empty) = if unicode { ('█', '░') } else { ('#', '.') };

    format!(
        "{}{}",
        full.to_string().repeat(filled.min(BAR_WIDTH)),
        empty.to_string().repeat(BAR_WIDTH.saturating_sub(filled))
    )
}

fn percentage(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 * 100.0 / total as f64
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let mut result: String = value.chars().take(max_chars - 3).collect();
    result.push_str("...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

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
        assert_eq!(progress_bar(-1.0, false), ".".repeat(BAR_WIDTH));
        assert_eq!(progress_bar(101.0, false), "#".repeat(BAR_WIDTH));
    }

    #[test]
    fn truncates_long_values_to_dashboard_width() {
        let value = "a".repeat(80);
        let truncated = truncate(&value, 10);

        assert_eq!(truncated, "aaaaaaa...");
        assert_eq!(truncated.chars().count(), 10);
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
            },
        );

        let expected_width = output.lines().next().unwrap().chars().count();

        for line in output.lines() {
            assert_eq!(
                line.chars().count(),
                expected_width,
                "misaligned line: {line}"
            );
        }
    }

    #[test]
    fn raw_mode_frame_returns_to_line_start() {
        assert_eq!(raw_mode_frame("a\nb\n"), "a\r\nb\r\n");
    }
}
