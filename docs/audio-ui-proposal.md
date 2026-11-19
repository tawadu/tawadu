# Audio settings — UI proposal (Phase 3 prep)

> Author: Prism (Brand & UI/UX) · 2026-11-19 · status: **proposal, no code touched**
> Contract: [`voice-pack-format.md`](voice-pack-format.md) · Tokens: [`../design/brand.md`](../design/brand.md)
> Sketch: [`../design/audio-settings-sketch.html`](../design/audio-settings-sketch.html) (theme toggle: Plaster / Ink / Ramadan; state toggle: default / empty / error)
> Copy in the sketch is **provisional placeholder** — Quill's `docs/audio-ui-strings.md` is the string source of truth; pack names/reciters/licences in the sketch are **examples**, real values come from Scout's `docs/audio-sources.md` + imported manifests. Nothing here invents a runtime value.

---

## 1. What the user asked for

Many voices, swappable **per event** (athan / iqamah / athkar / chime), played
**sequentially or shuffled**, with the ability to **cut the athan after N seconds**.
Plus: import packs, preview them, remove them, set volume. All of it calm, compact,
and honest — the settings page already scrolls, so the audio section must earn its
vertical space.

## 2. Placement

The settings surface is a `Flickable` with two columns (460 px left: LOCATION,
ALERTS · 420 px right: CALCULATION, APPEARANCE, NOTIFICATIONS, save row). There is
no room for a third column at 980 px window width.

**Proposal: one full-width AUDIO band below the existing columns, inside the same
Flickable**, made of three cards:

```
┌────────────────────────────────────────────────────────────┐
│ AUDIO · الصوت                                              │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ VOICE PACKS   [pack] [pack] [pack] [＋ Import pack…]   │ │  ← horizontal strip
│ └────────────────────────────────────────────────────────┘ │
│ ┌──────────────────────────┐ ┌───────────────────────────┐ │
│ │ EVENT VOICES             │ │ PLAYBACK                  │ │
│ │ Athan   [voice ▾] [seq]  │ │ Volume  ────●────  80 %   │ │
│ │ Iqamah  [voice ▾] [seq]  │ │ Stop athan after  − 30 +  │ │
│ │ Athkar  [voice ▾] [shuf] │ │ 0 = full · fades out 0.4 s│ │
│ │ Chime   [voice ▾] [seq]  │ │                           │ │
│ └──────────────────────────┘ └───────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

Estimated added height: ~330 px (packs ~150, columns ~180). One scroll-flick more;
acceptable. The save row (حفظ / إغلاق) stays **last**, below the audio band — audio
edits flow through the same save path as everything else (Step 3's debounced
save-on-change; حفظ remains immediate).

## 3. Card 1 — Voice packs

A **horizontal strip** of pack cards (fixed ~210 px each) + a trailing import
affordance. More packs than fit → the strip is a horizontal `Flickable` (Slint has
no flex-wrap; do not stack packs vertically — that would eat the page).

Each pack card shows, top to bottom:

| Element | Source | Notes |
|---|---|---|
| Name (Latin) | `manifest.name` | 15 px / 600, ink |
| Name (Arabic, if present) | manifest extension¹ | **separate RTL `Text` run**, never concatenated |
| Reciter | `manifest.reciter` | caption, ink-muted, one line, ellipsis |
| Licence badge | `manifest.license` (SPDX) | pill chip: `CC0-1.0`, `CC-BY-4.0`, `proprietary` |
| Duration | **computed at import**² | tabular numerals, e.g. `3:42` total |
| Preview button | — | soft button, play icon → becomes **Stop** while previewing |
| Remove | — | ghost icon button, two-step inline confirm (below) |

¹ The format doc has no `name_ar`/`reciter_ar` fields today. **Ask:** add optional
`name_ar` / `reciter_ar` to `manifest.json` (backward-compatible). Until then the
Arabic run simply doesn't render — never machine-translate a reciter's name.

² Duration is **not** in the manifest. The audio engine must probe decoded lengths
at import and expose them to the UI. The UI must never guess a duration. If probe
fails, show `—` (designed unknown, not a fabricated number).

**States** (brand rule: designed, never blank):

- **Selected pack** (the one whose voices are offered first / bundled default):
  `accent` border + `accent-soft` fill — exactly the prayer-row pattern. Selection
  is a *filter convenience*, not a restriction: per-event pickers list voices from
  **all** installed packs.
- **Bundled packs**: badge `Bundled` instead of a Remove button — they cannot be
  removed (they are the offline floor).
- **Previewing**: one preview at a time (Rust enforces). The active card's button
  reads Stop, filled `accent-soft`. **Preview plays with the current trim/fade
  edits applied**, so the user hears exactly what the event will sound like.
  Preview respects the volume slider.
- **Remove**: two-step inline — first click turns the button into a small
  `Remove? ✓ ✕` confirm pair on the card; no modal, no red. Removing a pack that
  voices are assigned to → those events fall back to the bundled default and a
  status line says so (honest, never silent).
- **Empty** (no packs installed — shouldn't happen with bundled packs, but design
  it): one calm card, mark + "No voice packs yet" + Import as primary action.
- **Import error**: a status strip under the strip, ink text on `surface-secondary`,
  exact reason from the validator, e.g. *"Pack rejected — missing licence for
  athan/fajr.mp3."* No red-urgency (brand §7). Dismissable; persists until next
  import attempt.

**Import affordance**: dashed-hairline card, plus icon, "Import pack", caption
`.tawadupack or folder`. Opens `rfd` picker (Forge). Validation is all-or-nothing
per the format doc: every file needs sha256 + licence, or the pack is rejected.

**Attribution**: licence badge is a click target → opens the attribution view
(`docs/attribution.md` content rendered in-app, exact `manifest.attribution` text,
`source_url`). This is a licence obligation, not decoration — it must be reachable
in two clicks from the card.

## 4. Card 2 — Event voices

Four rows, one per event kind: **Athan · Iqamah · Athkar · Chime**. Each row:

- Event label: Latin run + **separate Arabic RTL run** (e.g. `Athan` / `أذان`).
- **Voice pill**: shows the assigned voice as `Pack · Clip` (one line, ellipsis).
  Click → the row **expands inline** (push-down, like the banner — *not* an
  overlay popup; popups inside a `Flickable` clip and steal scroll). The expanded
  list groups voices by pack, each option a selectable row (accent-soft when
  active). Click an option → assigns, collapses. Click elsewhere / the pill →
  collapses. Only one row expanded at a time.
- **Mode chip**: tiny two-state pill `Sequential ⇄ Shuffle` (icons + tooltip;
  text label in the expanded area). Per-event, because the manifest's `sequences`
  are per-event — a single global toggle would lie about what actually plays.
  Default comes from the pack manifest; the user's choice is an override stored
  in settings.

**Gold's intentional job** (brand: *sacred/athkar accents only*): the **Athkar
row** gets a small gold dot before its label; its Arabic run renders in
`Palette.gold` **in Ink and Ramadan only**. In Plaster, gold-as-text measures
3.14:1 (fails the 4.5:1 bar — see phase5-ui-proposal.md §0), so the label stays
`ink` and the gold dot (non-text marker, ≥3:1) carries the accent. Nothing else
in the audio section uses gold — licence badges stay
neutral, actions stay accent. This also finally gives gold a home on the Configure
surface (Phase 6 debt: "gold unused"). **Theme note:** the Ramadan palette defines
no `--gold`; there, `--accent` *is* gold (`#D9A441`), so the athkar marker maps to
`Palette.accent` in Ramadan. Ink theme uses `#D0A45C`. All three themes survive.

**Chime** is not in the pack format's file tree today (athan/ + athkar/ only).
**Ask:** either a `chime/` directory in the format, or chime voices ship as
built-in non-pack sounds listed under a "Built-in" group in the picker. Recommend
the latter for v1 (one less import-validation path); flag for Atlas/Forge.

## 5. Card 3 — Playback

- **Volume**: one master slider, 0–100 %, tabular value readout. Custom-styled
  (track `surface-secondary`, fill `accent`, thumb `surface` + hairline border).
  Per-pack volume exists in the roadmap's mixer; **not exposed in v1 UI** — one
  honest master control beats five sliders nobody asked for. Revisit in Phase 5.
- **Stop athan after**: the original ask. A `Stepper` in **seconds**, step 5,
  range 0–300, value `0` labelled **Full** (play the whole recording). Caption:
  *"0 = full recording · fades out over 0.4 s"* — the 0.4 s is the format doc's
  default `fade_out_ms: 400`, not an invented number. The cut is a **fade-out,
  never a hard click** (mixer renders it; `stop_after_ms` in `edits`).
  Scope v1: applies to the **athan** event (the user's words). The engine applies
  edits per-file, so per-event trim can come later without UI surgery.
- **Fade hint**: the caption above is the whole fade UI in v1. Fade-in/out ms
  stay manifest-driven (advanced users edit the pack). No extra knobs.
- Sequential/shuffle lives in Card 2 (per-event), **not** here — one home per idea.

## 6. RTL & bilingual rules (binding, from HANDOFF §6)

1. Arabic and Latin **never share one `text:` string** — separate `Text` runs,
   always. Pack names, event labels, mode labels, captions: two runs each.
2. Mixed digit+Arabic runs (e.g. an Arabic duration caption) get the **RLM
   prefix** (`\u{200F}`) already proven for the Hijri date — or, preferred, keep
   numerals in a Latin tabular run and the words in an Arabic run.
3. Arabic UI font: IBM Plex Sans Arabic; Amiri only if a pack ships scripture
   display text (not needed in v1 audio settings).
4. No all-caps Latin eyebrows on the RTL layout — when language = Arabic, section
   eyebrows render the Arabic string in its own run at the same size, no
   letter-spacing (Arabic must not be letterspaced — it breaks joining visually).
5. The sketch keeps every Arabic span `dir="rtl" lang="ar"`; Forge mirrors this
   with separate runs, not direction hacks.

## 7. Slint implementation notes (for Forge — no code written here)

### New components (define **before** use, in `ui/main.slint`)

| Component | Inherits | Key properties | Callbacks |
|---|---|---|---|
| `Badge` | `Rectangle` | `label: string`, `tone: string` (`neutral`/`gold`) | — |
| `PackCard` | `Rectangle` | `pack: PackRow`, `previewing: bool`, `confirming-remove: bool` | `preview(string)`, `remove(string)`, `confirm-remove(string)`, `cancel-remove()`, `show-attribution(string)` |
| `VoiceRow` | `Rectangle` | `event: EventVoiceRow`, `expanded: bool`, `voices: [VoiceOption]` | `toggle-expanded(string)`, `pick-voice(string, string)`, `set-mode(string, string)` |
| `AudioSlider` | `Rectangle` | `value: int`, `min: int`, `max: int` | `changed(int)` — `TouchArea` drag math; thumb 16 px, track 4 px |

### Reuse, don't reinvent

- `Section` (card eyebrow), `Action` (Import primary, Preview soft via
  `primary: false`), `RowText` (all layout-row labels), `Segmented` **only if**
  a two-option choice ever goes global (mode is per-event → custom chip instead).
- `Stepper`: **extend** with `in property <int> step: 1`, `min: int`, `max: int`
  (clamp in `changed`). Existing iqamah steppers keep step 1 — zero behavior
  change. Trim uses `step: 5, min: 0, max: 300`.
- `Toggle` pattern (36×20 track) is the visual reference for the mode chip's
  on/off feel, but mode is two named states, not a bool — chip shows the active
  word, click flips.

### New structs (`ui/types.slint`)

```slint
export struct PackRow {
    id: string, name: string, name-ar: string, reciter: string,
    license: string, duration: string,      // "" → render "—"
    is-bundled: bool, is-selected: bool,
}
export struct VoiceOption {
    id: string, label: string, label-ar: string, pack-name: string,
}
export struct EventVoiceRow {
    kind: string,               // "athan" | "iqamah" | "athkar" | "chime"
    label: string, label-ar: string,
    voice-id: string, voice-label: string,  // "Pack · Clip", precomposed in Rust (Latin only)
    mode: string,               // "sequential" | "shuffle"
}
```

### MainWindow additions

```slint
// properties
in property <[PackRow]> audio_packs;
in property <[VoiceOption]> audio_voices;
in property <[EventVoiceRow]> event_voices;
in property <string> previewing_pack_id;   // "" = none
in property <string> expanded_event;       // "" = none
in property <int> audio_volume;            // 0..100
in property <int> athan_stop_after_s;      // 0 = full
in property <string> audio_message;        // import errors / fallback notices
// callbacks
callback import-pack();
callback preview-pack(string);             // toggles start/stop in Rust
callback remove-pack(string);              // Rust checks assignments → fallback + message
callback show-attribution(string);
callback pick-event-voice(string, string); // (event-kind, voice-id)
callback set-event-mode(string, string);   // (event-kind, mode)
callback set-audio-volume(int);
callback set-athan-trim(int);              // seconds; Rust ×1000 → stop_after_ms
```

### Gotchas that will bite (all from HANDOFF §6, re-verified against the file)

- `Text` inside fixed-height `Rectangle` → `height: parent.height` (pack-card
  buttons, badges, chips).
- Columns keep `alignment: start` — the audio band's two cards must not inflate
  to each other's height.
- The inline voice list is an `if (expanded)` **in flow** (banner pattern), so it
  pushes rows down and occupies zero space when collapsed. No overlays inside
  the Flickable.
- Horizontal pack strip: `Flickable { HorizontalLayout { … } }` with explicit
  card widths; viewport height fixed (~150 px) or the Rectangle-fills-parent trap
  turns it into a slab.
- `Palette` stays declared in root `main.slint` (already is) — new components
  reference it freely; do not move it to an imported file.
- Callbacks: unnamed params, `SharedString` in closures, register on the strong
  handle, capture `Weak`.
- Preview/trim/volume edits go through the **same debounced save path** Step 3
  lands; no audio-specific persistence side channel.

## 8. Open questions (need Atlas/Forge/Quill)

1. **Chime source**: `chime/` in the pack format vs built-in sounds (recommend
   built-in for v1). §4.
2. **`name_ar` / `reciter_ar`** optional manifest fields — format-doc addendum. §3.
3. **Mode override semantics**: manifest default vs user override precedence —
   proposal: user override wins, "Reset to pack default" appears in the expanded
   row only when an override exists. Confirm with Forge's mixer design.
4. **Duration probing** at import must be part of the importer's contract (§3²).
5. Quill's string table replaces every provisional label in the sketch; the
   sketch's structure, not its wording, is the deliverable.

## 9. Acceptance checklist (for the Phase 3 PR's visual gate)

- [ ] All three themes render the audio band with ≥ 7:1 body / ≥ 4.5:1 muted contrast.
- [ ] Gold appears **only** on the athkar row (accent in Ramadan).
- [ ] No mixed-script `text:` string anywhere in the new components.
- [ ] Empty / error / previewing / confirming-remove states all designed.
- [ ] Voice list expands in flow; nothing clips inside the Flickable at 980×620.
- [ ] Trim stepper: 0 shows "Full"; caption states the real fade default.
- [ ] Remove never silently orphan an assigned event voice.
- [ ] Attribution reachable in ≤ 2 clicks from any pack card.
