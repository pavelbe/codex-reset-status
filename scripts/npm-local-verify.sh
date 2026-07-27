#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
launcher="${1:?launcher tarball path is required}"
platform="${2:?platform tarball path is required}"
version="${3:?expected version is required}"

if [[ ! -f "$launcher" || ! -f "$platform" ]]; then
    echo "ERROR: npm tarballs are missing" >&2
    exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf '{"name":"codex-reset-status-local-smoke","private":true}\n' >"$tmp/package.json"
npm install --prefix "$tmp" --ignore-scripts --no-audit --no-fund "$platform" "$launcher" >/dev/null

"$tmp/node_modules/.bin/codex-reset-status" --version |
    grep -Fx "codex-reset-status $version" >/dev/null
"$tmp/node_modules/.bin/codex-reset-status" \
    --fixture "$ROOT/tests/fixtures/empty.json" |
    grep -F "No reset credits found." >/dev/null

rm -rf "$tmp/node_modules/codex-reset-status-linux-x64"
set +e
missing_output="$("$tmp/node_modules/.bin/codex-reset-status" --version 2>&1)"
missing_status=$?
set -e
if [[ "$missing_status" -eq 0 ]] || [[ "$missing_output" != *"native binary is not available"* ]]; then
    echo "ERROR: missing platform package did not fail clearly" >&2
    exit 1
fi

echo "Local npm tarball smoke: OK"
