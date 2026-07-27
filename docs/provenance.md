# Release Provenance

Local packaging creates an archive, SHA-256 sidecar and JSON release receipt.
The archive and platform npm package contain the same earlier build receipt; the
external release receipt is written only after archive-content, npm-content,
relocated and launcher-only resolution smokes pass, and it records the SHA-256 of
that embedded build receipt.

`scripts/verify-artifacts.sh` proves the exact file set, entry types, file modes
and checksums of the archive and both npm tarballs. `scripts/npm-local-verify.sh`
additionally installs only the launcher tarball against a disposable local
registry, so `optionalDependencies` resolution of
`codex-reset-status-linux-x64` is exercised instead of assumed.

Archive timestamps default to the HEAD commit time, and `make package-release`
refuses an unborn HEAD or a dirty tree and re-checks the same commit after all
smokes. Artifacts are therefore traceable to one commit. This is **not** a
reproducible-build claim: the toolchain, linker and packaging environment are not
pinned, and no independent rebuild has been compared.

The receipt records which commit, toolchain and bytes were built on the current
machine. It does not prove absence of defects, authorship or a cryptographic
signature. The initial receipt has `"signature": null`.

Release packaging builds with `--remap-path-prefix` for the Cargo home, the
repository root and the packager's home directory, because dependency panic
locations otherwise embed build-machine paths and the packager's user name in
the shipped binary. `scripts/verify-artifacts.sh` fails the release if any such
path survives. Ordinary `cargo build` / `make check` builds do not remap, so a
local development binary is not byte-identical to a released one.

The published binary links against glibc (highest required symbol version
`GLIBC_2.34`, needing `libc.so.6` and `libgcc_s.so.1`). The platform package
declares `os`, `cpu` and `libc: ["glibc"]`, and the launcher reports a musl host
as unsupported instead of failing obscurely.

No GitHub-hosted workflow is part of this trust boundary.
