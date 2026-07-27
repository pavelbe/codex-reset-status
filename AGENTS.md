# Agent Instructions

- The complete Rust source is public under MIT. Do not introduce a hidden or
  binary-only core.
- GitHub Actions are disabled by owner policy. Run checks and release builds
  locally or on an owner-controlled VPS.
- Never print, serialize, log or pass the Codex access token to a subprocess.
- The ChatGPT endpoint is undocumented. Keep public claims explicit and
  conservative.
- npm/crates.io publication and GitHub release creation are owner-only actions.
- A target is supported only after its binary and relocated package smoke pass.
