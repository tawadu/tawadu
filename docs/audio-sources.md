# Audio sources — licence-auditable candidate ledger

> Phase 3 prep (Scout). **No audio is committed.** This document is the candidate
> list Sentinel audits before any asset lands in `assets/`. Every row carries an
> explicit rights statement, its page URL, the exact credit text, and a hash of
> the exact bytes inspected. Rule from `attribution.md`: **no licence entry →
> do not bundle.** Quality over quantity.

- Compiled: 2026-11-19 · Scope: starter athan pack + Phase 4 athkar corpus.
- Licence ids are SPDX where one exists. "PDM" is called out as a *mark*, not a
  licence grant — see Tier B.
- Proposed `assets/licenses/<file>.txt` paths are **not yet created**.

---

## 1. Tier A — recommended (explicit permissive licence, cleared on paper)

| ID | File / source | Source URL | Licence | Licence page | Exact attribution text | Duration | Format | Bytes | SHA-256 | Proposed licence entry |
|---|---|---|---|---|---|---|---|---|---|---|
| **A1** | `Beautiful_adhan.ogg` — "Beautiful adhan", author `Adam-synagda`, own work, 2022-04-29 | https://commons.wikimedia.org/wiki/File:Beautiful_adhan.ogg | `CC0-1.0` | https://creativecommons.org/publicdomain/zero/1.0/ | `Beautiful adhan — Adam-synagda — Wikimedia Commons, 2022-04-29 — https://commons.wikimedia.org/wiki/File:Beautiful_adhan.ogg — CC0 1.0 Universal (https://creativecommons.org/publicdomain/zero/1.0/).` | 154.018 s (2:34) | Ogg Vorbis | 1,229,032 | `35fe06b08fe80505c550c33fed8a783fa9901ddc81ac884958b4be048f5b2a79` | `assets/licenses/commons-beautiful-adhan.txt` |
| **A2** | `Adhan.ogg` — "Adhan announcement that prayer is about to start", author `Aishatu98`, own work, 2026-07-15 | https://commons.wikimedia.org/wiki/File:Adhan.ogg | `CC0-1.0` | https://creativecommons.org/publicdomain/zero/1.0/ | `Adhan — Aishatu98 — Wikimedia Commons, 2026-07-15 — https://commons.wikimedia.org/wiki/File:Adhan.ogg — CC0 1.0 Universal (https://creativecommons.org/publicdomain/zero/1.0/).` | 42.098 s (0:42) | Ogg Vorbis | 338,741 | `faeb03e4338554fb8b54dba98ed4949648615b58e991bbd560a89f4837dde413` | `assets/licenses/commons-adhan-short.txt` |
| **A3** | `Azan.ogg` — "Adhan.", author `Andrewler`, own work, 2022-09-03 | https://commons.wikimedia.org/wiki/File:Azan.ogg | `CC BY-SA-4.0` | https://creativecommons.org/licenses/by-sa/4.0/ | `Azan — Andrewler — Wikimedia Commons, 2022-09-03 — https://commons.wikimedia.org/wiki/File:Azan.ogg — licensed CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/); source unmodified; playback trims/fades applied non-destructively.` | 183.484 s (3:03) | Ogg Vorbis | 1,473,809 | `2163ceafb8a05064a5ea5e1e0a7fac5fff958a21fcf830844d557d496b9b23c3` | `assets/licenses/commons-azan.txt` |
| **A4** | `The Adhan - Muslim Call to Prayer - Aaqib Azeez.mp3`, uploaded by `Atcovi`, "Own work", 2020-12-23; reciter credited in the title as Aaqib Azeez (identity unconfirmed — see §5) | https://commons.wikimedia.org/wiki/File:The_Adhan_-_Muslim_Call_to_Prayer_-_Aaqib_Azeez.mp3 | `CC BY-SA-4.0` | https://creativecommons.org/licenses/by-sa/4.0/ | `The Adhan — Muslim Call to Prayer — Aaqib Azeez; uploaded by Atcovi (Wikimedia Commons, 2020-12-23) — https://commons.wikimedia.org/wiki/File:The_Adhan_-_Muslim_Call_to_Prayer_-_Aaqib_Azeez.mp3 — licensed CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/).` | 86.513 s (1:27) | MP3 | 1,448,294 | `0faa59e2028bbb2f60a670b53ec89ffcf628f3c195b43fb41d9bdbc99000c1b3` | `assets/licenses/commons-aaqib-azeez.txt` |

**Licence text (exact, as published on the deeds).** CC0 1.0: "The person who
associated a work with this deed has dedicated the work to the public domain by
waiving all of his or her rights to the work worldwide under copyright law …
You can copy, modify, distribute and perform the work, even for commercial
purposes, all without asking permission." [7] CC BY-SA 4.0: "You are free to:
Share — copy and redistribute the material in any medium or format for any
purpose, even commercially. Adapt — remix, transform, and build upon the
material for any purpose, even commercially. … Attribution — You must give
appropriate credit, provide a link to the license, and indicate if changes were
made. … ShareAlike — If you remix, transform, or build upon the material, you
must distribute your contributions under the same license as the original." [8]

**CC BY-SA share-alike implication (A3, A4).** The recordings may be bundled
alongside MIT-licensed code (mere aggregation does not change the code's
licence), **but any derivative of the audio** — an extracted, trimmed, faded or
transcoded clip — must itself be distributed under CC BY-SA 4.0. The pack
`manifest.json` `license` field for those files must be `CC BY-SA-4.0`, never
`CC0-1.0`, and the attribution must travel with the pack into the Credits
screen. This is the main reason to prefer A1/A2 where a plain full-length athan
will do. CC0 (A1, A2) imposes no attribution duty at all; the credit above is
supplied voluntarily and is useful in-app.

**A2 is not a full athan.** At 42 s it is a short announcement; use it as a
pre-alert/chime or an optional trimmed sequence, not as the five-prayer athan.
A1 (2:34) is the best full-length, no-strings starter candidate. [1][2]

---

## 2. Tier B — conditional: explicit rights statement, but weaker than a licence

**Do not bundle until Sentinel signs off.** Both items declare **Public Domain
Mark 1.0**, whose exact deed text is: "This work has been identified as being
free of known restrictions under copyright law, including all related and
neighboring rights." [11] PDM is a *mark/assertion*, not a licence grant: it
carries no warranties and the identifier makes no legal waiver. These are the
only clearly **Fajr-labelled** athan recordings found with a rights statement at
all; they exist because no CC0/CC-BY Fajr recording was found (§5).

| ID | File / source | Source URL | Rights statement | Statement page | Exact attribution text | Duration | Format | Bytes | SHA-256 | Proposed licence entry |
|---|---|---|---|---|---|---|---|---|---|---|
| **B1** | `Adhan_Doha_Qatar_01_Fajr_Adhan.mp3` — "Adhan Recordings from Doha, Qatar", creator field `Doha, Qatar`, recorded 2013–2014, item `adhan.recordings.from.doha.qatar` | https://archive.org/details/adhan.recordings.from.doha.qatar | `PDM-1.0` (assertion, **not** a licence) | https://creativecommons.org/publicdomain/mark/1.0/ | `Adhan Recordings from Doha, Qatar (recorded 2013–2014), "Adhan_Doha_Qatar_01_Fajr_Adhan.mp3", Internet Archive — Public Domain Mark 1.0 (https://creativecommons.org/publicdomain/mark/1.0/). Source: https://archive.org/details/adhan.recordings.from.doha.qatar` | 217.168 s (3:37) | MP3 | 2,901,706 | `a64afe612048e2e068d9475cf481f34acc1fe3c7f34de700bebe6779fa3d1bc0` | `assets/licenses/archiveorg-doha-fajr.txt` |
| **B2** | `Mishary_Rashid_al_Afasy_Fajr_Adhan.mp3` — item `adhan.notifications`, "Adhan Notifications for Prayer Times." No creator field in item metadata; reciter named only in the filename | https://archive.org/details/adhan.notifications | `PDM-1.0` (assertion, **not** a licence) | https://creativecommons.org/publicdomain/mark/1.0/ | `Mishary Rashid al-Afasy Fajr Adhan, from "Adhan Notifications" — Internet Archive — Public Domain Mark 1.0 (https://creativecommons.org/publicdomain/mark/1.0/). Source: https://archive.org/details/adhan.notifications` | 187.977 s (3:08) | MP3 | 3,106,934 | `c6b3ace1a604c94ca6873a7e4f881fdbc67a304c3dc873d2d1ff4b67b9de6192` | `assets/licenses/archiveorg-mishary-fajr.txt` |

**Why tier B and not A:** the Doha item is a 2013–2014 *recording* whose PDM
assertion has no stated rationale or named individual rights holder; the
`adhan.notifications` item names no creator at all, and a named reciter
(Mishary Rashid al-Afasy) implies a separate performance right that the PDM tag
does not address. "The adhkar/athan as a practice is ancient" does not make a
modern recording public domain. Sentinel's call: accept PDM as sufficient for a
bundled starter pack, or reject. **Unanimous panel rule applies to this
decision** (`PIPELINE.md` conventions). [14][15]

---

## 3. Phase 4 — athkar corpus verification

### Primary: `YousefAsalya/Islamic-Pro-azkar-API` (MIT) — VERIFIED

- Repo: https://github.com/YousefAsalya/Islamic-Pro-azkar-API
- **Licence file path: `LICENSE` at repo root** —
  https://github.com/YousefAsalya/Islamic-Pro-azkar-API/blob/main/LICENSE
- Verified existence: file present, 1,093 bytes; body begins `MIT License` /
  `Copyright (c) 2026 Islamic Pro - Azkar API Contributors`. GitHub's repository
  API reports `license.spdx_id = MIT`. README carries a `License: MIT` badge
  linking to that file and a § License reading "MIT — free for any use". [12][13]
- **Covers the data:** `data/ar.json` (166,566 B), `data/en.json` (145,287 B),
  `data/tr.json` (144,559 B) are in-repo files of the same repository; `en.json`
  parses to a 132-category Hisn al-Muslim dataset with `text`, `audio` and
  repetition fields. Repo also declares an `audio/` directory of 267 MP3s. The
  MIT text names "software and associated documentation files (the Software)"
  without an explicit dataset clause, so the JSON is covered by reasonable
  reading plus the README's project-wide claim, not by an explicit data clause.
- Pinned for the audit: commit `d793023e3b69c91674023f692425950c8ebbb64d`
  (2026-05-10, "feat: complete azkar data - 132 categories ar/en/tr").
- **Caveat for Sentinel:** the 267 recitation MP3s are credited in the README
  to a third party (Hamad Al-Durayhim). Treat MIT as verified for the **text
  data**; do not assume the repo's MIT claim disposes of performance rights in
  the audio. Text first, audio only after a separate check.
- Proposed licence entry: `assets/licenses/azkar-api-mit.txt`.

Exact licence text (verbatim, root `LICENSE`):

> MIT License
>
> Copyright (c) 2026 Islamic Pro - Azkar API Contributors
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

### Fallback: OpenDua (CC BY 4.0) — VERIFIED

- URL: https://opendua.hdfund.org/ · Licence: `CC BY-4.0` ·
  Licence page: https://creativecommons.org/licenses/by/4.0/ [16][10]
- A versioned reference library of 267 Hisn al-Muslim entries (Arabic,
  OpenDua English, transliteration, source references, recitation audio),
  dataset v0.0.4 (2026-09-09), audio v0.0.2, built by Ihsan Labs at the Human
  Development Fund. Site states "OpenDua data · CC BY 4.0" and "Religious source
  text is not claimed; recitation terms are separate."
- Exact attribution text (verbatim; mirrors a third party's bundled
  `docs/licences/OpenDua-CC-BY-4.0.txt`): [17]

  > OpenDua dataset v0.0.4 (https://opendua.org), by Ihsan Labs at the Human
  > Development Fund. Based on Hisn al-Muslim by Sa'id ibn Ali ibn Wahf
  > al-Qahtani. OpenDua editorial text and dataset structure are licensed
  > CC BY 4.0. Recitation by Muhammad Jumah for OpenDua. OpenDua makes no
  > ownership claim over the underlying religious texts.

- Proposed licence entry: `assets/licenses/opendua-cc-by-4.0.txt`.
- Prefer the MIT corpus for Phase 4 text; OpenDua is the fallback if the
  repository disappears or its data clause is judged too implicit.

---

## 4. Excluded — do not bundle (already assessed)

| Source | Reason | Action |
|---|---|---|
| `Kiwifu/adhan-mp3` | No LICENSE file | Excluded — no change |
| `assabile.com` | Free downloads, no redistribution grant | Excluded — no change |
| `mohammed-2-5/islamic-library-data` | No LICENSE | Excluded — no change |
| MoIA (Ministry of Islamic Affairs) | Verbal clearance already recorded in `HANDOFF.md` §4 | **Not re-litigated, not contacted** — per user instruction |
| `Call to prayer by Sabah Fakhry.mp3` (Wikimedia) | Tagged "Public domain", but the stated permission is "Adhan has been in effect since c.622 A.D." — that conflates the *practice* with a **1985 sound recording**. No valid rights grant for the recording. [5] | Reject — do not bundle |
| `Islamic call to worship.oga`, `Maliki doctrine.oga` (Wikimedia, CC BY-SA 4.0) | Valid licence but content is a *lecture about* adhan, not an adhan recitation | Wrong content — exclude |
| `Eid al-Fitr Fajr azan at Malmö Mosque …webm` (Wikimedia, CC BY 3.0) [6] | Licence is valid and it is a genuine Fajr azan, but the source is a 78 MB **video** needing demux/transcode; not hashed, not a drop-in pack asset | Deferred, not a starter-pack candidate |
| `Adhan in Istanbul.webm` (Wikimedia, CC0) | Video (64.9 MB), not audio; extraction would be required | Deferred |

---

## 5. Dead ends and open verification items

1. **No CC0 / CC-BY Fajr adhan audio was found.** Commons searches for
   `fajr`, `subh`, `الصلاة خير من النوم` returned only a Qur'an chapter
   (Al-Fajr) and the Malmö video; archive.org Fajr-tagged athan items all carry
   PDM, not a licence grant. Tier B is the best available Fajr evidence, not a
   clean solution.
2. **Fajr phrase not acoustically verified.** None of A1–A4 states in its
   description that it contains "as-salatu khayrun min an-nawm". Do not label
   any Tier A file as a Fajr variant without a listening check.
3. **A4 author identity:** Wikimedia's `Artist` field is `Atcovi` while the
   title credits `Aaqib Azeez`; confirm who holds attribution before packaging.
4. **Hash provenance.** `upload.wikimedia.org` returned HTTP 429 (robot-policy
   throttle) during this audit. `Adhan.ogg` was hashed from a direct live
   download (338,741 B, matching `imageinfo.size`). A1, A3 and A4 were hashed
   from Internet Archive Wayback snapshots whose byte counts match the live
   Commons `imageinfo.size` values **exactly** (1,229,032 / 1,473,809 /
   1,448,294 B). B1/B2 were hashed from direct archive.org downloads whose
   sizes match their metadata API. Sentinel can re-verify against fresh bytes.
   No audio was written into the repository. [1][3][4][14][15]
6. **Atlas live re-verification (2026-11-19):** A1 and A2 were re-downloaded
   from live Commons-API-derived (`imageinfo`) URLs and SHA-256 **matched the
   ledger exactly** (sizes matched too). Sentinel's live-bytes condition is
   satisfied for the v1 starter assets. A3/A4 remain Wayback-derived pending
   the same check (not in v1). Reproducible script: `scripts/verify_audio_sources.py`
   — run it in the asset PR and paste the output as provenance.

---

## Sources

[1] https://commons.wikimedia.org/wiki/File:Beautiful_adhan.ogg
[2] https://commons.wikimedia.org/wiki/File:Adhan.ogg
[3] https://commons.wikimedia.org/wiki/File:Azan.ogg
[4] https://commons.wikimedia.org/wiki/File:The_Adhan_-_Muslim_Call_to_Prayer_-_Aaqib_Azeez.mp3
[5] https://commons.wikimedia.org/wiki/File:Call_to_prayer_by_Sabah_Fakhry.mp3
[6] https://commons.wikimedia.org/wiki/File:Eid_al-Fitr_Fajr_azan_at_Malm%C3%B6_Mosque_-_19_August_2012.webm
[7] https://creativecommons.org/publicdomain/zero/1.0
[8] https://creativecommons.org/licenses/by-sa/4.0
[10] https://creativecommons.org/licenses/by/4.0
[11] https://creativecommons.org/publicdomain/mark/1.0
[12] https://github.com/YousefAsalya/Islamic-Pro-azkar-API
[13] https://github.com/YousefAsalya/Islamic-Pro-azkar-API/blob/main/LICENSE
[14] https://archive.org/details/adhan.recordings.from.doha.qatar
[15] https://archive.org/details/adhan.notifications
[16] https://opendua.hdfund.org
[17] https://github.com/MohamedAbulgasem/Taqwa/blob/main/docs/ATTRIBUTION.md
