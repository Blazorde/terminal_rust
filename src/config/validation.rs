use super::schema::AppConfig;
use std::collections::HashSet;

pub fn validate(config: &AppConfig) -> Result<(), String> {
    if config.version != 1 {
        return Err(format!(
            "unsupported configuration version {} (expected 1)",
            config.version
        ));
    }
    if config.theme.trim().is_empty() {
        return Err("theme must not be empty".to_string());
    }
    if config.prompt.segments.is_empty() && config.prompt.right_segments.is_empty() {
        return Err("prompt layout must contain at least one segment".to_string());
    }
    let mut seen = HashSet::new();
    for (property, segments) in [
        ("prompt.segments", &config.prompt.segments),
        ("prompt.right_segments", &config.prompt.right_segments),
    ] {
        for segment in segments {
            if !matches!(
                segment.as_str(),
                "user"
                    | "host"
                    | "path"
                    | "git"
                    | "status"
                    | "runtime"
                    | "duration"
                    | "time"
                    | "symbol"
            ) {
                return Err(format!("{property} contains unknown segment '{segment}'"));
            }
            if !seen.insert(segment) {
                return Err(format!(
                    "prompt layout contains duplicate segment '{segment}'"
                ));
            }
        }
    }
    if config.prompt.separator.chars().any(char::is_control) {
        return Err("prompt.separator must not contain control characters".to_string());
    }
    if config.segments.path.style != "full" && config.segments.path.style != "folder" {
        return Err("segments.path.style must be 'full' or 'folder'".to_string());
    }
    if config.segments.symbol.value.is_empty()
        || config.segments.symbol.value.chars().any(char::is_control)
    {
        return Err("segments.symbol.value must be visible text".to_string());
    }

    let _ = config.segments.git.show_status;
    let _ = config.prompt.newline;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_supported_segments_across_both_positions() {
        let mut config = AppConfig::default();
        config.prompt.segments = vec![
            "user".into(),
            "host".into(),
            "path".into(),
            "git".into(),
            "symbol".into(),
        ];
        config.prompt.right_segments = vec![
            "status".into(),
            "runtime".into(),
            "duration".into(),
            "time".into(),
        ];

        assert_eq!(validate(&config), Ok(()));
    }

    #[test]
    fn rejects_duplicates_across_positions() {
        let mut config = AppConfig::default();
        config.prompt.right_segments.push("path".into());

        assert_eq!(
            validate(&config),
            Err("prompt layout contains duplicate segment 'path'".into())
        );
    }
}
