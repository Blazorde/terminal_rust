use std::fs::Metadata;
use std::io::Write;
use std::path::Path;

use crate::commands::ls::item::LsItem;
use crate::commands::ls::options::LsOptions;
use crate::errors::ls::LsError;

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
    writer: &mut impl Write,
) -> Result<(), LsError> {
    let indicator = type_indicator(item, options);

    if options.long_format {
        write!(
            writer,
            "{} {:>10} {:>12} {}{}",
            file_kind(&item.metadata),
            permissions(&item.metadata),
            format_size(item.metadata.len(), options.human_readable),
            item.name,
            indicator
        )
        .map_err(LsError::Output)?;
        writeln!(writer).map_err(LsError::Output)
    } else {
        writeln!(writer, "{}{}", item.name, indicator).map_err(LsError::Output)
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
