#!/bin/sh
# Offer Terminal Rust as an x-terminal-emulator alternative.
#
# The priority is low on purpose: installing the package never changes the
# system default. Choose it explicitly with
#   sudo update-alternatives --config x-terminal-emulator
set -e

if [ "$1" = "configure" ] && command -v update-alternatives >/dev/null 2>&1; then
    update-alternatives --install /usr/bin/x-terminal-emulator x-terminal-emulator \
        /usr/bin/terminal-rust-desktop 20
fi

exit 0
