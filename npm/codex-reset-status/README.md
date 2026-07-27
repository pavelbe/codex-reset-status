# codex-reset-status

Thin npm launcher for the native `codex-reset-status` CLI. It resolves the
matching platform package and executes the prebuilt binary; it never downloads
anything at install or run time and has no install scripts.

Supported platform: Linux x64 with glibc. Other platforms fail with an explicit
message instead of falling back to a reimplementation — build from source
instead.

Source, security policy, supported-target evidence and release provenance:
<https://github.com/pavelbe/codex-reset-status>.
