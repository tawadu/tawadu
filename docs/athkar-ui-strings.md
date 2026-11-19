# Athkar UI strings — Arabic / English

> **Phase 4 prep (athkar engine).** This file is the copy contract for the athkar
> UI surface — the home athkar card, the context switcher, the counter, the item
> view and the corpus attribution. It contains **no code and no UI changes**;
> Forge builds the view from it and Phase 6 turns it into Fluent catalogs.
>
> Conventions are inherited verbatim from
> [`docs/audio-ui-strings.md`](audio-ui-strings.md) §2: bilingual table with
> separate columns, Latin travels in `{ $… }` placeholders rendered as its own
> run, no mixed-script runs, no fabricated values, calm فصحى.
>
> Sources: [`docs/ROADMAP.md`](ROADMAP.md) Phase 4, [`docs/audio-sources.md`](audio-sources.md)
> §3 (corpus verification), [`docs/attribution.md`](attribution.md),
> [`crates/tawadu-core/src/scheduler.rs`](../crates/tawadu-core/src/scheduler.rs)
> (the three athkar windows), [`ui/main.slint`](../ui/main.slint) (the current card).
>
> Status: draft for review. Corpus text is **not** duplicated here — the Arabic
> dhikr text, its diacritics and its reference come from the corpus verbatim.

---

## 1. Scope

**In scope**

- context names and their timing labels;
- progress copy for items and for repetitions, plus the completion and zero states;
- the counter affordances (tap-to-count, reset + confirmation, repeat instruction);
- the item view (reference, translation, transliteration, show/hide translation);
- the corpus attribution block;
- the honest error and empty states (corpus missing, no items in a context).

**Out of scope**

- the dhikr text itself (it lives in the corpus, not in a catalogue);
- recitation audio (deferred: performance-rights check, see PIPELINE Phase 4);
- the athkar scheduling engine (Forge's side);
- Fluent catalog files (Phase 6).

---

## 2. Rules (binding)

1. **One script per string.** Arabic and Latin never share a string or a rendered
   run. Latin values (a reference, a count, a credit line) are `{ $… }`
   placeholders rendered in their own run. See §5.
2. **Corpus text is untouchable.** The Arabic dhikr, its vocalisation and its
   reference are copied from the corpus byte-for-byte. The UI never re-vocalises,
   truncates silently, or substitutes a similar wording.
3. **Counted nouns follow Arabic dual/plural rules.** See §6 — `مرة` / `مرتان` /
   `مرات` / `مرةً`, and the numeral is dropped for one and two.
4. **No fabricated values.** A missing reference hides the line; a missing
   translation hides the toggle; an unknown count shows `athkar-value-unavailable`.
   Nothing plausible is invented.
5. **Tone.** Simplified formal Arabic (فصحى مبسّطة), calm and exact. No
   superlatives, no exclamation marks, and **no religious overclaiming** — the
   app does not promise a reward, a virtue or a ruling, and does not invent a dua.
6. **Short.** Every string is sized for a card, a row or a toast.

---

## 3. Glossary (fixed terms — do not retranslate)

| English | Arabic | Note |
|---|---|---|
| athkar | الأذكار | plural of ذكر; the corpus calls the set حصن المسلم |
| context | السياق | one corpus category or an app mode |
| morning athkar | أذكار الصباح | corpus category |
| evening athkar | أذكار المساء | corpus category |
| after-prayer athkar | أذكار عقب الصلاة | corpus category |
| sleep athkar | أذكار النوم | corpus category |
| waking athkar | أذكار الاستيقاظ | corpus category |
| random | أذكار عشوائية | app mode, not a corpus category |
| manual pick | اختيار يدوي | app mode |
| repetition | التكرار | the corpus `repeat` field |
| reference | المرجع | the corpus source field (e.g. رواه البخاري) |
| transliteration | التهجئة | Latin rendering of the Arabic |
| corpus | مجموعة الأذكار | the bundled Hisn al-Muslim dataset |

---

## 4. Strings

Columns are **separate strings**. The Arabic column is pure Arabic; the English
column is pure English. `{ $… }` marks a placeholder (§5).

### 4.1 Context names

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-context-morning` | Morning athkar | أذكار الصباح | corpus category |
| `athkar-context-evening` | Evening athkar | أذكار المساء | corpus category |
| `athkar-context-after-prayer` | After-prayer athkar | أذكار عقب الصلاة | corpus category |
| `athkar-context-sleep` | Sleep athkar | أذكار النوم | corpus category |
| `athkar-context-waking` | Waking athkar | أذكار الاستيقاظ | corpus category |
| `athkar-context-random` | Random athkar | أذكار عشوائية | app mode — see §10 flag |
| `athkar-context-manual` | Manual pick | اختيار يدوي | app mode |

> The corpus carries 132 categories; only the seven above are in the v1 surface.
> Further categories are added with the same key pattern, and their Arabic is
> taken from the corpus's own category title — never invented here.

### 4.2 Timing labels

When each context is recited. These are descriptive; the computed window time is
a separate dynamic value shown beside the label.

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-time-morning` | After Fajr | بعد الفجر | scheduled window = Fajr + 30 min |
| `athkar-time-evening` | After Asr | بعد العصر | scheduled window = Asr + 30 min |
| `athkar-time-after-prayer` | After each prayer | عقب كل صلاة | user-driven; no scheduled window |
| `athkar-time-sleep` | After Isha | بعد العشاء | scheduled window = Isha + 30 min |
| `athkar-time-waking` | When you wake | عند الاستيقاظ | user-driven; no scheduled window |
| `athkar-time-random` | Any time | في أي وقت | |
| `athkar-time-manual` | Any time | في أي وقت | |

### 4.3 Context switcher

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-context-heading` | Context | السياق | label above the switcher |

### 4.4 Progress

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-progress-items` | { $done } of { $total } | { $done } من { $total } | items read in the context |
| `athkar-progress-reps` | { $done } of { $total } | { $done } من { $total } | repetitions counted on one item |
| `athkar-progress-zero` | Nothing counted yet | لم يُعدَّ شيء بعد | done = 0, total > 0 |
| `athkar-complete-title` | Athkar complete | اكتملت الأذكار | completion state |
| `athkar-complete-body` | You have completed this set. | لقد أكملت هذه المجموعة. | neutral — no promised reward |

### 4.5 Counter

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-tap-to-count` | Tap to count | انقر للعدّ | affordance hint |
| `athkar-count` | Count | العدد | the counter's own value |
| `athkar-target` | Target | الهدف | the item's repetition goal |
| `athkar-reset` | Reset | إعادة الضبط | action |
| `athkar-reset-title` | Reset the counter? | إعادة ضبط العدّاد؟ | confirmation title |
| `athkar-reset-body` | The count will return to zero. | سيُعاد العدّ إلى الصفر. | confirmation body |
| `athkar-repeat-count` | see plural below | see plural below | the per-item repeat instruction |

**`athkar-repeat-count`** — a Fluent plural. The Arabic drops the numeral for one
and two, which is the correct form:

```ftl
# English — i18n/en/athkar.ftl
athkar-repeat-count =
    { $n ->
        [one] { $n } time
       *[other] { $n } times
    }

# Arabic — i18n/ar/athkar.ftl
athkar-repeat-count =
    { $n ->
        [one] مرة
        [two] مرتان
        [few] { $n } مرات
        [many] { $n } مرةً
       *[other] { $n } مرة
    }
```

`few` is 3–10, `many` is 11–99, `other` is 0 and 100+. If the corpus ever carries
`repeat = 0`, hide the target line rather than rendering "0 مرة".

### 4.6 Item view

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-item-reference` | Reference | المرجع | value from the corpus source field; hide if absent |
| `athkar-item-translation` | Translation | الترجمة | English text |
| `athkar-item-transliteration` | Transliteration | التهجئة | see §10 flag |
| `athkar-show-translation` | Show translation | إظهار الترجمة | toggle |
| `athkar-hide-translation` | Hide translation | إخفاء الترجمة | toggle |

### 4.7 Corpus attribution

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-corpus-title` | Hisn al-Muslim | حصن المسلم | the corpus name |
| `athkar-corpus-credit` | { $credit } | { $credit } | pass-through: the exact credit text from the licence entry, **Latin-only run** |
| `athkar-corpus-licence` | Licence | الترخيص | label |
| `athkar-corpus-version` | Pinned version | الإصدار المثبّت | value = the corpus commit pin |

`{ $credit }` is read from `assets/licenses/azkar-api-mit.txt` (proposed) and
shown verbatim. The pin is `d793023e3b69c91674023f692425950c8ebbb64d`.

### 4.8 Errors and empty states

Every message is **what happened** + **what to do**.

| Key | English | Arabic |
|---|---|---|
| `athkar-error-corpus-missing-title` | Athkar corpus not found | مجموعة الأذكار غير موجودة |
| `athkar-error-corpus-missing-body` | The app could not read the athkar corpus. Reinstall the app or restore the corpus file, then try again. | تعذّرت قراءة مجموعة الأذكار. أعد تثبيت التطبيق أو استعد ملف المجموعة ثم أعد المحاولة. |
| `athkar-empty-title` | No athkar in this context | لا توجد أذكار في هذا السياق |
| `athkar-empty-body` | This context has no entries yet. Choose another context. | لا توجد مدخلات في هذا السياق بعد. اختر سياقاً آخر. |
| `athkar-error-generic-title` | Could not load the athkar | تعذّر تحميل الأذكار |
| `athkar-error-generic-body` | Something went wrong. Try again; if it keeps happening, check the corpus file. | حدث خطأ غير متوقع. أعد المحاولة، وإن تكرر الخطأ فتحقّق من ملف المجموعة. |

### 4.9 Screen and card heading

| Key | English | Arabic | Notes |
|---|---|---|---|
| `athkar-heading` | Athkar | الأذكار | replaces the static “MORNING ATHKAR” card label |

---

## 5. Placeholder reference

| Placeholder | Type | Value comes from | Rendered |
|---|---|---|---|
| `{ $done }` | number | the app's progress state | its own run, Western digits |
| `{ $total }` | number | the context's item/repetition count | its own run, Western digits |
| `{ $n }` | number | the corpus `repeat` field | its own run; dropped for 1 and 2 in Arabic |
| `{ $credit }` | string | `assets/licenses/azkar-api-mit.txt` | its own Latin run |
| `{ $version }` | string | the corpus commit pin | its own Latin run |

**Rendering rule.** A catalogue value may contain `{ $… }` but no Latin letters.
At render time the value is split at each placeholder into separate runs with
their own direction — the same pattern as `location_name` / `location_name_ar`
in `ui/main.slint`.

---

## 6. Counted-noun forms (the quality bar)

Arabic counts a noun differently from English. The repeat instruction must agree
with the number:

| Range | Arabic | Example |
|---|---|---|
| 1 | مرة (no numeral) | مرة |
| 2 | مرتان (no numeral) | مرتان |
| 3–10 | مرات | 3 مرات |
| 11–99 | مرةً | 11 مرةً |
| 100+ | مرة | 100 مرة |

The English side is the ordinary `{ $n } time` / `{ $n } times`. The progress
strings carry no noun (`{ $done } من { $total }`), so they need no agreement.

---

## 7. Fluent notes (Phase 6)

- **File layout:** `i18n/en/athkar.ftl`, `i18n/ar/athkar.ftl`. Keys identical in
  both files, `athkar-` prefix, lower kebab-case.
- **Plurals:** `athkar-repeat-count` only. Use the CLDR Arabic categories
  (`one`, `two`, `few`, `many`, `other`); `zero` is handled by hiding the line.
- **Bidi:** never concatenate an Arabic catalogue value with a Latin value in the
  UI. Pass the Latin as an argument and render it in its own run (§5).
- **The dhikr text is data, not a string.** It must never appear in a `.ftl`
  file; it is rendered from the corpus with its own direction and its own font
  (Amiri), right-aligned.

---

## 8. Attribution block structure

1. **Corpus title** — `athkar-corpus-title` (حصن المسلم).
2. **Licence row** — `athkar-corpus-licence` + the SPDX id (MIT), its own run.
3. **Pinned version** — `athkar-corpus-version` + `{ $version }`.
4. **Credit line** — `athkar-corpus-credit`, the exact text from the licence
   entry, unedited.
5. **Link** — the source URL from the licence entry (optional).

The block never shows a licence or credit that is not recorded. If the corpus
file is missing the block is not shown at all; the error state in §4.8 takes
over. OpenDua (CC BY 4.0) is the documented fallback corpus and would carry its
own credit text the same way.

---

## 9. Counts

| Group | Keys |
|---|---|
| 4.1 Context names | 7 |
| 4.2 Timing labels | 7 |
| 4.3 Context switcher | 1 |
| 4.4 Progress | 5 |
| 4.5 Counter | 7 |
| 4.6 Item view | 5 |
| 4.7 Corpus attribution | 4 |
| 4.8 Errors and empty states | 6 |
| 4.9 Heading | 1 |
| **Total** | **43** |

43 catalogue keys = 43 English strings + 43 Arabic strings (one of each is a
Fluent plural block). Every key is listed in §4 with its Arabic and English value.

---

## 10. Native-review flags

These are the keys where a native Arabic speaker should confirm the phrasing
before Phase 6 freezes the catalogs. Nothing here is a guess presented as final.

| Key | Why it needs a check |
|---|---|
| `athkar-context-random` | "أذكار عشوائية" is a literal rendering of "random athkar"; a native may prefer "من كل الأذكار" or "اختيار عشوائي". |
| `athkar-item-transliteration` | "التهجئة" for transliteration; a native may prefer "النطق" or "النسخ اللاتيني". |
| `athkar-repeat-count` | The dual/plural table in §6 follows CLDR; confirm the dropped numeral for 1 and 2 reads naturally in the card. |
| `athkar-time-*` | The window descriptions ("بعد الفجر" etc.) are deliberately loose; confirm they match how a reader expects the practice described. |
| corpus category titles | Any category beyond the seven in §4.1 must take its Arabic from the corpus's own title field, not from this document. |
