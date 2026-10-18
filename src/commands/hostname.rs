use crate::{prompt::context::first_non_empty_env, render};

pub fn hostname() -> std::io::Result<()> {
    let hostname = sysinfo::System::host_name()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| first_non_empty_env(&["HOSTNAME"]).unwrap_or_else(|| "unknown".to_string()));
    render::stdout_line(hostname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_returns_current_hostname() {
        let result = hostname();
        assert!(result.is_ok());
    }
}
