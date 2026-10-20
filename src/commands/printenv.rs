use std::env;
use std::io;

use crate::render;

pub fn printenv(args: &[String]) -> io::Result<()> {
    if args.is_empty() {
        return render::stdout(format_all(env::vars()));
    }

    let (output, missing) = format_selected(args, |name| env::var(name).ok());
    render::stdout(output)?;

    if missing.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!("not set: {}", missing.join(", "))))
    }
}

fn format_selected(
    names: &[String],
    lookup: impl Fn(&str) -> Option<String>,
) -> (String, Vec<&str>) {
    let mut output = String::new();
    let mut missing = Vec::new();

    for name in names {
        match lookup(name) {
            Some(value) => output.push_str(&format!("{value}\n")),
            None => missing.push(name.as_str()),
        }
    }

    (output, missing)
}

fn format_all(vars: impl Iterator<Item = (String, String)>) -> String {
    vars.map(|(key, value)| format!("{key}={value}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_lookup(name: &str) -> Option<String> {
        match name {
            "HOME" => Some("/home/test".to_string()),
            "USER" => Some("test".to_string()),
            _ => None,
        }
    }

    #[test]
    fn format_all_prints_key_value_lines() {
        let vars = vec![
            ("HOME".to_string(), "/home/test".to_string()),
            ("USER".to_string(), "test".to_string()),
        ];

        assert_eq!(format_all(vars.into_iter()), "HOME=/home/test\nUSER=test\n");
    }

    #[test]
    fn format_selected_prints_values_only() {
        let names = vec!["HOME".to_string(), "USER".to_string()];
        let (output, missing) = format_selected(&names, fake_lookup);

        assert_eq!(output, "/home/test\ntest\n");
        assert!(missing.is_empty());
    }

    #[test]
    fn format_selected_reports_missing_variables() {
        let names = vec!["HOME".to_string(), "NOPE".to_string()];
        let (output, missing) = format_selected(&names, fake_lookup);

        assert_eq!(output, "/home/test\n");
        assert_eq!(missing, vec!["NOPE"]);
    }
}
