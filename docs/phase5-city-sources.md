# City database sources — licence-auditable candidate ledger

> Phase 5 prep (Scout). **No city data is committed.** This document is the
> candidate list Atlas accepts and Sentinel audits before any asset lands in
> `assets/`. It mirrors the accepted `docs/audio-sources.md` format: candidate
> table, live-verified licence, exact attribution text, live-byte SHA-256
> provenance, per-candidate verdict.
>
> Rule from `docs/attribution.md`: **no licence entry → do not bundle.**
> Rule from `PIPELINE.md`: **the implementation PR must re-verify this ledger
> against fresh LIVE bytes** (a size-match is not a content-match). The
> Phase-3 lesson applies verbatim.

- Compiled: 2026-11-19 (Arab Standard Time) · Scope: bundled GeoNames-derived
  city database + search + optional GPS (Phase 5 task 1).
- All licence claims below were re-fetched from the live sites on 2026-11-19;
  URLs and access date are given per claim. Nothing here is remembered from a
  previous session.
- Proposed `assets/licenses/<file>.txt` paths are **not yet created**.

---

## 1. Primary candidate — GeoNames `cities1000.zip` — **BUNDLE-OK**

| Field | Value |
|---|---|
| ID | **G1** |
| File | `cities1000.zip` |
| Source URL | https://download.geonames.org/export/dump/cities1000.zip |
| Licence | `CC BY-4.0` (Creative Commons Attribution 4.0 International) |
| Licence page | https://creativecommons.org/licenses/by/4.0/ |
| Dump readme (terms) | https://download.geonames.org/export/dump/readme.txt |
| GeoNames terms page | https://www.geonames.org/export/ |
| Bytes (zip) | **11,101,196** |
| **SHA-256 (zip, live)** | **`1dcfe6ea1731529fe53d54c8c524add5a8f4b198cff7ff0ab5e5b774d528db17`** |
| Bytes (inner `cities1000.txt`) | 32,092,517 |
| SHA-256 (inner txt, live) | `d2e8118618ce8510c0f0dae658cd1d228da00c9ed19ad97863dfc0d8879a2bd0` |
| `Last-Modified` served | `Thu, 19 Nov 2026 02:52:31 GMT` |
| `ETag` served | `"a9640c-6609838032c63"` |
| Records | **172,038** rows, all 19 fields, all feature class `P` |
| Inner filename | `cities1000.txt` (tab-delimited UTF-8, **no header row**) |
| Update cadence | **daily** — the dump server re-publishes every day; `moddate` values in this file range `1993-12-10` … `2026-11-18` (i.e. the file is rebuilt daily but each row carries its own last-edit date) |
| Verdict | **BUNDLE-OK** — CC BY 4.0 permits redistribution and bundling with attribution |

### 1.1 Licence verification (live, 2026-11-19)

Three independent live sources, all read on 2026-11-19:

1. **Dump readme** (https://download.geonames.org/export/dump/readme.txt,
   served with the file): *"This work is licensed under a Creative Commons
   Attribution 4.0 License, see https://creativecommons.org/licenses/by/4.0/"*
   and *"The Data is provided 'as is' without warranty or any representation
   of accuracy, timeliness or completeness."*
2. **GeoNames export page** (https://www.geonames.org/export/): terms list
   *"cc-by licence (creative commons attribution license). You should give
   credit to GeoNames when using data or web services with a link or another
   reference to GeoNames."* · *"commercial usage is allowed"* · *"'as is'"*.
3. **About page** (https://www.geonames.org/about.html): *"available for
   download free of charge under a creative commons attribution license"* ·
   *"This work is licensed under a Creative Commons Attribution 4.0 License."*
   Names the operator: *"GeoNames is a project of Unxos GmbH, Tutilostrasse
   17d, 9011 St. Gallen, Switzerland."*

**Deed text** (https://creativecommons.org/licenses/by/4.0/, live 2026-11-19):
*"You are free to: 1. Share — copy and redistribute the material in any medium
or format for any purpose, even commercially. 2. Adapt — remix, transform, and
build upon the material for any purpose, even commercially. … Under the
following terms: 1. Attribution — You must give appropriate credit, provide a
link to the license, and indicate if changes were made. You may do so in any
reasonable manner, but not in any way that suggests the licensor endorses you
or your use. 2. No additional restrictions — You may not apply legal terms or
technological measures that legally restrict others from doing anything the
license permits."*

**What CC BY 4.0 means for bundling.** Redistribution inside a desktop app is
explicitly permitted; the only duties are (a) attribution, (b) a link to the
licence, (c) indicate if changes were made, (d) no extra legal/technical
restrictions on the data. The app is MIT-licensed code; bundling CC BY 4.0
*data* alongside MIT *code* is mere aggregation — the code's licence is
unaffected. **The database must ship with its licence entry and be credited
in the in-app Credits screen** (`docs/attribution.md` rules 1–3).

### 1.2 Exact attribution text (proposed, for `assets/licenses/geonames-cities1000-cc-by-4.0.txt`)

> City data derived from the GeoNames Gazetteer (https://www.geonames.org/),
> `cities1000.zip` — all cities with population > 1000 or seats of an
> administrative division down to PPLA3. This work is licensed under a
> Creative Commons Attribution 4.0 License
> (https://creativecommons.org/licenses/by/4.0/). GeoNames is a project of
> Unxos GmbH, St. Gallen, Switzerland. The data is provided "as is" without
> warranty or any representation of accuracy, timeliness or completeness.
> The database was filtered and re-formatted for Tawadu; the source records
> were otherwise unmodified.

Notes for the implementation PR: (a) "indicate if changes were made" is
satisfied by the last sentence — it must not be dropped; (b) the same text
should appear in the in-app Credits screen (Phase 6 task 6); (c) keep the
per-file SHA-256 in the licence entry so a reviewer can re-verify without
re-downloading.

### 1.3 Field layout (verbatim from the dump readme, `geoname` table)

Tab-delimited, UTF-8, **no header row**, 19 columns — every row in the live
file has exactly 19 fields:

| # | Field | Type / note | Non-empty in live file |
|---|---|---|---|
| 1 | `geonameid` | integer, stable id | 172,038 / 172,038 |
| 2 | `name` | name (UTF-8) | 172,038 |
| 3 | `asciiname` | plain-ASCII form | 172,038 |
| 4 | `alternatenames` | comma-separated, ASCII + transliterations | 144,642 |
| 5 | `latitude` | decimal degrees, WGS84 | 172,038 |
| 6 | `longitude` | decimal degrees, WGS84 | 172,038 |
| 7 | `feature class` | `P` for all rows here | 172,038 |
| 8 | `feature code` | `PPL`, `PPLA`, `PPLA2`…`PPLA5`, `PPLX`, … | 172,038 |
| 9 | `country code` | ISO-3166 alpha-2 | 172,038 |
| 10 | `cc2` | alternate country codes | 119 |
| 11 | `admin1 code` | FIPS-style (ISO for US/CH/BE/ME) | 171,939 |
| 12 | `admin2 code` | county-level | 150,395 |
| 13 | `admin3 code` | | 88,886 |
| 14 | `admin4 code` | | 21,754 |
| 15 | `population` | bigint; `0` for 11,823 rows | 172,038 |
| 16 | `elevation` | metres | 29,312 |
| 17 | `dem` | digital elevation model | 172,038 |
| 18 | `timezone` | **IANA timezone id** | **172,038 (0 empty)** |
| 19 | `modification date` | `yyyy-MM-dd` | 172,038 |

**Correction to the task brief:** the brief listed `lat/lon, country code,
admin1, population, timezone` — all present — but the actual column order is
the readme's 19-column `geoname` order, and **`timezone` is column 18, not
column 15**. Any importer written against the brief's order will silently
misread `dem`/`elevation` as the timezone. Column indices above are verified
against the live file, not assumed.

**Readme caveat (verbatim, important):** *"The field 'alternatenames' in the
table 'geoname' is a short version of the 'alternatenames' table without links
and postal codes but with ascii transliterations. You probably don't need
both. If you don't need to know the language of a name variant, the field
'alternatenames' will be sufficient. If you need to know the language of a
name variant, then you will need to load the table 'alternatenames' and you
can drop the column in the geoname table."* — see §2.

### 1.4 Live-file statistics (computed from the downloaded bytes)

- 172,038 records; feature class `P` for 100%; top feature codes: `PPL`
  98,508 · `PPLA3` 30,520 · `PPLA2` 23,376 · `PPLX` 9,717 · `PPLA4` 5,530 ·
  `PPLA` 3,476 · `PPLL` 450 · `PPLC` 241.
- 246 distinct country codes.
- Population: max 24,874,500; sum 4,578,875,334; 11,823 rows with `population`
  `0` (mostly seats of admin divisions that kept their place despite no
  counted population).
- Coordinates: lat range `+78.22334` … `-54.93355`. **196 records above 66.5°N;
  0 below −66.5°S.**
- `timezone`: 392 distinct values, **0 empty**. All 392 parse successfully as
  `chrono_tz::Tz` (verified by running the app's own `chrono-tz 0.10`
  dependency against the extracted list: `parsed_ok=392 failed=0`). The
  GeoNames dump today carries only IANA names — no legacy numeric/offset forms
  and no `Etc/` entries.
- Alternate names: 968,907 entries total; 6.7 per row on average; max 267 on a
  single row.
- 8,131 `(name, country)` groups have more than one row (14,135 extra rows) —
  e.g. several villages share a name inside one country. **Search must
  disambiguate by country/admin, or show a list, never silently pick one.**
- 0 duplicate `geonameid`s.

### 1.5 Not downloaded (documented from the site only)

| File | Bytes (live HEAD) | `Last-Modified` | Licence | Note |
|---|---|---|---|---|
| `allCountries.zip` | **422,561,625** (403M on the directory listing) | 2026-11-19 03:52 | CC BY-4.0 (same readme) | Explicitly out of scope: ~38× the size of `cities1000.zip` and mostly non-city features. Per the task brief, not downloaded. |
| `alternateNamesV2.zip` | **205,205,071** (196M on the directory listing) | 2026-11-19 03:57 | CC BY-4.0 (same readme) | See §2. Over the ~100 MB budget → hash **PENDING**. |

Both `HEAD` requests returned HTTP 200 with the sizes above; the sizes match
the directory listing at https://download.geonames.org/export/dump/ read on
2026-11-19.

---

## 2. Arabic-name coverage — the real question, answered from the live bytes

### 2.1 Case study: Riyadh (geonameid 108410)

Live record from `cities1000.txt`:

```
108410  Riyadh  Riyadh  Al-Riad,Al-Riyad,…,الرياض,رىياد,رياض,ریاض,ڕیاز,…
        24.68773  46.72185  P  PPLC  SA  (cc2 empty)  10  (a2-a4 empty)
        4205961  (elevation empty)  612  Asia/Riyadh  2024-03-26
```

- **`alternatenames` DOES contain Arabic script for Riyadh**: the five
  Arabic-script entries are `الرياض` (ar), `رىياد` (ug), `رياض` (ps),
  `ریاض` (fa/ur/pnb/azb/lrc/mzn), `ڕیاز` (ckb). The exact `ar` entry is
  **`الرياض`** — byte-identical to the app's current `name_ar` in
  `crates/tawadu-core/src/locations.rs:67`.
- Verified against GeoNames' own page
  (https://www.geonames.org/108410/riyadh.html, live 2026-11-19): the
  alternate-names tab lists `Arabic الرياض` and `Egyptian Arabic الرياض` with
  `locale` `ar` / `arz`, `isOfficialName: false`, `isPreferredName` absent.
  The embedded page data carries the same 97 alternate-name objects; **no
  entry anywhere is flagged `isOfficialName: true`**.
- **Consequence for the app:** the Arabic display name cannot be *derived*
  reliably from `alternatenames` alone, because (a) the column mixes scripts
  and languages with no language tags, (b) there is no preferred flag in the
  `geoname` dump, and (c) 30% of Saudi rows have no Arabic script at all. The
  honest engineering answer is: **use `alternatenames` Arabic-script entries as
  a search alias, and keep a small curated Arabic display-name table for the
  cities the app actually ships** (the current 28 plus any new defaults).
  `name_ar` is a UI label, not a data-mining output.

### 2.2 Coverage measured across the live file

| Country | Rows in `cities1000` | With Arabic-script alternate | % |
|---|---|---|---|
| SA | 159 | 112 | 70% |
| AE | 105 | 57 | 54% |
| EG | 260 | 215 | 82% |
| QA | 42 | 41 | 97% |
| KW | 28 | 25 | 89% |
| JO | 146 | 141 | 96% |
| IQ | 173 | 165 | 95% |
| TR | 2,841 | 264 | 9% |
| GB | 4,645 | 1,067 | 22% |
| US | 17,345 | 10,586 | 61% |
| NO | 533 | 50 | 9% |
| NZ | 647 | 40 | 6% |

Saudi rows with **no** Arabic script in `alternatenames`, population > 50,000:
`Ţurayf` (66,014) · `Unaizah` (183,319) · `Sulţānah` (946,697) · `Rābigh`
(72,928) · `King Faisal Military City` (65,000) · `Al Līth` (72,000) ·
`Al Khafjī` (54,857) · `Ad Dawādimī` (86,861) · `Bariq` (75,351).

Note `Sulţānah` (pop 946,697) — a major Saudi place whose `name` is a
transliteration with no Arabic alias in the dump. This is exactly the failure
mode that makes "derive `name_ar` from `alternatenames`" unsafe.

### 2.3 Second candidate — `alternateNamesV2.zip` — **CANDIDATE-LATER (hash PENDING)**

| Field | Value |
|---|---|
| ID | **G2** |
| File | `alternateNamesV2.zip` |
| Source URL | https://download.geonames.org/export/dump/alternateNamesV2.zip |
| Licence | `CC BY-4.0` (same dump readme) |
| Bytes | **205,205,071** (live HEAD, 2026-11-19) |
| SHA-256 | **PENDING — not downloaded** (205 MB > the ~100 MB budget set for this task) |
| `Last-Modified` | `Thu, 19 Nov 2026 02:57:33 GMT` |
| Fields | `alternateNameId`, `geonameid`, `isolanguage`, `alternate name`, `isPreferredName`, `isShortName`, `isColloquial`, `isHistoric`, `from`, `to` |
| Verdict | **CANDIDATE-LATER** — not needed for the v1 database; revisit only if Arabic display names must be exhaustive |

**Why it is not needed for v1.** `cities1000.txt` already carries ~969 k
alternate-name entries including Arabic script for 32,465 rows. The V2 file
adds **language tags and preferred/official flags** — which would let an
importer pick "the Arabic name" deterministically — but at 19× the size of the
city file and 205 MB uncompressed. The cost/benefit only makes sense if the
product decision becomes "every city shows its Arabic name", which Phase 5
task 1 does not require. If it is ever pulled in, the same live-hash discipline
applies (hash the downloaded bytes, record them here, add the licence entry).

**If G2 is used later, the attribution must be updated** to name both files;
the licence text in §1.2 covers only `cities1000.zip`.

---

## 3. Timezone correctness vs the app's current handling

The app resolves a city's timezone by parsing the IANA string into
`chrono_tz::Tz`, falling back to `UTC` on failure:

- `crates/tawadu-core/src/locations.rs:53` —
  `let timezone: Tz = self.timezone.parse().unwrap_or(Tz::UTC);`
- `crates/tawadu-core/src/settings.rs:126` — same pattern.

**Verified against the live dump:** all 392 distinct timezone strings in
`cities1000.txt` parse as `chrono_tz::Tz` with the app's own dependency
(`chrono-tz 0.10`) — `parsed_ok=392 failed=0`. **Zero rows fall back to UTC
in practice.** The exit-criteria cities resolve as:

| City | GeoNames `timezone` | App today | DST behaviour |
|---|---|---|---|
| Riyadh | `Asia/Riyadh` | `Asia/Riyadh` | No DST (UTC+3 year-round) |
| Oslo | `Europe/Oslo` | `Europe/Oslo` | DST: CET/CEST |
| Auckland | `Pacific/Auckland` | `Pacific/Auckland` | DST: NZST/NZDT |
| Tromsø | `Europe/Oslo` | *not in the 28-city list* | DST: CET/CEST |

Caveats to carry into the implementation:

1. **`chrono-tz` needs a tzdb version, not just a name.** The crate parses the
   IANA name and applies the tzdb rules compiled into the crate. A GeoNames
   zone name that is not in the crate's tzdb would fall back to UTC silently
   (the `unwrap_or`). Today this is empty, but the *set* of zone names drifts
   over years; the importer should log/record any name that fails to parse
   rather than silently shipping UTC. This is a data-quality gate, not a
   licence issue.
2. **GeoNames' own `timeZones.txt` is a separate file** (13,896 bytes, live
   HEAD 2026-11-19; 418 data rows + header). It carries country → timezone
   with Jan/Jul offsets and `rawOffset`. The `cities1000` per-row `timezone`
   is already the IANA id, so this file is **not required** for the city
   database — noted only so nobody imports it "for DST".
3. **DST is a property of the zone, not the coordinates.** Using the GeoNames
   IANA id is correct; recomputing offsets from latitude/longitude would be
   wrong. The exit-criteria cities are all in zones with DST except Riyadh.
4. **High latitude is not a timezone problem** — it is a *prayer-time* problem,
   handled in §6.

---

## 4. Alternatives — assessed and rejected/deferred

| ID | Source | Licence (verified live 2026-11-19) | Size / coverage | Verdict | Why |
|---|---|---|---|---|---|
| **A1** | SimpleMaps World Cities **Basic** (free tier) — https://simplemaps.com/data/world-cities | `CC BY-4.0` — licence page https://simplemaps.com/data/license, §18: *"Basic World Cities Database: Licensed under the Creative Commons Attribution 4.0 License"* | 5,221,554 B zip (live HEAD); ~50.2 k prominent places | **REJECT for bundling** | Licence is CC BY 4.0 and attribution is required, so it is *licence-compatible* — but coverage is the problem: ~50 k prominent cities only, and it is not GeoNames-derived, so it brings a second source and a second attribution chain for no gain. The page also says attribution is required "in some fashion" with a backlink to `simplemaps.com/data/world-cities`; our Credits screen can carry that, but the dataset is a strict subset of what we need. |
| **A2** | SimpleMaps World Cities **Pro/Comprehensive** — same page | *"Permissive, no redistribution"* — licence page §4: *"The Customer may not publish or otherwise expose any substantial portion of the Database in a publicly accessible form"*; §5: *"the Customer may not publicly redistribute the Database without prior written permission."* | 2.0 M / 4.4 M places ($199 / $499) | **REJECT — licence forbids our use** | Explicitly prohibits redistribution. Bundling a database inside a shipped app is redistribution. Not usable regardless of price. |
| **A3** | Natural Earth **Populated Places** — https://www.naturalearthdata.com/about/terms-of-use/ | **Public domain** — verbatim: *"All versions of Natural Earth raster + vector map data found on this website are in the public domain. You may use the maps in any manner, including modifying the content and design, electronic dissemination, and offset printing."* · *"No permission is needed to use Natural Earth. Crediting the authors is unnecessary."* | 2,811,199 B zip (live HEAD); 1:10m layer = 7,342 points, 1:110m = 243 points | **CANDIDATE-LATER** | Best licence of the set (PD, no attribution burden), but 7,342 points at 1:10m is a *cartographic* selection, not a gazetteer: no population, no admin hierarchy, no timezone, and small towns are absent by design. Cannot replace GeoNames for search. Worth revisiting only as a shape/basemap source, not as the city table. |
| **A4** | OpenStreetMap / Geofabrik extracts — https://www.openstreetmap.org/copyright/ | `ODbL-1.0` — *"You are free to copy, distribute, transmit and adapt our data, as long as you credit OpenStreetMap and its contributors. If you alter or build upon our data, you may distribute the result only under the same license."* Geofabrik: *"Derived databases have to retain the same licence."* | country `.osm.pbf` extracts, GB-sized | **REJECT** | Share-alike on a *derived database* would force the derived city table under ODbL — a viral licence that conflicts with shipping a clean MIT + CC BY data story. Not worth the legal surface for coordinates GeoNames already provides under CC BY 4.0. |
| **A5** | GeoNames **per-country** `SA.zip` / `XX.zip` | CC BY-4.0 (same readme) | `SA.zip` 1,232,808 B (live HEAD) | **CANDIDATE-LATER** | Same licence, same format, but a *different selection rule* (all features in the country, not the >1000-population world set). Useful if the product ever wants "every Saudi place" for a local search; not needed for the global database. |
| **A6** | Wikidata (city items) | CC0 | — | **REJECT for v1** | No stable offline bulk extract for our shape; coordinates need per-item queries. Not a drop-in gazetteer. Noted only as the source Natural Earth credits for names in 20+ languages. |

**Recommendation:** G1 (`cities1000.zip`) is the v1 database. A1/A2/A4 are
rejected on licence or coverage; A3 is public-domain but far too sparse; A5 is
the right follow-up if Saudi-local coverage ever needs to exceed the global
set.

---

## 5. Search + offline constraints (inputs for Forge/Prism, not decisions)

### 5.1 Offline-first / no-telemetry

- `PIPELINE.md` conventions: *"Offline-first; no telemetry."*
- `crates/tawadu-core/src/locations.rs:1-4` already states the intent:
  *"Offline-first: a bundled city database is the default. Network lookups are
  a later, optional enhancement and must degrade silently."*
- Therefore: **the database is compiled/bundled into the binary (or shipped
  beside it), not fetched at runtime.** No geocoding API call at startup, no
  "search online" fallback in v1. If an online search is ever added it must be
  opt-in, must never be the default, and must not send the query anywhere
  without an explicit user action.

### 5.2 Size budget (real numbers, not guesses)

| Artifact | Bytes (measured) | Notes |
|---|---|---|
| `cities1000.zip` (as downloaded) | 11,101,196 | Already compressed; fine as a build-time source |
| `cities1000.txt` (uncompressed) | 32,092,517 | Too big to embed verbatim |
| **Trimmed TSV** — columns 1,2,3,5,6,9,11,15,18 only | **12,657,748** (12.66 MB) | Measured by rebuilding the projection from the live file |
| **SQLite** with `asciiname` index | **17,313,792** (17.31 MB) | Measured with the project's own SQLite; page overhead + index |

Measured, not estimated: `alternatenames` alone is 10,356,315 B = **32% of the
raw text file**, so dropping it is the single biggest win (32.1 MB → 12.7 MB
before any SQLite/index overhead). Even the "small" options are an order of
magnitude larger than the current 28-row table, so the implementation must be
deliberate:

- **Do not commit the raw dump or the raw text.** 11 MB / 32 MB of build input
  is reproducible from the URL + hash.
- **The bundled artifact should be the trimmed, derived file**, and it is that
  file which gets the licence entry and its own recorded SHA-256.
- **17 MB of SQLite is acceptable** for a desktop app but it is not free: it
  sits inside the Phase 6 memory budget (< 120 MB) and the cold-start budget
  (< 1.5 s). A page-cached read of a 17 MB file is fine; building an index on
  the UI thread at startup is not. Measure both, per Phase 6 task 4.
- If the size must come down further, the levers are: drop `admin1` (1.5 MB),
  store coordinates as scaled integers, or ship a binary format. These are
  Forge's call, not Scout's — flagged here because the honest number is
  12–17 MB, not "a few MB".

### 5.3 Search approach (notes, not decisions)

- **Prefix search** on `asciiname` (case-insensitive, ASCII-folded) + exact
  match on `name` + **Arabic-script match** on the Arabic alias set.
- **Arabic normalization is required**, not optional: strip the Arabic diacritics
  (`\u064B-\u065F`, `\u0670`), unify `أ إ آ ا` → `ا`, unify `ى` → `ي`, unify
  `ة` → `ه` (or the reverse) for *search only* — never for display. The app's
  current Arabic names carry full diacritics (`مكة المكرمة`,
  `المدينة المنورة`), so a user typing `مكة` must still match. This is the
  same class of problem Quill solved for UI strings; the search index needs
  its own normalization pass.
- **Duplicate names are real** (8,131 `(name, country)` groups, §1.4). Search
  must either (a) rank by population and show the top N with country labels,
  or (b) require a country disambiguation step. Silently returning the first
  hit is a correctness bug.
- **Index shape for 172 k rows:** a sorted array of `(normalized_prefix_key,
  geonameid)` plus the main table is enough for prefix search; a full-text
  engine is not warranted at this size. Target: < 5 ms for a keystroke on a
  cold index, measured — not assumed (Phase 6 task 4 makes budgets binding).
- **`find_city` today returns the first exact/prefix match**
  (`locations.rs:292-305`); the new search must keep that function honest
  (return `None` when ambiguous, or return a list the UI disambiguates).
- **Timezone resolution is already correct** (§3); the new table just supplies
  more rows.

### 5.4 GPS (optional later) — platform implications only

- **Not part of task 1.** Optional, later.
- Windows: `windows.devices.geolocation` (WinRT) — needs a capability declaration
  in the packaged app; on an unpackaged `cargo run` build it is unreliable.
- macOS: CoreLocation — requires `NSLocationWhenInUseUsageDescription` in the
  bundle `Info.plist` and a user prompt.
- Linux: GeoClue2 via D-Bus (`org.freedesktop.GeoClue2`) — often absent on
  headless/minimal desktops; must degrade silently.
- Cross-platform note for Phase 7 (mobile): keep the GPS resolver behind a
  `tawadu-core` trait so `tawadu-core` stays platform-free (a hard repo rule).
- **Privacy:** a location fix is sensitive. If it is ever added, it must be
  opt-in, never sent off-device, and the app must work fully without it.

---

## 6. High-latitude honesty — the dataset supports the truth

Phase 5 task 2 requires that a user in Tromsø in June *"is told the truth
rather than shown a fabricated sunrise."* Two independent facts make this
achievable with G1:

1. **The core already returns `None` honestly.** `crates/tawadu-core/src/prayer.rs:287-301`
   documents: *"A time is `None` when the sun never reaches the position it
   needs, which happens inside the polar circles. That is the honest answer
   rather than an invented one; the caller decides what to show."* All seven
   times are `Option<DateTime<Utc>>`, and the tray already renders `None` as an
   em dash (`crates/tawadu-app/src/tray.rs:40-45`, test at `:212`). The polar
   case is unit-tested: `polar_day_has_no_sunrise_or_sunset` at
   `crates/tawadu-core/src/prayer.rs:668-678` asserts Tromsø midsummer
   (`69.6492, 18.9553`, 2026-06-21) yields `sunrise.is_none()` and
   `maghrib.is_none()`.
2. **The dataset carries real coordinates for the high-latitude places**, so
   the UX can show a genuine place with genuine `None` results instead of
   falling back to a low-latitude substitute. From the live file:

   | Place | geonameid | lat | lon | tz | pop |
   |---|---|---|---|---|---|
   | Longyearbyen (Svalbard) | 2729907 | 78.22334 | 15.64689 | `Arctic/Longyearbyen` | 2,368 |
   | Dikson (RU) | — | 73.50819 | — | `Asia/Krasnoyarsk` | 1,113 |
   | Upernavik (GL) | — | 72.78358 | — | `America/Nuuk` | 1,129 |
   | Pond Inlet (CA) | — | 72.6992 | — | `America/Iqaluit` | 1,555 |
   | Utqiagvik (US) | — | 71.29058 | — | `America/Anchorage` | 4,384 |
   | Honningsvåg (NO) | — | 70.98209 | — | `Europe/Oslo` | 2,484 |
   | Hammerfest (NO) | — | 70.66312 | — | `Europe/Oslo` | 11,448 |
   | Vardø (NO) | — | 70.37048 | — | `Europe/Oslo` | 1,875 |

   **196 records sit above 66.5°N**; the dataset's northernmost point is
   Longyearbyen at 78.22°N. Tromsø itself is present: geonameid `3133895`,
   `69.6489, 18.95508`, `Europe/Oslo`, pop 41,915.

**Caveat for task 2:** `HighLatitudeRule` (`prayer.rs:141-154`) is a *rule for
bounding Fajr/Isha* when twilight is persistent — it is not a fix for a
midnight sun. When the sun never reaches the required angle, the correct
output is `None`, and the UI must say so. The rule selector and the honest
`None` path are complementary, not alternatives.

---

## 7. Verdicts (one line each)

| ID | Candidate | Verdict |
|---|---|---|
| **G1** | GeoNames `cities1000.zip` — CC BY 4.0, 11,101,196 B, 172,038 rows, live hash recorded | **BUNDLE-OK** — v1 database |
| **G2** | GeoNames `alternateNamesV2.zip` — CC BY 4.0, 205,205,071 B, hash PENDING | **CANDIDATE-LATER** — only if exhaustive Arabic names become a requirement |
| **A1** | SimpleMaps Basic — CC BY 4.0, ~50 k places | **REJECT** — subset of what we need; second attribution chain for no gain |
| **A2** | SimpleMaps Pro/Comprehensive — "no redistribution" | **REJECT** — licence forbids bundling |
| **A3** | Natural Earth Populated Places — public domain, 7,342 points | **CANDIDATE-LATER** — PD but cartographic, not a gazetteer |
| **A4** | OpenStreetMap / Geofabrik — ODbL | **REJECT** — share-alike on derived database |
| **A5** | GeoNames **per-country** `SA.zip` / `XX.zip` | CC BY-4.0 (same readme) | `SA.zip` 1,232,808 B (live HEAD) | **CANDIDATE-LATER** — for a future Saudi-local search |
| **A6** | Wikidata — CC0 | **REJECT for v1** — not a drop-in offline gazetteer |

---

## 8. Binding conditions on the implementation PR (Atlas/Sentinel)

1. **Re-verify against fresh LIVE bytes** before any asset commit: download
   `cities1000.zip` again, compare SHA-256 to
   `1dcfe6ea1731529fe53d54c8c524add5a8f4b198cff7ff0ab5e5b774d528db17`.
   A size match is not a content match (Phase-3 lesson). If the dump rotated
   (it is daily), record the new hash + `Last-Modified` and update this ledger
   in the same PR.
2. **Licence entry is mandatory before the asset commit** —
   `assets/licenses/geonames-cities1000-cc-by-4.0.txt` with the §1.2
   attribution text, the file hash, and the source URL. No licence entry → no
   commit (repo rule).
3. **The raw 11 MB dump does not belong in the repo.** Commit the *derived*
   database (trimmed, with its own recorded hash) plus the licence entry. If
   the derived DB is committed, its hash goes in the licence entry.
4. **Column order is the 19-column `geoname` order; `timezone` is column 18.**
   Do not trust the task brief's shorter field list.
5. **Arabic display names are curated, not mined.** Use `alternatenames`
   Arabic-script entries as search aliases; keep the curated `name_ar` table
   for shipped cities. `Sulţānah` (pop 946,697, no Arabic alias) is the
   counter-example that proves the rule.
6. **Search must handle duplicate names** (8,131 `(name, country)` groups) —
   rank by population + show country, or disambiguate; never silently pick.
7. **Offline-first, no telemetry:** bundled data, no runtime fetch, no
   location or query leaves the device.
8. **High latitude:** show real coordinates; render `None` honestly. Do not
   invent a sunrise.

---

## 9. Open questions for Atlas

1. **Derived DB format:** SQLite (already a dependency, `storage.rs`) vs a
   purpose-built binary/TSV. SQLite is the path of least resistance and gives
   indexing for free; a binary format is smaller and faster. Atlas's call.
2. **Bundle the raw zip or only the derived DB?** Recommendation: derived only
   (§8.3), raw reproducible from URL + hash. Confirm.
3. **Arabic-name policy:** curated table for shipped cities (recommended) vs
   pull `alternateNamesV2.zip` later for exhaustive Arabic. Curated is cheaper
   and more accurate for the cities that matter; exhaustive is 205 MB.
4. **Saudi-local coverage:** is the >1000-population global set enough, or does
   the product want `SA.zip` (all Saudi features) for a local search? A5 is
   the follow-up.
5. **`name_ar` for the current 28 cities:** keep the app's existing hand-written
   Arabic names (they carry proper diacritics, e.g. `مكة المكرمة`) or switch to
   GeoNames' bare `الرياض`? Keeping the curated ones is better UX; the GeoNames
   Arabic alias is the *search* key.
6. **GPS timing:** confirm GPS stays out of Phase 5 task 1 (assumed yes).
7. **Budget check:** Phase 6 task 4 makes memory < 120 MB binding. The measured
   bundled database is **12.7 MB (trimmed TSV) / 17.3 MB (SQLite + index)** —
   see §5.2. That is affordable but not free, and the index must not be built
   on the UI thread at startup (cold start < 1.5 s budget).

---

## Sources

- [1] https://download.geonames.org/export/dump/readme.txt — dump format, field layout, licence line (live 2026-11-19)
- [2] https://download.geonames.org/export/dump/ — directory listing, sizes, `Last-Modified` (live 2026-11-19)
- [3] https://www.geonames.org/export/ — terms and conditions (live 2026-11-19)
- [4] https://www.geonames.org/about.html — licence + operator identity (live 2026-11-19)
- [5] https://creativecommons.org/licenses/by/4.0/ — CC BY 4.0 deed (live 2026-11-19)
- [6] https://www.geonames.org/108410/riyadh.html — Riyadh feature + alternate-names page (live 2026-11-19)
- [7] https://www.geonames.org/manual.html — alternate-name guidelines, `isPreferred`, language codes (live 2026-11-19)
- [8] https://simplemaps.com/data/world-cities — Basic/Pro/Comprehensive tiers, sizes, licence column (live 2026-11-19)
- [9] https://simplemaps.com/data/license — §4 no-publication, §18 Basic World Cities CC BY 4.0 (live 2026-11-19, effective 2026-10-29)
- [10] https://www.naturalearthdata.com/about/terms-of-use/ — public domain terms (live 2026-11-19)
- [11] https://www.openstreetmap.org/copyright/en-EN — ODbL terms + attribution (live 2026-11-19)
- [12] https://www.geofabrik.de/data/download.html — Geofabrik ODbL terms (live 2026-11-19)
- [13] `crates/tawadu-core/src/locations.rs` — current 28-city table, `find_city`, tz parse
- [14] `crates/tawadu-core/src/prayer.rs` — `Option` prayer times, `HighLatitudeRule`, polar test
- [15] `docs/audio-sources.md` — the ledger format this document mirrors

---

## 10. Atlas acceptance + decisions (2026-11-20 00:1x AST, post-power-outage recovery)

**ACCEPTED.** Ledger recovered from Scout's worktree after the session died in the
power outage (file complete: 538 lines, Sources intact); committed to main by Atlas.
Scout's worktree copy removed to avoid divergence. Answers to §9 (binding for Forge):

1. **Derived DB format = SQLite.** Already a dependency (storage.rs), indexing for
   free, 17.3 MB measured is inside the 120 MB memory budget. Index build must run
   OFF the UI thread (background at startup or lazy on first search); cold start
   < 1.5 s stays binding.
2. **Derived only, confirmed.** Raw zip never enters the repo; reproducible from
   URL + SHA-256 (§8.1). The derived DB carries its own recorded hash in the
   licence entry (§8.3).
3. **Arabic policy = curated + aliases.** Curated `name_ar` for shipped cities;
   GeoNames Arabic-script alternatenames are SEARCH ALIASES only. alternateNamesV2
   (205 MB) stays CANDIDATE-LATER.
4. **Saudi coverage:** cities1000 global set is v1. SA.zip (A5) = documented
   follow-up if small-town Saudi search becomes a product need.
5. **Keep the existing hand-written diacritized names** for the 28 cities
   (مكة المكرمة etc.); GeoNames bare Arabic is the search key, not the display name.
6. **GPS = final slice of Phase 5** (opt-in, platform-specific, after DB + search +
   mosque profiles). If it balloons it slips to Phase 6 WITHOUT blocking the phase
   exit — the exit criterion is correct/explainable times (Oslo/Auckland/Riyadh) +
   polar honesty (Tromsø), not GPS.
7. **Budget accepted** with the off-UI-thread index condition above.

Binding conditions §8.1–8.8 carry into the implementation PR verbatim (live-byte
re-verify before asset commit; licence entry first; 19-column order; duplicate-name
disambiguation never silent; offline-first; None honesty). Sentinel licence audit
dispatched on this ledger before any asset lands.
