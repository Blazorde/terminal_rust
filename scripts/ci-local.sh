#!/usr/bin/env bash
# Replay the Linux gate of .github/workflows/ci.yml, plus the Security checks, in
# the pinned validation image. Use it when hosted runners are unavailable.
#
#   scripts/ci-local.sh            run every step
#   scripts/ci-local.sh --no-deny  skip cargo-deny (the first run installs it)
#   scripts/ci-local.sh --fresh    drop the cached Cargo registry, target and tools
#
# Set DOCKER_CONTEXT to pick a Docker daemon, for example
#   DOCKER_CONTEXT=default scripts/ci-local.sh
#
# This proves Linux compilation, lints, tests and dependency policy. It does not
# cover Windows, macOS, a visible desktop window, or the test that needs the
# bundled Git runtime (TERMINAL_RUST_REQUIRE_BUNDLED_GIT_TEST).
set -euo pipefail

image="terminal-rust-linux-validation"
volumes=(terminal-rust-ci-registry terminal-rust-ci-target terminal-rust-ci-tools)
run_deny=1

for argument in "$@"; do
    case "$argument" in
        --no-deny) run_deny=0 ;;
        --fresh) docker volume rm --force "${volumes[@]}" >/dev/null ;;
        -h | --help)
            sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "ci-local: unknown option: $argument" >&2
            exit 2
            ;;
    esac
done

repository="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

docker build --quiet --file "$repository/.github/docker/linux-validation.Dockerfile" \
    --tag "$image" "$repository/.github/docker" >/dev/null

# The source is mounted read-only and copied inside the container: Tauri's build
# script writes permission manifests beside its configuration.
docker run --rm --interactive \
    --volume "$repository:/source:ro" \
    --volume "terminal-rust-ci-registry:/usr/local/cargo/registry" \
    --volume "terminal-rust-ci-target:/tmp/terminal-rust-target" \
    --volume "terminal-rust-ci-tools:/opt/ci-tools" \
    --env CARGO_TERM_COLOR=never \
    --env CARGO_INSTALL_ROOT=/opt/ci-tools \
    --env RUN_DENY="$run_deny" \
    "$image" bash -s <<'INNER'
set -uo pipefail
export PATH="/opt/ci-tools/bin:$PATH"
tar --exclude=.git --exclude=target --exclude=node_modules \
    --exclude=desktop/node_modules --exclude=desktop/dist \
    -C /source -cf - . | tar -C /workspace -xf -
cd /workspace || exit 1

declare -a order
declare -A result
logs=/tmp/ci-local-logs
mkdir -p "$logs"

step() {
    local name="$1"
    shift
    order+=("$name")
    local log="$logs/${#order[@]}.log"
    echo "-> $name"
    if "$@" >"$log" 2>&1; then
        result[$name]=PASS
    else
        result[$name]=FAIL
        echo "   FAILED, last lines:"
        tail -n 15 "$log" | sed 's/^/   | /'
    fi
}

step "workflow tests"     node --test scripts/check-branch-policy.test.mjs scripts/ci-scope.test.mjs
step "npm ci"             npm ci --prefix desktop --silent
step "frontend build"     npm run build --prefix desktop
step "frontend tests"     npm test --prefix desktop
step "notices:check"      npm run notices:check --prefix desktop
step "npm audit"          npm audit --prefix desktop --audit-level=high
step "cargo fmt"          cargo fmt --all -- --check
step "cargo check"        cargo check --all-targets --all-features --locked
step "check desktop"      cargo check -p terminal-rust-desktop --all-targets --all-features --locked
step "check plugins"      cargo check -p terminal-rust-plugin-sdk -p terminal-rust-example-plugin --all-targets --all-features --locked
step "clippy"             cargo clippy --all-targets --all-features --locked -- -D warnings
step "clippy desktop"     cargo clippy -p terminal-rust-desktop --all-targets --all-features --locked -- -D warnings
step "clippy plugins"     cargo clippy -p terminal-rust-plugin-sdk -p terminal-rust-example-plugin --all-targets --all-features --locked -- -D warnings
step "tests"              cargo test --all-targets --all-features --locked --no-fail-fast
step "tests desktop"      cargo test -p terminal-rust-desktop --all-targets --all-features --locked --no-fail-fast
step "tests plugins"      cargo test -p terminal-rust-plugin-sdk -p terminal-rust-example-plugin --all-targets --all-features --locked --no-fail-fast
step "install.sh syntax"  sh -n scripts/install.sh
if [ "$RUN_DENY" = 1 ]; then
    command -v cargo-deny >/dev/null 2>&1 || step "install cargo-deny" cargo install cargo-deny --locked
    step "cargo deny"     cargo deny check
fi

echo
echo "=== Summary ==="
failed=0
for name in "${order[@]}"; do
    printf '%-20s %s\n' "$name" "${result[$name]}"
    [ "${result[$name]}" = PASS ] || failed=1
done
exit "$failed"
INNER
