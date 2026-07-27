#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v rg >/dev/null 2>&1; then
    echo "ERROR: required command not found: rg" >&2
    exit 1
fi

required=(
    "independent, unofficial"
    "not affiliated with or"
    "undocumented ChatGPT backend endpoint"
    "npm and crates.io packages are not published yet"
    "does not use GitHub Actions"
)

for text in "${required[@]}"; do
    if ! rg -Fq "$text" "$ROOT/README.md" "$ROOT/NOTICE"; then
        echo "ERROR: missing public claim: $text" >&2
        exit 1
    fi
done

if find "$ROOT/.github/workflows" -type f -print -quit 2>/dev/null | grep -q .; then
    echo "ERROR: GitHub Actions workflows are forbidden by owner policy" >&2
    exit 1
fi

node -e '
const fs = require("node:fs");
const path = process.argv[1];
const doc = JSON.parse(fs.readFileSync(path, "utf8"));
const supported = doc.targets.filter((target) => target.status === "supported-locally");
if (supported.length !== 1 || supported[0].rustTriple !== "x86_64-unknown-linux-gnu") {
  throw new Error("supported target claims drifted");
}
' "$ROOT/docs/supported-targets.json"

echo "Public claims: OK"
