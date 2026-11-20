#!/usr/bin/env bash
set -euo pipefail

repository='https://github.com/Blazorde/terminal_rust.git'
branch='main'
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
target="$config_home/nvim"
temporary_root="$(mktemp -d)"
staging="$temporary_root/nvim"
trap 'rm -rf "$temporary_root"' EXIT

git clone --depth 1 --filter=blob:none --sparse --branch "$branch" \
  "$repository" "$temporary_root/repository"
git -C "$temporary_root/repository" sparse-checkout set config/nvim
cp -R "$temporary_root/repository/config/nvim" "$staging"

mkdir -p "$config_home"
if [ -e "$target" ]; then
  backup="$target.backup-$(date -u +%Y%m%dT%H%M%SZ)"
  mv "$target" "$backup"
  printf 'Previous Neovim configuration backed up to: %s\n' "$backup"
fi

mv "$staging" "$target"
printf 'Neovim configuration installed at: %s\n' "$target"
printf 'Start Neovim with: nvim\n'
