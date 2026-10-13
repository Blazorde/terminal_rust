use std::cell::RefCell;
use std::fmt;
use std::io::{self, Write};

use crate::prompt::segment::{PromptSegment, SegmentKind};

pub mod ansi;
pub mod icons;
pub mod palette;
pub mod separators;

use palette::Palette;
use separators::Separator;

#[derive(Default)]
pub(crate) struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

thread_local! {
    static OUTPUT_CAPTURE: RefCell<Option<CapturedOutput>> = const { RefCell::new(None) };
}

pub(crate) fn capture_output<T>(operation: impl FnOnce() -> T) -> (T, CapturedOutput) {
    OUTPUT_CAPTURE.with(|capture| {
        assert!(
            capture.borrow().is_none(),
            "nested output capture is not supported"
        );
        *capture.borrow_mut() = Some(CapturedOutput::default());
    });

    let result = operation();
    let output = OUTPUT_CAPTURE.with(|capture| {
        capture
            .borrow_mut()
            .take()
            .expect("output capture should be active")
    });

    (result, output)
}

pub fn prompt_segments(
    segments: &[PromptSegment],
    color: bool,
    icons_enabled: bool,
    separator: Option<&Separator>,
    plain_separator: &str,
    newline: bool,
    palette: &Palette,
) -> String {
    let mut output = String::new();

    for (index, segment) in segments.iter().enumerate() {
        if !output.is_empty() {
            if newline && segment.kind == SegmentKind::Symbol {
                output.push('\n');
            } else if let Some(separator) = separator {
                output.push_str(&separator.render(
                    icons_enabled,
                    color,
                    &segments[index - 1].style_key,
                    palette,
                ));
            } else {
                output.push_str(plain_separator);
            }
        }

        let value = segment.value.as_deref().unwrap_or_default();
        let decorated = icons::decorate(value, segment.icon.as_ref(), icons_enabled);
        output.push_str(&ansi::paint(
            &decorated,
            ansi::style_for_key(segment.style_key.as_str(), palette),
            color,
        ));
    }

    output.push(' ');
    output
}

pub fn stdout(message: impl fmt::Display) -> io::Result<()> {
    let message = message.to_string();
    if capture_text(OutputStream::Stdout, &message) {
        return Ok(());
    }

    with_stdout(|writer| {
        write_text(writer, &message)?;
        writer.flush()
    })
}

pub fn stdout_line(message: impl fmt::Display) -> io::Result<()> {
    let message = format!("{message}\n");
    if capture_text(OutputStream::Stdout, &message) {
        return Ok(());
    }

    with_stdout(|writer| write_text(writer, message))
}

pub fn stderr_line(message: impl fmt::Display) -> io::Result<()> {
    let message = format!("{message}\n");
    if capture_text(OutputStream::Stderr, &message) {
        return Ok(());
    }

    with_stderr(|writer| write_text(writer, message))
}

pub fn stderr(message: impl fmt::Display) -> io::Result<()> {
    let message = message.to_string();
    if capture_text(OutputStream::Stderr, &message) {
        return Ok(());
    }

    with_stderr(|writer| {
        write_text(writer, &message)?;
        writer.flush()
    })
}

pub fn write_text(writer: &mut impl Write, message: impl fmt::Display) -> io::Result<()> {
    write!(writer, "{message}")
}

fn with_stdout<T>(render: impl FnOnce(&mut io::StdoutLock<'_>) -> io::Result<T>) -> io::Result<T> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    render(&mut writer)
}

fn with_stderr<T>(render: impl FnOnce(&mut io::StderrLock<'_>) -> io::Result<T>) -> io::Result<T> {
    let stderr = io::stderr();
    let mut writer = stderr.lock();

    render(&mut writer)
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

fn capture_text(stream: OutputStream, message: &str) -> bool {
    OUTPUT_CAPTURE.with(|capture| {
        let mut capture = capture.borrow_mut();
        let Some(output) = capture.as_mut() else {
            return false;
        };

        match stream {
            OutputStream::Stdout => output.stdout.push_str(message),
            OutputStream::Stderr => output.stderr.push_str(message),
        }
        true
    })
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    #[test]
    fn renders_segments_with_plain_separator() {
        let segments = [
            PromptSegment::new(SegmentKind::Path, "project", "primary"),
            PromptSegment::new(SegmentKind::Symbol, ">", "secondary"),
        ];

        assert_eq!(
            prompt_segments(
                &segments,
                false,
                false,
                None,
                " | ",
                false,
                &Palette::default(),
            ),
            "project | > "
        );
    }
}
