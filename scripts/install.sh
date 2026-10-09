#!/usr/bin/env sh
set -eu

version=${TERMINAL_RUST_VERSION:-latest}
repository=${TERMINAL_RUST_REPOSITORY:-Blazorde/terminal_rust}
install_dir=${TERMINAL_RUST_INSTALL_DIR:-"$HOME/.local/bin"}

os=$(uname -s)
arch=$(uname -m)

case "$os:$arch" in
    Linux:x86_64|Linux:amd64)
        archive_name=terminal_rust-linux-X64.tar.gz
        ;;
    Darwin:arm64|Darwin:aarch64)
        archive_name=terminal_rust-macos-ARM64.tar.gz
        ;;
    Darwin:x86_64)
        echo "Terminal Rust does not yet publish a macOS Intel archive; compile from source." >&2
        exit 1
        ;;
    *)
        echo "Unsupported platform: $os $arch" >&2
        exit 1
        ;;
esac

if [ "$version" = latest ]; then
    release_base="https://github.com/$repository/releases/latest/download"
else
    release_base="https://github.com/$repository/releases/download/$version"
fi

if command -v curl >/dev/null 2>&1; then
    download() {
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            curl --fail --location --silent --show-error \
                --header "Authorization: Bearer $GITHUB_TOKEN" "$1" --output "$2"
        else
            curl --fail --location --silent --show-error "$1" --output "$2"
        fi
    }
elif command -v wget >/dev/null 2>&1; then
    download() {
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            wget --quiet --header="Authorization: Bearer $GITHUB_TOKEN" --output-document="$2" "$1"
        else
            wget --quiet --output-document="$2" "$1"
        fi
    }
else
    echo "Install curl or wget before running this installer." >&2
    exit 1
fi

temporary_directory=$(mktemp -d "${TMPDIR:-/tmp}/terminal-rust-install.XXXXXX")
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM
archive_path="$temporary_directory/$archive_name"
checksums_path="$temporary_directory/SHA256SUMS.txt"

echo "Terminal Rust installer"
echo "  Release: $version"
echo "  Source:  $release_base/$archive_name"
echo "  Target:  $install_dir/terminal_rust"

if [ -n "${GITHUB_TOKEN:-}" ]; then
    if ! command -v gh >/dev/null 2>&1; then
        echo "Private releases require the GitHub CLI (gh) when GITHUB_TOKEN is set." >&2
        exit 1
    fi
    if [ "$version" = latest ]; then
        gh release download --repo "$repository" --pattern "$archive_name" \
            --pattern SHA256SUMS.txt --dir "$temporary_directory"
    else
        gh release download "$version" --repo "$repository" --pattern "$archive_name" \
            --pattern SHA256SUMS.txt --dir "$temporary_directory"
    fi
else
    download "$release_base/$archive_name" "$archive_path"
    download "$release_base/SHA256SUMS.txt" "$checksums_path"
fi

expected_hash=$(awk -v file="$archive_name" '$2 == file || $2 == "*" file { print $1; exit }' "$checksums_path")
if [ -z "$expected_hash" ]; then
    echo "No checksum was published for $archive_name." >&2
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    actual_hash=$(sha256sum "$archive_path" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
    actual_hash=$(shasum -a 256 "$archive_path" | awk '{ print $1 }')
else
    echo "A SHA-256 tool (sha256sum or shasum) is required." >&2
    exit 1
fi

if [ "$actual_hash" != "$expected_hash" ]; then
    echo "Checksum verification failed for $archive_name." >&2
    exit 1
fi

tar -xzf "$archive_path" -C "$temporary_directory"
if [ ! -f "$temporary_directory/terminal_rust" ]; then
    echo "The downloaded archive does not contain terminal_rust." >&2
    exit 1
fi

mkdir -p "$install_dir"
install -m 755 "$temporary_directory/terminal_rust" "$install_dir/terminal_rust"
"$install_dir/terminal_rust" help >/dev/null

echo "Installed Terminal Rust successfully: $install_dir/terminal_rust"
case ":$PATH:" in
    *:"$install_dir":*) ;;
    *)
        echo "Add this line to your shell profile, then open a new terminal:"
        echo "  export PATH=\"$install_dir:\$PATH\""
        ;;
esac
