use sysinfo::{CpuRefreshKind, System};

use crate::render;

pub fn nproc() -> std::io::Result<()> {
    render::stdout_line(logical_cpu_count()?)
}

fn logical_cpu_count() -> std::io::Result<usize> {
    let mut system = System::new().map_err(|error| {
        std::io::Error::other(format!("cannot read the processor count: {error}"))
    })?;
    system.refresh_cpu_list(CpuRefreshKind::nothing());
    Ok(system.cpus().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nproc_counts_at_least_one_cpu() {
        assert!(logical_cpu_count().is_ok_and(|count| count > 0));
    }
}
