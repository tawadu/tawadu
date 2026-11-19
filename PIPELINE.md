# Tawadu — PIPELINE (living state; read this first in any new session)

> Repo: https://github.com/tawadu/tawadu · local: C:\Users\XBY\Projects\tawadu · branch model:
> feature branch → PR → Sentinel review + CI ×3 green + Atlas gate → squash merge to main.
> Plan of record: docs/ROADMAP.md. Orientation: HANDOFF.md. Tray decision: docs/adr/0002.

## Status snapshot (2026-11-19)

| Phase | State | Evidence |
|---|---|---|
| 0 core | DONE | 60 core tests, fixtures, polar/far-east cases |
| 1 usable shell | DONE | settings write path verified end to end |
| 2 notifications + tray | **DONE — merged PR #1 (3f0b578)** | 82 tests; CI ×3 green (run 44429295414); desktop exit test: close→hidden→pre-alert/athan/iqamah fire while hidden→restore+banner→Quit exit 0; Sentinel APPROVE; tray icon registered (NotifyIconSettings) |
| Steps 2–3 (autostart + defects) | IN FLIGHT — Forge | branch feat/step2-3-autostart-defects |
| 3 audio | PREP IN FLIGHT (Scout sources / Prism UI; **Quill strings DONE `826a6ca`**) | docs/voice-pack-format.md is the contract; docs/audio-ui-strings.md = 68 AR/EN keys |
| 4 athkar | queued | MIT Hisn al-Muslim corpus candidate |
| 5–6 coverage/i18n/budgets/packaging | queued | RTL unverified = biggest UI unknown |

## Next actions (owner → action → done-when)

1. **Forge** → Steps 2+3 on feat/step2-3-autostart-defects: auto-launch opt-in (default off, APPEARANCE switch); fired-event ledger (persist last fired event.at per kind, no re-announce on restart); quiet hours at EVENT time; debounced save-on-change (keep حفظ as immediate save; doc+code agree). → PR open, gate green, tests RED-first.
2. **Scout** → docs/audio-sources.md: athan recordings with explicit permissive licences (Wikimedia Commons CC0/CC-BY first), per-file licence text + attribution + URL + sha256; verify MIT Hisn al-Muslim corpus for Phase 4. NO MoIA re-litigation (verbal clearance recorded; user said stop). → table Sentinel can audit.
3. ~~**Quill** → docs/audio-ui-strings.md~~ **DONE — `826a6ca`**: 68 AR/EN keys for audio settings (packs, per-event voice, trim, volume, shuffle/sequential), import errors, both empty states, and the credits page. Bilingual table, no mixed-script runs; Latin travels in `{ $… }` placeholders rendered as their own run. Consumed by Phase 3 and the Phase 6 Fluent catalogs.
4. **Prism** → audio settings UI proposal (brand tokens, calm layout, RTL-aware): pack cards, per-event selectors, trim control. → sketch + Slint layout notes.
5. **Sentinel** → review Forge's PR when opened; audit Scout's licence table before ANY asset commit.
6. **Atlas** → gate + merge Steps 2–3; then dispatch Phase 3 implementation (tawadu-audio: rodio/symphonia mixer, pack format, trim/sequence/shuffle, rfd importer, per-event voice); tray pixel follow-up below.

## Conventions (binding)

- TDD; green gate before every commit: `cargo fmt --all -- --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.
- **GUI test windows**: announce `GUI test window: HH:MM–HH:MM UTC` in PR comments before any taskkill/cargo-test with GUI; single-instance lock is machine-global; user's app instance may be running.
- Windows: `taskkill /F /IM tawadu-app.exe` before builds; confirm BUILD OK + exe timestamp (stale-binary trap).
- Debug clock: TAWADU_DEBUG_NOW / TIME_SCALE / BANNER_MS / MAX_SLICE_MS / AUTOCLOSE_MS / AUTOQUIT_MS. Scale×slice must stay inside 120s late-tolerance (scale 12 + 5s slice, or scale 60 + 1s slice).
- Never commit an asset without a licence entry (assets/licenses/). Never fabricate a UI value. tawadu-core stays UI/platform-free. Repo-local git identity unchanged. Offline-first; no telemetry.
- Decisions: unanimous subagent panel before risky directions (user standing instruction).

## Open debt / follow-ups

- **Tray pixel check** (cosmetic): icon registration + full functional path verified; literal taskbar screenshot pending (taskbar auto-hidden during test; do it when naturally visible).
- Restore-channel hardening (Windows DACL), zbus 5.19+6.3 lock duplication comment, restore-test listener leaks, restore-bind race window.
- Phase 6: gold #B08A4A unused + Today-panel tint undocumented; banner i18n; now-clock display.
- Phase 4: "MORNING ATHKAR" label shown pre-Fajr (static card until engine lands).
- README stale (says Phase 1 / 42 tests) — refresh at next docs pass.

## Heartbeat (for scheduled sessions)

Read this file → check `gh pr list`, latest CI runs, team log freshness
(profiles/{forge,sentinel,prism,scout,quill}/logs/agent.log mtimes) → execute the
next unblocked action yourself or DM the owner (message_agent; hop limit 12) →
update this file's snapshot. Never wait passively on notifications; poll state.
Escalate to the user only for irreversible/money decisions or true blockers.
