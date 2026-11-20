use std::io;

use crate::render;

pub fn echo(args: &[String]) -> io::Result<()> {
    if args.first().map(String::as_str) == Some("-n") {
        render::stdout(echo_line(&args[1..]))
    } else {
        render::stdout_line(echo_line(args))
    }
}

fn echo_line(args: &[String]) -> String {
    args.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_joins_words_with_spaces() {
        let args = vec!["Hello".into(), "World".into()];
        assert_eq!(echo_line(&args), "Hello World");
    }

    #[test]
    fn echo_without_arguments_prints_empty_line() {
        let args = vec![];
        assert_eq!(echo_line(&args), "");
    }
}
