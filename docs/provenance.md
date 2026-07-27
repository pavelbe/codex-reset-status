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

## Published 0.1.0 (2026-07-28)

Built and packaged from commit `a147fc5408983dbc7205d45b75ed4cbac20bc4e3` with a
clean tree; binary SHA-256
`b9d2e197a75324b833bcc2e71be5aa7bd5bd32e5a8fbb972bf31efa122b21c75`.

| Artifact | Local SHA-256 | Registry shasum |
| --- | --- | --- |
| `codex-reset-status-linux-x64-0.1.0.tgz` | `1e9eb14f2ab2ce7b6883fa0434c6219b3e7a491448475fc67c79a683fcf3dd0d` | `f1f13e273ddd2d957499aa4f0981213922680887` |
| `codex-reset-status-0.1.0.tgz` | `e5179e0adb693ac1b17c4da0cb7c4264445890a6a30ff2e4c9c70e446be8138e` | `0b3c3c0c09124efd688a4d994e0dd6eba6aa1395` |
| `codex-reset-status-0.1.0-linux-x86_64.tar.gz` | `3119a28ab2c205c85f3be68322aeb1a80bb5cae0c6191bb59cc29b28fff2da43` | not published |

Registry integrity values:

- `codex-reset-status-linux-x64@0.1.0` —
  `sha512-aQEJXC3FGAF9PSvNxO3oNEWPt5yz6ALg/Q7aa+jg6Fy1kVJLsPpqJjQGYPUnmkr6zdAZjP2s2KIY+YRA1ICTEg==`
- `codex-reset-status@0.1.0` —
  `sha512-1poqrC/PM/1DLJF8seuS2sxLmvTA2lX/b8ETEcrxYbqMYINRnD61IKSgIwJnd3XxUx8shTqtDprGDacuetLbSw==`

The platform package was published first, then the launcher, from exactly those
tarballs with no rebuild in between. Both carry `dist-tags.latest = 0.1.0`, five
files, no `scripts` and no runtime dependencies. A launcher-only install from the
public registry resolved `codex-reset-status-linux-x64@0.1.0`, extracted the
binary with mode `755` and the same SHA-256 as the receipt records, and the
installed CLI ran.

The `.tar.gz` archive is a local artifact only; no GitHub Release exists.
