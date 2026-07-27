# Changelog

## Unreleased

- Initial Rust CLI, versioned JSON output and deterministic fixture tests.
- Local Linux x86_64 (glibc) archive and npm launcher/platform packages.
- Launcher distinguishes unsupported platform, musl host and missing optional
  native package, and refuses to repair file modes inside `node_modules`.
- Platform package declares `os`, `cpu` and `libc: ["glibc"]`; documented
  minimum `GLIBC_2.34`.
- Auth `account_id` is validated before it can reach a request header.
- Strict version parity across `Cargo.toml` (source of truth), both npm
  manifests, the optional dependency pin, the binary and the release receipt.
- Packaging verifies exact archive/tarball contents, entry types, modes and
  checksums, and proves `optionalDependencies` resolution through a disposable
  local registry.
- `make package-release` refuses an unborn HEAD or a dirty tree and re-checks
  the same commit after every smoke.
- No GitHub Actions or external publication.
