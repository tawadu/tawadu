# Phase 5 — Accuracy & coverage: UI proposal

> Author: Prism (Brand & UI/UX) · 2026-11-19 · status: **proposal, no code touched**
> Plan of record: [`ROADMAP.md`](ROADMAP.md) Phase 5 · Honesty rules: [`architecture.md`](architecture.md)
> Sketch: [`../design/phase5-sketch.html`](../design/phase5-sketch.html) — views: City picker · Settings (mosques + rules) · Tromsø home · Calendar; theme toggle Plaster/Ink/Ramadan; state toggles per view.
> Predecessor: [`audio-ui-proposal.md`](audio-ui-proposal.md) — same band language, same RTL rule, same gate style.
> All copy is **provisional** (Quill's catalog is the string source of truth). All times/dates in the sketch are **engine-shaped examples**; the calendar's Hijri numbers are computed live via Umm al-Qura (`Intl` islamic-umalqura) so the sketch itself never fabricates a date.

---

## 0. Contrast audit (computed, not claimed)

WCAG ratios for every pair this proposal uses (body ≥ 7:1, muted/secondary ≥ 4.5:1, non-text markers ≥ 3:1):

| Pair | Plaster | Ink | Ramadan |
|---|---|---|---|
| ink on surface (body) | 16.64 ✅ | 14.24 ✅ | 14.05 ✅ |
| ink-muted on surface | 5.57 ✅ | 7.28 ✅ | 8.34 ✅ |
| ink-muted on surface-2 | 4.64 ✅ | 6.57 ✅ | 7.58 ✅ |
| accent on accent-soft (selected) | 6.41 ✅ | 6.52 ✅ | 6.49 ✅ |
| **gold on surface (as text)** | **3.14 ❌** | 7.61 ✅ | 7.63 ✅ |
| gold on surface (as marker) | 3.14 ≥ 3 ✅ | ✅ | ✅ |

**Binding refinement (applies to the audio sketch too):** in **Plaster, gold is
non-text only** (dots, icons, hairline markers — WCAG 1.4.11 ≥ 3:1). Gold *text*
(the Athkar Arabic label) renders in Ink/Ramadan only; in Plaster that label stays
`ink` and the gold dot carries the sacred accent. Tokens are not changed — the
usage rule is. The audio sketch + proposal are patched to match (same commit).

## 1. What Phase 5 must deliver (ROADMAP exit)

> A user in **Oslo, Auckland and Riyadh** gets correct, **explainable** times, and
> a user in **Tromsø in June** is told the truth rather than shown a fabricated
> sunrise.

Four surfaces: city search over a bundled GeoNames-derived DB, mosque profiles,
high-latitude honesty UX, calendar views + Ramadan mode. Plus a settings structure
that absorbs them without bloat.

## 2. Settings structure — what moves, what stays

The band system stays (one calm scroll, no tabs — tabs would hide the honesty
controls). Changes:

| Band | Before | After |
|---|---|---|
| LOCATION (left) | search field + results inline | **shrinks to a summary card**: current city (EN + AR runs), country/admin1, coordinates + IANA timezone caption, **Change…** button → opens the dedicated City picker (§3). GeoNames attribution caption lives here *and* in the picker footer. |
| MOSQUES (left, **new**) | — | profile list + inline editor (§4). **The five iqamah steppers move here** from ALERTS — they belong to a mosque, not to the app. |
| ALERTS (left) | pre-alert + 5 iqamah steppers | pre-alert stepper only. |
| CALCULATION (right) | method lists, madhab/high-lat/rounding Segmented | method lists stay; **high-lat becomes rule rows with plain-language descriptions** (§5); rounding stays Segmented; new expandable **Fine-tune times** (per-prayer ± minutes, §5.4). |
| APPEARANCE / NOTIFICATIONS / AUDIO | unchanged | unchanged; APPEARANCE gains a passive "Ramadan mode is on — manage" row *only while active* (§6.3). |

Order left: LOCATION, MOSQUES, ALERTS. Right: CALCULATION, APPEARANCE,
NOTIFICATIONS. AUDIO band full-width below (Phase 3). Save row last. Columns keep
`alignment: start` — natural heights, no dead-space stretching.

## 3. City search — a dedicated picker view (proposal)

**Decision: dedicated view (`page: "city-picker"`), not an evolved inline band.**
Reasons: (a) GeoNames-scale results need a stable, tall viewport — inside the
settings Flickable a long result list fights the page scroll; (b) keyboard
navigation needs one focused scope, not a field buried among steppers; (c) picking
a city is a *task with an end* — enter, search, choose, return — which matches
"one idea per surface". The LOCATION band becomes the calm summary + door.

### Layout (932 px content, same padding as settings)

```
← Back to settings                     CITY PICKER
┌──────────────────────────────────────────────────────────┐
│ [ 🔍 Search cities — English or Arabic…            ]     │
│ Recent: [Riyadh] [Makkah] [Oslo]            (≤ 5 chips)  │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Riyadh                                  Saudi Arabia │ │
│ │ الرياض                        Ar Riyāḑ · UTC+3       │ │  ← result row: EN name + AR run,
│ │ …                                                    │ │    country right, admin1 · tz muted
│ └──────────────────────────────────────────────────────┘ │
│ ⌨ ↑↓ move · Enter select · Esc back                      │
│ ──────────────────────────────────────────────────────── │
│ 📍 Use my location        Read once, stays on this       │
│    (opt-in)               device. No network, no account.│
│ ✎ Enter coordinates manually                     [open] │
│ ──────────────────────────────────────────────────────── │
│ City data: GeoNames (CC BY 4.0) · geonames.org           │
└──────────────────────────────────────────────────────────┘
```

- **Result row**: name (15 px/600 ink) + **separate Arabic RTL run** beneath;
  country right-aligned; `admin1 · IANA timezone` muted caption — this is the
  disambiguation for the 14 Springfields. Rows are 2-line, hairline-separated,
  highlight = `accent-soft` fill (keyboard and hover share one highlight state).
- **Keyboard**: ↑/↓ move the highlight (wraps), Enter selects, Esc returns to
  settings without changing anything. Typing keeps focus in the field; the list
  never steals it. Slint: `FocusScope` + `key-pressed` (§8).
- **Recent cities**: ≤ 5 chips from a persisted history (most-recent-first,
  deduped). Empty history → the row simply doesn't render (no "No recents" box).
- **Search behavior**: prefix + substring over EN **and** AR names (core already
  does this for 28 cities; same contract, bigger table). **Resolved by Scout's
  ledger** ([`phase5-city-sources.md`](phase5-city-sources.md) §10): the table is
  GeoNames **cities1000** (172,038 rows) as a derived **SQLite** DB (17.3 MB
  measured), index built **off the UI thread**; Arabic display names are
  **curated** for shipped cities — GeoNames Arabic alternatenames are *search
  aliases only*, never mined into display labels. Search-only Arabic
  normalization (diacritics stripped, أإآ→ا, ى→ي, ة→ه) per ledger §5.3 — never
  applied to display. Debounced ~150 ms;
  results capped (e.g. 50) with a "Showing first 50 — keep typing to narrow"
  caption. Honest, never an infinite scroll illusion.
- **No results** (designed): *"No matches for “X”."* + three calm offers: try the
  Arabic name · enter coordinates manually · use my location. Never a blank list.
- **Use my location**: opt-in row, **off by default**, privacy caption always
  visible (*"Read once, stays on this device. No network, no account."*). Three
  honest outcomes: success → coordinates shown for confirmation before applying
  (GPS is a suggestion, not a silent override); denied → *"Location permission
  denied — you can enter coordinates manually."*; unavailable (no service on this
  OS/device) → row disabled with the reason, manual entry remains. **No spinner
  without a timeout**: > 5 s → "Taking too long — enter coordinates manually."
- **Manual coordinates** (expandable, in flow): latitude + longitude fields
  (decimal degrees, validated −90..90 / −180..180, inline error text), **plus an
  explicit IANA timezone picker** (searchable bundled list). Coordinates alone
  cannot honestly imply a timezone — the user confirms it; we never guess from
  longitude. Optional name field ("Name this place") so the home header isn't
  blank; empty → header shows the coordinates, honestly.
- **Attribution placement** (Scout's ledger confirms the licence): one caption
  line in the picker footer — *"City data: GeoNames (CC BY 4.0) · geonames.org"*
  — plus the full entry in the Phase 6 About/licences page. The caption is
  visible without scrolling past results (footer is pinned below the list, list
  scrolls internally). If Scout's ledger changes the licence, only this string
  changes.

## 4. Mosque profiles

**MOSQUES band** (left column). Calm card language, same row patterns as prayer
rows and voice rows.

- **List**: one row per profile — name (EN run + AR run if named), muted caption
  `Fajr +15 · Dhuhr +10 · …` (only non-zero offsets; all-zero → *"No offsets set"*).
  **Active profile**: `accent` border + `accent-soft` fill + small `Active` badge
  (the prayer-row pattern — users already know it). Click a non-active row →
  makes it active (one click, debounced save, home card rebinds).
- **Add profile**: ghost `＋ Add mosque` row → inline editor expands with an
  empty name field focused. Name required to save; empty name → inline hint,
  never a silent "Mosque 2".
- **Edit**: pencil affordance on the row → **inline expansion** (in flow, the
  voice-row pattern): name field (EN) + optional Arabic name field (separate
  input, separate run — never one mixed field) + **five iqamah steppers** (moved
  from ALERTS, step 1, min 0, max 60 — offsets are minutes after the prayer;
  0 = not set, per ROADMAP open decision) + `Done` (collapses) — edits apply
  live through the debounced save path; no modal.
- **Remove**: two-step inline confirm (audio-pack pattern). Removing the active
  profile → app falls back to **no active mosque**: home NEXT IQAMAH card shows
  its existing honest empty state (*"Set your mosque's iqamah offsets"*), and a
  status line says what happened. Never silently reassign another profile.
- **Home binding**: NEXT IQAMAH card gains a caption line with the active
  profile's name (separate AR run when present): `Al-Noor Mosque · Fajr +15 min`.
  No active profile → today's empty state, unchanged.
- **Empty band** (no profiles): one calm row — mark + *"No mosque profiles yet.
  Add your mosque to get iqamah times."* + Add as the primary action.
- v1 scope: name + offsets only. ROADMAP mentions location per profile — **defer
  profile-location** (a mosque in another city implies per-profile timelines;
  that is a Phase 5+ engine decision, not a UI one). Flag in §10.

## 5. High-latitude UX — the honesty centerpiece

The app must **explain**, not just display. Three layers:

### 5.1 The rule selector (CALCULATION band)

Replace the cryptic `High lat.` Segmented with **three selectable rule rows**,
each: name + **one-line plain-language description** (selected = accent-soft):

| Rule | Plain description (provisional — Quill verifies against adhan docs) |
|---|---|
| Angle-based | "Estimates Fajr and Isha from the sun's angle — a common choice for high latitudes." |
| Middle of the night | "Places Fajr and Isha at the midpoint between sunset and sunrise." |
| Seventh of the night | "Places Fajr and Isha one seventh of the night after sunset / before sunrise." |

Caption under the selector: *"These rules only matter when the sun doesn't reach
the usual angles — near the polar circles."* The selector is **always visible**
(not hidden behind "advanced"): the Tromsø user must find the explanation without
knowing the word "rule".

### 5.2 The `None` display (home Today panel + countdown)

- A prayer/sun event with no computable time shows **`—`** in the time slot
  (tabular, ink-muted) — never `00:00`, never a guess, never a hidden row.
- The row gets a small **info dot** (ink-muted, non-text) and the row is
  tappable → expands the explanation strip (§5.3). Sunrise in Tromsø June is the
  canonical case.
- Estimated-but-defined times (Fajr/Isha under a rule) show their time **plus a
  muted `estimated` caption** on the row — the user can tell computed-from-sun
  from computed-by-rule at a glance. (Arabic run: separate, Quill's string.)
- If the **next prayer itself** were ever `None`: countdown area shows `—` with
  *"No defined time today — see why"* (link → explanation). No fabricated
  countdown, no frozen ticker.

### 5.3 The explanation strip (in flow, below the Today list)

Appears whenever any displayed event is `None` **or** estimated. Calm
`surface-secondary` card, ink text, no red, no alarm icon:

> **Midnight sun in Tromsø**
> Tromsø is inside the Arctic Circle. At this time of year the sun never sets,
> so there is no sunrise or sunset to compute. Fajr and Isha are estimated using
> **Angle-based**. [Change rule] · [Why these rules?]

Contract: the strip's **facts come from the engine** — location name, polar
state (day/night), the active rule, and (if the engine provides it) the polar
window's start/end dates. **If the engine can't supply a fact, the sentence
omits it — the UI never fills a date from thin air.** "Change rule" deep-links to
CALCULATION; "Why these rules?" expands the three descriptions inline.
Polar **night** variant (Tromsø December): same strip, *"the sun never rises"*
wording — one template, engine-selected variant, both designed.

### 5.4 Explainability for the normal case (Oslo / Auckland / Riyadh)

Expandable **"How these times are computed"** row at the Today panel's foot
(collapsed by default, one line): method name, madhab, high-lat rule, rounding,
and any non-zero per-prayer adjustment — all engine values, listed plainly.
Plus the variance honesty line: *"Different sources can differ by about
2 minutes. Fine-tune any prayer below."*

**Fine-tune times** (CALCULATION band, expandable): five ± steppers (minutes,
−30..+30, default 0), each labelled with the prayer name; non-zero values are
summarized in the collapsed row (*"Asr +2 min"*) so an adjustment is never
invisible. This is ROADMAP principle 6 made visible.

### 5.5 Falsifiable Tromsø gate (the exit criterion, made testable)

With debug clock `TAWADU_DEBUG_NOW=2027-06-21T12:00:00+02:00`, city = Tromsø
(69.65° N), any method:

1. Sunrise row shows `—` — **and Maghrib too**: the core polar test
   (`polar_day_has_no_sunrise_or_sunset`, Scout ledger §6) asserts Tromsø
   midsummer yields `sunrise.is_none()` **and `maghrib.is_none()`**. No numeric
   time anywhere for those events, in any theme. (Sunset, where displayed, same.)
2. The explanation strip is visible without scrolling on the 980×620 home,
   names the polar state and the active rule.
3. Fajr/Isha rows show times **labelled `estimated`** (or `—` if the rule yields
   None — both honest, never a bare number).
4. Switching the rule in CALCULATION changes the strip's rule name and the
   estimated times **in the same session** (recompute path).
5. December variant (`2027-12-21`): strip says the sun never **rises**; no
   fabricated sunrise.
6. Oslo / Auckland / Riyadh on the same date: all five prayers numeric, no
   `estimated` captions at Oslo's latitude under default rules, "How computed"
   expander lists the actual config, Auckland's times match the far-east
   anchoring fixtures (±1 min).

## 6. Calendar views + Ramadan mode

### 6.1 Entry & layout

New `page: "calendar"`, opened by a ghost icon button in the home header (next
to الإعدادات). Month view, 7 columns:

- **Dual numbering, always**: Gregorian day (primary, 15 px/600) + Hijri day
  (secondary, 11 px muted) in every cell — no mode where one calendar hides the
  other; the **toggle switches which is primary** (Hijri-primary for Ramadan
  planning). Both numbers come from the engine (`hijridate`, Umm al-Qura) — the
  sketch computes them live via `Intl` islamic-umalqura to prove the layout.
- **Today anchor**: `accent` ring + `accent-soft` fill on today's cell; a
  **Today** button re-anchors after navigation. Out-of-month days: muted, still
  numbered (no blank cells — blanks read as broken).
- **Month nav**: ‹ month-year › + Today. Header shows both calendars' month names
  as **separate runs** (e.g. `June 2027` + `ذو الحجة 1448` — engine-provided).
- **Hijri range honesty**: `hijridate` is table-driven with bounds. Navigation
  beyond the supported range is **disabled with a caption** (*"Hijri dates
  unavailable beyond 1450 AH"* — actual bound from the engine), never an
  extrapolated guess.
- Day cells are **not** prayer-time grids in v1 (that's a Phase 5+ density
  decision); a cell tap selects the day and shows that day's five times in a
  calm strip below the grid (engine values, `—` where None — same honesty rule).
  Ramadan days get a **gold dot** (non-text marker — passes §0 in all themes).

### 6.2 Ramadan mode — scope proposal (bind it now, build in Phase 6)

Entry: when the *current* Hijri month is Ramadan, the calendar shows a calm
banner — *"Ramadan Mubarak"* + **Turn on Ramadan mode** (opt-in, never
auto-applied). Also reachable from APPEARANCE while active. **Scope — exactly
three things, nothing invented:**

1. **Theme**: suggests switching to the Ramadan theme (one click, user confirms;
   turning mode off offers to restore the previous theme — the app remembers it).
2. **Reminder cadence**: enables the **Fajr pre-alert as a suhur reminder** and
   the **Maghrib pre-alert as an iftar reminder** — existing pre-alert machinery,
   existing event kinds, renamed copy only. No new notification kinds.
3. **Calendar emphasis**: Ramadan cells carry the gold dot; the Hijri toggle
   defaults to Hijri-primary while mode is on.

Explicitly **out of scope**: fasting trackers, Quran schedules, invented
countdowns, special athkar sets (Phase 4 owns athkar). Mode state persists in
Settings; it ends when the user turns it off (or Ramadan ends → a gentle
*"Ramadan has ended — turn off mode?"* notice, never a silent switch).

### 6.3 States

Empty/error: engine date failure → *"Dates unavailable — check the Hijri table
range"* card, Gregorian grid still works (fail-soft, architecture rule 5).
Loading: month computation is local and fast; no spinner needed — if it ever
exceeds a frame, show the previous month dimmed, never a blank grid.

## 7. RTL & bilingual rules (unchanged, binding)

1. Arabic and Latin **never share one `text:` string** — separate runs: city
   names, mosque names, rule descriptions, calendar month names, strip copy.
2. Mixed digit+Arabic → RLM prefix (`\u{200F}`, proven on the Hijri date) or,
   preferred, numerals in a Latin tabular run + words in an Arabic run.
3. Arabic UI = IBM Plex Sans Arabic; **no letter-spacing on Arabic** (eyebrows
   included); Amiri only for scripture (none in Phase 5 surfaces).
4. Keyboard hints, captions, privacy lines: Quill's AR strings in their own runs.
5. Gold: **non-text markers only in Plaster** (§0); text gold in Ink/Ramadan.

## 8. Slint implementation notes (for Forge — no code written here)

### New components (define before use, root `ui/main.slint`)

| Component | Inherits | Key properties | Callbacks |
|---|---|---|---|
| `CityResultRow` | `Rectangle` | `city: CityResult`, `highlighted: bool` | `picked(string)` |
| `MosqueRow` | `Rectangle` | `mosque: MosqueProfile`, `expanded: bool` | `activate(string)`, `toggle-edit(string)`, `remove(string)`, `confirm-remove(string)` |
| `RuleRow` | `Rectangle` | `rule: RuleOption`, `selected: bool` | `picked(string)` |
| `DayCell` | `Rectangle` | `day: CalendarDay` | `picked(string)` |
| `ExplainStrip` | `Rectangle` | `title: string`, `body: string`, `rule-name: string` | `change-rule()`, `expand-rules()` |
| `CoordinateForm` | `VerticalLayout` | `lat: string`, `lon: string`, `tz-query: string`, `error: string` | `apply(string, string, string)`, `tz-picked(string)` |

Reuse: `Section`, `Action`, `RowText`, `Stepper` (with the Phase 3 `step/min/max`
extension — iqamah 1/0/60, fine-tune 1/−30/+30), `Toggle` (GPS opt-in, Ramadan
mode), `Segmented` (calendar primary toggle, rounding), `Badge` (`Active`,
`estimated`), the audio sketch's two-step confirm and in-flow expansion patterns.

### New structs (`ui/types.slint`)

```slint
export struct CityResult {
    id: string, name: string, name-ar: string,
    country: string, admin1: string, timezone: string,
}
export struct MosqueProfile {
    id: string, name: string, name-ar: string,
    offsets-summary: string,        // precomposed Latin, e.g. "Fajr +15 · Dhuhr +10"
    is-active: bool,
    fajr: int, dhuhr: int, asr: int, maghrib: int, isha: int,
}
export struct RuleOption { key: string, label: string, label-ar: string, desc: string, desc-ar: string; }
export struct CalendarDay {
    iso: string, gregorian: int, hijri: int,
    in-month: bool, is-today: bool, is-ramadan: bool,
}
export struct DayTime { name: string, name-ar: string, time: string, estimated: bool, is-none: bool; }
```

### MainWindow additions (properties → callbacks)

```slint
// city picker
in property <string> city_query;  in property <[CityResult]> city_results;
in property <[CityResult]> recent_cities;  in property <int> highlighted_result;
in property <string> picker_message;      // no-results / GPS denied / timeout copy
in property <bool> gps_available;  in property <bool> coords_open;
callback search-cities(string);  callback pick-city(string);
callback pick-recent(string);    callback use-gps();
callback apply-coords(string, string, string);   // lat, lon, IANA tz — all explicit
callback picker-back();
// mosques
in property <[MosqueProfile]> mosques;  in property <string> editing_mosque;
callback add-mosque();  callback activate-mosque(string);
callback set-mosque-name(string, string);  callback set-mosque-name-ar(string, string);
callback set-mosque-offset(string, string, int);   // (id, prayer, minutes)
callback remove-mosque(string);  callback done-editing();
// high latitude
in property <[RuleOption]> high_lat_rules;  in property <string> selected_high_lat;
in property <bool> explain_visible;  in property <string> explain_title;
in property <string> explain_body;       // engine-composed facts, never UI-invented
in property <[DayTime]> today_times;     // replaces [Row] for the Today panel
in property <string> computed_summary;   // "How computed" expander content
in property <[int]> fine_tune;           // 5 values, minutes
callback set-high-lat(string);  callback set-fine-tune(string, int);
// calendar
in property <string> calendar_month_label;  in property <string> calendar_month_label_ar;
in property <[CalendarDay]> calendar_days;  in property <string> calendar_primary; // "greg"|"hijri"
in property <bool> ramadan_month;  in property <bool> ramadan_mode;
in property <string> hijri_range_note;    // "" when in range
in property <[DayTime]> selected_day_times;
callback calendar-nav(int);  callback calendar-today();
callback set-calendar-primary(string);  callback pick-day(string);
callback set-ramadan-mode(bool);
```

### Gotchas (HANDOFF §6 + new ones this design introduces)

- **Keyboard nav**: wrap the picker in a `FocusScope`; `key-pressed` returns
  `EventResult.accept` for ↑/↓/Enter/Esc so the TextInput doesn't eat them;
  highlight is a plain `int` property — Rust clamps/wraps it. Verify Slint 1.19's
  `KeyEvent` field names against the vendored source before wiring (the tray
  lesson: grep the facade **and** the core crates).
- The picker's result list is an **inner vertical Flickable with a fixed
  viewport height** — the outer page must not double-scroll. Fixed-height
  Rectangles inside it need `height: parent.height` on their Texts.
- Calendar grid: `for week in weeks : HorizontalLayout` of 7 fixed-width
  `DayCell`s (or `GridLayout`); cells fixed-height, Texts fill them — the
  Rectangle-fills-parent slab trap applies per cell.
- Explanation strip and all editors are `if (…)` **in flow** (banner pattern):
  zero space when absent, push-down when present. No overlays.
- `Palette` stays in root; new components reference it; nothing moves to an
  imported file (globals-in-imports trap).
- Callbacks: unnamed params, `SharedString` in closures, register on the strong
  handle, capture `Weak`. All edits ride the **debounced save path** (Steps 2–3);
  حفظ stays immediate.
- **Never compose factual strings in Slint** — polar dates, rule effects,
  offsets summaries, computed summaries are composed in Rust from engine values
  and passed as whole runs. The UI's job is placement, not arithmetic.

## 9. Visual gate checklist (Forge builds against this; Atlas verifies)

**Themes & tokens**
- [ ] All four views render in Plaster, Ink, Ramadan with the §0 ratios
      (body ≥ 7:1, muted ≥ 4.5:1); screenshot each.
- [ ] Gold appears only as: Athkar marker, Ramadan day dots — **non-text in
      Plaster**; no gold body text anywhere in Plaster.
- [ ] No dead-space stretching: columns `alignment: start`, cards at natural
      height in every view and state.

**Arabic**
- [ ] Zero mixed-script `text:` strings in new components (grep-able).
- [ ] Arabic runs render shaped + RTL on Windows (macOS/Linux flagged for
      Phase 6 verification, not claimed).
- [ ] No letter-spacing on any Arabic run.

**City picker**
- [ ] ↑/↓/Enter/Esc work with focus in the search field; highlight visible in
      all themes; Esc changes nothing.
- [ ] Result rows show country + admin1 + timezone; two Springfields are
      distinguishable without clicking.
- [ ] No-results, GPS-denied, GPS-unavailable, GPS-timeout, invalid-coordinates
      states all render designed copy (no blank, no spinner > 5 s).
- [ ] Manual entry requires an explicit timezone; applying it changes the home
      header honestly (name or coordinates).
- [ ] GeoNames attribution caption visible in the picker footer without
      scrolling past results; matches Scout's ledger string exactly.

**Mosques**
- [ ] Add → name → offsets → Done persists across restart (DB read-back).
- [ ] Active indicator = accent border + soft fill + badge; one-click switch
      rebinds home NEXT IQAMAH (name caption appears).
- [ ] Remove active profile → honest empty state + status line; no silent
      reassignment. Two-step confirm on every remove.
- [ ] Iqamah steppers no longer exist in ALERTS (moved, not duplicated).

**High latitude (the exit criterion — §5.5 is the test script)**
- [ ] Tromsø June: sunrise `—`, explanation strip visible at 980×620 without
      scrolling, names polar state + active rule; Fajr/Isha labelled `estimated`.
- [ ] Tromsø December: "never rises" variant; still no fabricated time.
- [ ] Rule switch updates strip + times in-session.
- [ ] Oslo/Auckland/Riyadh: all prayers numeric; "How computed" lists actual
      method/madhab/rule/rounding/adjustments; Auckland matches fixtures ±1 min.
- [ ] Fine-tune ± changes the displayed time and is summarized when collapsed.
- [ ] No `None` ever renders as `00:00`, a blank, or a hidden row — grep the
      render path for the `—` branch.

**Calendar & Ramadan**
- [ ] Dual numbering in every cell; toggle swaps primary; today ring + Today
      button re-anchor; out-of-month days muted but numbered.
- [ ] Hijri numbers match `hijridate` (Umm al-Qura) — spot-check 3 dates against
      the CLI; out-of-range nav disabled with the engine's bound in the caption.
- [ ] Ramadan banner appears only when the current Hijri month is Ramadan;
      mode is opt-in; scope = theme suggestion + suhur/iftar pre-alerts + gold
      dots only. Off → previous theme offered back.
- [ ] Day tap shows that day's times with the same `—`/`estimated` honesty.

## 10. Open questions (Atlas / Forge / Scout / Quill)

1. **Mosque profile location** (ROADMAP task 3 says "name, iqamah offsets,
   location"): v1 UI omits per-profile location — confirm deferral or spec the
   per-profile timeline engine first. §4.
2. **GPS backend on Windows/macOS/Linux**: ~~which crate/API~~ **Resolved by
   Scout ledger §5.4 + §10.6**: GPS is the **final slice** of Phase 5 (slips to
   Phase 6 without blocking the exit if it balloons); platform paths documented
   (WinRT Geolocation / CoreLocation / GeoClue2). The UI's three honest outcomes
   (§3) wire to those real results; the row renders disabled-with-reason until
   the slice lands.
3. **Polar window dates**: can the engine supply "midnight sun from X to Y"?
   If yes, the strip shows them; if not, the sentence omits them (§5.3 contract).
4. **`estimated` flag plumbing**: `PrayerTimes` needs to expose per-event
   provenance (sun-computed vs rule-estimated vs None) — core change, Forge's
   call on shape.
5. **GeoNames subset & size**: ~~full cities500 vs cities15000~~ **Resolved by
   Scout ledger §10**: `cities1000`, derived SQLite (17.3 MB measured, inside the
   120 MB budget), index off the UI thread, derived-only commit with live-byte
   re-verification. Search cap + "keep typing" caption (§3) unchanged.
6. **Hijri table bounds**: exact range from `hijridate` for the nav-disable
   caption (§6.1) — engine value, not a guess.
7. Quill: rule descriptions must be verified against adhan's documented
   semantics before shipping — plain language, but **accurate** plain language.
