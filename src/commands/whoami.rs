use crate::{prompt::context::first_non_empty_env, render};

pub fn whoami() -> std::io::Result<()> {
    let user = first_non_empty_env(&["USER", "USERNAME"]).unwrap_or_else(|| "Unknown".to_string());
    render::stdout_line(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whoami_returns_current_user() {
        let result = whoami();
        assert!(result.is_ok());
    }
}
