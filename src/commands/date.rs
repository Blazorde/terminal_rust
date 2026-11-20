use std::io;

use jiff::Zoned;

use crate::render;

pub fn date(args: &[String]) -> io::Result<()> {
    let format = args
        .iter()
        .find(|arg| arg.starts_with('+'))
        .map(|arg| &arg[1..])
        .unwrap_or("%a %b %e %T %Z %Y");

    let date = Zoned::now().strftime(format).to_string();
    render::stdout_line(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_returns_current_date() {
        let args = vec![];
        let result = date(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn date_accepts_custom_format() {
        let args = vec!["+%Y-%m-%d".to_string()];
        let result = date(&args);
        assert!(result.is_ok());
    }
}
