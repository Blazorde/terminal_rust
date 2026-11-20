#!/bin/sh
# Remove the x-terminal-emulator alternative registered by postinst.sh.
set -e

case "$1" in
    remove | deconfigure)
        if command -v update-alternatives >/dev/null 2>&1; then
            update-alternatives --remove x-terminal-emulator /usr/bin/terminal-rust-desktop
        fi
        ;;
esac

exit 0
