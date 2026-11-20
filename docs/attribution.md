# Attribution & licences

Tawadu's code is MIT. Everything bundled besides the code keeps its own licence.
**Every bundled file needs an entry here and in its pack's `licenses/` folder.**

## Status (2026-11-17)

| Source | Contents | Licence | Status |
|---|---|---|---|
| Ministry of Islamic Affairs (MoIA) | Adhan + athkar recordings | Permission via ministry contact | Cleared to use; keep the written/email record |
| Wikimedia Commons | Adhan recordings (Makkah, Madinah, Malmö, …) | CC0 / CC BY 3.0 / CC BY-SA 4.0 | Usable with attribution |
| `YousefAsalya/Islamic-Pro-azkar-API` | Hisn al-Muslim TEXT: 132 categories / 267 items, AR/EN (pinned `d793023e`; per-file SHA-256 + `scripts/verify_athkar_corpus.py`; ledger `docs/athkar-corpus.md`) | MIT (`assets/licenses/azkar-api-mit.txt`) | **Bundled (Phase 4, text only)** — recitation MP3s NOT bundled (third-party performance rights, separate check) |
| OpenDua dataset (Human Development Fund) | Adhkar text | CC BY 4.0 | Usable with attribution |
| GeoNames `cities1000.zip` | Derived city DB: 172,038 cities, AR/EN search aliases, IANA timezones (pinned raw SHA `1dcfe6ea…`, derived SHA `640f86c4…`; `scripts/build_city_db.py`; ledger `docs/phase5-city-sources.md`) | CC BY 4.0 (`assets/licenses/geonames-cities1000-cc-by-4.0.txt`) | **Bundled (Phase 5, derived SQLite only — raw zip never in repo)** |
| `Kiwifu/adhan-mp3` | 224 adhan MP3s | **no LICENSE file** | **Do not bundle** until licensed |
| `assabile.com` | Adhan / athkar downloads | free downloads, no redistribution grant | Verify before bundling |
| `mohammed-2-5/islamic-library-data` | data + 4 adhan | **no LICENSE** | Avoid |

## Rules

1. One entry per third-party asset: source URL, author/reciter, licence, version/date, exact attribution text.
2. Store as a pack manifest entry or `assets/licenses/<file>.txt`.
3. Surface every entry in an in-app **Credits** screen.
4. If an asset cannot be public, keep the code open and host the asset in a
   controlled repository or authenticated download.
5. When in doubt, do not bundle — ask.

## Fonts

| Font | Licence | Use |
|---|---|---|
| IBM Plex Sans | SIL OFL 1.1 | Latin UI |
| IBM Plex Sans Arabic | SIL OFL 1.1 | Arabic UI |
| Amiri | SIL OFL 1.1 | Scripture / athkar display |

## Rust dependencies added for the tray (PR #1, ADR 0002)

Verified against each crate's published `Cargo.toml` licence field. All are
MIT-compatible; the GTK stack (`glib-sys`/`gtk` via muda's default features)
was deliberately removed — the Linux tray uses the pure-Rust `ksni` backend.
Surface these in the in-app Credits screen alongside the asset entries.

| Crate | Version | Licence | Role |
|---|---|---|---|
| `auto-launch` | 0.6.0 | MIT | Opt-in start-at-login (HKCU Run / launch agent / .desktop); transitives MIT-compatible |
| `windows-registry` | 0.6.1 | MIT OR Apache-2.0 | Windows-only: read the registered Run value to detect a moved exe path |
| `tray-icon` | 0.26.1 | MIT OR Apache-2.0 | System tray icon (ksni backend on Linux) |
| `muda` | 0.21.2 | Apache-2.0 OR MIT | Tray/app menus, MenuEvent channel |
| `ksni` | 0.3.7 | Unlicense | Pure-Rust StatusNotifierItem (Linux tray) |
| `zbus` | 5.19.0 | MIT | SNI-watcher probe (Linux); also pulled by ksni |
| `windows` | 0.62.2 | MIT OR Apache-2.0 | AUMID registration + restore event (Windows) |

Note: `notify-rust` independently pulls a second `zbus` major (6.x) for its
XDG backend; the two majors cannot unify. `single-instance` 0.3 (MIT) was
already a dependency before PR #1.
