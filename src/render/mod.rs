use std::fmt;
use std::io::{self, Write};

pub fn stdout(message: impl fmt::Display) -> io::Result<()> {
    with_stdout(|writer| {
        write_text(writer, message)?;
        writer.flush()
    })
}

pub fn stdout_line(message: impl fmt::Display) -> io::Result<()> {
    with_stdout(|writer| write_line(writer, message))
}

pub fn stdout_lines(lines: &[&str]) -> io::Result<()> {
    with_stdout(|writer| lines.iter().try_for_each(|line| write_line(writer, line)))
}

pub fn stderr_line(message: impl fmt::Display) -> io::Result<()> {
    with_stderr(|writer| write_line(writer, message))
}

pub fn write_text(writer: &mut impl Write, message: impl fmt::Display) -> io::Result<()> {
    write!(writer, "{message}")
}

pub fn write_line(writer: &mut impl Write, message: impl fmt::Display) -> io::Result<()> {
    writeln!(writer, "{message}")
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
