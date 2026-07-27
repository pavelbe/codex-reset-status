#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="${CARGO:-cargo}"
if [[ -z "${RUSTC:-}" ]]; then
    cargo_sibling="$(dirname -- "$CARGO")/rustc"
    if [[ "$CARGO" == */* && -x "$cargo_sibling" ]]; then
        RUSTC="$cargo_sibling"
    else
        RUSTC="rustc"
    fi
fi
OUTPUT_DIR="$ROOT/dist"

REQUIRE_CLEAN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-dir)
            OUTPUT_DIR="${2:?--output-dir requires a path}"
            shift 2
            ;;
        --require-clean)
            REQUIRE_CLEAN=1
            shift
            ;;
        -h|--help)
            echo "Usage: scripts/package-release.sh [--output-dir DIR] [--require-clean]"
            exit 0
            ;;
        *)
            echo "ERROR: unknown option: $1" >&2
            exit 2
            ;;
    esac
done

for command in "$CARGO" "$RUSTC" git jq npm node sha256sum tar; do
    if ! command -v "$command" >/dev/null 2>&1 && [[ ! -x "$command" ]]; then
        echo "ERROR: required command not found: $command" >&2
        exit 1
    fi
done

# Cargo.toml is the single version source of truth; parity is asserted before
# anything is built or packed.
node "$ROOT/scripts/check-version-parity.mjs"
version="$(sed -n '/^\[package\]/,/^\[/{s/^version *= *"\([^"]*\)".*/\1/p}' "$ROOT/Cargo.toml" | head -1)"
if [[ -z "$version" ]]; then
    echo "ERROR: cannot read the version from Cargo.toml" >&2
    exit 1
fi
target="$("$CARGO" -vV | sed -n 's/^host: //p')"
if [[ "$target" != "x86_64-unknown-linux-gnu" ]]; then
    echo "ERROR: local v0.1 packaging supports only x86_64-unknown-linux-gnu, got $target" >&2
    exit 1
fi

# Keep build-machine paths out of the shipped binary. Panic locations from
# dependencies otherwise embed the packager's home directory and user name.
# `trim-paths` is not stable in Cargo 1.96, so remap explicitly (longest prefix
# first). verify-artifacts.sh fails the release if any path survives.
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$cargo_home=/cargo --remap-path-prefix=$ROOT=/src --remap-path-prefix=$HOME=/build"

"$CARGO" build --release --locked --manifest-path "$ROOT/Cargo.toml"

binary="$ROOT/target/release/codex-reset-status"
if [[ ! -x "$binary" ]]; then
    echo "ERROR: release binary was not built: $binary" >&2
    exit 1
fi
node "$ROOT/scripts/check-version-parity.mjs" --binary "$binary"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
package_name="codex-reset-status-$version-linux-x86_64"
stage="$tmp/$package_name"
mkdir -p "$stage/bin" "$stage/docs" "$OUTPUT_DIR/npm"

install -m 0755 "$binary" "$stage/bin/codex-reset-status"
install -m 0644 "$ROOT/LICENSE" "$ROOT/NOTICE" "$ROOT/README.md" "$stage/"
install -m 0644 "$ROOT/docs/provenance.md" "$ROOT/docs/supported-targets.json" "$stage/docs/"

if git -C "$ROOT" rev-parse --verify HEAD >/dev/null 2>&1; then
    git_head="$(git -C "$ROOT" rev-parse HEAD)"
else
    git_head="unborn"
fi
if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
    git_dirty=true
else
    git_dirty=false
fi
if [[ "$REQUIRE_CLEAN" -eq 1 ]] && { [[ "$git_head" == "unborn" ]] || [[ "$git_dirty" == "true" ]]; }; then
    echo "ERROR: --require-clean needs a committed HEAD and a clean tree (head=$git_head dirty=$git_dirty)" >&2
    exit 1
fi
built_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rustc_version="$("$RUSTC" --version)"
lock_sha="$(sha256sum "$ROOT/Cargo.lock" | awk '{print $1}')"
binary_sha="$(sha256sum "$stage/bin/codex-reset-status" | awk '{print $1}')"
binary_bytes="$(stat -c '%s' "$stage/bin/codex-reset-status")"

"$stage/bin/codex-reset-status" --version |
    grep -Fx "codex-reset-status $version" >/dev/null

jq -n \
    --arg schema "codex-reset-status-build-receipt/v1" \
    --arg version "$version" \
    --arg target "$target" \
    --arg gitHead "$git_head" \
    --argjson gitDirty "$git_dirty" \
    --arg builtAt "$built_at" \
    --arg rustc "$rustc_version" \
    --arg cargoLockSha256 "$lock_sha" \
    --arg binarySha256 "$binary_sha" \
    --argjson binaryBytes "$binary_bytes" \
    '{
      schema: $schema,
      version: $version,
      target: $target,
      git: {head: $gitHead, dirty: $gitDirty},
      builtAt: $builtAt,
      rustc: $rustc,
      cargoLockSha256: $cargoLockSha256,
      files: [{
        path: "bin/codex-reset-status",
        sha256: $binarySha256,
        bytes: $binaryBytes
      }],
      smoke: [{name: "staged-version", status: "passed"}],
      signature: null
    }' >"$stage/build-receipt.json"

# Prefer the commit time so the same commit produces the same archive bytes.
if [[ -n "${SOURCE_DATE_EPOCH:-}" ]]; then
    epoch="$SOURCE_DATE_EPOCH"
elif [[ "$git_head" != "unborn" ]]; then
    epoch="$(git -C "$ROOT" show -s --format=%ct "$git_head")"
else
    epoch="$(date +%s)"
fi
archive="$OUTPUT_DIR/$package_name.tar.gz"
mkdir -p "$OUTPUT_DIR"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" \
    -C "$tmp" -czf "$archive" "$package_name"
archive_sha="$(sha256sum "$archive" | awk '{print $1}')"
printf '%s  %s\n' "$archive_sha" "$(basename "$archive")" >"$archive.sha256"

npm_stage="$tmp/npm"
mkdir -p "$npm_stage"
cp -R "$ROOT/npm/codex-reset-status" "$npm_stage/"
cp -R "$ROOT/npm/codex-reset-status-linux-x64" "$npm_stage/"
mkdir -p "$npm_stage/codex-reset-status-linux-x64/bin"
install -m 0755 "$binary" \
    "$npm_stage/codex-reset-status-linux-x64/bin/codex-reset-status"
install -m 0644 "$stage/build-receipt.json" \
    "$npm_stage/codex-reset-status-linux-x64/build-receipt.json"

npm pack "$npm_stage/codex-reset-status-linux-x64" \
    --pack-destination "$OUTPUT_DIR/npm" >/dev/null
npm pack "$npm_stage/codex-reset-status" \
    --pack-destination "$OUTPUT_DIR/npm" >/dev/null

launcher_tgz="$OUTPUT_DIR/npm/codex-reset-status-$version.tgz"
platform_tgz="$OUTPUT_DIR/npm/codex-reset-status-linux-x64-$version.tgz"
bash "$ROOT/scripts/verify-artifacts.sh" \
    --archive "$archive" \
    --launcher-tgz "$launcher_tgz" \
    --platform-tgz "$platform_tgz" \
    --version "$version"
bash "$ROOT/scripts/npm-local-verify.sh" "$launcher_tgz" "$platform_tgz" "$version"

if [[ "$REQUIRE_CLEAN" -eq 1 ]]; then
    # The snapshot must still be the approved one after every build and smoke.
    if [[ "$(git -C "$ROOT" rev-parse HEAD)" != "$git_head" ]] ||
        [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
        echo "ERROR: the working tree changed during packaging; artifacts are not tied to $git_head" >&2
        exit 1
    fi
fi

launcher_sha="$(sha256sum "$launcher_tgz" | awk '{print $1}')"
platform_sha="$(sha256sum "$platform_tgz" | awk '{print $1}')"
build_receipt_sha="$(sha256sum "$stage/build-receipt.json" | awk '{print $1}')"
receipt="$OUTPUT_DIR/codex-reset-status-$version-release-receipt.json"
jq -n \
    --arg schema "codex-reset-status-release-receipt/v1" \
    --arg version "$version" \
    --arg target "$target" \
    --arg gitHead "$git_head" \
    --argjson gitDirty "$git_dirty" \
    --arg builtAt "$built_at" \
    --arg binarySha256 "$binary_sha" \
    --arg buildReceiptSha256 "$build_receipt_sha" \
    --argjson sourceDateEpoch "$epoch" \
    --arg archivePath "$(basename "$archive")" \
    --arg archiveSha256 "$archive_sha" \
    --arg launcherPath "npm/$(basename "$launcher_tgz")" \
    --arg launcherSha256 "$launcher_sha" \
    --arg platformPath "npm/$(basename "$platform_tgz")" \
    --arg platformSha256 "$platform_sha" \
    '{
      schema: $schema,
      version: $version,
      target: $target,
      git: {head: $gitHead, dirty: $gitDirty},
      builtAt: $builtAt,
      sourceDateEpoch: $sourceDateEpoch,
      binarySha256: $binarySha256,
      buildReceiptSha256: $buildReceiptSha256,
      artifacts: [
        {path: $archivePath, sha256: $archiveSha256},
        {path: $launcherPath, sha256: $launcherSha256},
        {path: $platformPath, sha256: $platformSha256}
      ],
      smoke: [
        {name: "version-parity", status: "passed"},
        {name: "staged-version", status: "passed"},
        {name: "archive-contents", status: "passed"},
        {name: "archive-version", status: "passed"},
        {name: "npm-tarball-contents", status: "passed"},
        {name: "npm-relocated", status: "passed"},
        {name: "npm-missing-platform", status: "passed"},
        {name: "npm-optional-dependency-resolution", status: "passed"}
      ],
      signature: null
    }' >"$receipt"

node "$ROOT/scripts/check-version-parity.mjs" --receipt "$receipt" >/dev/null ||
    echo "WARNING: receipt is not publishable yet (see errors above)" >&2

echo "Release archive: $archive"
echo "Release checksum: $archive.sha256"
echo "Release receipt: $receipt"
echo "npm tarballs: $OUTPUT_DIR/npm"
