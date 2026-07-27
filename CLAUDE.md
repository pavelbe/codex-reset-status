# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`AGENTS.md` holds the owner policy invariants (public MIT source, no hidden core,
no Actions, never leak the token, owner-only publication). Read it first; this
file adds build/architecture detail and does not repeat those rules.

## Commands

Heavy gates are serialized through an optional wrapper (`HEAVY_LOCK`), so prefer
the Makefile over raw `cargo`:

```bash
HEAVY_LOCK="$HOME/.claude/bin/heavy-lock.sh" make check          # test + fmt + clippy + build + shell guards + node test
HEAVY_LOCK=... make test | make fmt | make clippy | make build   # individual gates, run one at a time
HEAVY_LOCK=... make package-local                                # scripts/package-release.sh --output-dir dist
make clean                                                      # cargo clean + rm -rf dist
bash -n scripts/*.sh                                            # shell syntax gate
```

Single-test runs:

```bash
cargo test --locked <test_name>            # e.g. renders_fixture_table, rejects_unknown_options
cargo test --locked --test cli             # integration tests only (tests/cli.rs)
cargo test --locked --lib                  # unit tests inside src/*.rs #[cfg(test)] modules
node --test npm/codex-reset-status/test/cli.test.mjs
```

Manual CLI smoke without network: `./target/release/codex-reset-status --fixture tests/fixtures/ok.json [--json]`.
Fixtures: `ok.json` (populated), `empty.json` (no credits), `drift-no-credits.json` (schema drift → must fail closed).

`make check` uses `$(HOME)/.cargo/bin/cargo` when present; `rust-toolchain.toml`
pins 1.96.0 while `Cargo.toml` declares MSRV 1.85.

## Architecture

Single Rust binary, everything in-process (no curl/python/node at runtime). Flow
in `src/main.rs`: `cli::parse` → `auth::load` (or `--fixture` file read) →
`http::fetch` → `payload` extraction → `render` (table or `codex-reset-status/v1`
JSON). `error::ErrorKind` maps failures to distinct exit codes (usage = 2).

Security-relevant invariants live in the module boundaries and are enforced by
tests plus `scripts/check-*.sh` — do not soften them:

- `secret.rs`: token type with no `Debug`/`Display`; only `expose_for_header()`
  plus `redact()` for error strings.
- `cli.rs`: `--endpoint` accepts *only* the built-in ChatGPT URL, or an exact
  loopback URI when `CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK=1`; loopback
  detection rejects userinfo (`@`) and suffix hosts (`127.0.0.1.example`).
- `http.rs`: one request, `max_redirects(0)`, no retries, `https_only` off only
  for loopback (which also sets `proxy(None)`), 1 MiB response cap, platform
  cert verifier. Raw body is never printed; only allowlisted fields propagate.
- `payload.rs`: tolerant known-field extraction, but a missing/unknown
  credit-list shape is an error, never "no credits".
- `render.rs`: strips terminal control characters from endpoint-derived strings.
- `check-no-subprocess.sh`: bans `Command::new` / `std::process::Command` in `src`.
- `check-public-claims.sh`: pins required disclaimers in `README.md`/`NOTICE`,
  forbids `.github/workflows/**`, and asserts `docs/supported-targets.json`
  lists exactly one `supported-locally` target (`x86_64-unknown-linux-gnu`).

## npm distribution

Two packages under `npm/`:

- `codex-reset-status` — pure-JS launcher (`bin/cli.js`). Maps platform/arch →
  native package name (only `linux`+`x64`), resolves
  `codex-reset-status-linux-x64/bin/codex-reset-status` via `require.resolve`,
  chmods `0755` if needed, `spawn`s it with `stdio: inherit`, forwards
  SIGINT/SIGTERM/SIGHUP, re-raises the child's signal. No install scripts, no
  runtime downloads.
- `codex-reset-status-linux-x64` — `os`/`cpu`-gated binary carrier; its
  `bin/codex-reset-status` and `build-receipt.json` are injected at package time
  (repo keeps only `bin/.gitkeep`).

`scripts/package-release.sh` is the release SSOT: refuses non-`x86_64-unknown-linux-gnu`
hosts, builds `--release --locked`, stages archive + docs, writes
`build-receipt.json` (rustc, `Cargo.lock` sha, binary sha/bytes, git head/dirty),
tars deterministically (`--sort=name --owner=0 --group=0 --mtime=@SOURCE_DATE_EPOCH`),
extracts and re-verifies `--version`, `npm pack`s both packages, runs
`scripts/npm-local-verify.sh`, then writes the release receipt with SHA-256 per
artifact. Version currently comes from `npm/codex-reset-status/package.json`.

A receipt whose `git.head` is `unborn` or `dirty: true` is not publishable —
re-run packaging on a clean HEAD that matches `origin/main`.

`dist/` is gitignored build output; treat existing tarballs as stale unless a
receipt from the current clean HEAD says otherwise.

## Release/publication gates

Publishing order is fixed: platform tgz first, launcher tgz second, exact
artifacts from the digested build, no rebuild in between. Publication, git tags,
GitHub Releases, crates.io, `npm login` and registry visibility changes are
owner-gated actions — never run them without an explicit fresh owner approval
for that specific artifact tuple.

A target may be marked supported only after its binary smoke *and* relocated
package smoke pass on that target; keep `docs/supported-targets.json`,
`README.md` and the claims guard in sync.
