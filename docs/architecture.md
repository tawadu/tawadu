# Architecture

Tawadu is three crates plus a Slint interface. The rule that shapes everything:
**the core is pure logic — no UI, no platform types — so it can be shared with a
future mobile shell.**

```
crates/tawadu-core     pure logic        (prayer, scheduler, locations, hijri, qibla, athkar, settings, storage)
crates/tawadu-audio    audio engine      (mixer, fades, voice-pack editor)
crates/tawadu-app      desktop shell     (Slint UI, tray, notifications, scheduler wiring)
ui/                    .slint components and themes
assets/                voices, athkar corpus, sounds, fonts, licences
design/                brand & visual style (tokens are the source of truth)
```

## Core concepts

- **Timeline** — for a date + location + method, an ordered list of absolute
  events: the five prayers, sunrise, `iqamah_<prayer>` (offset), athkar windows
  (morning / evening / sleep / after-prayer) and pre-alerts (`−N` minutes).
- **Scheduler** — computes the next event, sleeps until it, fires it, recomputes.
  Recomputes on start-up, location/method change, settings change, system clock
  change, **wake from sleep**, and every midnight.
- **Athan state machine** — `idle → pre-alert → playing → (post-athan dua/athkar) → idle`,
  with snooze / dismiss / manual play.
- **Audio engine** — one mixer, channels `athan` / `athkar` / `chime`; per-channel
  volume, fade envelopes, ducking, output-device selection, hard stop.
- **Voice-pack editor** — decode → trim → fade → (resample) → concatenate → encode,
  entirely offline. See [voice-pack-format.md](voice-pack-format.md).
- **Persistence** — SQLite for settings, history, athkar progress, pack metadata
  and mosque profiles; JSON export for portability.

## Correctness rules (non-negotiable)

1. Astronomy is computed in **UTC**; local offsets are added only at the edge.
2. High-latitude rules are explicit; polar day/night never panics.
3. The countdown is anchored to a wall clock and ticked monotonically — no drift.
4. Audio and IO never block the UI thread.
5. Everything fails soft: no audio device → notification only; no city database →
   manual entry; no network → fully functional.

## Platform integration

| Concern | Crate | Notes |
|---|---|---|
| Tray | `tray-icon` | Linux needs AppIndicator/StatusNotifier |
| Notifications | `notify-rust` | Windows/macOS/Linux; always keep an in-app fallback |
| Autostart | `auto-launch` | |
| Single instance | `single-instance` | |
| File dialogs | `rfd` | |
| Hotkeys | `global-hotkey` | X11 caveat on Linux |

## Testing

- **Reference fixtures** — captured prayer times for ~20 cities × 12 months × all
  methods; assert within 1 minute.
- Unit tests for methods, madhabs, high-latitude, DST, Hijri, iqamah offsets.
- Property tests: timeline is monotonic; no overlapping athan; offsets ≥ 0.
- Manual QA: sleep/wake, clock change, DST, timezone travel, no-audio, no-network,
  125/150% DPI.

See [adr/](adr/) for decision records.
