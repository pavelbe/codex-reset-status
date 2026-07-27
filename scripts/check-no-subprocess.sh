#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v rg >/dev/null 2>&1; then
    echo "ERROR: required command not found: rg" >&2
    exit 1
fi

if rg -n 'Command::new|std::process::Command' "$ROOT/src"; then
    echo "ERROR: runtime source must not spawn subprocesses" >&2
    exit 1
fi

echo "No runtime subprocesses: OK"
