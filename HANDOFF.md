# Tawadu — HANDOFF

> Written 2026-11-19 for the next team (human or agent).
> **Read [`docs/ROADMAP.md`](docs/ROADMAP.md) alongside this** — it is the plan of
> record and carries the full phase list, pitfalls and open decisions. This file is
> the orientation: what exists, what does not, and where to put your hands first.

---

## 1. The goal

**Tawadu (تواضع) is a calm, offline-first, no-ads, open-source prayer companion for
the desktop.** Accurate prayer times, real adhan audio, your mosque's iqamah, and
your athkar. No account, no telemetry, no ads, no tracking.

It exists because the incumbent apps are paid, ad-filled, stuck in a 2000s UI, or
missing whole features. The bar is therefore not "works" — it is **quiet, fast,
honest, and beautiful**.

**Non-negotiable principles** (do not trade these away; they are why the project
exists):

1. **Offline-first.** No account, no telemetry, no ads. Network features are opt-in
   and must degrade silently.
2. **Astronomy in UTC; local offset only at the edge.** Never convert inside a
   calculation module.
3. **"Next prayer" means the next *obligatory* prayer.** Sunrise and qiyam are not
   prayers; including them names the wrong event after Isha.
4. **The day is one ordered timeline of events**, and "what is next?" is answered
   from that timeline — not by scheduling each feature separately.
5. **Never invent a time.** A polar day with no sunrise is `None` and the UI says so.
6. **Cross-source differences up to ~2 minutes are normal.** Expose per-prayer
   adjustments instead of claiming one source is exact.
7. **Assets keep their own licences.** Never commit an asset without a licence entry
   in `assets/licenses/`.
8. **Look and feel is a feature, not a coat of paint.**

**Platforms:** Windows 10/11 first, then macOS, then Linux. Arabic + English at
launch; RTL is unverified and is the single biggest UI unknown.

**Licence:** MIT for code. Bundled assets stay under their own licences.

---

## 2. Tech stack and folder structure

### Stack

| Layer | Choice | Why |
|---|---|---|
| Language | **Rust** | one codebase, three OSes |
| UI | **Slint 1.19** | pure Rust, no WebView2 (runs on an old Win10 PC), GPU-rendered |
| Calculation | **`adhan` 0.2** | exposes the whole config surface (`salah` kept types private — do not go back) |
| Hijri | **`hijridate`** | table-driven, not arithmetic |
| Timezones | **`chrono` + `chrono-tz`** | |
| Persistence | **`rusqlite`** (SQLite) | `%APPDATA%\tawadu\tawadu.db` on Windows |
| CLI | **`clap`** | `tawadu times`, `tawadu cities` |
| OS notifications | **`notify-rust` 4** | with an in-app banner as the correctness floor |
| Single instance | **`single-instance` 0.3** | |
| Fonts / brand | see `design/` | tokens are the source of truth |
| CI | GitHub Actions | fmt + clippy `-D warnings` + tests on ubuntu / windows / macOS |

### Layout

```
crates/
  tawadu-core/     pure logic — no UI, no platform types (shareable with mobile later)
    src/prayer.rs      688 LOC  adhan wrapper, transit anchoring, Option times, adjustments
    src/scheduler.rs   328 LOC  day timeline, EventKind, due() sweep, IqamahOffsets
    src/settings.rs    161 LOC  the one serialisable Settings blob
    src/storage.rs     270 LOC  SQLite + LoadOutcome
    src/notify.rs      216 LOC  NotificationKind, quiet hours, pure should_notify()
    src/locations.rs   354 LOC  28 bundled cities with IANA timezones + search
    src/hijri.rs       101 LOC
    src/qibla.rs        80 LOC
    src/athkar.rs        1 LOC  stub — Phase 4
    src/lib.rs          22 LOC
  tawadu-audio/    10 LOC  empty skeleton — Phase 3
  tawadu-app/      Slint shell
    src/main.rs      1033 LOC  UI, state, callbacks, scheduler thread, banner
    src/cli.rs        181 LOC
ui/
  main.slint       810 LOC  home + settings + banner + `export global Palette`
  types.slint       26 LOC  CityRow / MethodRow / OptionRow
assets/            voices, athkar corpus, sounds, fonts (all empty but .gitkeep)
design/            brand.md, style-guide.html, logos, banners
docs/              ROADMAP.md (plan of record), architecture.md, adr/, voice-pack-format.md, attribution.md
```

### Repo facts

- **Remote:** `https://github.com/tawadu/tawadu` (org `tawadu`, public)
- **Branch:** `main`. **HEAD at handoff:** `d8d8b1e`
- **Git identity is repo-local:** `XGradien <173515018+XGradien@users.noreply.github.com>`
- **Working tree is clean.**
- **Tests:** 104 (69 core + 34 app unit + 1 process-level lifecycle, 5 phases; macOS adds 1 socket-permission test), `fmt` + `clippy -D warnings` clean.

### Build and verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p tawadu-app          # then run target/debug/tawadu-app.exe
cargo run -p tawadu-app -- times --city "Riyadh" --method umm-al-qura
cargo run -p tawadu-app -- cities
```

> **Windows trap (has bitten us twice):** a running instance locks
> `target/debug/tawadu-app.exe`, so `cargo build` fails with "Access denied" and you
> silently launch the **stale** binary — which looks exactly like a regression.
> `taskkill /F /IM tawadu-app.exe` **before** building, and confirm `BUILD OK` plus the
> exe timestamp before drawing any conclusion from a run.

### The controlled clock (essential for testing notifications)

The app reads three env vars, all off by default:

| Var | Effect |
|---|---|
| `TAWADU_DEBUG_NOW` | RFC 3339 base instant; advances in real time from process start |
| `TAWADU_DEBUG_TIME_SCALE` | multiplies elapsed real time (e.g. `12` = 60 s behaves as 12 min) |
| `TAWADU_DEBUG_BANNER_MS` | overrides the 8 s banner auto-hide |
| `TAWADU_DEBUG_AUTOCLOSE_MS` | dispatches a real `CloseRequested` after N ms (tray regression test) |
| `TAWADU_DEBUG_AUTOQUIT_MS` | calls `quit_event_loop()` after N ms (tray regression test); flushes pending debounced save |
| `TAWADU_DEBUG_TOUCH_SETTING_MS` | mutates `pre_alert_minutes` to 15 via the normal change path after N ms (debounce durability test) |
| `TAWADU_DEBUG_MAX_SLICE_MS` | overrides the scheduler's 60 s max sleep slice (REAL time). Needed at high time scales: a 60 s slice at scale 12 advances the debug clock 720 s, past the 120 s late tolerance, so every event is dropped. Use `5000` with scale 12 |

This is how the sleep/wake path was proven without putting the machine to sleep:
launch 1 minute before the Fajr pre-alert with scale `12`, wait ~65 s, and the sweep
fires the athan and iqamah while correctly dropping the stale pre-alert.

---

## 3. What is already done

### Phase 0 — the core (complete)

- **Prayer times** for all 13 methods incl. Umm al-Qura; Shafi/Hanafi Asr;
  high-latitude rule; rounding; per-prayer manual adjustments.
- **Transit anchoring.** `adhan` returns date-less UTC clock readings, so `prayer.rs`
  reconstructs the absolute instant against the day's transit. Without this, far-east
  cities (Auckland, UTC+12) shift by a whole day.
- **`Option` everywhere a time can be undefined** (polar day/night). Never a
  substituted value.
- **Qibla**, **Umm al-Qura Hijri** (table-driven), **28-city database** with IANA
  timezones and English/Arabic prefix + substring search.
- **Day timeline** (`scheduler.rs`): prayers, sunrise, iqamah offsets, pre-alerts,
  athkar windows — one ordered list, with a stable `priority()` for ties.
- **`Timeline::due(after, now, tolerance)`** — the sleep/wake sweep.
- **Typed settings + SQLite persistence** with `LoadOutcome`.
- **CLI**: `tawadu times`, `tawadu cities`.
- **60 core tests**, including a polar-day case, a far-east anchoring case, the
  sleep/wake sweep, and proof that a corrupt blob is not a fresh install.

### Phase 1 — usable (complete)

- **Home surface**: live `HH:MM:SS` countdown ticked from a single timer, next prayer,
  next iqamah card, athkar card, today's list.
- **Settings surface**: location search, method, madhab, high-latitude rule, rounding,
  pre-alert minutes, per-prayer iqamah steppers, theme, language — all persisted.
- **Write path verified end to end** on a real desktop (edit → حفظ → read the DB).
- **Three themes** (Plaster / Ink / Ramadan) actually applied end to end.
- **Single-instance** enforcement.
- **Storage failures surfaced** instead of silently becoming defaults.

### Phase 2 — notifications (engine done, shell incomplete)

- **Scheduler thread**: sleeps in ≤60 s slices, recomputes every slice (correct across
  clock change, wake, and settings edits), sweeps `(watermark, now]` after each wake,
  drops events older than 120 s.
- **OS notifications** via `notify-rust` **plus an always-shown in-app banner** — the
  banner is the correctness floor, because `notify-rust` returning `Ok` is not proof a
  toast appeared (an unpackaged Windows exe needs an AUMID).
- **Quiet hours** (overnight windows supported) and per-kind switches; the athan is
  **never** suppressed by quiet hours.
- **Notification controls UI**: five toggles + quiet-hours toggle + from/to fields,
  wired to `NotificationSettings` (keys match the enum's `snake_case` serde names).

### Design

- Brand identity, logo set, style guide, three themes, OG/wide banners in `design/`.
- Tokens: bg `#F6F1E8`, surface `#FFFDF8`, surface-secondary `#EFE8DB`, ink `#211C15`,
  ink-muted `#6E665A`, border `#E2D9C9`, accent `#2E5C4E`, accent-soft `#E3EEE9`,
  gold `#B08A4A`. Fonts: IBM Plex Sans + IBM Plex Sans Arabic (UI), Amiri (scripture).

### Docs

`docs/ROADMAP.md` (phases 0–7, pitfalls, debt, open decisions),
`docs/architecture.md`, `docs/adr/0001-slint-ui.md`,
`docs/voice-pack-format.md`, `docs/attribution.md`,
`assets/licenses/README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`.

---

## 4. Bugs and unfinished work

### Blocking / user-visible

1. **Closing the window ends the scheduler.** ~~The tray is **blocked**~~
   **RESOLVED 2026-11-19:** the "no close hook" claim was a false negative —
   Slint 1.19 has a public `Window::on_close_requested`; the loop quits only
   because `hide()` drops the last keepalive. Unanimous panel chose option (c):
   close hook + `KeepAlive` + `tray-icon`/`muda`. See
   [`docs/adr/0002-tray-close-to-tray.md`](docs/adr/0002-tray-close-to-tray.md).
   **IMPLEMENTED** on `feat/tray-close-to-tray` (tray, banner floor, AUMID,
   Linux SNI fallback, macOS Cmd+Q, second-launch restore; process-level
   regression test). Pending: desktop exit test + merge.
2. **No audio at all.** `tawadu-audio` is a 10-line skeleton. No athan, no athkar.
   This is the feature the user originally asked for.
3. **The athkar card is hardcoded** — it shows one static dhikr. (The fabricated
   "4 / 9" counter was removed rather than papered over.)
4. **Iqamah offsets default to `0`**, so no iqamah events are ever scheduled until the
   user sets them. Deliberate, but the empty state must be obvious.

### Known defects, recorded but not fixed

5. ~~**No fired-event ledger** — a restart within 120 s of an event re-announces it.~~
   **RESOLVED (Steps 2–3):** schema-v2 `fired(kind, prayer, at)` ledger; the fire is
   persisted BEFORE the announcement; `fireable` filters per identity (fail-open).
6. ~~**Quiet hours are evaluated at fire time, not event time**~~ **RESOLVED
   (Steps 2–3):** judged at the event's scheduled local instant; the athan is
   essential (`is_essential()` returns before the quiet-hours check) and never suppressed.
7. ~~**Settings save only on the حفظ button**, while Phase 1 specifies a debounced
   save-on-change. **Code and doc disagree — pick one.**~~ **RESOLVED (Steps 2–3):**
   debounced save-on-change implemented — 800 ms after the last mutation
   (drained on a 200 ms UI tick), حفظ remains an immediate save that cancels
   any pending debounce write.
8. ~~**No autostart.**~~ **RESOLVED (Steps 2–3):** opt-in toggle in APPEARANCE
   (default OFF, `serde(default)` so old DBs upgrade to OFF); startup reconcile is
   idempotent and re-registers when the exe path moved (Windows HKCU Run path check).

### Debt and unknowns

9. **Hardcoded UI strings** in `.slint` — no real i18n, no Fluent catalogs.
10. **RTL and Arabic shaping are unverified** on macOS and Linux.
11. **The README is stale** — it says "Phase 1 in progress" and "42 tests".
12. **Performance budgets are unmeasured** (targets: idle CPU ≈0 %, memory <120 MB,
    cold start <1.5 s, athan latency <300 ms).
13. **No packaging** — nothing to hand to a non-developer.
14. **No UI tests.**
15. **Only 28 cities**, no GPS.
16. `assets/` is empty (no voices, no athkar corpus, no fonts committed).

### Asset licensing state (important)

- **MoIA (Ministry of Islamic Affairs) gave a *verbal* clearance** for their athan and
  athkar recordings. **No written letter.** Keep the message saved; stores may ask for
  proof. The user explicitly said to stop raising this — it is recorded here so the
  next team does not rediscover it by accident.
- `Kiwifu/adhan-mp3` — **no licence, do not bundle.**
- `assabile.com`, `mohammed-2-5/islamic-library-data` — no licence, avoid.
- Verified usable: Wikimedia Commons, `YousefAsalya/Islamic-Pro-azkar-API` (MIT),
  OpenDua (CC BY 4.0).
- **Never commit an asset without a licence entry.**

---

## 5. Exact next steps

Ordered by user value per unit of risk. Each ends with the same green gate:
`cargo fmt --all` → `cargo clippy --workspace --all-targets -- -D warnings` →
`cargo test --workspace` → CI green on all three OSes → a visual check on a real
desktop.

### Step 1 — Decide the tray question (small, unblocks everything)

> **DONE 2026-11-19 — merged PR #1 (3f0b578).** Option (c) shipped: public
> `on_close_requested` + `KeepAlive` + `tray-icon`/`muda`, banner floor while
> hidden, AUMID, Linux SNI fallback, second-launch restore, alerts-while-hidden
> regression test. Evidence and conditions:
> [`docs/adr/0002-tray-close-to-tray.md`](docs/adr/0002-tray-close-to-tray.md).
> Next: Step 2 (autostart) + Step 3 (defects) — see PIPELINE.md.
> The original options are kept for the record:

Either:
- **(a)** add a backend-specific close hook (winit/`windows` crate) so closing hides
  to the tray, then add `tray-icon` + `muda` with a generated RGBA icon and a
  Show/Quit menu; or
- **(b)** move the scheduler into a hidden secondary window that owns the tray, leaving
  the main window free to close.

Then build the tray icon (a generated rounded-square RGBA mark avoids an image
dependency), and wire the menu through `MenuEvent::receiver()`.

**Exit:** close the window → the tray icon remains → a pre-alert, an athan and an
iqamah still fire, including after the machine sleeps.

### Step 2 — Autostart (opt-in, off by default)

`Settings.start_at_login` already exists. Wire it to `auto-launch`, default **off**,
and surface a switch in APPEARANCE.

### Step 3 — Fix the three recorded defects (cheap, do them together)

- A **fired-event ledger** (persist the last fired `event.at` per kind) so a restart
  does not re-announce.
- Evaluate quiet hours at **event time**, not fire time.
- ~~**Pick the save model** and make code and doc agree (recommend debounced
  save-on-change, as documented).~~ **Done (Steps 2–3):** debounced
  save-on-change (800 ms) implemented; حفظ stays immediate.

### Step 4 — Phase 3: audio

1. `tawadu-audio`: output via `rodio`, decode via `symphonia`, a mixer with ducking,
   fade in/out, per-pack volume, hard stop.
2. The **voice-pack format** in `docs/voice-pack-format.md` (folder + manifest + audio
   + licence).
3. Sequential and shuffled series across packs; **trim a recording after N seconds**.
4. An importer (file picker via `rfd`) that validates the manifest **and the licence**.
5. Bundled starter packs with attribution.
6. Per-event voice choice (athan / iqamah / athkar / chime).

**This is what the user actually asked for**: many voices, sequence them or shuffle
them, and cut the athan after a chosen length.

### Step 5 — Phase 4: the athkar engine

Import a licensed corpus (MIT Hisn al-Muslim is the candidate) into SQLite with
categories, repetitions and Arabic/English text. Contexts: morning, evening, sleep,
after-prayer, random, manual. Adaptive windows with catch-up and a "do not repeat
until read" rule. Then the card shows a **real** dhikr with a **real** progress count.

### Step 6 — Then, in order

Phase 5 (GeoNames coverage, GPS, mosque profiles, Ramadan mode, calendar views) →
Phase 6 (Fluent i18n + RTL verification, fonts, measure the budgets, packaging) →
Phase 7 (mobile, only after Phase 6).

---

## 6. Conventions the team has agreed

- **TDD.** Write the failing test first; the green gate is mandatory, not optional.
- **Decisions go to a subagent panel first.** The user's standing instruction:
  consult subagents across several free models and **reach a unanimous call before
  committing to a direction**. Pin via `hermes config set delegation.model <id>`
  (the config file is write-protected to the agent). Match the model to the job —
  coding models for code, vision-capable models for screenshots, writing models for
  copy. `qwen/qwen4-max:free` has been used and works.
- **Never commit an asset without a licence entry.**
- **Do not change the repo-local git identity** without asking.
- **Never fabricate a UI value.** The "4 / 9" counter was removed rather than made
  plausible; a visible lie in a religious app is worse than an empty card.

### Slint gotchas learned the hard way (read before touching `ui/`)

- **A `Text` inside a fixed-height `Rectangle` must set `height: parent.height`**;
  `vertical-alignment: center` alone centres within the text's own box and glyphs sit
  high.
- **Arabic and Latin never share one `text:` string.** Bidi reorders them (a hijri
  date rendered as `Riyadh · 1448 جمادى الآخرة 8 · الرياض`). Use separate `Text` runs.
- **A `Rectangle` outside a layout with no `height` fills its parent** — that is how
  the banner became a 360×620 slab.
- **Layouts default to `alignment: Stretch`**, so a short column's cards inflate to a
  tall column's height. Set `alignment: start` when you want natural heights.
- **A `global` declared in an *imported* file is inlined away** and unreachable from
  Rust. Declare it in the compiled root (`ui/main.slint`) to get a `global_*` accessor.
- **Component definitions must precede their use** in the file (`RowText` had to move).
- Callbacks take **unnamed** parameters; `component X extends Y` → `inherits Y`;
  `TextInput.edited` takes **no arguments** and you read `self.text`; use `SharedString`
  in callback closures; callbacks must be registered on the **strong** handle while
  closures capture a `Weak`.
- The settings page is inside a `Flickable` because five cards outgrow the window.

---

## 7. Quick reference

| | |
|---|---|
| Repo | https://github.com/tawadu/tawadu |
| Plan of record | `docs/ROADMAP.md` |
| Tests | 104 (69 core + 34 app unit + 1 lifecycle, 5 phases; +1 macOS-only) |
| Settings DB | `%APPDATA%\tawadu\tawadu.db` (Windows) |
| Default settings | Riyadh · Umm al-Qura · pre-alert 10 · all notifications on · quiet hours 22:00–06:00 (off) |
| Verified timings 2026-11-17 | Riyadh: Fajr 04:49 · Sunrise 06:11 · Dhuhr 11:38 · Asr 14:44 · Maghrib 17:06 · Isha 18:36 (UTC+3) |
| Debug clock | `TAWADU_DEBUG_NOW`, `TAWADU_DEBUG_TIME_SCALE`, `TAWADU_DEBUG_BANNER_MS`, `TAWADU_DEBUG_AUTOCLOSE_MS`, `TAWADU_DEBUG_AUTOQUIT_MS`, `TAWADU_DEBUG_MAX_SLICE_MS`, `TAWADU_DEBUG_TOUCH_SETTING_MS` |

**First thing to do:** read `docs/ROADMAP.md` end to end, run the green gate, launch
the app, and confirm the home screen shows Riyadh with a live countdown. Then Step 1.
