# codex-reset-status

`codex-reset-status` shows how many ChatGPT/Codex usage-limit reset credits you
have and when each one expires — from the terminal, without opening the ChatGPT
settings panel.

![Terminal showing codex-reset-status output: 2 available resets with expiry dates and time left](https://raw.githubusercontent.com/pavelbe/codex-reset-status/main/docs/assets/cli-output.png?v=2)

```text
Codex Reset Credits
Available: 2 resets
Total earned: 3
Use the first one within: 3d 22h
Checked: 2026-07-28 00:29 MSK (Europe/Moscow)

#  Status     Type               Use before            Time left
-  ---------  -----------------  --------------------  ---------
1  available  codex_rate_limits  2026-07-31 23:18 MSK  3d 22h
2  available  codex_rate_limits  2026-08-12 21:12 MSK  15d 20h
```

`Time left` is how long the reset stays usable: after `Use before` it is gone.

This is the same information the ChatGPT interface shows under **Usage limit
resets**:

![ChatGPT UI panel listing two available full resets expiring 7/31 and 8/12](https://raw.githubusercontent.com/pavelbe/codex-reset-status/main/docs/assets/chatgpt-usage-limit-resets.png?v=2)

The CLI is read-only. It reads the existing Codex authentication file, makes one
request, and prints an allowlisted summary. It never prints the access token or
passes it to a subprocess.

> This is an independent, unofficial project. It is not affiliated with or
> endorsed by OpenAI. It uses an undocumented ChatGPT backend endpoint that can
> change or disappear without notice. The screenshot above is a screenshot of
> the ChatGPT interface, included only to identify which data this tool reads.

## Install

The launcher installs the prebuilt binary through `optionalDependencies` — no
install scripts and no runtime downloads:

```bash
npm install -g codex-reset-status
codex-reset-status
```

Or run it once without installing:

```bash
npx codex-reset-status
```

Or build from source (always available, MIT):

```bash
git clone https://github.com/pavelbe/codex-reset-status.git
cd codex-reset-status
cargo build --release --locked
./target/release/codex-reset-status
```

By default the CLI reads `$CODEX_HOME/auth.json`, falling back to
`~/.codex/auth.json`. Sign in with the Codex CLI first, so that file exists.

## Current Status

Version `0.1.0` is published on npm as
[`codex-reset-status`](https://www.npmjs.com/package/codex-reset-status) plus the
platform package `codex-reset-status-linux-x64`. Linux x86_64 with glibc
(including WSL) is the only verified target; the binary requires `GLIBC_2.34` or
newer and musl hosts are reported as unsupported.
The crates.io package is not published yet.

The complete Rust source is public under the MIT license. The npm packages
contain a small launcher and the compiled platform binary; they do not duplicate
or hide the Rust source in this repository.

## Usage

```text
codex-reset-status
codex-reset-status --json
codex-reset-status --utc
codex-reset-status --auth-file ~/.codex/auth.json
codex-reset-status --fixture tests/fixtures/ok.json
codex-reset-status --timeout 15
```

`--fixture` performs no network request and is intended for deterministic
testing and endpoint-shape diagnostics.

### Time zones

Expiry timestamps arrive from the endpoint in UTC. By default they are converted
to the host time zone — `TZ` if set, otherwise `/etc/localtime` — and the zone is
named in the `Checked:` line so the output is never ambiguous. `--utc` (or
`CODEX_RESET_STATUS_UTC=1`) keeps everything in UTC, which is what you want in
logs and shared output. If the host zone cannot be resolved, the CLI falls back to
UTC **and says so** in `Warnings:` rather than silently pretending.

For compatibility with the current endpoint, the request sends the
`originator: Codex Desktop` header used by the working Codex client flow. The
`User-Agent` identifies this project. Endpoint overrides are restricted to the
built-in ChatGPT URL or an explicitly opted-in loopback test, so a command
cannot redirect the access token to an arbitrary host.

Exit codes: `0` success, `2` usage, `3` auth file, `4` transport, `5` rejected
authentication, `6` unexpected response.

### JSON output

```bash
codex-reset-status --json
```

Machine-readable output uses the versioned allowlist schema
`codex-reset-status/v1` with `tool`, `source`, `checkedAt` (UTC), `checkedAtLocal`,
`timeZone`, `nextExpirySeconds`, `availableCount`, `totalEarnedCount`, `credits[]`
and `warnings[]`. Each credit carries both `expiresAtUtc` and `expiresLocal` plus
`secondsLeft`, `timeLeft` and `expired`. It never returns the raw endpoint
response, so an added upstream field cannot leak into your logs.

## How it works

A single Rust binary: HTTPS, auth parsing, response parsing and rendering all
run in-process. One request, no redirects, no retries, a bounded response size,
and terminal control characters stripped from remote strings. See
[docs/architecture.md](docs/architecture.md).

## Local Gates

```bash
make check           # tests, fmt, clippy, release build, guards, launcher tests
make package-local   # archive + npm tarballs + content/resolution smokes
make package-release # same, but refuses an unborn HEAD or a dirty tree
```

`make package-local` verifies the exact archive/tarball contents (file set,
modes, no symlinks or traversal), the embedded build receipt, and installs the
launcher alone against a disposable local registry to prove
`optionalDependencies` resolution. Release provenance and its limits are
documented in [docs/provenance.md](docs/provenance.md); verified targets in
[docs/supported-targets.json](docs/supported-targets.json).

All checks and release packaging are local. This project does not use GitHub Actions.

## Contributing

Issues and pull requests are welcome. Run `make check` before opening a pull
request, and see [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md). Do not attach `~/.codex/auth.json`, raw HTTP
responses, or bearer tokens to bug reports.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
