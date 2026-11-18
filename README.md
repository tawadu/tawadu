# Tawadu · تواضع

**A calm, offline-first prayer companion.** Accurate prayer times, real adhan audio, your mosque's iqamah, and your athkar — no ads, no account, no tracking.

> **Phase 1 in progress.** The calculation core, CLI and a first Slint home surface are built and tested; the plan of record is [docs/ROADMAP.md](docs/ROADMAP.md).

---

## What it is

Tawadu is a cross-platform desktop app (Windows 10/11, macOS, Linux) built in **Rust** with a **Slint** interface. It is offline-first: prayer times are computed on your machine, the athan and athkar audio live on your disk, and nothing is sent anywhere.

The design identity lives in [`design/`](design/) — see [`design/brand.md`](design/brand.md) and the interactive [`design/style-guide.html`](design/style-guide.html).

## Planned features

- **Prayer times** — all major methods (incl. Umm al-Qura), Hanafi/Shafi Asr, high-latitude rules, per-prayer manual adjustments, DST-safe timezones.
- **Countdown** — live `HH:MM:SS` to the next prayer, tray tooltip, mini-mode.
- **Iqamah** — per-prayer offsets and a separate iqamah alert; mosque profiles.
- **Athan** — per-prayer voices, volume, fade, output device, full/short/Fajr variants, plus a **voice-pack editor** (trim, fade, sequence, shuffle).
- **Athkar** — morning/evening/sleep/after-prayer sets, random or sequential, word-by-word audio, tasbih counter, memorisation, share-as-card.
- **More** — Hijri calendar, Ramadan mode, monthly timetable, dua & 99 names, Qibla, themes, backup.

## Status

| | |
|---|---|
| Phase 0 — core | **done** — prayer, qibla, hijri, locations, scheduler, storage, CLI |
| Phase 1 — usable | **in progress** — home surface renders; settings surface next |
| Tests | 42, plus `fmt` and `clippy -D warnings` |
| CI | build + test on Linux, Windows, macOS |
| Licence | MIT (code) · assets under their own licences |

The plan of record — phases, pitfalls, known debt and open decisions — is
[docs/ROADMAP.md](docs/ROADMAP.md).

## Layout

```
crates/
  tawadu-core/    pure logic: prayer, scheduler, locations, hijri, athkar, storage
  tawadu-audio/   playback mixer, fades, voice-pack editor
  tawadu-app/     Slint shell, tray, notifications, scheduler wiring
ui/               .slint components and themes
assets/           voices, athkar corpus, sounds, fonts, licences
design/           brand & visual style
docs/             architecture, ADRs, voice-pack format, attribution
```

## Build

```bash
cargo check --workspace
cargo test --workspace
cargo run -p tawadu-app          # GUI
cargo run -p tawadu-app -- times --city "Riyadh" --method umm-al-qura
cargo run -p tawadu-app -- cities
```

On Linux, Slint needs `libfontconfig1-dev libfreetype6-dev libxkbcommon-dev
libwayland-dev libx11-dev libxdo-dev` (the CI workflow lists the same set).

## Licence

MIT — see [LICENSE](LICENSE). Bundled audio, fonts and datasets remain under their own licences; see [docs/attribution.md](docs/attribution.md).
