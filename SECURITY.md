# Security Policy

## Reporting a vulnerability

**Do not open a public issue** for security problems.

Email the maintainers (see the repository profile) with:

- a description of the issue and its impact,
- steps to reproduce,
- affected versions/commits,
- any suggested mitigation.

We aim to acknowledge within 72 hours and to keep you informed. Please give us a reasonable window before any public disclosure.

## Scope

In scope: the app, its bundled assets, and its update mechanism.

Out of scope: third-party services we merely link to, and local attacks requiring physical access to an unlocked machine.

## Our posture

- Offline-first: no telemetry, no crash reporters that phone home, no third-party SDKs with network access.
- Local data (settings, history, athkar progress) stays in a local SQLite file.
- Bundled audio and datasets are pinned and attributed in `docs/attribution.md`.
- Dependencies are audited in CI (`cargo-deny`).

## Supported versions

Only the latest `main` and the most recent release receive security fixes.
