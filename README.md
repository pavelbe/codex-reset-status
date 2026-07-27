# codex-reset-status

`codex-reset-status` shows available ChatGPT/Codex reset credits and when each
credit expires.

The CLI is read-only. It reads the existing Codex authentication file, makes
one request, and prints an allowlisted summary. It never prints the access
token or passes it to a subprocess.

> This is an independent, unofficial project. It is not affiliated with or
> endorsed by OpenAI. It uses an undocumented ChatGPT backend endpoint that can
> change or disappear without notice.

## Current Status

Version `0.1.0` is in development. Linux x86_64 and WSL are the only locally
verified targets. npm and crates.io packages are not published yet.

The complete Rust source is public under the MIT license. Future npm packages
may contain a small launcher and compiled platform binary without duplicating
the Rust source already available in this repository.

## Build

```bash
cargo build --release --locked
./target/release/codex-reset-status
```

By default the CLI reads `$CODEX_HOME/auth.json`, falling back to
`~/.codex/auth.json`.

## Usage

```text
codex-reset-status
codex-reset-status --json
codex-reset-status --auth-file ~/.codex/auth.json
codex-reset-status --fixture tests/fixtures/ok.json
```

`--fixture` performs no network request and is intended for deterministic
testing and endpoint-shape diagnostics.

For compatibility with the current endpoint, the request sends the
`originator: Codex Desktop` header used by the working Codex client flow. The
`User-Agent` identifies this project. Endpoint overrides are restricted to the
built-in ChatGPT URL or an explicitly opted-in loopback test, so a command
cannot redirect the access token to an arbitrary host.

## Output

```text
Codex Reset Credits
Available: 2 resets
Checked: 2026-07-27 18:46 MSK

#  Status     Type               Expires (local)       Time left
-  ---------  -----------------  --------------------  ---------
1  available  codex_rate_limits  2026-07-31 23:18 MSK  4d 4h
2  available  codex_rate_limits  2026-08-12 21:12 MSK  16d 2h
```

Machine-readable output uses the versioned allowlist schema
`codex-reset-status/v1`; it never returns the raw endpoint response.

## Local Gates

```bash
make check
make package-local
```

All checks and release packaging are local. This project does not use GitHub Actions.

## Security

See [SECURITY.md](SECURITY.md). Do not attach `~/.codex/auth.json`, raw HTTP
responses, or bearer tokens to bug reports.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
