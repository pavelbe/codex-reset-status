#!/usr/bin/env bash
# Verifies release archive and npm tarball contents: exact file set, no extras,
# no symlink/hardlink/traversal/world-writable entries, executable mode after
# extraction and matching SHA-256 sidecar.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
archive=""
launcher_tgz=""
platform_tgz=""
version=""

usage() {
    echo "Usage: scripts/verify-artifacts.sh --archive PATH --launcher-tgz PATH --platform-tgz PATH --version VERSION"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --archive) archive="${2:?--archive requires a path}"; shift 2 ;;
        --launcher-tgz) launcher_tgz="${2:?--launcher-tgz requires a path}"; shift 2 ;;
        --platform-tgz) platform_tgz="${2:?--platform-tgz requires a path}"; shift 2 ;;
        --version) version="${2:?--version requires a value}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "ERROR: unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ -z "$archive" || -z "$launcher_tgz" || -z "$platform_tgz" || -z "$version" ]]; then
    usage >&2
    exit 2
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

# Rejects entries that must never appear in a published artifact, before any
# extraction touches the filesystem.
check_listing() {
    local tarball="$1"
    local listing="$tmp/listing"
    tar -tvf "$tarball" >"$listing"
    local mode type name
    while read -r mode _ _ _ _ _ name _; do
        type="${mode:0:1}"
        case "$type" in
            -|d) ;;
            l) fail "$tarball contains a symlink: $name" ;;
            h) fail "$tarball contains a hardlink: $name" ;;
            *) fail "$tarball contains an unsupported entry type '$type': $name" ;;
        esac
        case "$name" in
            /*) fail "$tarball contains an absolute path: $name" ;;
            *..*) fail "$tarball contains a traversal path: $name" ;;
        esac
        if [[ "${mode:8:1}" == "w" ]]; then
            fail "$tarball contains a world-writable entry: $name ($mode)"
        fi
    done <"$listing"
}

# Compares the extracted tree against an exact expected "<type> <mode> <path>"
# set, so extras and missing files both fail.
check_tree() {
    local root="$1"
    local label="$2"
    shift 2
    local expected="$tmp/expected"
    local actual="$tmp/actual"
    printf '%s\n' "$@" | LC_ALL=C sort >"$expected"
    find "$root" -mindepth 1 -printf '%y %m %P\n' | LC_ALL=C sort >"$actual"
    if ! diff -u "$expected" "$actual" >"$tmp/diff"; then
        echo "ERROR: $label content set does not match the expected set" >&2
        cat "$tmp/diff" >&2
        fail "$label verification failed"
    fi
}

# 1. Release archive.
[[ -f "$archive" ]] || fail "archive not found: $archive"
[[ -f "$archive.sha256" ]] || fail "checksum sidecar not found: $archive.sha256"
(cd -- "$(dirname -- "$archive")" && sha256sum -c "$(basename -- "$archive").sha256" >/dev/null) ||
    fail "archive checksum does not match its sidecar"

package_name="codex-reset-status-$version-linux-x86_64"
check_listing "$archive"
mkdir -p "$tmp/archive"
tar -xzf "$archive" -C "$tmp/archive" --no-same-owner
[[ -d "$tmp/archive/$package_name" ]] || fail "archive does not contain $package_name/"
check_tree "$tmp/archive/$package_name" "release archive" \
    "d 755 bin" \
    "d 755 docs" \
    "f 755 bin/codex-reset-status" \
    "f 644 build-receipt.json" \
    "f 644 docs/provenance.md" \
    "f 644 docs/supported-targets.json" \
    "f 644 LICENSE" \
    "f 644 NOTICE" \
    "f 644 README.md"
[[ -x "$tmp/archive/$package_name/bin/codex-reset-status" ]] ||
    fail "extracted binary is not executable"
"$tmp/archive/$package_name/bin/codex-reset-status" --version |
    grep -Fxq "codex-reset-status $version" ||
    fail "extracted binary reports the wrong version"
node "$ROOT/scripts/check-version-parity.mjs" \
    --binary "$tmp/archive/$package_name/bin/codex-reset-status" >/dev/null

# 2. Platform npm tarball.
[[ -f "$platform_tgz" ]] || fail "platform tarball not found: $platform_tgz"
check_listing "$platform_tgz"
mkdir -p "$tmp/platform"
tar -xzf "$platform_tgz" -C "$tmp/platform" --no-same-owner
check_tree "$tmp/platform/package" "platform npm tarball" \
    "d 755 bin" \
    "f 755 bin/codex-reset-status" \
    "f 644 build-receipt.json" \
    "f 644 LICENSE" \
    "f 644 NOTICE" \
    "f 644 package.json"
[[ -x "$tmp/platform/package/bin/codex-reset-status" ]] ||
    fail "platform tarball binary is not executable"

# 2b. Embedded build receipt must describe the shipped binary, and both artifacts
# must carry the same receipt.
archive_receipt="$tmp/archive/$package_name/build-receipt.json"
platform_receipt="$tmp/platform/package/build-receipt.json"
cmp -s "$archive_receipt" "$platform_receipt" ||
    fail "archive and platform tarball carry different build receipts"
receipt_version="$(jq -r '.version' "$archive_receipt")"
[[ "$receipt_version" == "$version" ]] ||
    fail "build receipt version is $receipt_version, expected $version"
receipt_binary_sha="$(jq -r '.files[] | select(.path == "bin/codex-reset-status") | .sha256' "$archive_receipt")"
actual_binary_sha="$(sha256sum "$tmp/platform/package/bin/codex-reset-status" | awk '{print $1}')"
[[ "$receipt_binary_sha" == "$actual_binary_sha" ]] ||
    fail "build receipt binary sha256 does not match the shipped binary"
archive_binary_sha="$(sha256sum "$tmp/archive/$package_name/bin/codex-reset-status" | awk '{print $1}')"
[[ "$archive_binary_sha" == "$actual_binary_sha" ]] ||
    fail "archive and platform tarball ship different binaries"

# 3. Launcher npm tarball.
[[ -f "$launcher_tgz" ]] || fail "launcher tarball not found: $launcher_tgz"
check_listing "$launcher_tgz"
mkdir -p "$tmp/launcher"
tar -xzf "$launcher_tgz" -C "$tmp/launcher" --no-same-owner
check_tree "$tmp/launcher/package" "launcher npm tarball" \
    "d 755 bin" \
    "f 755 bin/cli.js" \
    "f 644 LICENSE" \
    "f 644 NOTICE" \
    "f 644 package.json" \
    "f 644 README.md"
if grep -qE 'https?://[^ "'"'"']*\.(tar\.gz|tgz|zip)' "$tmp/launcher/package/bin/cli.js"; then
    fail "launcher must not reference a downloadable archive"
fi

echo "Artifact contents ($version): OK"
