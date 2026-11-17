# Tawadu — Brand & Visual Style v1

> تواضع — humility. A calm, offline-first prayer companion.
> Generated 2026-11-17. These tokens are ready to become Slint design tokens.

This document is the single source of truth for the look and feel. The interactive version is `style-guide.html` (open in a browser; theme toggle top-right).

---

## 1. Brand idea

**Humility, made visible.** The identity is quiet, warm and spacious — nothing shouts, nothing shines. The mark abstracts the mihrab niche and the worshipper into one arch and one dot: standing in the niche, bowed.

**Tone of voice:** plain, warm, exact. No marketing superlatives. No exclamation marks. Address the user like a calm friend.

| | |
|---|---|
| Surfaces | Monitor (home) · Configure (settings) |
| Mood | calm · grounded · warm · exact |
| Accent use | state and action only, never decoration |

---

## 2. Logo

**Primary mark — "Niche"** (`logo-mark.svg`, `logo-mark-green.svg`, `logo-mono.svg`)

```
viewBox 0 0 64 64
arch:  M16 52 V30 a16 16 0 0 1 32 0 V52   (stroke 4.5, round cap)
dot:   circle cx32 cy45 r4.6
```

- **Clear space:** one dot-radius (4.6 units) on all sides.
- **Minimum size:** 20 px (mark alone), 96 px (lockup).
- **Lockups:** `logo-lockup.svg` (Latin), `logo-lockup-ar.svg` (Arabic).
- **Mono:** `logo-mono.svg` (white) for accent backgrounds, tray, notifications, favicon.
- **Don't:** recolour the mark, rotate it, add shadows, put it on a busy photo, or stretch it.

Alternates explored: `B · Bow` (single rukuʿ arc over the dot) and `C · Two dots` (niche + the two dots of tāʾ). Keep in `style-guide.html` for reference; not the primary.

---

## 3. Colour

Three themes, user-toggleable. Accent is used sparingly; large areas stay neutral.

### Plaster (light, default)
| Token | Hex | Use |
|---|---|---|
| `--bg` | `#F6F1E8` | page background |
| `--surface` | `#FFFDF8` | cards, sheets |
| `--surface-2` | `#EFE8DB` | inset wells, table stripes |
| `--ink` | `#211C15` | primary text |
| `--ink-muted` | `#6E665A` | secondary text |
| `--border` | `#E2D9C9` | hairlines |
| `--accent` | `#2E5C4E` | deep green — primary action, active state |
| `--accent-soft` | `#E3EEE9` | selected row fill |
| `--accent-ink` | `#FFFFFF` | text on accent |
| `--gold` | `#B08A4A` | sacred/athkar accents only |

### Ink (dark)
| Token | Hex |
|---|---|
| `--bg` | `#14110D` |
| `--surface` | `#1D1915` |
| `--surface-2` | `#27221C` |
| `--ink` | `#EFE7DA` |
| `--ink-muted` | `#B0A697` |
| `--border` | `#332C24` |
| `--accent` | `#8FC3AE` |
| `--accent-soft` | `#24352E` |
| `--accent-ink` | `#0F1A15` |
| `--gold` | `#D0A45C` |

### Ramadan (dark, gold)
| Token | Hex |
|---|---|
| `--bg` | `#171209` |
| `--surface` | `#221A10` |
| `--surface-2` | `#2C2214` |
| `--ink` | `#F3E7D3` |
| `--ink-muted` | `#C2B399` |
| `--border` | `#3A2E1C` |
| `--accent` | `#D9A441` |
| `--accent-soft` | `#33270F` |
| `--accent-ink` | `#1A1206` |

**Contrast:** body text ≥ 7:1 on its surface in every theme; muted text ≥ 4.5:1. Verify any new token pair before shipping.

---

## 4. Typography

| Role | Family | Notes |
|---|---|---|
| Latin UI | **IBM Plex Sans** (300/400/500/600/700) | OFL — bundle it |
| Arabic UI | **IBM Plex Sans Arabic** (300–700) | same skeleton as Latin; OFL |
| Scripture / athkar display | **Amiri** (400/700) | classical naskh; OFL |

Scale (px / weight / tracking):
| Token | Size | Weight | Tracking |
|---|---|---|---|
| display | 64 | 400 | −0.04em |
| title | 28 | 600 | −0.02em |
| heading | 20 | 600 | −0.01em |
| body | 15 | 400 | 0 |
| caption | 12.5 | 500 | +0.02em |

- Numerals in countdowns and times: tabular (`font-variant-numeric: tabular-nums`).
- Arabic and Latin share a baseline; do not nudge Arabic up/down.

---

## 5. Space, shape, motion

- **Spacing scale:** 4 · 8 · 12 · 16 · 24 · 32 · 48 · 64.
- **Radii:** 6 (small) · 10 (controls) · 16 (cards) · full (pills).
- **Elevation:** one soft shadow, `0 1px 2px rgba(33,28,21,.05), 0 4px 14px rgba(33,28,21,.05)` (light) / darker in Ink.
- **Motion:** 180 ms hover · 260 ms toggle · 320 ms panel; `cubic-bezier(.22,.61,.36,1)`. No loops. `prefers-reduced-motion` disables non-essential animation.

---

## 6. Iconography

Custom stroke set: 24 px grid, 1.5 px stroke, round caps/joins, no fills (except the logo mark). Do not mix with an emoji set.

---

## 7. Components

- **Buttons:** primary (accent fill), soft (accent-soft), ghost (hairline border). No accent rail, no glass.
- **Prayer row:** hairline border; the next prayer gets `accent` border + `accent-soft` fill. Time right-aligned, tabular numerals.
- **Countdown:** display weight, tabular numerals, optional progress ring (accent, round cap).
- **Athkar card:** Amiri text right-aligned; thin progress bar in accent.
- **Toast / notification:** surface + hairline border + accent text; calm, never red-urgency.
- **Empty / loading / error:** designed states, never blank.

---

## 8. Banners

- `banner-og.svg` — 1200×630 social/OG.
- `banner-wide.svg` — 1920×640 header (Ink theme).

Both use the faint girih grid (hairline, 4–5% opacity) — texture, not decoration.

---

## 9. Do / Don't

**Do:** one idea per surface · accent for state/action only · Arabic and Latin on one baseline · weight and space over boxes · design empty/loading/error states.

**Don't:** gradients, glass, neon · emoji as icons · a centred hero on a Monitor surface · all-caps Latin labels on RTL layouts · animation that loops without purpose · recolor the mark.

---

## 10. Asset checklist

| File | Use |
|---|---|
| `style-guide.html` | interactive brand board (theme toggle) |
| `logo-mark.svg` | mark, `currentColor` |
| `logo-mark-green.svg` | mark, fixed green |
| `logo-mono.svg` | mark, white, for accent/tray/favicon |
| `logo-lockup.svg` | Latin lockup |
| `logo-lockup-ar.svg` | Arabic lockup |
| `banner-og.svg` | social/OG |
| `banner-wide.svg` | header |

*Next: turn these tokens into Slint `.slint` theme files and a component library when development starts.*
