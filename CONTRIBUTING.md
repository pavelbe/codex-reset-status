# Contributing

Thanks for considering a contribution.

## Before you start

- Open an issue for anything larger than a bug fix, so the scope and the
  supported-target claims can be agreed first.
- This project deliberately stays small: one Rust binary, one npm launcher, one
  platform package per verified target.

## Ground rules

- The complete Rust source stays public under MIT. No hidden or binary-only
  core.
- No install scripts and no runtime downloads in the npm packages.
- The access token is never printed, logged, serialized or passed to a
  subprocess. `scripts/check-no-subprocess.sh` enforces the last part.
- A target is "supported" only after its binary smoke and relocated package
  smoke pass on that target; update
  [docs/supported-targets.json](docs/supported-targets.json) with the evidence.
- Public claims stay conservative: the ChatGPT endpoint is undocumented.
  `scripts/check-public-claims.sh` pins the required wording.
- This repository does not use GitHub Actions. Do not add
  `.github/workflows/**`; run gates locally.

## Development

```bash
make check                      # tests, fmt, clippy, release build, guards, launcher tests
cargo test --locked <name>      # single Rust test
node --test npm/codex-reset-status/test/cli.test.mjs
make package-local              # full packaging with content and resolution smokes
```

Use `--fixture tests/fixtures/ok.json` to work without network access or a real
token. Add a fixture instead of pasting real endpoint output into an issue: real
responses may contain account data.

## Pull requests

- Keep the diff scoped to the change; do not reformat unrelated code.
- Add or extend a test that fails without your change.
- `make check` must be green, and say so in the pull request with the actual
  output.
- Note any change to public claims, supported targets or the security boundary
  explicitly.

## Reporting security issues

See [SECURITY.md](SECURITY.md). Never attach `~/.codex/auth.json`, raw HTTP
responses or bearer tokens.
