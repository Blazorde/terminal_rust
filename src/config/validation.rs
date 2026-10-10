use super::schema::AppConfig;

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
    if config.prompt.segments.is_empty() {
        return Err("prompt.segments must contain at least one segment".to_string());
    }
    for segment in &config.prompt.segments {
        if !matches!(segment.as_str(), "path" | "git" | "symbol") {
            return Err(format!(
                "prompt.segments contains unknown segment '{segment}'"
            ));
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
