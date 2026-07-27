# Release Provenance

Local packaging creates an archive, SHA-256 sidecar and JSON release receipt.
The archive and platform npm package contain an earlier build receipt; the
external release receipt is written only after archive and relocated npm
smokes pass.

The receipt records which commit, toolchain and bytes were built on the current
machine. It does not prove reproducibility, absence of defects, authorship or a
cryptographic signature. The initial receipt has `"signature": null`.

No GitHub-hosted workflow is part of this trust boundary.
