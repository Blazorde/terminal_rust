use std::path::PathBuf;

use crate::errors::ls::LsError;

#[derive(Debug, Default)]
pub struct LsOptions {
    pub all: bool,
    pub almost_all: bool,
    pub directory_itself: bool,
    pub human_readable: bool,
    pub long_format: bool,
    pub slash_directories: bool,
    pub recursive: bool,
    pub reverse: bool,
    pub show_type_indicator: bool,
    pub sort_mode: SortMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SortMode {
    #[default]
    Name,
    Size,
    Time,
}

pub fn parse_args(args: &[String]) -> Result<(LsOptions, Vec<PathBuf>), LsError> {
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

fn apply_short_flags(arg: &str, options: &mut LsOptions) -> Result<(), LsError> {
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
            _ => return Err(LsError::UnknownFlag(flag)),
        }
    }

    Ok(())
}

fn apply_long_flag(arg: &str, options: &mut LsOptions) -> Result<(), LsError> {
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
        _ => return Err(LsError::UnknownOption(arg.to_string())),
    }

    Ok(())
}

fn apply_sort_mode(arg: &str, options: &mut LsOptions) -> Result<(), LsError> {
    match arg.strip_prefix("--sort=") {
        Some("name") => options.sort_mode = SortMode::Name,
        Some("size") => options.sort_mode = SortMode::Size,
        Some("time") => options.sort_mode = SortMode::Time,
        Some(sort_mode) => return Err(LsError::UnknownSortMode(sort_mode.to_string())),
        None => unreachable!("apply_sort_mode is only called with --sort options"),
    }

    Ok(())
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

        assert!(matches!(error, LsError::UnknownFlag('z')));
    }

    #[test]
    fn reject_unknown_long_options() {
        let error = parse_args(&args(&["--bad-option"])).expect_err("unknown options should fail");

        assert!(matches!(error, LsError::UnknownOption(option) if option == "--bad-option"));
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

        assert!(matches!(error, LsError::UnknownSortMode(mode) if mode == "extension"));
    }
}
