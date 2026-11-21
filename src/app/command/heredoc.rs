//! Here-documents, `<<WORD` (see ADR-0023).
//!
//! `cat <<EOF`, the lines that follow, and the line `EOF` are rewritten before the
//! text is parsed: the operator becomes a here-string `<<< "…"` that holds the
//! body, and the body lines disappear. The rest of the machinery (pipelines,
//! redirections, expansions) only ever sees a here-string.

/// One `<<WORD` found in a line.
struct Marker {
    /// Where the operator starts and ends in the line (bytes), `<<` and the word.
    start: usize,
    end: usize,
    delimiter: String,
    /// The word was quoted: the body is taken literally.
    literal: bool,
    /// `<<-`: leading tabs are removed from the body and the delimiter line.
    strip_tabs: bool,
}

/// Whether `text` holds a here-document whose delimiter line has not come yet: the
/// input must go on over the next line.
pub(super) fn is_open(text: &str) -> bool {
    matches!(scan(text), Err(Problem::Unfinished))
}

/// Rewrite every here-document of `text` as a here-string. `text` is returned as it
/// is when there is none.
pub(super) fn rewrite(text: &str) -> Result<String, String> {
    if !text.contains("<<") {
        return Ok(text.to_string());
    }
    match scan(text) {
        Ok(rewritten) => Ok(rewritten),
        Err(Problem::Unfinished) => {
            Err("here-document: reached the end of the input before its delimiter".to_string())
        }
    }
}

enum Problem {
    Unfinished,
}

/// Go through the lines, and give each here-document the lines up to its delimiter.
fn scan(text: &str) -> Result<String, Problem> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut output: Vec<String> = Vec::with_capacity(lines.len());
    let mut quote: Option<char> = None;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        index += 1;
        let (found, next_quote) = markers(line, quote);
        quote = next_quote;
        if found.is_empty() {
            output.push(line.to_string());
            continue;
        }
        // Each marker takes its body from the lines that follow, in order.
        let mut rewritten = String::new();
        let mut copied = 0;
        for marker in &found {
            let mut body = Vec::new();
            loop {
                let Some(candidate) = lines.get(index) else {
                    return Err(Problem::Unfinished);
                };
                index += 1;
                let candidate = if marker.strip_tabs {
                    candidate.trim_start_matches('\t')
                } else {
                    candidate
                };
                if candidate == marker.delimiter {
                    break;
                }
                body.push(candidate.to_string());
            }
            rewritten.push_str(&line[copied..marker.start]);
            rewritten.push_str("<<< ");
            rewritten.push_str(&quote_body(&body.join("\n"), marker.literal));
            copied = marker.end;
        }
        rewritten.push_str(&line[copied..]);
        output.push(rewritten);
    }
    Ok(output.join("\n"))
}

/// The here-document operators of one line, outside quotes and substitutions, and
/// the quote that is still open at its end.
fn markers(line: &str, mut quote: Option<char>) -> (Vec<Marker>, Option<char>) {
    let characters: Vec<(usize, char)> = line.char_indices().collect();
    let mut found = Vec::new();
    let mut position = 0;
    let mut substitution = 0_usize;
    while position < characters.len() {
        let (_, character) = characters[position];
        let next = characters.get(position + 1).map(|(_, c)| *c);
        match (quote, character) {
            (Some(open), close) if open == close => quote = None,
            (Some('"'), '\\') => position += 1,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '\\') => position += 1,
            (None, '$') if next == Some('(') => {
                substitution += 1;
                position += 1;
            }
            (None, '(') if substitution > 0 => substitution += 1,
            (None, ')') if substitution > 0 => substitution -= 1,
            // `<<<` is a here-string: it has no word of its own, so `read_marker`
            // finds nothing and it is skipped.
            (None, '<') if next == Some('<') && substitution == 0 => {
                if let Some((marker, after)) = read_marker(line, &characters, position) {
                    found.push(marker);
                    position = after;
                    continue;
                }
                position += 1;
            }
            _ => {}
        }
        position += 1;
    }
    (found, quote)
}

/// Read `<<[-] WORD` that starts at `characters[start]`: the marker, and the position
/// after the word. `None` when there is no word, which is left to the parser.
fn read_marker(line: &str, characters: &[(usize, char)], start: usize) -> Option<(Marker, usize)> {
    let mut position = start + 2;
    let strip_tabs = characters.get(position).map(|(_, c)| *c) == Some('-');
    if strip_tabs {
        position += 1;
    }
    while characters
        .get(position)
        .is_some_and(|(_, c)| *c == ' ' || *c == '\t')
    {
        position += 1;
    }
    let mut delimiter = String::new();
    let mut literal = false;
    let mut quote: Option<char> = None;
    while let Some(&(_, character)) = characters.get(position) {
        match (quote, character) {
            (Some(open), close) if open == close => quote = None,
            (Some(_), _) => delimiter.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                literal = true;
            }
            (None, '\\') => {
                literal = true;
                position += 1;
                if let Some(&(_, escaped)) = characters.get(position) {
                    delimiter.push(escaped);
                }
            }
            (None, c)
                if c.is_whitespace() || matches!(c, '|' | '&' | ';' | '<' | '>' | '(' | ')') =>
            {
                break;
            }
            (None, _) => delimiter.push(character),
        }
        position += 1;
    }
    if delimiter.is_empty() {
        return None;
    }
    let end = characters
        .get(position)
        .map_or(line.len(), |(offset, _)| *offset);
    Some((
        Marker {
            start: characters[start].0,
            end,
            delimiter,
            literal,
            strip_tabs,
        },
        position,
    ))
}

/// The body as one shell word: single quotes when it is literal, double quotes
/// (so that `$name` and `$(…)` are expanded) when it is not.
fn quote_body(body: &str, literal: bool) -> String {
    if literal {
        return format!("'{}'", body.replace('\'', "'\\''"));
    }
    let mut quoted = String::with_capacity(body.len() + 2);
    quoted.push('"');
    let mut characters = body.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '"' => quoted.push_str("\\\""),
            // `\"` is a backslash and a quote in a here-document: keep both.
            '\\' if characters.peek() == Some(&'"') => quoted.push_str("\\\\"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewritten(text: &str) -> String {
        rewrite(text).expect("rewrites")
    }

    #[test]
    fn a_line_without_a_here_document_is_left_alone() {
        for text in [
            "echo hi",
            "cat < file",
            "echo a <<< word",
            "echo '<<EOF'",
            "echo \"<<EOF\"",
        ] {
            assert_eq!(rewritten(text), text);
            assert!(!is_open(text), "{text}");
        }
    }

    #[test]
    fn the_body_becomes_a_here_string() {
        assert_eq!(
            rewritten("cat <<EOF\nline one\nline two\nEOF"),
            "cat <<< \"line one\nline two\""
        );
        assert_eq!(rewritten("cat <<EOF\nEOF"), "cat <<< \"\"");
        // The rest of the line and the lines after the delimiter are kept.
        assert_eq!(
            rewritten("cat <<EOF > out.txt\nbody\nEOF\necho after"),
            "cat <<< \"body\" > out.txt\necho after"
        );
        assert_eq!(
            rewritten("echo before\ncat <<END | sort\nb\na\nEND"),
            "echo before\ncat <<< \"b\na\" | sort"
        );
    }

    #[test]
    fn the_delimiter_can_be_spaced_quoted_or_escaped() {
        assert_eq!(rewritten("cat << EOF\nx\nEOF"), "cat <<< \"x\"");
        assert_eq!(rewritten("cat <<\tEOF\nx\nEOF"), "cat <<< \"x\"");
        // Quoted: the body is literal, `$` and quotes included.
        assert_eq!(
            rewritten("cat <<'EOF'\n$HOME \"q\" it's\nEOF"),
            "cat <<< '$HOME \"q\" it'\\''s'"
        );
        assert_eq!(rewritten("cat <<\"EOF\"\n$x\nEOF"), "cat <<< '$x'");
        assert_eq!(rewritten("cat <<\\EOF\n$x\nEOF"), "cat <<< '$x'");
        // Unquoted: expansions stay, double quotes are escaped.
        assert_eq!(
            rewritten("cat <<EOF\n$HOME \"q\" \\\"x\\\"\nEOF"),
            "cat <<< \"$HOME \\\"q\\\" \\\\\\\"x\\\\\\\"\""
        );
    }

    #[test]
    fn a_dash_strips_the_leading_tabs() {
        assert_eq!(
            rewritten("cat <<-EOF\n\tindented\n\t\tdeeper\n\tEOF"),
            "cat <<< \"indented\ndeeper\""
        );
        // Without the dash, the delimiter must be alone on its line.
        assert!(rewrite("cat <<EOF\nx\n\tEOF").is_err());
        assert!(rewrite("cat <<EOF\nx\nEOF extra").is_err());
    }

    #[test]
    fn several_here_documents_on_one_line_take_their_bodies_in_order() {
        assert_eq!(
            rewritten("cmd <<A <<B\nfirst\nA\nsecond\nB"),
            "cmd <<< \"first\" <<< \"second\""
        );
    }

    #[test]
    fn a_here_document_that_is_not_closed_asks_for_more_input() {
        assert!(is_open("cat <<EOF"));
        assert!(is_open("cat <<EOF\nbody"));
        assert!(is_open("cat <<EOF\nbody\nEO"));
        assert!(!is_open("cat <<EOF\nbody\nEOF"));
        assert!(!is_open("cat <<EOF\nbody\nEOF\necho x"));
        assert!(is_open("cat <<A <<B\nx\nA\ny"));
        let error = rewrite("cat <<EOF\nbody").unwrap_err();
        assert!(error.contains("before its delimiter"), "{error}");
    }

    #[test]
    fn operators_in_quotes_substitutions_and_here_strings_are_not_here_documents() {
        for text in [
            "echo '<<EOF'\nx",
            "echo \"a <<EOF\"\nx",
            "echo \\<<EOF\nx",
            "echo $(cat <<EOF)\nx",
            "echo <<<EOF\nx",
        ] {
            assert!(!is_open(text), "{text}");
            assert_eq!(rewritten(text), text);
        }
    }

    #[test]
    fn a_lone_operator_without_a_word_is_left_to_the_parser() {
        assert_eq!(rewritten("cat <<"), "cat <<");
        assert_eq!(rewritten("cat << | x"), "cat << | x");
        assert!(!is_open("cat <<"));
    }

    #[test]
    fn random_text_never_panics() {
        const PIECES: [&str; 18] = [
            "<<", "<<<", "<<-", "EOF", "'", "\"", "\\", "\n", "\t", " ", "$(", ")", "x", "|", ";",
            "é", "<", ">",
        ];
        let mut seed = 0x0123_4567_89AB_CDEF_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..100_000 {
            let length = 1 + (next() % 14) as usize;
            let text: String = (0..length)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect();
            let outcome = std::panic::catch_unwind(|| {
                let _ = rewrite(&text);
                let _ = is_open(&text);
            });
            assert!(outcome.is_ok(), "here-document scan panicked on {text:?}");
        }
    }
}
