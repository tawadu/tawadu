# Athkar corpus ledger (Phase 4)

**Source:** `YousefAsalya/Islamic-Pro-azkar-API` — Hisn al-Muslim TEXT data.
**Licence:** MIT (verbatim text bundled at `assets/licenses/azkar-api-mit.txt`;
audit: `docs/audio-sources.md` §3). **Pinned commit:**
`d793023e3b69c91674023f692425950c8ebbb64d` (2026-05-10, "feat: complete azkar
data - 132 categories ar/en/tr").

**Scope decision (PIPELINE.md, on record):** TEXT only. The repo's 267 recitation
MP3s are credited to a third party (Hamad Al-Durayhim); MIT is verified for the
text data, NOT for performance rights in the audio. Recitation audio is out of
Phase 4 — separate performance-rights check first. `data/tr.json` is not bundled
(the app is AR/EN). OpenDua CC BY 4.0 remains the documented fallback only.

## Per-file provenance (Sentinel binding condition)

| Bundled file | Upstream path | Bytes | SHA-256 |
|---|---|---|---|
| `assets/athkar/ar.json` | `data/ar.json` | 166566 | `7affdd9c1356f66f77057b631ac239cc41bde68f9bdc85d344a852a0a6ff6732` |
| `assets/athkar/en.json` | `data/en.json` | 145287 | `49d155762b9eac1011127a3afabd4a4094206138b7c161f587b29a473f59f3f1` |
| `assets/licenses/azkar-api-mit.txt` | `LICENSE` | 1093 | `d6930e8db4f791647d10abb43c218d10b6f6fb5fd9905aab2602ce41dd1d33ba` |

**Reproduce:** `python3 scripts/verify_athkar_corpus.py` (offline: bundled bytes
vs this ledger) · `python3 scripts/verify_athkar_corpus.py --live` (also
re-downloads the pinned upstream bytes and compares). Stdlib only.

## Structure (as parsed by `tawadu-core::athkar`)

- 132 categories, 267 items total; AR and EN files align by category `id` and
  item `id` (verified at import).
- Category: `{id, category (name), array: [items]}`. Item:
  `{id, text, count, transliteration (nullable), audio, filename}`.
- AR `text` = Arabic (primary, RTL); EN `text` = English translation;
  `transliteration` is sparse in this corpus (1 non-null EN, 0 AR) — rendered
  when present, never fabricated. `count` = repetition target (20 items > 1).
- `audio`/`filename` fields are IGNORED at import (no recitation bundling).
- Source reference shown per item: "Hisn al-Muslim · <category> · #<item id>"
  (the corpus provides category-level provenance, not per-item hadith grading).
- Context mapping follows the corpus's own structure: category 27 is the
  combined morning-and-evening set (Morning and Evening contexts share it with
  separate per-window progress); 28 = sleep; 25 = after-prayer; 1 = waking.
  All 132 categories remain browsable via manual pick.
