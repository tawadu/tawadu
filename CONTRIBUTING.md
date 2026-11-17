# Contributing to Tawadu

Thanks for helping. Tawadu is a calm tool for worship — keep the code and the tone equally unhurried.

## Ground rules

- **Open an issue first** for anything non-trivial. Describe the behaviour, not just the fix.
- **Offline-first is a hard constraint.** Core features must work with no network. Network features are opt-in and must degrade silently.
- **No ads, no tracking, no accounts, no telemetry.** If you think a feature needs one, open an issue and explain why.
- **Arabic is a first-class citizen.** Every user-facing string needs an Arabic string; RTL must be tested, not assumed.
- **Design tokens over hard-coded values.** Colours, spacing, radii and motion come from the token set in `design/brand.md`.

## Development setup

```bash
git clone https://github.com/tawadu/tawadu.git
cd tawadu
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Linux also needs the system audio/window development packages (e.g. `libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev` on Debian/Ubuntu).

## Pull requests

1. Branch from `main`; keep commits small and named (`feat:`, `fix:`, `docs:`, `refactor:`).
2. Add tests for logic. Prayer-time maths must be covered by the reference fixtures.
3. Update `docs/` when behaviour changes.
4. Keep the diff focused — no drive-by reformatting.

## Reporting issues

Include: OS and version, Tawadu version/commit, steps to reproduce, expected vs actual, and whether it happens offline.

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
