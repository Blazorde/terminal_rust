mod display;
mod fs;
mod item;
mod options;
mod sort;

use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;

use crate::errors::ls::LsError;
use crate::render;
use crate::render::ansi;

use self::display::{print_item, print_items};
use self::fs::{metadata, read_directory_items};
use self::item::LsItem;
use self::options::LsOptions;
use self::sort::sort_items;

pub fn ls(args: &[String]) -> Result<(), LsError> {
    let (options, targets) = options::parse_args(args)?;
    let mut writer = Vec::new();
    let interactive = io::stdout().is_terminal() || env::var_os("TERMINAL_RUST_GUI").is_some();
    let terminal_width = interactive
        .then(|| crossterm::terminal::size().map_or(80, |(columns, _)| usize::from(columns)));
    let color = ansi::color_enabled(interactive);

    if targets.is_empty() {
        list_path(Path::new("."), &options, terminal_width, color, &mut writer)?;
    } else {
        for (index, target) in targets.iter().enumerate() {
            display::print_target_header(target, index, targets.len(), &mut writer)?;
            list_path(target, &options, terminal_width, color, &mut writer)?;
        }
    }

    let output = String::from_utf8(writer)
        .map_err(|error| LsError::Output(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    render::stdout(output).map_err(LsError::Output)
}

fn list_path(
    path: &Path,
    options: &LsOptions,
    terminal_width: Option<usize>,
    color: bool,
    writer: &mut impl io::Write,
) -> Result<(), LsError> {
    let metadata = metadata(path)?;

    if options.directory_itself || !metadata.is_dir() {
        let item = LsItem::from_path(path, metadata);
        print_item(&item, options, color, writer)?;
        return Ok(());
    }

    list_directory(path, options, terminal_width, color, writer)
}

fn list_directory(
    path: &Path,
    options: &LsOptions,
    terminal_width: Option<usize>,
    color: bool,
    writer: &mut impl io::Write,
) -> Result<(), LsError> {
    let mut items = read_directory_items(path, options)?;
    sort_items(&mut items, options);

    print_items(&items, options, terminal_width, color, writer)?;

    if options.recursive {
        for item in items {
            if item.metadata.is_dir() {
                display::print_recursive_header(&item.path, writer)?;
                list_directory(&item.path, options, terminal_width, color, writer)?;
            }
        }
    }

    Ok(())
}
