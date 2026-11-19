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
| Steps 2–3 (autostart + defects) | **DONE — merged PR #2 (`638ec36`, 03:15:40Z)** | Final head `cbff5f9`: 4 slices + Sentinel-findings fix + tray-conditional test + stub SNI watcher (`c8fab2e`) + watcher fix (`cbff5f9`: GLib main loop + `kill -0` liveness — dead watcher can never again green a fallback run). CI ×3 GREEN on `cbff5f9` (run 44445672484, completed 02:42:30Z) + Sentinel APPROVE (01:51Z). **Local green gate (heartbeat takeover 03:05–03:25 UTC window, announced in PR): fmt ✓ clippy ✓ `cargo test --workspace` exit 0 — 105 tests, 0 failures; `tray_lifecycle` 5-phase GUI gate ok in 167.97s on the REAL Windows tray path (phase 1 'REAL tray path confirmed', phase 5 'hide-flush verified').** Squash-merged 03:15:40Z; remote branch deleted; rescue branch + atlas worktree cleaned; step23 worktree reset to merged head; user app relaunched on merged build. Post-merge CI on `638ec36` triggered by the merge (check next beat) |
| 3 audio | **PREP DONE — all 3 docs accepted** | strings `826a6ca`; proposal+sketch `6873a7f`; ledger `07ef5ec`; Sentinel licence audit 2026-11-19 passed with binding conditions (A1/A3/A4 live re-verify before any asset commit; PDM REJECT — no Tier B bundling) |
| 4 athkar | queued | MIT Hisn al-Muslim corpus candidate |
| 5–6 coverage/i18n/budgets/packaging | queued | RTL unverified = biggest UI unknown |

## Next actions (owner → action → done-when)

1. **Forge** → **PR #2 DONE — merged `638ec36` (03:15:40Z).** **Phase 3 (audio) is now yours — dispatched 03:17 UTC** (full spec in PR #2 comment 6998677001 + `docs/voice-pack-format.md` + decisions 1–8 above). Worktree `tawadu-step23` is yours (detached @ `638ec36`, clean — create `feat/phase3-audio` from it) or a new `tawadu-audio` worktree. Read PIPELINE.md Phase-3 section + all three prep docs first. → waiting for Forge to start (watch log freshness; nudge if idle >~45 min after dispatch).
2. ~~**Scout** → docs/audio-sources.md~~ **DONE — `07ef5ec`**, Sentinel audit passed 2026-11-19 (conditions recorded below; A1/A3/A4 live-byte re-verify + download provenance are preconditions of the asset PR).
3. ~~**Quill** → docs/audio-ui-strings.md~~ **DONE — `826a6ca`**: 68 AR/EN keys for audio settings (packs, per-event voice, trim, volume, shuffle/sequential), import errors, both empty states, and the credits page. Bilingual table, no mixed-script runs; Latin travels in `{ $… }` placeholders rendered as their own run. Consumed by Phase 3 and the Phase 6 Fluent catalogs.
4. ~~**Prism** → audio settings UI proposal~~ **DONE — `6873a7f`** (proposal + interactive HTML sketch accepted; polish notes above are the visual gate).
5. **Sentinel** → ~~final review pass~~ **DONE — APPROVE in hand (01:51:24Z)**: both boundary verdicts correct; stale-prayer product rule confirmed (>120 s late = never announce); ubuntu CI triage with Atlas (option a = SNI host preferred).
6. **Atlas** → ~~gate + merge Steps 2–3~~ **DONE — merged `638ec36` 03:15:40Z** (local green gate + CI ×3 + Sentinel approve); cleanup done; user app relaunched on merged build; Phase 3 dispatched to Forge 03:17 UTC (PR comment 6998677001). → next: verify Forge started Phase 3; when Forge opens the audio PR → review, gate (incl. AUDIBLE check of the generated chime + A1/A2 packs), merge; tray pixel follow-up below.

## Conventions (binding)

- TDD; green gate before every commit: `cargo fmt --all -- --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.
- **GUI test windows**: announce `GUI test window: HH:MM–HH:MM UTC` in PR comments before any taskkill/cargo-test with GUI; single-instance lock is machine-global; user's app instance may be running.
- Windows: `taskkill /F /IM tawadu-app.exe` before builds; confirm BUILD OK + exe timestamp (stale-binary trap).
- Debug clock: TAWADU_DEBUG_NOW / TIME_SCALE / BANNER_MS / MAX_SLICE_MS / AUTOCLOSE_MS / AUTOQUIT_MS. Scale×slice must stay inside 120s late-tolerance (scale 12 + 5s slice, or scale 60 + 1s slice).
- Never commit an asset without a licence entry (assets/licenses/). Never fabricate a UI value. tawadu-core stays UI/platform-free. Repo-local git identity unchanged. Offline-first; no telemetry.
- **Worktree isolation (binding, added 2026-11-19 after a shared-tree collision):** the shared checkout `C:\Users\XBY\Projects\tawadu` stays on `main` and is owned by Atlas. Bots work in their OWN git worktrees (Forge: `C:\Users\XBY\Projects\tawadu-step23`). NEVER switch branches in the shared tree while another agent has WIP. Rescue branch `rescue/team-main-20261119` preserves the collision state — delete only after PR #2 merges.
- Decisions: unanimous subagent panel before risky directions (user standing instruction).

## Phase 3 product decisions (Atlas owns; answers to audio-ui-strings.md §10)

1. **Duration** on a pack card = total playing time of the pack's primary athan sequence.
2. **Default** voice = the pack the user marks default; if none marked, the first imported pack; if no packs, the honest empty state (never a guessed pack).
3. **Supported formats** = the decoder's (symphonia) real runtime list, passed as `{ $formats }` — never hardcoded into copy.
4. **Remove pack** also deletes its cached render files — the copy "removed from this device" stays true.
5. **Chime** = built-in generated calm chime for v1 (no asset licence burden); pack-provided chimes are a later format addendum, not v1.
6. **`name_ar` / `reciter_ar`** = OPTIONAL manifest fields (addendum landed in voice-pack-format.md); importer tolerates absence, falls back to name/reciter.
7. **Mode override precedence**: user override wins over the manifest default; "Reset to pack default" appears only when an override exists.
8. **Duration probing at import is mandatory** (symphonia probe); the UI never guesses a duration — unknown renders غير متاح.

**Prism sketch polish notes (for the Phase 3 visual gate):** volume slider needs a filled track (value affordance); Arabic secondary runs follow one consistent placement rule (English-primary until Phase 6 i18n); column heights stay natural (alignment: start — no dead-space stretching); his acceptance checklist (§9) is the gate, incl. contrast ≥7:1 body / ≥4.5:1 muted.

## Phase 3 prep — COMPLETE (all three docs accepted by Atlas)

- `docs/audio-ui-strings.md` (Quill, 826a6ca): 68 AR/EN keys — accepted.
- `docs/audio-ui-proposal.md` + `design/audio-settings-sketch.html` (Prism, 6873a7f): structure contract — accepted; polish notes below.
- `docs/audio-sources.md` (Scout, 07ef5ec): licence ledger — accepted; Sentinel audit triggered.

**Starter pack decision (Atlas, v1):** CC0 only — **A1** `Beautiful_adhan.ogg` (2:34, full athan, default for all prayers) + **A2** `Adhan.ogg` (0:42, short announcement/pre-alert option). A3/A4 (CC BY-SA) stay documented candidates for later packs; the importer must honour share-alike in the manifest. **No Fajr-specific labelling in v1** — no Tier A file is acoustically verified for the Fajr addition; honest generic-athan label until a listening check. Fajr-specific voice = follow-up: Sentinel PDM verdict + unanimous panel on Tier B (B1 Doha / B2 Mishary), never bundled before that.
**Phase 4 decision:** MIT corpus TEXT first (pinned d793023e), recitation audio only after a separate performance-rights check; OpenDua CC BY 4.0 fallback.

**Sentinel licence audit (2026-11-19) — binding conditions:**
- Ledger structurally COMPLETE. **A1/A3/A4 hashes must be re-verified against fresh LIVE bytes before any asset commit** (Wayback size-match ≠ content-match); the asset PR must carry the download provenance (script or CI artifact). A2 already live-verified. **UPDATE: Atlas live-verified A1+A2 on 2026-11-19 (API-derived URLs, exact SHA-256 match) — v1 condition satisfied; reproducible script at scripts/verify_audio_sources.py. A3/A4 still pending (not v1).**
- Phase 4: state the commit-pin integrity anchor explicitly or add per-file SHA-256 for the MIT JSONs; OpenDua needs a hash or archived page before use.
- A4 attribution identity (Atcovi vs Aaqib Azeez) must be settled before packaging (A4 not in v1).
- **PDM ruling: REJECT** — Tier B is not sufficient for bundling (assertion ≠ grant; B2 performance rights unaddressed). Future Fajr pack: genuine CC0/CC-BY recording or in-house recording (also solves the Fajr-phrase verification). Panel rule applies only if revisited.
- Sabah Fakhry rejection confirmed.

## Open debt / follow-ups

- ~~**PR #2 gate + merge**~~ **DONE 2026-11-19 03:15:40Z** — heartbeat takeover gate (window 03:05–03:25 UTC, PR comment 6998413066): fmt ✓, clippy ✓, `cargo test --workspace` exit 0 (105 tests, 0 failures; `tray_lifecycle` 5-phase GUI gate ok 167.97s, REAL Windows tray path phases 1+5 confirmed), BUILD OK 06:05:37 +03. Squash-merged `638ec36`; remote branch deleted; rescue branch + atlas worktree removed; step23 worktree detached @ `638ec36` (clean, ready for Forge); user app relaunched on merged build (PID 42304 @ 06:18 AST). Post-merge CI on `638ec36`: check next beat.
- **Linux real hide-path CI coverage: IMPLEMENTED in c8fab2e, STUB FIXED in cbff5f9** (scripts/ci/sni_watcher.py — python3-dbus stub StatusNotifierWatcher, private dbus session + xvfb; lifecycle eprintln proves which path ran). Run 44444569651 (c8fab2e) was green ×3 but ubuntu ran the FALLBACK path — stub watcher crashed at startup (dbus-python RuntimeError: no main loop attached); the log proof caught it. `cbff5f9`: `DBusGMainLoop(set_as_default=True)` + `GLib.MainLoop().run()` + `python3-gi` + `kill -0` watcher liveness check (dead watcher = hard CI fail). Run 44445672484 (cbff5f9) green ×3 at 02:42:30Z; Forge to report the ubuntu path line verbatim. Conditional fallback stays as safety net.

- **Tray pixel check** (cosmetic): icon registration + full functional path verified; literal taskbar screenshot pending (taskbar auto-hidden during test; do it when naturally visible).
- Restore-channel hardening (Windows DACL), zbus 5.19+6.3 lock duplication comment, restore-test listener leaks, restore-bind race window.
- Phase 6: gold #B08A4A unused + Today-panel tint undocumented; banner i18n; now-clock display.
- Phase 4: "MORNING ATHKAR" label shown pre-Fajr (static card until engine lands).
- ~~README stale (says Phase 1 / 42 tests)~~ refreshed 2026-11-19 (heartbeat docs pass).

## Heartbeat log (latest first)

- **2026-11-19 06:20 AST (03:20 UTC)** — **GATE TAKEOVER + MERGE DONE.** Window 02:50–03:05 UTC passed with no interactive gate session; per the 05:44 beat's own rule ("if no gate activity by ~02:52Z, the heartbeat takes over"), announced takeover + extended GUI window 03:05–03:25 UTC in PR (comment 6998413066) at 03:04Z. Ran the full gate in the atlas worktree @ `cbff5f9`: fmt ✓, clippy ✓ (14.6s), taskkill of the user's running instance (PID 44904, announced), BUILD OK 06:05:37 +03, `cargo test --workspace` **exit 0 — 105 tests, 0 failures** (tawadu-app unit 34, core 70, `tray_lifecycle` 5-phase GUI gate ok 167.97s on the **REAL Windows tray path** — phase 1 'REAL tray path confirmed (hidden to tray)', phase 5 'hide-flush verified'). **Squash-merged PR #2 → `638ec36` (03:15:40Z)**, remote branch deleted. Post-merge cleanup: local rescue branch deleted (never existed remotely), atlas worktree removed, step23 worktree detached @ `638ec36` + stale feat branch deleted + `scripts/ci/__pycache__/` removed, user app rebuilt from merged main + **relaunched (PID 42304 @ 06:18 AST)**. **Phase 3 (audio) dispatched to Forge 03:17 UTC** — full spec posted as PR comment 6998677001 (tawadu-audio crate, pack importer + sha256 + duration probe, starter pack A1/A2 with licence entries + `verify_audio_sources.py` provenance, Prism's settings UI, scheduler integration, offline-first); quality bars: TDD RED-first, honest states, green gate per commit, GUI windows announced, Sentinel reviews, Atlas gates (audible check for chime/packs). Next beat: verify Forge started (log freshness; nudge if idle >~45 min after dispatch) + post-merge CI on `638ec36`.
- **2026-11-19 05:44 AST (02:44 UTC)** — Monitor-only beat (window 02:50–03:05 UTC opens in ~6 min). Head moved twice since last beat: `c8fab2e` (stub SNI watcher; CI 44444569651 green ×3) → Forge 02:38 honest-path report: ubuntu silently ran the **FALLBACK path** (watcher crashed at startup — dbus-python RuntimeError without an attached main loop; exactly what the log-proof eprintln is for) → **`cbff5f9`** (02:37:27Z): GLib main loop fix + `python3-gi` + `kill -0` liveness check (dead watcher = hard CI fail — no more silent fallback greens). **CI ×3 GREEN on `cbff5f9`** — run 44445672484: ubuntu ✓ macos ✓ windows ✓ (completed 02:42:30Z). Executed: Atlas worktree fast-forwarded to `cbff5f9` (detached, clean). All merge conditions satisfied — next beat verifies the gate ran + squash-merge landed; stall line ~03:10 UTC. Forge active in reservation (log fresh 02:41Z) — no nudge. Nit: `scripts/ci/__pycache__/` untracked in step23 worktree (harmless; post-merge cleanup candidate).
- **2026-11-19 05:19 AST (02:19 UTC)** — Monitor-only run (inside Forge's 01:55–02:25 reservation; Atlas window 02:26–02:40 UTC still ahead). State moved since last beat: **`5e1d43a` (option a) FAILED CI — snixembed absent on ubuntu-24.04** (`E: Unable to locate package snixembed`, run 44440795375); Forge pushed **`9566a0f`**: YAML-only revert to sanctioned option (b) (tray-or-fallback conditional `505eb08`), Rust tree identical to fully gated `5e1d43a`. **CI ×3 GREEN on `9566a0f`** — run 44442381908: macos ✓ ubuntu ✓ windows ✓ (completed 02:10:43Z, all 3 jobs verified individually). PR #2: open, head 9566a0f, mergeable_state clean, 10 commits. Sentinel approve stands (01:51Z) — post-approve deltas are CI/test/docs only, no product code. tawadu-atlas worktree still @ e670f70 (parallel Atlas session must fast-forward to 9566a0f before its GUI gate). Forge active (log fresh 02:18Z, in-window) — no nudge. Coordination comment posted on PR (head move + CI green + window unchanged + ubuntu-tray follow-up). Next beat (~05:4x AST): confirm Atlas gate ran at final head + merge landed → post-merge cleanup (rescue branch + atlas worktree) + dispatch Phase 3 audio to Forge. Stall line: PR still unmerged past ~02:50 UTC → investigate.
- **2026-11-19 04:59 AST** — Monitor-only run (per `f424f40` + PR comment 01:46:45Z). Verified: head `5e1d43a` (snixembed SNI host + stale-prayer rule + TOUCH nit), CI 44440795375 in flight; `505eb08` already ×3 green (44439767131). Sentinel APPROVE in hand since 01:51Z. ubuntu root cause closed: CI xvfb has no StatusNotifier watcher → app correctly falls back KeepWindowShown (ADR 0002 cond. 4) → hard hide-assert panicked at tray_lifecycle.rs:492 (runs 44437829313, 44439110058); fixed by `505eb08` (test branches on tray availability) + `5e1d43a` (real SNI host so ubuntu exercises the real tray path). Parallel Atlas session owns gate+merge (tawadu-atlas worktree @ e670f70, non-GUI gate done; GUI window 02:10–02:25 UTC, extend if Forge's 01:53–02:23 window runs long). Next beat: if CI on `5e1d43a` ×3 green + Atlas gate done → confirm merge; else watch.
- **2026-11-19 04:25 AST** — PR #2 open (4 slices), Forge mid-wiring (uncommitted WIP, log fresh). **CI anomaly:** pull_request event dropped — push 01:07:03Z + PR open 01:07:29Z raced; 18+ min, zero check suites on `bb301ff`. Fixed forward: `c0ee22f` adds `workflow_dispatch` to ci.yml (main; PR branch gains it on next rebase/merge). Forge's next push fires the normal pull_request event; if it doesn't, Atlas re-triggers manually. GUI window 01:08–01:42 UTC respected — no local GUI test before close. Sentinel: staged review (final pass after wiring) — watch for staleness next beat.

## Heartbeat (for scheduled sessions)

Read this file → check `gh pr list`, latest CI runs, team log freshness
(profiles/{forge,sentinel,prism,scout,quill}/logs/agent.log mtimes) → execute the
next unblocked action yourself or DM the owner (message_agent; hop limit 12) →
update this file's snapshot. Never wait passively on notifications; poll state.
Escalate to the user only for irreversible/money decisions or true blockers.
