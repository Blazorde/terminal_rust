use std::io;

use crate::render;

/// `echo [-n] [-e|-E] [text...]`: print the words separated by spaces.
///
/// `-n` leaves out the final newline, `-e` reads backslash escapes (`\n`, `\t`,
/// `\e`, `\xHH`, `\0NNN`, `\uHHHH`, and `\c` to stop there), and `-E` turns them
/// off again. Options can be grouped (`-ne`) and only come first: anything after
/// the first word that is not an option is printed as it is.
pub fn echo(args: &[String]) -> io::Result<()> {
    let (text, newline) = output(args);
    if newline {
        render::stdout_line(text)
    } else {
        render::stdout(text)
    }
}

/// What `echo` prints, and whether a newline follows it.
fn output(args: &[String]) -> (String, bool) {
    let (options, words) = split_options(args);
    let joined = words.join(" ");
    let (text, stopped) = if options.escapes {
        interpret(&joined)
    } else {
        (joined, false)
    };
    (text, options.newline && !stopped)
}

struct Options {
    newline: bool,
    escapes: bool,
}

/// The leading options and the words that follow them.
fn split_options(args: &[String]) -> (Options, &[String]) {
    let mut options = Options {
        newline: true,
        escapes: false,
    };
    let mut consumed = 0;
    for arg in args {
        let letters = match arg.strip_prefix('-') {
            Some(letters)
                if !letters.is_empty() && letters.chars().all(|c| matches!(c, 'n' | 'e' | 'E')) =>
            {
                letters
            }
            _ => break,
        };
        for letter in letters.chars() {
            match letter {
                'n' => options.newline = false,
                'e' => options.escapes = true,
                _ => options.escapes = false,
            }
        }
        consumed += 1;
    }
    (options, &args[consumed..])
}

/// Replace the backslash escapes of `text`. The flag is true when `\c` ended the
/// output, which also drops the final newline.
fn interpret(text: &str) -> (String, bool) {
    let characters: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        index += 1;
        if character != '\\' {
            output.push(character);
            continue;
        }
        let Some(&escape) = characters.get(index) else {
            output.push('\\');
            break;
        };
        index += 1;
        match escape {
            'a' => output.push('\x07'),
            'b' => output.push('\x08'),
            'c' => return (output, true),
            'e' | 'E' => output.push('\x1b'),
            'f' => output.push('\x0c'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            'v' => output.push('\x0b'),
            '\\' => output.push('\\'),
            '0' => {
                let digits = take_digits(&characters, &mut index, 3, 8);
                push_code(
                    &mut output,
                    u32::from_str_radix(&digits, 8).unwrap_or(0),
                    "\\0",
                    &digits,
                );
            }
            'x' => push_numbered(&mut output, &characters, &mut index, 2, "\\x"),
            'u' => push_numbered(&mut output, &characters, &mut index, 4, "\\u"),
            'U' => push_numbered(&mut output, &characters, &mut index, 8, "\\U"),
            other => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    (output, false)
}

/// Read up to `limit` digits of `radix` at `index`.
fn take_digits(characters: &[char], index: &mut usize, limit: usize, radix: u32) -> String {
    let mut digits = String::new();
    while digits.len() < limit
        && let Some(&next) = characters.get(*index)
        && next.is_digit(radix)
    {
        digits.push(next);
        *index += 1;
    }
    digits
}

/// A hexadecimal escape: without any digit it stays as written.
fn push_numbered(
    output: &mut String,
    characters: &[char],
    index: &mut usize,
    limit: usize,
    prefix: &str,
) {
    let digits = take_digits(characters, index, limit, 16);
    if digits.is_empty() {
        output.push_str(prefix);
    } else {
        push_code(
            output,
            u32::from_str_radix(&digits, 16).unwrap_or(0),
            prefix,
            &digits,
        );
    }
}

/// Add the character `code` stands for. A value that is not a character (a
/// surrogate, or beyond Unicode) stays as it was written.
fn push_code(output: &mut String, code: u32, prefix: &str, digits: &str) {
    match char::from_u32(code) {
        Some(character) => output.push(character),
        None => {
            output.push_str(prefix);
            output.push_str(digits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    /// The text printed, whether a newline follows, and whether escapes were read.
    fn rendered(values: &[&str]) -> (String, bool, bool) {
        let list = args(values);
        let (text, newline) = output(&list);
        (text, newline, split_options(&list).0.escapes)
    }

    #[test]
    fn echo_joins_words_with_spaces() {
        assert_eq!(rendered(&["Hello", "World"]).0, "Hello World");
    }

    #[test]
    fn echo_without_arguments_prints_empty_line() {
        assert_eq!(rendered(&[]), (String::new(), true, false));
    }

    #[test]
    fn options_are_read_only_at_the_start_and_can_be_grouped() {
        assert_eq!(rendered(&["-n", "x"]), ("x".to_string(), false, false));
        assert_eq!(rendered(&["-e", "x"]), ("x".to_string(), true, true));
        assert_eq!(rendered(&["-ne", "x"]), ("x".to_string(), false, true));
        assert_eq!(rendered(&["-en", "x"]), ("x".to_string(), false, true));
        assert_eq!(rendered(&["-n", "-e", "x"]), ("x".to_string(), false, true));
        // `-E` turns the escapes off again; the last one wins.
        assert_eq!(rendered(&["-e", "-E", "a\\tb"]).0, "a\\tb");
        assert_eq!(rendered(&["-E", "-e", "a\\tb"]).0, "a\tb");
        // Not options: printed as they are.
        assert_eq!(rendered(&["-x", "y"]).0, "-x y");
        assert_eq!(rendered(&["-"]).0, "-");
        assert_eq!(rendered(&["--"]).0, "--");
        assert_eq!(rendered(&["-nx", "y"]).0, "-nx y");
        assert_eq!(rendered(&["x", "-n"]).0, "x -n");
    }

    #[test]
    fn escapes_are_left_alone_without_the_option() {
        assert_eq!(rendered(&["a\\tb\\n"]).0, "a\\tb\\n");
    }

    #[test]
    fn the_usual_escapes_are_replaced() {
        let (text, _, _) = rendered(&["-e", "a\\tb\\nc\\\\d\\re\\ae\\bf\\vg\\fh"]);
        assert_eq!(text, "a\tb\nc\\d\re\x07e\x08f\x0bg\x0ch");
        assert_eq!(
            rendered(&["-e", "\\e[31mred\\E[0m"]).0,
            "\x1b[31mred\x1b[0m"
        );
    }

    #[test]
    fn numbered_escapes_read_octal_hexadecimal_and_unicode() {
        assert_eq!(
            rendered(&["-e", "\\0101\\x41\\u00e9\\U0001F600"]).0,
            "AAé😀"
        );
        // At most three octal digits, two hexadecimal ones for `\x`, and a value
        // with fewer digits is read as far as it goes.
        assert_eq!(rendered(&["-e", "\\01012"]).0, "A2");
        assert_eq!(rendered(&["-e", "\\x414"]).0, "A4");
        assert_eq!(rendered(&["-e", "\\x4z"]).0, "\x04z");
        assert_eq!(rendered(&["-e", "\\00"]).0, "\0");
    }

    #[test]
    fn a_bad_or_unknown_escape_stays_as_written() {
        assert_eq!(rendered(&["-e", "\\q"]).0, "\\q");
        assert_eq!(rendered(&["-e", "\\xZ"]).0, "\\xZ");
        assert_eq!(rendered(&["-e", "\\uZ"]).0, "\\uZ");
        // A surrogate and a value beyond Unicode are not characters.
        assert_eq!(rendered(&["-e", "\\ud800"]).0, "\\ud800");
        assert_eq!(rendered(&["-e", "\\UFFFFFFFF"]).0, "\\UFFFFFFFF");
        assert_eq!(rendered(&["-e", "end\\"]).0, "end\\");
    }

    #[test]
    fn backslash_c_stops_the_output_and_the_newline() {
        assert_eq!(
            rendered(&["-e", "one\\ctwo", "three"]),
            ("one".to_string(), false, true)
        );
        assert_eq!(rendered(&["-e", "\\c"]), (String::new(), false, true));
    }

    #[test]
    fn random_text_never_panics() {
        const PIECES: [&str; 14] = [
            "\\", "x", "u", "U", "0", "c", "e", "1", "F", "-", " ", "é", "\n", "9",
        ];
        let mut seed = 0x1234_5678_9ABC_DEF1_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..50_000 {
            let length = 1 + (next() % 12) as usize;
            let text: String = (0..length)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect();
            let outcome = std::panic::catch_unwind(|| interpret(&text));
            assert!(outcome.is_ok(), "echo -e panicked on {text:?}");
        }
    }
}
