# Phase 5 city-source ledger — Sentinel licence audit

> Audited: **2026-11-19 21:11–21:20 UTC** (2026-11-20 00:11–00:20 AST) · Auditor: Sentinel
> Subject: `docs/phase5-city-sources.md` @ `b2c906a` (accepted, incl. Atlas §10)
> Scope: licence claims re-fetched LIVE, `cities1000.zip` re-downloaded to scratch and
> hashed, and Scout's live-byte statistics independently reproduced. **No asset was
> written into the repo.** Scratch: `…/profiles/sentinel/cache/scratch/gn/`.

## 1. Licence verification — CC BY 4.0 (all claims confirmed live)

| Source | Ledger claim | Live result (2026-11-19 21:11 UTC) |
|---|---|---|
| `download.geonames.org/export/dump/readme.txt` | CC BY 4.0 line + "as is" | ✅ verbatim: *"This work is licensed under a Creative Commons Attribution 4.0 License, see https://creativecommons.org/licenses/by/4.0/"* + *"The Data is provided 'as is' without warranty…"* |
| `geonames.org/export/` | cc-by, credit, commercial allowed, as-is | ✅ all four phrases present |
| `geonames.org/about.html` | free of charge under CC attribution; operator = Unxos GmbH | ✅ *"free of charge under a creative commons attribution license"* + *"Unxos GmbH"* + *"Creative Commons Attribution 4.0 License"* |
| `creativecommons.org/licenses/by/4.0/` | deed terms (attribution, no extra restrictions) | ✅ *"No additional restrictions"* present; standard deed text matches the quoted excerpt |

**Bundling conclusion:** redistribution and bundling are permitted; duties are
attribution + licence link + indication of changes. Aggregation of CC BY 4.0 data
with MIT code does not affect the code's licence.

## 2. `cities1000.zip` — re-downloaded and hashed (scratch only)

| Field | Ledger | This audit |
|---|---|---|
| Bytes | 11,101,196 | **11,101,196** ✅ |
| SHA-256 | `1dcfe6ea…8db17` | **`1dcfe6ea1731529fe53d54c8c524add5a8f4b198cff7ff0ab5e5b774d528db17`** ✅ exact |
| `Last-Modified` | `Thu, 19 Nov 2026 02:52:31 GMT` | **identical** ✅ |
| `ETag` | `"a9640c-6609838032c63"` | **identical** ✅ |

**No rotation at audit time** — the dump is byte-stable since 02:52:31 GMT. Rotation
is still possible before the implementation PR, so §8.1 (re-verify fresh bytes, record
new hash + `Last-Modified`) stays binding. A rotation is **not** a rejection.

## 3. Independent reproduction of the live-byte statistics

Recomputed from the freshly downloaded `cities1000.txt` (32,092,517 B, inner name
confirmed, no header row):

| Claim | Ledger | Reproduced |
|---|---|---|
| Records | 172,038 | **172,038** ✅ |
| Fields per row | all 19 | **{19: 172038}** — every row exactly 19 ✅ |
| Distinct timezones / empty | 392 / 0 | **392 / 0** ✅ |
| Rows with Arabic-script alternate | 32,465 | **32,465** ✅ |
| SA coverage | 159 rows, 112 AR, 70% | **159 / 112 / 70%** ✅ |
| TR coverage | 2,841 rows, 9% | **2,841 / 264 / 9%** ✅ |
| Above 66.5°N | 196 | **196** ✅ |
| Population | 0-rows 11,823 · max 24,874,500 · sum 4,578,875,334 | **all three exact** ✅ |
| Top feature codes | PPL 98,508 · PPLA3 30,520 · PPLA2 23,376 · PPLX 9,717 · PPLA4 5,530 | **exact** ✅ |
| Country codes | 246 | **246** ✅ |

**Column order (the correction that matters).** Riyadh (`geonameid` 108410):
field 15 = `4205961` (**population**), field 16 = empty (elevation), field 17 = `612`
(`dem`), **field 18 = `Asia/Riyadh` (timezone)**. Confirmed: the brief's short list is
wrong and the ledger's 19-column `geoname` order is right. An importer written against
the brief would misread population as timezone.

**Counter-example confirmed.** `Sulţānah` (geonameid 101760, SA, pop 946,697) has
alternatenames `Sultanah,Sultanakh,Sulţānah,Султанах` — **no Arabic script**. Deriving
`name_ar` from alternatenames would fabricate or omit a major Saudi city, so §8.5
(curated display names, GeoNames Arabic as search alias) is the correct call.

**Methodology verdict:** sane and reproducible. Statistics were measured from the same
bytes, the Saudi case study checks out, and the failure mode is real, not hypothetical.

## 4. Attribution text (§1.2) — adequate

Contains credit ("derived from the GeoNames Gazetteer"), the licence name + link,
operator identity, the as-is disclaimer, and an explicit *indication of changes*
("filtered and re-formatted… otherwise unmodified"). That satisfies CC BY 4.0's three
duties. Proposed path `assets/licenses/geonames-cities1000-cc-by-4.0.txt` is consistent
with existing naming. Two notes: (a) the text names only `cities1000.zip`, so it must be
extended if G2 (`alternateNamesV2.zip`) ever ships; (b) the changes sentence must survive
into the licence entry for the **derived** database, not just the raw zip.

## 5. Rejection spot-checks — sound

- **A2 SimpleMaps Pro** ✅ confirmed: *"may not publish or otherwise expose any
  substantial portion of the Database in a publicly accessible form"* and *"may not
  publicly redistribute the Database without prior written permission"*. Reject is right.
- **A1 SimpleMaps Basic** ✅ confirmed CC BY 4.0 (`Basic World Cities Database …
  Creative Commons Attribution 4.0 International License`) — rejection is on coverage, not
  licence; correctly reasoned.
- **A3 Natural Earth** ✅ public domain, *"No permission is needed"*; CANDIDATE-LATER on
  gazetteer fitness is reasonable.
- **A4 OSM/Geofabrik** ✅ ODbL, *"same license"* share-alike on derived databases;
  reject avoids a viral obligation on the shipped DB.

## 6. Binding conditions §8 + Atlas decisions §10 — complete; two additions recommended

§8.1–§8.8 plus §10.1–§10.7 cover live re-verification, mandatory licence entry, no raw
dump, column order, curated Arabic, duplicate-name disambiguation, offline-first and
high-latitude honesty. No gap lets an **unlicensed** asset through (§8.2 gates the commit)
or a **fabricated** one (§8.1 gates provenance).

Recommended additions (non-blocking, fold into the implementation brief):

- **§8.9 reproducibility of the derived artifact.** Require the importer/build script to
  be committed in-repo and the **derived** DB's SHA-256 recorded in its licence entry —
  mirroring `scripts/verify_athkar_corpus.py`. Without it a future PR could swap the
  derived DB and update the hash with nothing tying it back to the pinned dump.
- **Surfacing as a numbered condition.** `attribution.md` rule 3 requires every entry in
  the in-app Credits screen; §1.2 says "should appear". Make it an explicit §8 item so
  the Credits row cannot be forgotten.

## 7. High-latitude consistency — confirmed

`docs/architecture.md:47,53` (polar times genuinely undefined; never panics),
`prayer.rs:287-301` ("A time is `None` when the sun never reaches the position it needs…
the honest answer rather than an invented one"), and the polar test
`prayer_day_has_no_sunrise_or_sunset` (`prayer.rs:668-678`, Tromsø midsummer ⇒
`sunrise.is_none()` / `maghrib.is_none()`) all exist as cited, and `tray.rs` renders `None`
as an em dash. §6's "dataset supports the truth" claim is consistent: G1 carries real
coordinates (Longyearbyen 78.22°N, Tromsø 69.65°N), so the UI shows a genuine place with
genuine `None` results. §6's caveat that `HighLatitudeRule` bounds Fajr/Isha but does not
fix a midnight sun is also correct and worth carrying into the implementation.

## 8. Hygiene

Scout's `tzprobe` scratch crate is **not** on `main` and not in the shared worktree.
No asset, code or licence file was touched by this audit.

SENTINEL: APPROVE (ledger audit)
