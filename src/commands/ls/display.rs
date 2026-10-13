use std::fs::Metadata;
use std::io::Write;
use std::path::Path;

use crate::commands::ls::item::LsItem;
use crate::commands::ls::options::LsOptions;
use crate::errors::ls::LsError;
use crate::render::ansi::{self, AnsiColor, AnsiStyle};

pub fn print_target_header(
    path: &Path,
    index: usize,
    target_count: usize,
    writer: &mut impl Write,
) -> Result<(), LsError> {
    if target_count <= 1 {
        return Ok(());
    }

    if index > 0 {
        writeln!(writer).map_err(LsError::Output)?;
    }

    writeln!(writer, "{}:", path.display()).map_err(LsError::Output)
}

pub fn print_recursive_header(path: &Path, writer: &mut impl Write) -> Result<(), LsError> {
    writeln!(writer).map_err(LsError::Output)?;
    writeln!(writer, "{}:", path.display()).map_err(LsError::Output)
}

pub fn print_item(
    item: &LsItem,
    options: &LsOptions,
    color: bool,
    writer: &mut impl Write,
) -> Result<(), LsError> {
    let indicator = type_indicator(item, options);
    let label = format!("{}{}", item.name, indicator);
    let label = ansi::paint(&label, item_style(item), color);

    if options.long_format {
        write!(
            writer,
            "{} {:>10} {:>12} {}",
            file_kind(&item.metadata),
            permissions(&item.metadata),
            format_size(item.metadata.len(), options.human_readable),
            label
        )
        .map_err(LsError::Output)?;
        writeln!(writer).map_err(LsError::Output)
    } else {
        writeln!(writer, "{label}").map_err(LsError::Output)
    }
}

pub fn print_items(
    items: &[LsItem],
    options: &LsOptions,
    terminal_width: Option<usize>,
    color: bool,
    writer: &mut impl Write,
) -> Result<(), LsError> {
    if options.long_format || options.one_per_line || terminal_width.is_none() {
        for item in items {
            print_item(item, options, color, writer)?;
        }
        return Ok(());
    }

    let labels = items
        .iter()
        .map(|item| {
            let plain = format!("{}{}", item.name, type_indicator(item, options));
            let styled = ansi::paint(&plain, item_style(item), color);
            (plain, styled)
        })
        .collect::<Vec<_>>();
    print_columns(&labels, terminal_width.unwrap_or(80), writer)
}

fn print_columns(
    labels: &[(String, String)],
    terminal_width: usize,
    writer: &mut impl Write,
) -> Result<(), LsError> {
    let mut line_width = 0;
    for (plain, styled) in labels {
        let label_width = plain.chars().count();
        if line_width > 0 && line_width + 3 + label_width > terminal_width {
            writeln!(writer).map_err(LsError::Output)?;
            line_width = 0;
        }

        if line_width > 0 {
            write!(writer, "   ").map_err(LsError::Output)?;
            line_width += 3;
        }
        write!(writer, "{styled}").map_err(LsError::Output)?;
        line_width += label_width;
    }
    if line_width > 0 {
        writeln!(writer).map_err(LsError::Output)?;
    }

    Ok(())
}

fn item_style(item: &LsItem) -> AnsiStyle {
    if item.metadata.file_type().is_symlink() {
        return AnsiStyle::foreground(AnsiColor::Cyan).bold();
    }
    if item.metadata.is_dir() {
        return if matches!(item.name.as_str(), "src" | "tests" | "docs" | ".git") {
            AnsiStyle::foreground(AnsiColor::Cyan).bold()
        } else if matches!(item.name.as_str(), "target" | "node_modules" | ".cache") {
            AnsiStyle::foreground(AnsiColor::Blue).dim()
        } else {
            AnsiStyle::foreground(AnsiColor::Blue).bold()
        };
    }

    let lowercase = item.name.to_ascii_lowercase();
    if matches!(
        lowercase.as_str(),
        "cargo.toml"
            | "cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "go.sum"
    ) {
        return AnsiStyle::foreground(AnsiColor::Yellow).bold();
    }

    let extension = item
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "exe" | "cmd" | "bat" | "ps1" | "sh" => AnsiStyle::foreground(AnsiColor::Green).bold(),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "cpp" | "h" => {
            AnsiStyle::foreground(AnsiColor::Cyan)
        }
        "zip" | "gz" | "tar" | "7z" | "rar" => AnsiStyle::foreground(AnsiColor::Magenta),
        "toml" | "json" | "yaml" | "yml" => AnsiStyle::foreground(AnsiColor::Yellow),
        _ => AnsiStyle::default(),
    }
}

fn type_indicator(item: &LsItem, options: &LsOptions) -> &'static str {
    if !options.show_type_indicator && !options.slash_directories {
        return "";
    }

    if item.metadata.is_dir() {
        "/"
    } else if options.show_type_indicator && item.metadata.file_type().is_symlink() {
        "@"
    } else {
        ""
    }
}

fn file_kind(metadata: &Metadata) -> &'static str {
    if metadata.is_dir() {
        "d"
    } else if metadata.is_file() {
        "-"
    } else {
        "?"
    }
}

fn permissions(metadata: &Metadata) -> &'static str {
    if metadata.permissions().readonly() {
        "readonly"
    } else {
        "writable"
    }
}

fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }

    let units = ["B", "K", "M", "G", "T"];
    let mut value = size as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{size}B")
    } else {
        format!("{value:.1}{}", units[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::print_columns;

    #[test]
    fn columns_use_the_available_width_without_trailing_padding() {
        let labels = ["alpha", "b", "charlie", "delta"].map(|label| (label.into(), label.into()));
        let mut output = Vec::new();

        print_columns(&labels, 15, &mut output).expect("column output should render");

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "alpha   b\ncharlie   delta\n"
        );
    }
}
