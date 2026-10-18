use sysinfo::System;

use crate::{commands::dashboard::format_duration, render};

pub fn uptime() -> std::io::Result<()> {
    let uptime = System::uptime();
    render::stdout_line(format_duration(uptime).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_returns_valid_uptime() {
        let result = uptime();
        assert!(result.is_ok());
    }
}
