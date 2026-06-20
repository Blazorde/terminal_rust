use std::io;

use crate::render;

pub fn print_help() -> io::Result<()> {
    render::stdout_lines(&[
        "Available commands:",
        "  pwd                  print current directory",
        "  cd [path]            change current directory",
        "  ls [-1aAdFhlpRrSt] [path...]",
        "  mkdir [directory]    create a new directory",
        "  touch [file]         create an empty file",
        "  cat [file]           print file content",
        "  rm [-dfirRv] [path...]",
        "  exit                 close the terminal",
        "  quit                 close the terminal",
    ])
}
