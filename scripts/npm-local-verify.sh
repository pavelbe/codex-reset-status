#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
launcher="${1:?launcher tarball path is required}"
platform="${2:?platform tarball path is required}"
version="${3:?expected version is required}"
platform_package="codex-reset-status-linux-x64"

if [[ ! -f "$launcher" || ! -f "$platform" ]]; then
    echo "ERROR: npm tarballs are missing" >&2
    exit 1
fi

tmp="$(mktemp -d)"
registry_pid=""
cleanup() {
    if [[ -n "$registry_pid" ]] && kill -0 "$registry_pid" 2>/dev/null; then
        kill "$registry_pid" 2>/dev/null || true
        wait "$registry_pid" 2>/dev/null || true
    fi
    rm -rf "$tmp"
}
trap cleanup EXIT

run_cli() {
    local prefix="$1"
    shift
    "$prefix/node_modules/.bin/codex-reset-status" "$@"
}

# 1. Both tarballs installed explicitly: proves the packed launcher and packed
# binary work together, but says nothing about dependency resolution.
explicit="$tmp/explicit"
mkdir -p "$explicit"
printf '{"name":"codex-reset-status-local-smoke","private":true}\n' >"$explicit/package.json"
npm install --prefix "$explicit" --ignore-scripts --no-audit --no-fund "$platform" "$launcher" >/dev/null

run_cli "$explicit" --version | grep -Fx "codex-reset-status $version" >/dev/null
run_cli "$explicit" --fixture "$ROOT/tests/fixtures/empty.json" |
    grep -F "No reset credits found." >/dev/null

# 2. Supported platform, optional package removed: the launcher must name the
# missing dependency instead of claiming the platform is unsupported.
rm -rf "$explicit/node_modules/$platform_package"
set +e
missing_output="$(run_cli "$explicit" --version 2>&1)"
missing_status=$?
set -e
if [[ "$missing_status" -eq 0 ]] ||
    [[ "$missing_output" != *"could not load its native package $platform_package"* ]] ||
    [[ "$missing_output" == *"no native binary for"* ]]; then
    echo "ERROR: missing platform package did not fail with the reinstall message" >&2
    echo "$missing_output" >&2
    exit 1
fi

# 3. Launcher-only install against a disposable registry that serves the platform
# package under its production name and version: proves optionalDependencies
# resolution, not just co-installed files.
port_file="$tmp/registry-port"
node "$ROOT/scripts/local-registry.mjs" \
    --tarball "$platform" \
    --name "$platform_package" \
    --version "$version" \
    --port-file "$port_file" >"$tmp/registry.log" 2>&1 &
registry_pid=$!

for _ in $(seq 1 100); do
    if [[ -s "$port_file" ]]; then
        break
    fi
    if ! kill -0 "$registry_pid" 2>/dev/null; then
        echo "ERROR: fixture registry exited early" >&2
        cat "$tmp/registry.log" >&2
        exit 1
    fi
    sleep 0.1
done
if [[ ! -s "$port_file" ]]; then
    echo "ERROR: fixture registry did not report a port within 10s" >&2
    cat "$tmp/registry.log" >&2
    exit 1
fi
registry_url="http://127.0.0.1:$(tr -d '[:space:]' <"$port_file")"

resolved="$tmp/resolved"
mkdir -p "$resolved"
printf '{"name":"codex-reset-status-resolution-smoke","private":true}\n' >"$resolved/package.json"
# A loopback registry must not be reached through an environment HTTP proxy (this
# is a no-op on machines without one), and a private cache keeps the resolution
# proof independent of earlier installs.
if ! env HTTP_PROXY= HTTPS_PROXY= ALL_PROXY= http_proxy= https_proxy= all_proxy= \
    NO_PROXY="127.0.0.1,localhost" no_proxy="127.0.0.1,localhost" \
    npm install --prefix "$resolved" --ignore-scripts --no-audit --no-fund \
    --package-lock=false --cache "$tmp/npm-cache" --noproxy "127.0.0.1,localhost" \
    --registry "$registry_url" "$launcher" >"$tmp/resolve-install.log" 2>&1; then
    echo "ERROR: launcher-only install failed" >&2
    cat "$tmp/resolve-install.log" >&2
    cat "$tmp/registry.log" >&2
    exit 1
fi

if [[ ! -x "$resolved/node_modules/$platform_package/bin/codex-reset-status" ]]; then
    echo "ERROR: optionalDependencies resolution did not install $platform_package" >&2
    cat "$tmp/resolve-install.log" >&2
    cat "$tmp/registry.log" >&2
    exit 1
fi
# The install must have come from the fixture registry, not from a co-installed
# file or a warm cache.
for expected_request in "GET /$platform_package" "GET /$platform_package/-/$platform_package-$version.tgz"; do
    if ! grep -Fq "request $expected_request" "$tmp/registry.log"; then
        echo "ERROR: fixture registry never served: $expected_request" >&2
        cat "$tmp/registry.log" >&2
        exit 1
    fi
done
npm ls --prefix "$resolved" --depth 1 2>/dev/null |
    grep -Fq "$platform_package@$version" ||
    {
        echo "ERROR: npm ls does not report $platform_package@$version" >&2
        exit 1
    }
installed_sha="$(sha256sum "$resolved/node_modules/$platform_package/bin/codex-reset-status" | awk '{print $1}')"
if [[ "$installed_sha" != "$(sha256sum "$ROOT/target/release/codex-reset-status" | awk '{print $1}')" ]]; then
    echo "ERROR: installed binary does not match the built release binary" >&2
    exit 1
fi
# A published install must work without the launcher repairing file modes.
installed_mode="$(stat -c '%a' "$resolved/node_modules/$platform_package/bin/codex-reset-status")"
if [[ "$installed_mode" != "755" ]]; then
    echo "ERROR: installed binary mode is $installed_mode, expected 755" >&2
    exit 1
fi
run_cli "$resolved" --version | grep -Fx "codex-reset-status $version" >/dev/null
run_cli "$resolved" --fixture "$ROOT/tests/fixtures/empty.json" |
    grep -F "No reset credits found." >/dev/null

echo "Local npm tarball smoke: OK"
