use crate::render;

pub fn arch() -> std::io::Result<()> {
    render::stdout_line(std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_returns_architecture() {
        let result = arch();
        assert!(result.is_ok());
    }
}
