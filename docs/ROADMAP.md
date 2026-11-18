# Tawadu — roadmap

A calm, offline-first, no-ads, open-source prayer companion.
Cross-platform (Windows first, then macOS and Linux), Arabic + English at launch.

This document is the plan of record. The phase list is ordered by **user value per
unit of risk** — make it usable, then make it notify, then make it sing, then make
it beautiful and widely available.

---

## 1. Where we are

**Done and verified**

- **Core (`tawadu-core`)** — prayer times via `adhan` 0.2 (all 13 methods, Shafi/Hanafi,
  high-latitude rule, rounding, per-prayer adjustments), Qibla, Umm al-Qura Hijri,
  a 28-city database with IANA timezones, a day timeline (prayers, iqamah offsets,
  pre-alerts, athkar windows), typed settings and SQLite persistence, and a CLI
  (`tawadu times`, `tawadu cities`).
- **Shell (`tawadu-app`)** — a Slint home surface: countdown, next iqamah, athkar
  card, today's list, live tick, persisted settings.
- **CI** — fmt, clippy `-D warnings` and tests on ubuntu / windows / macOS.
- **Brand** — logo, style guide, themes, banners (`design/`).
- **63 tests** (60 core + 3 CLI), including a polar-day case, a far-east
  UTC-anchoring case, a sleep/wake sweep proving the athan is not dropped, and
  proof that a corrupt settings blob is *not* mistaken for a fresh install.

**Known gaps (the honest list)**

| # | Gap | Consequence |
|---|---|---|
| ~~1~~ | ~~No settings screen~~ | **Done** — settings surface + SQLite write path, verified end to end |
| 2 | Athkar card is hardcoded | It is a mock, not a feature |
| 3 | Iqamah offsets default to `0` | No iqamah events are ever scheduled |
| 4 | Tray decided (ADR 0002), implementing; autostart pending | Alerts still end when the window closes — until Step 1 lands |
| 5 | `tawadu-audio` is an empty skeleton | No athan, no athkar audio |
| 6 | Strings are hardcoded in `.slint` | No real i18n; RTL unverified |
| 7 | 28 cities, no search, no GPS | Most users cannot set their town |
| ~~8~~ | ~~Storage errors swallowed with `.ok()`~~ | **Done** — `LoadOutcome` separates fresh / corrupt / failed, and the UI says so |
| 9 | Performance budgets unmeasured | We cannot claim the numbers in the plan |
| 10 | No packaging | Nothing to hand to a non-developer |

---

## 2. Principles (do not trade these away)

1. **Offline-first.** No account, no telemetry, no ads, no tracking. Network features
   are opt-in and must degrade silently.
2. **Astronomy in UTC; local offset only at the edge.** Never convert inside a
   calculation module.
3. **"Next prayer" means the next *obligatory* prayer.** Sunrise and qiyam are not
   prayers; including them names the wrong event after Isha.
4. **The day is an ordered timeline of events**, and "what is next?" is answered
   from that timeline — not by scheduling each feature separately.
5. **Never invent a time.** A polar day with no sunrise is `None`, and the UI says so.
6. **Cross-source differences up to ~2 minutes are normal.** Expose per-prayer
   adjustments instead of claiming one source is exact.
7. **Assets keep their own licences.** Never commit an asset without a licence entry
   in `assets/licenses/`.
8. **Look and feel is a feature, not a coat of paint.**

---

## 3. Phases

Each phase ends with a **green gate**: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, a successful three-OS CI run,
and a visual check on a real desktop.

### Phase 1 — Make it usable (settings + the write path)

**Why first:** without it the app is a demo. Everything else assumes the user can
choose a city, a method and iqamah offsets.

**Tasks**

1. Settings surface in Slint: location search, method, madhab, high-latitude rule,
   rounding, per-prayer iqamah offsets, pre-alert minutes, theme, language.
2. A write path: `Storage::save_settings` on change (debounced), read on launch.
3. Decide the iqamah default. Recommendation: `0` (meaning "not set") plus a clear
   empty state, rather than inventing offsets.
4. A header control that opens settings; the home screen refreshes on return.
5. Replace swallowed storage errors with a visible, non-fatal state.
6. City search that matches English and Arabic, prefix + substring, with a
   "use my coordinates" manual entry.

**Exit:** change city → close → reopen → the new city and every dependent time
survive; iqamah offsets produce real iqamah events.

### Phase 2 — The notification engine and tray

**Why second:** the app's whole point is alerting. This is the highest-value slice
after usability.

**Tasks**

1. A real scheduler loop over the day timeline: sleep until the next event, fire it,
   advance. Recompute on start-up, settings change, clock change, and wake from sleep.
2. Notification taxonomy: pre-alert, athan, iqamah, athkar, system — each with its own
   sound, quiet hours and per-event enable/disable.
3. OS notifications (`notify-rust`) with an in-app fallback when the platform has no
   notification service.
4. System tray icon + menu (show, settings, quit), single-instance enforcement.
5. Autostart on login (opt-in, off by default).
6. Quiet hours that suppress non-essential notifications without suppressing the athan.

**Exit:** with the app closed to the tray, a pre-alert, an athan and an iqamah each
produce a notification at the right moment, including after the machine sleeps.

### Phase 3 — Audio: athan and voice packs

**Tasks**

1. `tawadu-audio`: output via `rodio`, decoding via `symphonia`, a mixer with ducking,
   fade in/out, per-pack volume, and a hard stop.
2. The voice-pack format from `docs/voice-pack-format.md`: a pack is a folder with a
   manifest, audio files and a licence.
3. Sequential and shuffled series across packs; trim a recording after N seconds.
4. An importer (file picker via `rfd`) that validates the manifest and the licence.
5. Bundled starter packs with attribution (MoIA recordings; Wikimedia Commons for
   permissive alternatives).
6. Per-event voice choice: athan, iqamah, athkar, notification chime.

**Exit:** an athan plays at the correct moment, can be swapped for another voice,
and a pack can be cut to a chosen length and replayed.

### Phase 4 — The athkar engine

**Tasks**

1. Import a licensed athkar corpus (the MIT-licensed Hisn al-Muslim set is the
   candidate) into SQLite with categories, repetitions and Arabic/English text.
2. Contexts: morning, evening, sleep, after-prayer, random, and a manual pick.
3. Adaptive windows with catch-up, and a "do not repeat until read" rule.
4. Word-by-word highlighting where timestamps exist; a tasbih counter with a target.
5. Share-as-card (an image) and a personal athkar list.

**Exit:** the athkar card shows a real dhikr with a real progress count, and the
right context fires at the right window.

### Phase 5 — Accuracy and coverage

**Tasks**

1. Replace the 28-city list with a full GeoNames-derived database (bundled, with a
   licenced attribution), plus search and optional GPS.
2. A high-latitude UX: explain what the rule does, and show `None` honestly.
3. Mosque profiles (name, iqamah offsets, location) so a user can keep several.
4. Hijri/Gregorian calendar views and a Ramadan mode.

**Exit:** a user in Oslo, Auckland and Riyadh gets correct, explainable times, and a
user in Tromsø in June is told the truth rather than shown a fabricated sunrise.

### Phase 6 — Polish, i18n, accessibility, performance, packaging

**Tasks**

1. Full i18n (Fluent) for Arabic and English, with a RTL layout pass; verify shaping
   on all three OSes — **this is the single biggest UI unknown.**
2. Fonts: Amiri for scripture, IBM Plex Sans Arabic for UI; licence entries.
3. Ship the theme system (Plaster / Ink / Ramadan) as user-selectable skins.
4. Measure the budgets: idle CPU ≈ 0 %, memory < 120 MB, cold start < 1.5 s,
   athan latency < 300 ms. Enforce with a benchmark, not a claim.
5. Packaging: Windows installer, macOS `.app`, Linux AppImage/deb. Code-signing is a
   later, separately-funded decision.
6. A first-run experience and an in-app about/licences page.

**Exit:** an installer exists for each platform, the budgets are measured, and the
Arabic UI is verified on real machines.

### Phase 7 — Mobile (only after desktop matures)

Keep `tawadu-core` platform-agnostic (it already is) and add a Slint mobile shell.
Do not start this before Phase 6 is done.

---

## 4. Keep in mind (pitfalls)

- **Always confirm the build succeeded before launching.** On Windows a running
  instance locks `target/debug/*.exe`, so `cargo build` fails with "Access denied"
  and you end up launching a stale binary — which looks exactly like a regression.
  Check for `BUILD OK` and the exe timestamp.
- **Cross-source variance is not a bug.** Asr especially differs by ~2 minutes.
  Assert within 2, and give the user adjustments.
- **The reference API takes `DD-MM-YYYY`.** `YYYY-MM-DD` is silently misread and
  returns a different day, producing a fixture that looks like a maths bug.
- **Derive "current/next" from an injectable `now`**, never from a clock read inside
  a calculation module.
- **Check that a crate's configuration types are nameable from outside** before
  building on it. `salah` kept `HighLatitudeRule` private; that cost a migration.
- **Slint RTL and Arabic shaping are unverified.** Assume nothing until it is
  screenshotted on Windows, macOS and Linux.
- **A timer callback must not redo expensive work.** Compute once, tick the text.
- **Do not bundle an asset without a licence entry.** One missing file can force a
  re-release.
- **The MoIA clearance is verbal.** Keep the message saved; stores and platforms may
  ask for written proof.

---

## 5. Things to change (known debt)

1. **Hardcoded UI strings** → Fluent catalogs (Phase 6, but start extracting now).
2. **Hardcoded athkar card** → real engine (Phase 4).
3. **Swallowed storage errors** (`.ok()` everywhere) → a `Result` that surfaces a
   non-fatal warning state.
4. **`#![allow(clippy::derivable_impls)]`** in `lib.rs` — derive `Default` properly
   once the config types settle.
5. **`Settings::default()`** is used as the fallback for a failed load; separate
   "no settings yet" from "settings failed to load".
6. **No UI tests.** Add Slint-side tests for the screens that matter.
7. **The day timeline is rebuilt on every refresh.** Fine at this scale; revisit if
   the tick ever costs more than a few hundred microseconds (it is ~165 µs now).

---

## 6. Open decisions

| Decision | Options | Recommendation |
|---|---|---|
| Iqamah defaults | `0` / sensible guesses | `0` + explicit empty state |
| City data | bundled GeoNames / online lookup | bundled, with opt-in online refresh |
| Audio sources | MoIA only / plus Commons | MoIA primary, Commons for permissive extras |
| Notifications | OS only / OS + in-app | both, OS primary |
| Mobile | Slint / native | Slint, after desktop matures |
| Signing | now / later | later, when there are real users |

---

## 7. Immediate next steps

Phase 2's engine is built and verified; **Phase 2 is not closed** — see gap #4.

1. ~~**Single-instance first.**~~ **Done** — a second launch exits at once, and the
   CLI still runs alongside a live GUI. (The handle is deliberately leaked: dropping
   it releases the lock.)
2. ~~**Notification controls UI**~~ **Done** — five per-kind switches plus quiet hours
   are on screen and reach `NotificationSettings`.
3. ~~**Surface storage errors**~~ **Done** — see gap #8.
4. **Tray — implemented, pending exit test.** The "no close hook" premise was a false
   negative (the previous team grepped only the `slint` facade crate). Slint 1.19
   exposes `Window::on_close_requested` publicly; the missing piece is a
   `KeepAlive` so hiding the last window does not quit the loop. Unanimous panel
   (Forge/Sentinel/Prism, 2026-11-19) chose option (c): public close hook +
   KeepAlive + `tray-icon`/`muda`. See [`adr/0002-tray-close-to-tray.md`](adr/0002-tray-close-to-tray.md)
   for evidence and the binding conditions (banner floor while hidden, AUMID,
   Linux fallback, component lifetime).
5. **Autostart** (opt-in, off by default).
6. Only then Phase 3, audio. An athan with no tray and no mute is an app you cannot
   silence — the exact complaint this project exists to fix.

### Found in review, 2026-11-18

A four-way review (three decision panels + a defect pass, run on a second model)
converged on the ordering above and caught real defects:

- **Fixed:** the scheduler fired only the one event it was waiting on, so a machine
  that slept past the pre-alert *and* the athan dropped the athan — the one alert that
  must never be missed. `Timeline::due` now sweeps `(watermark, now]` after every wake;
  verified end to end with a 12× debug clock.
- **Fixed:** the in-app banner had no height, so it defaulted to a 360×620 slab over
  the left third of the window.
- **Fixed:** the athkar card showed a fabricated "4 / 9" progress next to a real
  dhikr that says "one hundred times" — a visible lie in a religious app. Removed.
- **Fixed:** yesterday's timeline was discarded at local midnight, dropping a late
  Isha iqamah or sleep-athkar window in high-latitude locations.
- **Open:** no fired-event ledger, so a restart within 120 s re-announces an event.
- **Open:** quiet hours are evaluated at fire time rather than event time, so a
  late event after wake is judged against the wrong wall clock.
- **Open:** settings save only on the حفظ button, while Phase 1 specifies a
  debounced save-on-change. Code and doc now disagree; pick one.
- **Note:** `notify-rust` returning `Ok` is not proof a toast appeared — on an
  unpackaged Windows exe it needs an AUMID. The banner is the correctness floor.
