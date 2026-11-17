# Voice-pack format

A **voice pack** is a folder (or a `.tawadupack` zip) describing a set of athan and
athkar recordings plus the metadata needed to edit, sequence and attribute them.

## Layout

```
my-pack/
├── manifest.json
├── licenses/
│   └── <file>.txt          # one licence/attribution entry per source
├── athan/
│   ├── fajr.mp3
│   ├── dhuhr.mp3
│   ├── asr.mp3
│   ├── maghrib.mp3
│   └── isha.mp3
└── athkar/
    ├── morning_01.mp3
    └── ...
```

## `manifest.json`

```jsonc
{
  "format": "tawadu-pack/1",
  "name": "My Pack",
  "reciter": "…",
  "description": "…",
  "license": "CC0-1.0",            // SPDX id, or "proprietary"
  "attribution": "Recorded by …",  // exact credit text shown in-app
  "source_url": "https://…",
  "version": "1.0.0",
  "updated": "2026-11-17",
  "files": {
    "athan/fajr.mp3":    { "sha256": "…", "license": "CC0-1.0" },
    "athan/dhuhr.mp3":   { "sha256": "…", "license": "CC0-1.0" }
  },
  "sequences": {
    "fajr": { "mode": "sequential", "clips": ["athan/fajr.mp3"] },
    "dhuhr": { "mode": "shuffle",   "clips": ["athan/dhuhr.mp3", "athan/dhuhr_alt.mp3"] }
  },
  "edits": {
    "athan/fajr.mp3": { "trim_start_ms": 0, "trim_end_ms": 0, "fade_in_ms": 120, "fade_out_ms": 400, "stop_after_ms": 0 }
  }
}
```

## Rules

- **Every file has a sha256 and a licence.** No licence entry → the pack is
  rejected at import.
- `edits` are applied non-destructively: the source file is never modified; the
  edit is rendered to a cache file on first play.
- `sequences` support `sequential` and `shuffle`; a sequence may mix athan and
  athkar clips.
- All processing is local. Packs never "phone home".
- See [attribution.md](attribution.md) for the current source list.
