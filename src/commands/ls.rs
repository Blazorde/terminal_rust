use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Default)]
struct LsOptions {
    all: bool,
    almost_all: bool,
    directory_itself: bool,
    human_readable: bool,
    long_format: bool,
    slash_directories: bool,
    recursive: bool,
    reverse: bool,
    show_type_indicator: bool,
    sort_mode: SortMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SortMode {
    #[default]
    Name,
    Size,
    Time,
}

#[derive(Debug)]
struct LsItem {
    name: String,
    path: PathBuf,
    metadata: Metadata,
}

pub fn ls(args: &[String]) -> io::Result<()> {
    let (options, targets) = parse_args(args)?;
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    if targets.is_empty() {
        return list_path(Path::new("."), &options, &mut writer);
    }

    for (index, target) in targets.iter().enumerate() {
        if targets.len() > 1 {
            if index > 0 {
                writeln!(writer)?;
            }
            writeln!(writer, "{}:", target.display())?;
        }

        list_path(target, &options, &mut writer)?;
    }

    Ok(())
}

fn parse_args(args: &[String]) -> io::Result<(LsOptions, Vec<PathBuf>)> {
    let mut options = LsOptions::default();
    let mut targets = Vec::new();
    let mut parsing_flags = true;

    for arg in args {
        if parsing_flags && arg == "--" {
            parsing_flags = false;
            continue;
        }

        if parsing_flags && arg.starts_with("--") && arg.len() > 2 {
            apply_long_flag(arg, &mut options)?;
        } else if parsing_flags && arg.starts_with('-') && arg.len() > 1 {
            apply_short_flags(arg, &mut options)?;
        } else {
            targets.push(PathBuf::from(arg));
        }
    }

    Ok((options, targets))
}

fn apply_short_flags(arg: &str, options: &mut LsOptions) -> io::Result<()> {
    for flag in arg.chars().skip(1) {
        match flag {
            '1' => {}
            'a' => options.all = true,
            'A' => options.almost_all = true,
            'd' => options.directory_itself = true,
            'F' => options.show_type_indicator = true,
            'h' => options.human_readable = true,
            'l' => options.long_format = true,
            'p' => options.slash_directories = true,
            'R' => options.recursive = true,
            'r' => options.reverse = true,
            'S' => options.sort_mode = SortMode::Size,
            't' => options.sort_mode = SortMode::Time,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag '-{flag}'"),
                ));
            }
        }
    }

    Ok(())
}

fn apply_long_flag(arg: &str, options: &mut LsOptions) -> io::Result<()> {
    match arg {
        "--all" => options.all = true,
        "--almost-all" => options.almost_all = true,
        "--directory" => options.directory_itself = true,
        "--classify" => options.show_type_indicator = true,
        "--human-readable" => options.human_readable = true,
        "--long" => options.long_format = true,
        "--recursive" => options.recursive = true,
        "--reverse" => options.reverse = true,
        "--indicator-style=slash" => options.slash_directories = true,
        option if option.starts_with("--sort=") => apply_sort_mode(option, options)?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown option '{arg}'"),
            ));
        }
    }

    Ok(())
}

fn apply_sort_mode(arg: &str, options: &mut LsOptions) -> io::Result<()> {
    match arg.strip_prefix("--sort=") {
        Some("name") => options.sort_mode = SortMode::Name,
        Some("size") => options.sort_mode = SortMode::Size,
        Some("time") => options.sort_mode = SortMode::Time,
        Some(sort_mode) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown sort mode '{sort_mode}'"),
            ));
        }
        None => unreachable!("apply_sort_mode is only called with --sort options"),
    }

    Ok(())
}

fn list_path(path: &Path, options: &LsOptions, writer: &mut impl Write) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;

    if options.directory_itself || !metadata.is_dir() {
        let item = LsItem {
            name: file_name(path),
            path: path.to_path_buf(),
            metadata,
        };
        print_item(&item, options, writer)?;
        return Ok(());
    }

    list_directory(path, options, writer)
}

fn list_directory(path: &Path, options: &LsOptions, writer: &mut impl Write) -> io::Result<()> {
    let mut items = read_directory_items(path, options)?;
    sort_items(&mut items, options);

    for item in &items {
        print_item(item, options, writer)?;
    }

    if options.recursive {
        for item in items {
            if item.metadata.is_dir() {
                writeln!(writer)?;
                writeln!(writer, "{}:", item.path.display())?;
                list_directory(&item.path, options, writer)?;
            }
        }
    }

    Ok(())
}

fn read_directory_items(path: &Path, options: &LsOptions) -> io::Result<Vec<LsItem>> {
    let mut items = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if !options.all && !options.almost_all && name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        items.push(LsItem {
            metadata: fs::symlink_metadata(&path)?,
            path,
            name,
        });
    }

    Ok(items)
}

fn sort_items(items: &mut [LsItem], options: &LsOptions) {
    match options.sort_mode {
        SortMode::Name => items.sort_unstable_by(|left, right| left.name.cmp(&right.name)),
        SortMode::Size => items.sort_unstable_by(|left, right| {
            right
                .metadata
                .len()
                .cmp(&left.metadata.len())
                .then_with(|| left.name.cmp(&right.name))
        }),
        SortMode::Time => items.sort_unstable_by(|left, right| {
            modified_timestamp(&right.metadata)
                .cmp(&modified_timestamp(&left.metadata))
                .then_with(|| left.name.cmp(&right.name))
        }),
    }

    if options.reverse {
        items.reverse();
    }
}

fn print_item(item: &LsItem, options: &LsOptions, writer: &mut impl Write) -> io::Result<()> {
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
        )?;
        writeln!(writer)
    } else {
        writeln!(writer, "{}{}", item.name, indicator)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
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

fn modified_timestamp(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_combined_flags() {
        let (options, targets) = parse_args(&args(&["-lah", "src"])).expect("flags should parse");

        assert!(options.long_format);
        assert!(options.all);
        assert!(options.human_readable);
        assert_eq!(targets, vec![PathBuf::from("src")]);
    }

    #[test]
    fn parse_double_dash_stops_flag_parsing() {
        let (_, targets) = parse_args(&args(&["--", "-not-a-flag"])).expect("args should parse");

        assert_eq!(targets, vec![PathBuf::from("-not-a-flag")]);
    }

    #[test]
    fn reject_unknown_flags() {
        let error = parse_args(&args(&["-z"])).expect_err("unknown flags should fail");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_sort_modes() {
        let (options, _) = parse_args(&args(&["--sort=size"])).expect("size sort should parse");
        assert_eq!(options.sort_mode, SortMode::Size);

        let (options, _) = parse_args(&args(&["--sort=time"])).expect("time sort should parse");
        assert_eq!(options.sort_mode, SortMode::Time);

        let (options, _) = parse_args(&args(&["--sort=name"])).expect("name sort should parse");
        assert_eq!(options.sort_mode, SortMode::Name);
    }

    #[test]
    fn reject_unknown_sort_mode() {
        let error =
            parse_args(&args(&["--sort=extension"])).expect_err("unknown sort mode should fail");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
