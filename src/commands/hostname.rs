use crate::{prompt::context::first_non_empty_env, render};

pub fn hostname() -> std::io::Result<()> {
    render::stdout_line(current_hostname().unwrap_or_else(|| "unknown".to_string()))
}

pub(crate) fn current_hostname() -> Option<String> {
    sysinfo::System::host_name()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| first_non_empty_env(&["HOSTNAME", "COMPUTERNAME"]))
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
