# Opus Review Triage

**Review:** repo-aware read-only, Claude Opus 5, `xhigh`
**Planning evidence:** an existing local reset-credit parser/tests and an
existing Rust release-integrity implementation.
**Final review:** every source, test, release, npm and public-document file in
this repository.

## Accepted

- Pure Rust binary with in-process HTTPS and a non-printable secret type.
- Allowlisted versioned JSON; no raw response passthrough.
- Fixed body/auth caps, one request, no redirects and no retries.
- Local Linux x86_64 npm launcher/platform-package prototype.
- Checksums, bounded provenance receipt and relocated archive/package smoke.
- MIT plus explicit unofficial/not-affiliated notice.

## Narrowed

- v0.1 exposes only `--json`, `--auth-file`, `--fixture`, `--endpoint`,
  `--timeout`, `--version` and help.
- Response size is a fixed safety invariant, not user configurability.
- Missing credit-list shape fails closed by default; no `--strict-schema`
  mode is needed.

## Deferred

- npm/crates.io publication and GitHub releases.
- musl, arm64, macOS and Windows artifacts.
- signing, watch mode, completions, raw output and schema negotiation.
- user-facing diagnostics and custom originator values.

## Rejected

- curl subprocess transport: it weakens the single-binary contract and creates
  avoidable token-handling risk.
- GitHub Actions: prohibited by owner policy.

## Final Findings

The final implementation review reported no P0 and a conditional GO. All five
P1 findings were fixed before the first commit: exact loopback URI validation
with proxy bypass, terminal control-character stripping, truthful MSRV syntax,
evidence-ordered smoke receipts and exact-version npm tarball verification.
