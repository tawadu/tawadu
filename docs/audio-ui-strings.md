# Audio UI strings — Arabic / English

> **Phase 3 prep (audio).** This file is the copy contract for the audio settings
> surface, the importer's validation messages, and the in-app credits page.
> It contains **no code and no UI changes** — Phase 3 implementation and the
> Phase 6 Fluent catalogs both read from here.
>
> Sources: [`docs/voice-pack-format.md`](voice-pack-format.md) (the pack contract),
> [`design/brand.md`](../design/brand.md) (tone and tokens),
> [`docs/attribution.md`](attribution.md) (licences and credits),
> [`docs/ROADMAP.md`](ROADMAP.md) Phases 3 and 6.
>
> Status: draft for Sentinel / Atlas review.

---

## 1. Scope

**In scope**

- the AUDIO card in Settings: voice packs, per-event voice, playback;
- the importer's success, cancel and validation messages;
- the two empty states: *no packs yet* and *no voice chosen*;
- the in-app credits / attribution page copy.

**Out of scope**

- the home surface and its strings;
- notification copy (already shipped in Phase 2);
- athkar-engine copy (Phase 4);
- the Fluent catalog files themselves (Phase 6) — this file is their source.

---

## 2. Rules (binding)

1. **One script per string.** Arabic and Latin never share one string, and never
   share one rendered run. This is the Slint bidi rule already recorded in
   `HANDOFF.md` §6 and `ui/main.slint`; it carries into Fluent unchanged.
2. **Placeholders carry the Latin.** Every Latin token — a file name, an SPDX id,
   a number, a reciter name, a source URL — arrives as `{ $placeholder }` and is
   rendered in **its own run**. An Arabic catalogue value therefore never
   contains a Latin letter. See §5.
3. **Numerals stay Western (0–9)**, matching the existing countdown and the hijri
   date. Numbers are always placeholders, never typed into the Arabic string.
4. **No fabricated values.** Nothing here may be filled with a plausible-looking
   placeholder. When a value is unknown the UI shows `audio-value-unavailable`
   ("غير متاح" / "Not available"). A missing duration is "غير متاح", never "0:00".
5. **Tone.** Simplified formal Arabic (فصحى مبسّطة), calm and exact. No
   superlatives, no exclamation marks, no marketing claims, and **no religious
   overclaiming** — the app does not promise a virtue, a reward, or a ruling.
   Address the user as a calm friend, not a sermoniser.
6. **Short.** Every string is sized for a UI card or a toast, not a paragraph.
   Error bodies are one sentence of *what happened* plus one of *what to do*.

---

## 3. Glossary (fixed terms — do not retranslate)

| English | Arabic | Note |
|---|---|---|
| voice pack | حزمة صوتية | a folder or `.tawadupack` with a manifest, audio and a licence |
| manifest | ملف التعريف | the pack's `manifest.json`; the file name itself is a Latin token (§5) |
| reciter | القارئ | an adhan pack may name a مؤذن (mu'adhdhin) instead — use the source's word |
| licence | الترخيص | SPDX id, shown as a Latin badge |
| duration | المدة | total playing time, `mm:ss`; "غير متاح" when unknown |
| adhan | الأذان | |
| iqamah | الإقامة | |
| athkar | الأذكار | plural of ذكر |
| chime | نغمة التنبيه | the short alert tone |
| trim | اقتطاع | cutting playback short; the UI label is إيقاف بعد |
| checksum | بصمة التحقق | the recorded `sha256` |
| credits | التقديرات والتراخيص | the in-app attribution page |
| default | الافتراضي | the pack the app picks when no per-event choice is made |

---

## 4. Strings

Columns are **separate strings**. The Arabic column is pure Arabic; the English
column is pure English. `{ $… }` marks a placeholder (§5).

### 4.1 The AUDIO card

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-title` | Audio | الصوت | card title |
| `audio-packs-heading` | Voice packs | حزم الأصوات | subsection |
| `audio-events-heading` | Voice per event | الصوت لكل حدث | subsection |
| `audio-playback-heading` | Playback | التشغيل | subsection |

### 4.2 Voice packs

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-add-pack` | Add pack | إضافة حزمة | primary action |
| `audio-import-pack` | Import from file… | استيراد من ملف… | file picker (folder or `.tawadupack`) |
| `audio-remove-pack` | Remove | إزالة | row action |
| `audio-remove-pack-title` | Remove this pack? | إزالة الحزمة؟ | confirm dialog title |
| `audio-remove-pack-body` | “{ $name }” will be removed from this device. The original pack source is not affected. | سيُزال «{ $name }» من هذا الجهاز، ولن يتأثر المصدر الأصلي للحزمة. | `{ $name }` is the pack name |
| `audio-remove-confirm` | Remove | إزالة | confirm dialog action |
| `audio-cancel` | Cancel | إلغاء | confirm dialog action |
| `audio-pack-reciter` | Reciter | القارئ | card label |
| `audio-pack-license` | Licence | الترخيص | card label |
| `audio-pack-license-badge` | { $spdx } | { $spdx } | pass-through: the SPDX id, **Latin-only run**, no translation |
| `audio-pack-duration` | Duration | المدة | card label; `mm:ss` |
| `audio-pack-version` | Version | الإصدار | card label (optional) |
| `audio-pack-files` | Files | الملفات | card label (optional) |
| `audio-value-unavailable` | Not available | غير متاح | shown for any unknown value |

### 4.3 Per-event voice

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-voice-default` | Default | الافتراضي | first option in every selector |
| `audio-voice-unset` | Not set | غير محدد | shown when no choice has been made — never a guessed pack |
| `audio-event-athan` | Adhan | الأذان | selector row |
| `audio-event-iqamah` | Iqamah | الإقامة | selector row |
| `audio-event-athkar` | Athkar | الأذكار | selector row |
| `audio-event-chime` | Alert chime | نغمة التنبيه | selector row |
| `audio-voice-empty-title` | No voice selected | لم يُحدد صوت | empty state, per-event |
| `audio-voice-empty-body` | Choose a pack for this event, or leave it on Default. | اختر حزمة لهذا الحدث، أو اتركها على الافتراضي. | when no packs exist, point here at the packs empty state |

### 4.4 Playback

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-volume` | Volume | مستوى الصوت | slider label |
| `audio-mode` | Playback mode | أسلوب التشغيل | segmented control |
| `audio-mode-sequential` | Sequential | متسلسل | |
| `audio-mode-shuffle` | Shuffle | عشوائي | |
| `audio-trim` | Stop after | إيقاف بعد | the user's original ask: cut the athan at a chosen length |
| `audio-trim-none` | No trim | بلا قص | `stop_after_ms = 0` |
| `audio-trim-seconds` | see plural below | see plural below | unit for the trim value |
| `audio-preview` | Preview | معاينة | plays the selection with the current settings |
| `audio-play` | Play | تشغيل | |
| `audio-stop` | Stop | إيقاف | hard stop |
| `audio-playing` | Playing | جارٍ التشغيل | live state |
| `audio-preview-hint` | Plays the selected clip with the current settings. | يشغّل المقطع المحدد بالإعدادات الحالية. | caption under Preview |

**`audio-trim-seconds`** — a Fluent plural, not a flat string:

```ftl
# English — i18n/en/audio.ftl
audio-trim-seconds =
    { $n ->
        [one] { $n } second
       *[other] { $n } seconds
    }

# Arabic — i18n/ar/audio.ftl
audio-trim-seconds =
    { $n ->
        [one] { $n } ثانية
        [two] { $n } ثانيتان
        [few] { $n } ثوانٍ
        [many] { $n } ثانيةً
       *[other] { $n } ثانية
    }
```

For `n = 2` the renderer may drop the numeral and show ثانيتان alone; the numeral
is its own run either way.

### 4.5 Import feedback

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-import-success` | “{ $name }” was added. | أُضيفت «{ $name }». | toast |
| `audio-import-cancelled` | Import cancelled. | أُلغي الاستيراد. | toast; the picker was dismissed |

### 4.6 Import validation errors

Every message is **what happened** + **what to do**. Titles are short; bodies are
one or two sentences.

| Key | English | Arabic |
|---|---|---|
| `audio-error-manifest-missing-title` | Manifest not found | ملف التعريف غير موجود |
| `audio-error-manifest-missing-body` | { $file } was not found in the pack. Choose the whole pack folder and try again. | لم يُعثر على { $file } داخل الحزمة. اختر مجلد الحزمة كاملاً ثم أعد المحاولة. |
| `audio-error-manifest-invalid-title` | Manifest is not valid | ملف التعريف غير صالح |
| `audio-error-manifest-invalid-body` | The manifest could not be read. The file may be damaged, or the pack format is unsupported. Check the file and try again. | تعذّرت قراءة { $file }. تأكّد من سلامة الملف ومن أن صيغة الحزمة مدعومة، ثم أعد المحاولة. |
| `audio-error-license-missing-title` | Licence entry missing | الترخيص مفقود |
| `audio-error-license-missing-body` | The pack was rejected: { $file } has no licence entry. Add the licence to the pack and import again. | رُفضت الحزمة: لا يوجد ترخيص للملف { $file }. أضف الترخيص إلى الحزمة ثم أعد الاستيراد. |
| `audio-error-checksum-title` | Checksum mismatch | عدم تطابق بصمة التحقق |
| `audio-error-checksum-body` | { $file } does not match its recorded checksum; it may be corrupt or changed. Re-download the pack or replace the file and import again. | لا يطابق الملف { $file } بصمة التحقق المسجّلة؛ قد يكون تالفاً أو معدّلاً. أعد تنزيل الحزمة أو استبدل الملف ثم أعد الاستيراد. |
| `audio-error-unsupported-audio-title` | Unsupported audio format | صيغة الصوت غير مدعومة |
| `audio-error-unsupported-audio-body` | { $file } is in a format the app cannot play. Convert it to a supported format and import again. | الملف { $file } بصيغة لا يستطيع التطبيق تشغيلها. حوّله إلى صيغة مدعومة ثم أعد الاستيراد. |
| `audio-error-generic-title` | Could not import the pack | تعذّر استيراد الحزمة |
| `audio-error-generic-body` | Something went wrong. Try again; if it keeps happening, check the pack files. | حدث خطأ غير متوقع. أعد المحاولة، وإن تكرر الخطأ فتحقّق من ملفات الحزمة. |

> If the implementation wants to name the supported formats in
> `audio-error-unsupported-audio-body`, pass them as `{ $formats }` and render
> that value in its own Latin run — do not type format names into the Arabic
> string.

### 4.7 Empty states

| Key | English | Arabic | Notes |
|---|---|---|---|
| `audio-empty-title` | No voice packs yet | لا توجد حزم صوتية بعد | packs list |
| `audio-empty-body` | Add a voice pack to play the adhan and athkar. | أضف حزمة صوتية لتشغيل الأذان والأذكار. | honest: no sample pack is invented |
| `audio-empty-action` | Add pack | إضافة حزمة | button in the empty state |

### 4.8 Credits / attribution page

| Key | English | Arabic | Notes |
|---|---|---|---|
| `credits-title` | Credits and licences | التقديرات والتراخيص | page title |
| `credits-intro` | Every bundled recording and file keeps its own licence, shown here as provided. | كل تسجيل أو ملف مرفق يحتفظ بترخيصه الخاص، ويظهر هنا كما ورد. | no licence is invented or paraphrased |
| `credits-packs-heading` | Voice packs | حزم الأصوات | |
| `credits-fonts-heading` | Fonts | الخطوط | |
| `credits-libraries-heading` | Software libraries | مكتبات البرمجيات | |
| `credits-reciter-label` | Reciter | القارئ | |
| `credits-source-label` | Source | المصدر | |
| `credits-license-label` | Licence | الترخيص | |
| `credits-attribution-label` | Attribution | نص الإسناد | the exact credit text from the licence file |
| `credits-line` | Reciter: { $reciter } · Source: { $source } · Licence: { $license } | القارئ: { $reciter } · المصدر: { $source } · الترخيص: { $license } | one line per pack; the three values are Latin/neutral and render in their own runs |
| `credits-view-license` | View licence | عرض الترخيص | opens the pack's `licenses/<file>.txt` |
| `credits-empty` | No licences to show yet. | لا توجد تراخيص لعرضها بعد. | |
| `credits-code-license` | Tawadu is released under the { $license } licence. | يُصدر تطبيق تواضع وفق ترخيص { $license }. | code licence (MIT) |

---

## 5. Placeholder reference

| Placeholder | Type | Value comes from | Rendered |
|---|---|---|---|
| `{ $name }` | string | `manifest.name` | its own run |
| `{ $file }` | string | the offending path inside the pack | its own Latin run |
| `{ $spdx }` | string | `manifest.license` / the file's licence entry | its own Latin run |
| `{ $n }` | number | the trim length | its own run, Western digits |
| `{ $reciter }` | string | the pack's reciter | its own run |
| `{ $source }` | string | `manifest.source_url` or the licence file | its own run |
| `{ $license }` | string | SPDX id | its own Latin run |
| `{ $formats }` | string | the decoder's supported formats | its own Latin run (optional) |

**Rendering rule.** A catalogue value may contain `{ $… }` but no Latin letters.
At render time the value is split at each placeholder: Arabic segments and
placeholder segments become **separate runs** with their own direction. This is
the same pattern the home surface already uses for `location_name` /
`location_name_ar` in `ui/main.slint`.

---

## 6. Import validation map

| # | Condition (implementation side) | Keys shown | What the user is told to do |
|---|---|---|---|
| 1 | no `manifest.json` in the picked folder | `audio-error-manifest-missing-title` + `-body` | choose the whole pack folder |
| 2 | manifest unreadable, or `format` is not `tawadu-pack/1` | `audio-error-manifest-invalid-title` + `-body` | check the file and the pack format |
| 3 | a file has no licence entry | `audio-error-license-missing-title` + `-body` | add the licence and import again — **the pack is rejected** |
| 4 | `sha256` does not match the manifest | `audio-error-checksum-title` + `-body` | re-download the pack or replace the file |
| 5 | the audio codec is not decodable | `audio-error-unsupported-audio-title` + `-body` | convert to a supported format |
| — | anything else | `audio-error-generic-title` + `-body` | try again; check the pack files |

Condition 3 is the hard rule from `docs/voice-pack-format.md` §Rules: *no licence
entry → the pack is rejected at import.* The message says so plainly.

---

## 7. Fluent notes (Phase 6)

- **File layout:** `i18n/en/audio.ftl`, `i18n/ar/audio.ftl`,
  `i18n/en/credits.ftl`, `i18n/ar/credits.ftl`. Keys are identical in both files.
- **Key style:** lower kebab-case, `audio-` / `credits-` prefix, matching the
  names in the tables above. No spaces, no dots, no camelCase.
- **Plurals:** only `audio-trim-seconds` needs a selector today. Add more only
  with a real UI need, and use the CLDR Arabic categories (`zero`, `one`, `two`,
  `few`, `many`, `other`).
- **Bidi:** never concatenate an Arabic catalogue value with a Latin value in
  the UI. Pass the Latin as an argument and render it in its own run (§5).
- **Attributes:** if a long error body ever needs a styled link, use a Fluent
  attribute (`audio-error-…-body = { $file } … { "view-license" -> … }`) rather
  than a second mixed string. Not needed for the current cards.

---

## 8. Attribution page structure

1. **Header** — app name and version, then `credits-code-license`.
2. **Intro** — `credits-intro`: every bundled item keeps its own licence.
3. **Voice packs** — `credits-packs-heading`; one card per pack: pack name,
   `credits-line`, and `credits-view-license`. The card shows the exact text from
   the pack's `licenses/` file, unedited.
4. **Fonts** — `credits-fonts-heading`; one row per font (IBM Plex Sans,
   IBM Plex Sans Arabic, Amiri) with its SIL OFL 1.1 entry.
5. **Software libraries** — `credits-libraries-heading`; one row per crate from
   `docs/attribution.md` §Rust dependencies.
6. **Footer** — `credits-empty` when there is nothing to show; otherwise a line
   pointing to `docs/attribution.md` as the fuller record.

The page never shows a licence that is not recorded, and never paraphrases a
credit line. If a pack was rejected at import it never reaches this page.

---

## 9. Counts

| Group | Keys |
|---|---|
| 4.1 Audio card | 4 |
| 4.2 Voice packs | 14 |
| 4.3 Per-event voice | 8 |
| 4.4 Playback | 12 |
| 4.5 Import feedback | 2 |
| 4.6 Import validation errors | 12 |
| 4.7 Empty states | 3 |
| 4.8 Credits page | 13 |
| **Total** | **68** |

68 catalogue keys = 68 English strings + 68 Arabic strings (one of each is a
Fluent plural block). Every key is listed in §4 with its Arabic and English value.

---

## 10. Open questions for the implementer

1. **Duration** — is it the total of the pack's primary sequence, or per clip?
   The label is neutral; the definition belongs to Phase 3.
2. **Per-event voice** — does "Default" mean the first pack, or a pack the app
   marks as default? The string is agnostic; the behaviour needs a decision.
3. **Supported formats** — if the codec list is user-facing, it arrives as
   `{ $formats }` (§5). Confirm the decoder's real list before showing it.
4. **Remove-pack scope** — does removing a pack also delete its cached render
   files? The copy says only "removed from this device"; keep it true.
