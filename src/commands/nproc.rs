use sysinfo::{CpuRefreshKind, System};

use crate::render;

pub fn nproc() -> std::io::Result<()> {
    render::stdout_line(logical_cpu_count())
}

fn logical_cpu_count() -> usize {
    let mut system = System::new();
    system.refresh_cpu_list(CpuRefreshKind::nothing());
    system.cpus().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nproc_counts_at_least_one_cpu() {
        assert!(logical_cpu_count() > 0);
    }
}
