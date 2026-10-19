use std::env;
use std::io::{self, IsTerminal};

use crossterm::terminal;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{CONTENT_WIDTH, SystemSnapshot};

pub(super) const BAR_WIDTH: usize = 18;
const CYAN: &str = "\u{1b}[36m";
const BOLD: &str = "\u{1b}[1m";
const RESET: &str = "\u{1b}[0m";

#[derive(Clone, Copy)]
pub(super) struct DashboardStyle {
    pub(super) color: bool,
    pub(super) unicode: bool,
    pub(super) content_width: usize,
}

impl DashboardStyle {
    pub(super) fn detect() -> Self {
        let terminal = io::stdout().is_terminal() || env::var_os("TERMINAL_RUST_GUI").is_some();
        let dumb_terminal = env::var("TERM").is_ok_and(|term| term == "dumb");

        Self {
            color: terminal && env::var_os("NO_COLOR").is_none() && !dumb_terminal,
            unicode: terminal && !dumb_terminal,
            content_width: if terminal {
                terminal_content_width(terminal::size().ok(), CONTENT_WIDTH)
            } else {
                CONTENT_WIDTH
            },
        }
    }
}

pub(super) fn terminal_content_width(size: Option<(u16, u16)>, fallback: usize) -> usize {
    size.map(|(width, _)| usize::from(width).saturating_sub(2).min(CONTENT_WIDTH))
        .unwrap_or(fallback)
        .max(3)
}

pub(super) fn render_dashboard(snapshot: &SystemSnapshot, style: DashboardStyle) -> String {
    let border = if style.unicode {
        Border::unicode(style.content_width)
    } else {
        Border::ascii(style.content_width)
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
    content_width: usize,
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
    const fn unicode(content_width: usize) -> Self {
        Self {
            content_width,
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

    const fn ascii(content_width: usize) -> Self {
        Self {
            content_width,
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
        let title = truncate(title, self.content_width.saturating_sub(2));
        let title = format!(" {title} ");
        let remaining = self
            .content_width
            .saturating_sub(UnicodeWidthStr::width(title.as_str()));
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
            self.horizontal.to_string().repeat(self.content_width),
            self.bottom_right
        )
    }
}

fn section(title: &str, border: &Border, color: bool) -> String {
    let title = truncate(title, border.content_width.saturating_sub(2));
    let content = format!(" {} ", title);
    let remaining = border
        .content_width
        .saturating_sub(UnicodeWidthStr::width(content.as_str()));
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
    let label_width = 13.min(border.content_width.saturating_sub(9));
    let value_width = border.content_width.saturating_sub(label_width + 3);
    let label = truncate(&format!("{label}:"), label_width);
    let value = truncate(value, value_width);
    let value_padding =
        " ".repeat(value_width.saturating_sub(UnicodeWidthStr::width(value.as_str())));

    format!(
        "{} {:<label_width$} {}{} {}\n",
        border.vertical, label, value, value_padding, border.vertical
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
    let label_width = 13.min(border.content_width.saturating_sub(9));
    let value_width = border.content_width.saturating_sub(label_width + 3);
    let bar_width = value_width.saturating_sub(7).min(BAR_WIDTH);
    let bar = progress_bar(percent, unicode, bar_width);
    let detail = detail.map_or_else(String::new, |detail| format!("  {detail}"));
    let value = if bar_width == 0 {
        format!("{percent:>5.1}%{detail}")
    } else {
        format!("{bar} {percent:>5.1}%{detail}")
    };
    row(label, &value, border)
}

pub(super) fn progress_bar(percent: f64, unicode: bool, width: usize) -> String {
    let filled = ((percent.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    let (full, empty) = if unicode { ('█', '░') } else { ('#', '.') };

    format!(
        "{}{}",
        full.to_string().repeat(filled.min(width)),
        empty.to_string().repeat(width.saturating_sub(filled))
    )
}

fn percentage(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 * 100.0 / total as f64
    }
}

pub(super) fn format_bytes(bytes: u64) -> String {
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

pub fn format_duration(seconds: u64) -> String {
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

pub(super) fn truncate(value: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(value) <= max_width {
        return value.to_string();
    }

    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let mut result = String::new();
    let mut width = 0;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width > max_width - 3 {
            break;
        }
        result.push(character);
        width += character_width;
    }
    result.push_str("...");
    result
}
