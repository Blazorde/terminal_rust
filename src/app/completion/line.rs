//! Reading the line before the cursor: where the word being typed starts and
//! what it is for. Nothing here looks at the disk or the environment.

/// Words after which a command still comes.
const KEYWORDS: [&str; 9] = [
    "!", "{", "if", "then", "elif", "else", "while", "until", "do",
];

/// Commands that run the command given to them, each with its options that take
/// a value: `sudo -u root <command>`.
const WRAPPERS: [(&str, &[&str]); 7] = [
    ("command", &[]),
    ("env", &["-C", "-u"]),
    ("exec", &["-a"]),
    ("nohup", &[]),
    (
        "sudo",
        &[
            "-C", "-D", "-g", "-h", "-p", "-R", "-r", "-T", "-t", "-U", "-u",
        ],
    ),
    ("time", &[]),
    ("xargs", &["-a", "-d", "-E", "-I", "-L", "-n", "-P", "-s"]),
];

/// What the word before the cursor is.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct LineContext<'a> {
    /// Byte index where the word starts.
    pub(super) start: usize,
    /// The word as typed, quotes and escapes included.
    pub(super) word: &'a str,
    /// Whether a command name goes here.
    pub(super) command_position: bool,
    /// Whether the word is the file of a redirection.
    pub(super) redirect: bool,
    /// The words of the current command before this one, its name first, without
    /// the keywords and wrappers that precede it.
    pub(super) command: Vec<&'a str>,
}

#[derive(Default)]
struct Command<'a> {
    words: Vec<&'a str>,
    /// Where the word being read starts.
    start: Option<usize>,
    /// Whether that word, or the next one, is the file of a redirection.
    redirect: bool,
}

#[derive(Eq, PartialEq)]
enum Opener {
    Parenthesis,
    Substitution,
    Backquote,
}

/// The command and quote left aside while a nested list is read.
struct Frame<'a> {
    opener: Opener,
    command: Command<'a>,
    quote: Option<char>,
}

#[derive(Default)]
struct Reader<'a> {
    text: &'a str,
    command: Command<'a>,
    quote: Option<char>,
    frames: Vec<Frame<'a>>,
}

impl<'a> Reader<'a> {
    fn start_word(&mut self, at: usize) {
        self.command.start.get_or_insert(at);
    }

    fn end_word(&mut self, end: usize) {
        let Some(start) = self.command.start.take() else {
            return;
        };
        if !std::mem::take(&mut self.command.redirect) {
            self.command.words.push(&self.text[start..end]);
        }
    }

    fn open(&mut self, opener: Opener) {
        self.frames.push(Frame {
            opener,
            command: std::mem::take(&mut self.command),
            quote: self.quote.take(),
        });
    }

    fn close(&mut self) -> Option<Opener> {
        let frame = self.frames.pop()?;
        self.command = frame.command;
        self.quote = frame.quote;
        Some(frame.opener)
    }

    fn backquote(&mut self, at: usize) {
        if self
            .frames
            .last()
            .is_some_and(|frame| frame.opener == Opener::Backquote)
        {
            self.close();
        } else {
            self.start_word(at);
            self.open(Opener::Backquote);
        }
    }

    fn closing_parenthesis(&mut self, at: usize) {
        self.end_word(at);
        let nested = self
            .frames
            .last()
            .is_some_and(|frame| frame.opener != Opener::Backquote);
        let empty = self.command.words.is_empty();
        match nested.then(|| self.close()).flatten() {
            // `name()`: the body of a function comes next.
            Some(Opener::Parenthesis) if empty => self.command = Command::default(),
            // After a subshell only redirections follow.
            Some(Opener::Parenthesis) => self.command.words.push(")"),
            Some(_) => {}
            // The pattern of a `case` arm: its commands come next.
            None => self.command = Command::default(),
        }
    }

    fn read(&mut self) {
        let mut characters = self.text.char_indices().peekable();
        while let Some((at, character)) = characters.next() {
            if self.quote == Some('\'') {
                if character == '\'' {
                    self.quote = None;
                }
                continue;
            }
            let after_redirection = self.text[..at].ends_with(['<', '>']);
            match character {
                '\\' => {
                    self.start_word(at);
                    characters.next();
                }
                '$' if characters.peek().is_some_and(|(_, next)| *next == '(') => {
                    characters.next();
                    self.start_word(at);
                    self.open(Opener::Substitution);
                }
                '`' => self.backquote(at),
                '"' if self.quote == Some('"') => self.quote = None,
                _ if self.quote.is_some() => {}
                '"' | '\'' => {
                    self.start_word(at);
                    self.quote = Some(character);
                }
                '(' => {
                    self.end_word(at);
                    self.open(Opener::Parenthesis);
                }
                ')' => self.closing_parenthesis(at),
                '<' | '>' => {
                    // In `2>file` the number is the descriptor, not a word.
                    let descriptor = self.command.start.is_some_and(|start| {
                        self.text[start..at].bytes().all(|b| b.is_ascii_digit())
                    });
                    if descriptor {
                        self.command.start = None;
                    }
                    self.end_word(at);
                    self.command.redirect = true;
                }
                // `>&2`, `>|file` and `&>file` are redirections.
                '&' | '|' if after_redirection => {}
                '&' if characters.peek().is_some_and(|(_, next)| *next == '>') => {
                    self.end_word(at);
                }
                ';' | '|' | '&' | '\n' => {
                    self.end_word(at);
                    self.command = Command::default();
                }
                _ if character.is_whitespace() => self.end_word(at),
                _ => self.start_word(at),
            }
        }
    }
}

/// How many of `words` are keywords, wrappers and their options. More than there
/// are when the next word is the value of such an option.
fn prefix_length(words: &[&str]) -> usize {
    let mut index = 0;
    while let Some(&word) = words.get(index) {
        if KEYWORDS.contains(&word) {
            index += 1;
            continue;
        }
        let Some((wrapper, valued)) = WRAPPERS.iter().find(|(name, _)| *name == word) else {
            break;
        };
        index += 1;
        while let Some(next) = words.get(index) {
            if next.starts_with('-') {
                index += if valued.contains(next) { 2 } else { 1 };
            } else if *wrapper == "env" && next.contains('=') {
                index += 1;
            } else {
                break;
            }
        }
    }
    index
}

/// Read `before_cursor`, the line up to the cursor.
pub(super) fn analyze(before_cursor: &str) -> LineContext<'_> {
    let mut reader = Reader {
        text: before_cursor,
        ..Reader::default()
    };
    reader.read();
    let Command {
        mut words,
        start,
        redirect,
    } = reader.command;
    let start = start.unwrap_or(before_cursor.len());
    let prefix = prefix_length(&words);
    let command_position = !redirect && prefix == words.len();
    words.drain(..prefix.min(words.len()));
    LineContext {
        start,
        word: &before_cursor[start..],
        command_position,
        redirect,
        command: words,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_command(line: &str) -> bool {
        analyze(line).command_position
    }

    #[test]
    fn a_command_follows_every_separator_and_opening() {
        for line in [
            "",
            "   ",
            "gi",
            "ls | gi",
            "a;gi",
            "a && gi",
            "a || gi",
            "a|gi",
            "a & gi",
            "a\ngi",
            "echo $(gi",
            "echo \"$(gi",
            "echo x$(gi",
            "echo `gi",
            "echo \"`gi",
            "(gi",
            "( cd x; gi",
            "{ gi",
            "f() { gi",
            "if gi",
            "if a; then gi",
            "while gi",
            "until gi",
            "for x in a; do gi",
            "a; else gi",
            "elif gi",
            "! gi",
            "case x in a) gi",
            "x=$(gi",
        ] {
            assert!(is_command(line), "{line:?}");
        }
    }

    #[test]
    fn an_argument_is_not_a_command() {
        for line in [
            "ls ",
            "echo a | cat x ",
            "echo then gi",
            "echo sudo gi",
            "echo 'a; gi",
            "echo \"a | gi",
            "echo a\\; gi",
            "echo '$(gi",
            "echo $(pwd) gi",
            "echo `pwd` gi",
            "(ls) gi",
            "echo {a gi",
            "> gi",
            "ls 2> gi",
            "ls >gi",
            "ls &> gi",
            "ls >| gi",
            "ls 2>&1 gi",
        ] {
            assert!(!is_command(line), "{line:?}");
        }
    }

    #[test]
    fn a_wrapper_is_followed_by_a_command() {
        for line in [
            "sudo gi",
            "env gi",
            "time gi",
            "nohup gi",
            "xargs gi",
            "exec gi",
            "command gi",
            "ls | xargs -0 gi",
            "xargs -n 1 gi",
            "sudo -E gi",
            "sudo -u root gi",
            "env -i A=1 gi",
            "sudo env gi",
            "if sudo gi",
        ] {
            assert!(is_command(line), "{line:?}");
        }
        // The value of an option, then the arguments of the command.
        assert!(!is_command("sudo -u ro"));
        assert!(!is_command("sudo git ch"));
        assert_eq!(analyze("sudo -u root git ch").command, ["git"]);
        assert_eq!(analyze("if ! git ch").command, ["git"]);
    }

    #[test]
    fn the_words_of_the_current_command_are_given() {
        assert_eq!(analyze("a b; git checkout ma").command, ["git", "checkout"]);
        assert_eq!(
            analyze("git > out checkout ma").command,
            ["git", "checkout"]
        );
        assert_eq!(analyze("echo $(git log ").command, ["git", "log"]);
        assert_eq!(analyze("cat 'a b' c").command, ["cat", "'a b'"]);
        assert_eq!(analyze("git $(x) ch").command, ["git", "$(x)"]);
    }

    #[test]
    fn the_word_keeps_its_quotes_and_escapes() {
        for (line, word) in [
            ("ls  fi", "fi"),
            ("ls", "ls"),
            ("", ""),
            ("cd \"a b", "\"a b"),
            ("cd 'a; b", "'a; b"),
            ("cd a\\ b", "a\\ b"),
            ("cd dir/\"a ", "dir/\"a "),
            ("echo $HO", "$HO"),
            ("echo ${HO", "${HO"),
            ("ls >fi", "fi"),
            ("echo $(gi", "gi"),
        ] {
            let context = analyze(line);
            assert_eq!(context.word, word, "{line:?}");
            assert_eq!(context.start, line.len() - word.len(), "{line:?}");
        }
    }

    #[test]
    fn a_word_starts_after_a_multi_byte_whitespace() {
        // U+00A0 (no-break space) and U+3000 (ideographic space) take 2 and 3 bytes.
        assert_eq!(analyze("ls\u{a0}").start, "ls\u{a0}".len());
        assert_eq!(analyze("ls\u{3000}fi").start, "ls\u{3000}".len());
        assert_eq!(analyze("\u{e9}cho x").start, "\u{e9}cho ".len());
    }
}
