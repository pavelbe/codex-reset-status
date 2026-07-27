# Architecture

## Decision

The public CLI is a single Rust binary. HTTPS, authentication parsing, response
parsing and rendering run in-process. JavaScript exists only as an optional npm
launcher for a matching prebuilt binary.

This keeps the token out of subprocess arguments and avoids a runtime
dependency on Python, curl or Node for direct binary users.

## Boundaries

- `auth`: bounded local auth-file reading and tolerant known-field discovery.
- `http`: one bounded request, no redirects or retries, no raw-body output.
- `payload`: tolerant known-field extraction with fail-closed root/list shape.
- `timefmt`: UTC normalization, explicit zone resolution (host zone or `--utc`,
  with a warning instead of a silent UTC fallback) and countdown calculation.
- `render`: human output and versioned allowlist JSON.

## Endpoint

The endpoint is undocumented and is not a stable OpenAI API contract. Schema
changes are reported as response errors instead of silently claiming that no
credits exist.
