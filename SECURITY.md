# Security

## Sensitive Data

`~/.codex/auth.json` contains an access token. Never publish or attach that
file to an issue.

The CLI:

- performs HTTPS in-process;
- does not pass the token through command-line arguments or child processes;
- does not expose raw endpoint responses;
- caps authentication and response body sizes;
- follows no redirects;
- makes one request and performs no automatic retries.

The `--fixture` input may itself contain private data. Output is allowlisted,
but review fixtures before sharing them.

## Reporting

Open a GitHub issue without secrets for ordinary bugs. For a suspected token
exposure, contact the repository owner privately and revoke the affected Codex
session.
