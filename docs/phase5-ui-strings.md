# Phase 5 UI strings — Arabic / English

> **Phase 5 prep (accuracy & coverage).** Copy contract for city search, mosque
> profiles, high-latitude honesty, calendar views and Ramadan mode. **No code and
> no UI changes** — Forge builds from it, Phase 6 turns it into Fluent catalogs.
>
> Conventions inherited verbatim from [`docs/audio-ui-strings.md`](audio-ui-strings.md)
> §2 and [`docs/athkar-ui-strings.md`](athkar-ui-strings.md) §2: bilingual table,
> Latin travels in `{ $… }` placeholders rendered as its own run, no mixed-script
> runs, no fabricated values, calm فصحى, no religious overclaiming.
>
> Sources: [`docs/ROADMAP.md`](ROADMAP.md) Phase 5 ·
> [`docs/phase5-city-sources.md`](phase5-city-sources.md) (Scout's ledger +
> Atlas decisions §10) · [`docs/phase5-ui-proposal.md`](phase5-ui-proposal.md)
> (Prism's design contract) · [`docs/architecture.md`](architecture.md) ·
> `crates/tawadu-core/src/prayer.rs` · `adhan` 0.2 `calculation.rs`.
>
> Status: **ACCEPTED by Atlas 2026-11-20 01:2x AST** (112 keys verified; F1 one-word
> Arabic fix applied by Atlas — variance `[one]` دقيقة واحدة; §12 native flags carried).
>
> **N6 catalog pass: 2026-11-20** — 14 keys added (deviation-4 / F1 / F2 per-event
> strip sentences, notification prefixes, timezone-picker placeholder, status and
> bound lines) and the two polar explain bodies re-pointed at the composed
> per-event sentences. Total 112 → **126**. Source: Sentinel N6 (PR #5 verdict
> comment 7034283246) + `crates/tawadu-core/src/prayer.rs::explain_strip` and
> `crates/tawadu-app/src/main.rs` **as shipped**. AR coverage gap (18/111 values
> rendered) recorded in §10; new AR wording flagged for native review in §12.

---

## 1. Scope

**In scope**

1. city search & selection — picker, results, disambiguation, recents, optional
   GPS, manual coordinates, current-location confirmation;
2. mosque profiles — list, add/edit/remove, offsets, active state, home binding;
3. high-latitude honesty — rule explanations, the `None` display, polar
   day/night copy, the explanation strip, fine-tune;
4. calendar views — Hijri/Gregorian toggle, month names, today, nav bounds,
   Ramadan mode;
5. error and empty states — database missing, GPS denied/unavailable.

**Out of scope**

- the city database itself and its schema (Scout's ledger);
- the GPS platform layer and the polar-window engine (Forge's side);
- prayer-time calculation internals (`tawadu-core` stays UI-free);
- Fluent catalog files (Phase 6).

---

## 2. Rules (binding)

1. **One script per string.** Arabic and Latin never share a string or a rendered
   run. Latin values (coordinates, country names, timezone ids, rule names) are
   `{ $… }` placeholders rendered in their own run.
2. **Never invent a time.** Where the engine returns `None` the UI shows an em
   dash and says why. No `00:00`, no substitute city, no borrowed time, no
   rounded guess presented as a fact.
3. **Rules explain, they do not rescue.** The high-latitude rules bound **Fajr
   and Isha only**. They never produce a sunrise or a sunset. Copy must say so.
4. **No copy may imply connectivity.** The city database is bundled and search
   is local. Nothing here says "online", "sync", "download" or "server". See §11.
5. **Counted nouns follow Arabic dual/plural rules.** See §6.
6. **No fabricated values.** A missing country, region or credit hides the line;
   an unknown bound shows the engine's value, never a typed-in date.
7. **Tone.** Simplified formal Arabic (فصحى مبسّطة), calm and exact. No
   superlatives, no exclamation marks, no religious overclaiming — Ramadan mode
   describes what it *does*, never what it earns.
8. **Short.** Every string is sized for a card, row or toast.

---

## 3. Glossary (fixed terms — do not retranslate)

| English | Arabic | Note |
|---|---|---|
| city picker | اختيار المدينة | the dedicated search view |
| region | المنطقة | GeoNames `admin1`, shown as a Latin value |
| city data | بيانات المدن | GeoNames, CC BY 4.0 |
| mosque profile | ملف مسجد | name + five iqamah offsets |
| iqamah offset | فرق الإقامة | minutes after the athan; `0` = not set |
| high-latitude rule | قاعدة خطوط العرض العليا | bounds Fajr and Isha only |
| middle of the night | منتصف الليل | `MiddleOfTheNight` |
| seventh of the night | سُبع الليل | `SeventhOfTheNight` |
| twilight angle | زاوية الشفق | `TwilightAngle` |
| polar day | شمس منتصف الليل | the sun never sets |
| polar night | الليل القطبي | the sun never rises |
| estimated | مُقدَّر | a rule-produced Fajr/Isha, not a sun-computed time |
| fine-tune | ضبط دقيق | per-prayer ± minutes |
| Ramadan mode | وضع رمضان | theme + reminder cadence only |

---

## 4. Strings

Columns are **separate strings**. Arabic cells contain no Latin letters; Latin
tokens arrive as `{ $… }` placeholders.

### 4.1 City search & selection

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-city-picker-title` | City picker | اختيار المدينة | view title |
| `phase5-city-picker-back` | Back to settings | العودة إلى الإعدادات | |
| `phase5-city-search-placeholder` | Search cities — English or Arabic | ابحث عن مدينة — بالإنجليزية أو العربية | |
| `phase5-city-results-count` | see plural below | see plural below | "{ $n } cities" |
| `phase5-city-results-cap` | Showing the first { $n } — keep typing to narrow the search. | تُعرض أول { $n } نتيجة — تابع الكتابة لتضييق البحث. | results cap |
| `phase5-city-no-results-title` | No matches for “{ $query }” | لا توجد نتائج مطابقة لـ«{ $query }» | `{ $query }` is the user's text, own run |
| `phase5-city-no-results-body` | Try the Arabic name, enter coordinates manually, or use your location. | جرّب الاسم بالعربية، أو أدخل الإحداثيات يدوياً، أو استخدم موقعك. | three calm offers |
| `phase5-city-disambiguation` | { $n } places share this name — choose by region and country. | أماكن تحمل هذا الاسم: { $n } — اختر حسب المنطقة والدولة. | 8,131 duplicate groups — never silently pick |
| `phase5-city-region-line` | { $admin1 } · { $timezone } | { $admin1 } · { $timezone } | Latin-only run (GeoNames values) |
| `phase5-city-country-line` | { $country } | { $country } | Latin-only run |
| `phase5-city-recent-heading` | Recent | الأخيرة | empty history renders nothing — no empty box |
| `phase5-city-search-offline` | Search runs offline over the bundled city list. | يعمل البحث دون اتصال عبر قائمة المدن المدمجة. | §11 |
| `phase5-city-use-location` | Use my location | استخدم موقعي | opt-in, off by default |
| `phase5-city-use-location-privacy` | Read once, stays on this device. Nothing leaves this device. | تُقرأ مرة واحدة وتبقى على هذا الجهاز. لا يُرسَل شيء إلى أي مكان. | always visible |
| `phase5-city-use-location-denied` | Location permission denied — enter coordinates manually. | تم رفض إذن الموقع — أدخل الإحداثيات يدوياً. | honest outcome 1 |
| `phase5-city-use-location-unavailable` | Location is unavailable on this device — enter coordinates manually. | الموقع غير متاح على هذا الجهاز — أدخل الإحداثيات يدوياً. | honest outcome 2 |
| `phase5-city-use-location-timeout` | Taking too long — enter coordinates manually. | يستغرق الأمر وقتاً أطول من المعتاد — أدخل الإحداثيات يدوياً. | > 5 s, no endless spinner |
| `phase5-city-use-location-confirm-title` | Use these coordinates? | أستخدم هذه الإحداثيات؟ | confirm before applying |
| `phase5-city-use-location-confirm-body` | Latitude { $lat }, longitude { $lon }. This becomes your location. | خط العرض { $lat } وخط الطول { $lon }. سيصبح هذا موقعك. | |
| `phase5-city-use-location-confirm-action` | Use | استخدام | |
| `phase5-city-manual-title` | Enter coordinates manually | أدخل الإحداثيات يدوياً | |
| `phase5-city-manual-lat` | Latitude | خط العرض | |
| `phase5-city-manual-lon` | Longitude | خط الطول | |
| `phase5-city-manual-timezone` | Timezone | المنطقة الزمنية | explicit picker — never guessed from longitude |
| `phase5-city-manual-name` | Name this place (optional) | سمِّ هذا المكان (اختياري) | empty → header shows coordinates |
| `phase5-city-manual-apply` | Use these coordinates | استخدم هذه الإحداثيات | |
| `phase5-city-manual-error-lat` | Latitude must be between { $min } and { $max }. | يجب أن يكون خط العرض بين { $min } و { $max }. | |
| `phase5-city-manual-error-lon` | Longitude must be between { $min } and { $max }. | يجب أن يكون خط الطول بين { $min } و { $max }. | |
| `phase5-city-manual-error-timezone` | Choose a timezone. | اختر المنطقة الزمنية. | |
| `phase5-city-selected-message` | Location set to { $name }. | تم تعيين الموقع: { $name }. | current-location confirmation |
| `phase5-city-attribution-label` | City data: | بيانات المدن: | label only |
| `phase5-city-attribution-credit` | { $credit } | { $credit } | pass-through from `assets/licenses/geonames-cities1000-cc-by-4.0.txt` |
| `phase5-city-tz-search-placeholder` | Search IANA timezones | ابحث عن منطقة زمنية | timezone-picker search box; the AR cell avoids the Latin acronym (rule 1) |

### 4.2 Mosque profiles

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-mosque-heading` | Mosques | المساجد | band title |
| `phase5-mosque-add` | Add mosque | إضافة مسجد | ghost row |
| `phase5-mosque-name` | Mosque name | اسم المسجد | required |
| `phase5-mosque-name-ar` | Arabic name (optional) | الاسم بالعربية (اختياري) | separate field, separate run |
| `phase5-mosque-name-required` | Enter a name to save. | أدخل اسماً للحفظ. | inline hint, never "Mosque 2" |
| `phase5-mosque-offsets-heading` | Iqamah offsets (minutes after the athan) | فروق الإقامة (بالدقائق بعد الأذان) | |
| `phase5-mosque-offset-fajr` | Fajr | الفجر | |
| `phase5-mosque-offset-dhuhr` | Dhuhr | الظهر | |
| `phase5-mosque-offset-asr` | Asr | العصر | |
| `phase5-mosque-offset-maghrib` | Maghrib | المغرب | |
| `phase5-mosque-offset-isha` | Isha | العشاء | |
| `phase5-mosque-minutes-unit` | see plural below | see plural below | "{ $n } min" |
| `phase5-mosque-no-offsets` | No offsets set | لم تُضبط فروق الإقامة | all-zero summary |
| `phase5-mosque-active-badge` | Active | مُفعّل | badge on the active row |
| `phase5-mosque-empty-title` | No mosque profiles yet | لا توجد ملفات مساجد بعد | |
| `phase5-mosque-empty-body` | Add your mosque to get iqamah times. | أضف مسجدك للحصول على أوقات الإقامة. | |
| `phase5-mosque-remove` | Remove | إزالة | two-step confirm |
| `phase5-mosque-remove-title` | Remove this mosque? | إزالة هذا المسجد؟ | |
| `phase5-mosque-remove-body` | “{ $name }” will be removed. The app will use no active mosque. | سيُزال «{ $name }». لن يستخدم التطبيق مسجداً مفعّلاً. | never a silent reassignment |
| `phase5-mosque-remove-active-note` | This is the active mosque. | هذا هو المسجد المفعّل. | |
| `phase5-mosque-done` | Done | إتمام | collapses the editor |
| `phase5-mosque-home-caption` | { $name } · { $offsets } | { $name } · { $offsets } | home NEXT IQAMAH caption; both values are precomposed runs |
| `phase5-mosque-home-empty` | Set your mosque's iqamah offsets | اضبط فروق إقامة مسجدك | evolves the current honest empty state |
| `phase5-mosque-removed-status` | Mosque removed — no active mosque. | أُزيل المسجد — لا يوجد مسجد مفعّل. | status line after removing the active profile; never a silent reassignment |

### 4.3 High-latitude honesty

Verified against `adhan` 0.2 `calculation.rs`: Fajr is bounded at
`sunrise − portion × night`, Isha at `sunset + portion × night`; the rule applies
only when the sun never reaches the twilight angle.

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-highlat-heading` | High-latitude rule | قاعدة خطوط العرض العليا | always visible, not hidden |
| `phase5-highlat-rule-middle` | Middle of the night | منتصف الليل | `MiddleOfTheNight` |
| `phase5-highlat-rule-seventh` | Seventh of the night | سُبع الليل | `SeventhOfTheNight` |
| `phase5-highlat-rule-angle` | Twilight angle | زاوية الشفق | `TwilightAngle` |
| `phase5-highlat-rule-middle-desc` | Sets Fajr half the night before sunrise and Isha half the night after sunset, when the sun never reaches the twilight angle. | يقدّر الفجر قبل الشروق بنصف الليل، والعشاء بعد الغروب بنصف الليل، عندما لا تصل الشمس إلى زاوية الشفق. | portion = 1/2 |
| `phase5-highlat-rule-seventh-desc` | Sets Fajr one seventh of the night before sunrise and Isha one seventh of the night after sunset, when the sun never reaches the twilight angle. | يقدّر الفجر قبل الشروق بسُبع الليل، والعشاء بعد الغروب بسُبع الليل، عندما لا تصل الشمس إلى زاوية الشفق. | portion = 1/7 |
| `phase5-highlat-rule-angle-desc` | Places Fajr and Isha at a fraction of the night set by the method's twilight angles, when the sun never reaches those angles. | يضع الفجر والعشاء عند جزء من الليل تحدده زوايا الشفق للطريقة المختارة، عندما لا تصل الشمس إلى تلك الزوايا. | portion = angle ÷ 60 |
| `phase5-highlat-selector-caption` | These rules change Fajr and Isha only, and only near the polar circles. They never create a sunrise or sunset. | تغيّر هذه القواعد الفجر والعشاء فقط، وقرب الدوائر القطبية فقط. وهي لا تخلق شروقاً ولا غروباً. | the honesty line |
| `phase5-highlat-none` | — | — | one em dash, never `00:00` |
| `phase5-highlat-none-caption` | No defined time | لا وقت محدد | |
| `phase5-highlat-estimated` | Estimated | مُقدَّر | muted caption on a rule-produced time |
| `phase5-highlat-explain-title-day` | Midnight sun in { $city } | شمس منتصف الليل في { $city } | polar day |
| `phase5-highlat-explain-title-night` | Polar night in { $city } | الليل القطبي في { $city } | polar night |
| `phase5-highlat-explain-body-day` | { $city } is inside the polar circle. At this time of year the sun never sets, so there is no sunrise or sunset to compute. { $details } | يقع { $city } داخل الدائرة القطبية. في هذا الوقت من العام لا تغيب الشمس أبداً، فلا يوجد شروق أو غروب يُحسب. { $details } | `{ $details }` = composed per-event sentences (§8); omit any fact the engine cannot supply |
| `phase5-highlat-explain-body-night` | { $city } is inside the polar circle. At this time of year the sun never rises, so there is no sunrise or sunset to compute. { $details } | يقع { $city } داخل الدائرة القطبية. في هذا الوقت من العام لا تشرق الشمس أبداً، فلا يوجد شروق أو غروب يُحسب. { $details } | |
| `phase5-highlat-explain-window` | Midnight sun from { $from } to { $to }. | شمس منتصف الليل من { $from } إلى { $to }. | omit if the engine supplies no dates |
| `phase5-highlat-explain-window-night` | Polar night from { $from } to { $to }. | الليل القطبي من { $from } إلى { $to }. | |
| `phase5-highlat-change-rule` | Change rule | تغيير القاعدة | deep-links to CALCULATION |
| `phase5-highlat-why-rules` | Why these rules? | لماذا هذه القواعد؟ | expands §5.1 inline |
| `phase5-highlat-next-none` | No defined time today — see why | لا وقت محدد اليوم — اعرف لماذا | countdown area when the next prayer is `None` |
| `phase5-highlat-how-computed` | How these times are computed | كيف تُحساب هذه الأوقات | expandable, collapsed by default |
| `phase5-highlat-variance` | see plural below | see plural below | "{ $minutes } minutes" |
| `phase5-highlat-finetune` | Fine-tune times | ضبط دقيق للأوقات | per-prayer ± steppers |
| `phase5-highlat-strip-title-estimated` | Some times are estimated | بعض الأوقات مُقدَّرة | non-polar strip title |
| `phase5-highlat-strip-rule-estimated` | { $events } {is\|are} estimated using { $rule }. | يُقدَّر { $events } باستخدام { $rule }. | `{ $events }` precomposed run (§8); `{is\|are}` resolved in Rust |
| `phase5-highlat-strip-plain-estimated` | { $events } {is\|are} estimated. | يُقدَّر { $events }. | rule-INDEPENDENT estimated — never names a rule |
| `phase5-highlat-strip-undefined` | { $events } {has\|have} no defined time. | { $events }: لا وقت محدد. | per-event honest Undefined |
| `phase5-highlat-moonsighting-substitution` | At this latitude the Moonsighting Committee method substitutes seasonal estimates for { $events }. | عند هذا خط العرض، تُقدَّر { $events } تقديراً موسمياً بطريقة لجنة رؤية الهلال. | never claims angle-failure |
| `phase5-highlat-sun-never-reaches` | The sun never reaches the twilight angle here today, so { $estimated } | الشمس لا تصل إلى زاوية الشفق هنا اليوم، لذا { $estimated } | `{ $estimated }` = composed event sentences |
| `phase5-highlat-sun-computed` | Sunrise and sunset are sun-computed. | الشروق والغروب محسوبان من الشمس. | appended only when both are Sun |
| `phase5-highlat-variance-unknown` | Different sources can give different times. Fine-tune any prayer below. | قد تختلف المصادر في الأوقات. يمكنك ضبط أي صلاة أدناه. | F2/n-a None fallback (no numeric spread) |

### 4.4 Calendar views & Ramadan mode

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-cal-title` | Calendar | التقويم | |
| `phase5-cal-primary-hijri` | Hijri | الهجري | toggle: which calendar is primary |
| `phase5-cal-primary-gregorian` | Gregorian | الميلادي | |
| `phase5-cal-today` | Today | اليوم | re-anchors after navigation |
| `phase5-cal-nav-prev` | Previous month | الشهر السابق | |
| `phase5-cal-nav-next` | Next month | الشهر التالي | |
| `phase5-cal-nav-disabled` | Dates unavailable beyond { $bound } | التواريخ غير متاحة بعد { $bound } | engine bound, never extrapolated |
| `phase5-cal-nav-disabled-before` | Dates unavailable before { $bound } | التواريخ غير متاحة قبل { $bound } | prev-side bound; the key above stays the "beyond" side |
| `phase5-cal-hijri-month-01` | Muharram | محرم | fixed terms |
| `phase5-cal-hijri-month-02` | Safar | صفر | |
| `phase5-cal-hijri-month-03` | Rabi' al-Awwal | ربيع الأول | |
| `phase5-cal-hijri-month-04` | Rabi' al-Thani | ربيع الآخر | app form; crate says ربيع الثاني — see §10 |
| `phase5-cal-hijri-month-05` | Jumada al-Ula | جمادى الأولى | |
| `phase5-cal-hijri-month-06` | Jumada al-Akhirah | جمادى الآخرة | |
| `phase5-cal-hijri-month-07` | Rajab | رجب | |
| `phase5-cal-hijri-month-08` | Sha'ban | شعبان | |
| `phase5-cal-hijri-month-09` | Ramadan | رمضان | |
| `phase5-cal-hijri-month-10` | Shawwal | شوال | |
| `phase5-cal-hijri-month-11` | Dhu al-Qa'dah | ذو القعدة | |
| `phase5-cal-hijri-month-12` | Dhu al-Hijjah | ذو الحجة | |
| `phase5-cal-day-strip` | Times for this day | أوقات هذا اليوم | selected day's five times, same `—` rule |
| `phase5-cal-error-title` | Dates unavailable | التواريخ غير متاحة | fail-soft: Gregorian grid still works |
| `phase5-cal-error-body` | The Hijri table does not cover this range. The Gregorian calendar still works. | جدول التاريخ الهجري لا يغطي هذا النطاق. التقويم الميلادي ما زال يعمل. | |
| `phase5-ramadan-banner-title` | Ramadan | رمضان | shown only in the Hijri month of Ramadan |
| `phase5-ramadan-mode` | Ramadan mode | وضع رمضان | |
| `phase5-ramadan-mode-desc` | Suggests the Ramadan theme, turns the Fajr pre-alert into a suhur reminder and the Maghrib pre-alert into an iftar reminder, and marks Ramadan days in the calendar. Nothing else changes. | يقترح سمة رمضان، ويحوّل تنبيه الفجر إلى تذكير بالسحور وتنبيه المغرب إلى تذكير بالإفطار، ويميّز أيام رمضان في التقويم. لا يتغيّر شيء آخر. | scope = theme + cadence + markers only |
| `phase5-ramadan-turn-on` | Turn on Ramadan mode | تفعيل وضع رمضان | opt-in, never auto-applied |
| `phase5-ramadan-turn-off` | Turn off Ramadan mode | إيقاف وضع رمضان | |
| `phase5-ramadan-ended` | Ramadan has ended — turn off mode? | انتهى شهر رمضان — أتريد إيقاف الوضع؟ | a question, never a silent switch |
| `phase5-ramadan-restore-theme` | Restore the previous theme? | أأستعيد السمة السابقة؟ | offered when mode turns off |

### 4.5 Shared actions & error states

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-cancel` | Cancel | إلغاء | shared confirm action |
| `phase5-error-db-missing-title` | City database not found | قاعدة بيانات المدن غير موجودة | |
| `phase5-error-db-missing-body` | The app could not read its bundled city database. Reinstall the app to restore it. | تعذّرت قراءة قاعدة بيانات المدن المدمجة. أعد تثبيت التطبيق لاستعادتها. | bundled — no network implied |
| `phase5-error-generic-title` | Something went wrong | حدث خطأ غير متوقع | |
| `phase5-error-generic-body` | Try again; if it keeps happening, reinstall the app. | أعد المحاولة؛ فإن تكرر الخطأ فأعد تثبيت التطبيق. | |
| `phase5-city-manual-error-both` | Latitude must be between { $min } and { $max }. Longitude must be between { $min } and { $max }. | يجب أن يكون خط العرض بين { $min } و { $max }. ويجب أن يكون خط الطول بين { $min } و { $max }. | combined parse error (both fields invalid) |

### 4.6 Notification titles

| Key | English | Arabic | Notes |
|---|---|---|---|
| `phase5-notify-suhur` | Suhur reminder · { $title } | تذكير بالسحور · { $title } | Ramadan-mode Fajr pre-alert prefix |
| `phase5-notify-iftar` | Iftar reminder · { $title } | تذكير بالإفطار · { $title } | Ramadan-mode Maghrib pre-alert prefix |

---

## 5. Placeholder reference

| Placeholder | Type | Value comes from | Rendered |
|---|---|---|---|
| `{ $n }` | number | result count | own run, Western digits |
| `{ $query }` | string | the user's search text | own run (either script) |
| `{ $admin1 }` | string | GeoNames `admin1` | own Latin run |
| `{ $timezone }` | string | GeoNames IANA id | own Latin run |
| `{ $country }` | string | country name | own Latin run |
| `{ $lat }`, `{ $lon }` | string | the resolved coordinates | own Latin run |
| `{ $min }`, `{ $max }` | number | coordinate bounds | own Latin run |
| `{ $name }` | string | city or mosque name | own run |
| `{ $offsets }` | string | precomposed offsets summary | own run |
| `{ $credit }` | string | the licence entry | own Latin run |
| `{ $city }` | string | location name | own run |
| `{ $rule }` | string | the active rule's name | own run |
| `{ $from }`, `{ $to }` | string | polar window dates | own Latin run |
| `{ $bound }` | string | the engine's Hijri range bound | own Latin run |
| `{ $minutes }` | number | the variance figure | own Latin run |
| `{ $events }` | string | precomposed event names for the strip | own run (per §8) |
| `{ $details }` | string | composed per-event strip sentences | own run |
| `{ $estimated }` | string | composed "estimated" sentences | own run |
| `{ $title }` | string | the notification's base title | own run |

**Rendering rule.** A catalogue value may contain `{ $… }` but no Latin letters.
At render time it splits at each placeholder into separate runs with their own
direction — the same pattern as `location_name` / `location_name_ar`.

---

## 6. Counted-noun forms (the quality bar)

| Key | English | Arabic |
|---|---|---|
| `phase5-city-results-count` | `{ $n } city` / `{ $n } cities` | مدينة / مدينتان / `{ $n }` مدن / `{ $n }` مدينةً / `{ $n }` مدينة |
| `phase5-mosque-minutes-unit` | `{ $n } min` | دقيقة / دقيقتان / `{ $n }` دقائق / `{ $n }` دقيقةً / `{ $n }` دقيقة |
| `phase5-highlat-variance` | `Different sources can differ by about { $minutes } minutes. Fine-tune any prayer below.` | `قد تختلف المصادر بنحو { $minutes } دقائق. يمكنك ضبط أي صلاة أدناه.` |

```ftl
# English — i18n/en/phase5.ftl
phase5-city-results-count =
    { $n ->
        [one] { $n } city
       *[other] { $n } cities
    }
phase5-mosque-minutes-unit =
    { $n } min
phase5-highlat-variance =
    { $minutes ->
        [one] Different sources can differ by about { $minutes } minute. Fine-tune any prayer below.
       *[other] Different sources can differ by about { $minutes } minutes. Fine-tune any prayer below.
    }

# Arabic — i18n/ar/phase5.ftl
phase5-city-results-count =
    { $n ->
        [one] مدينة
        [two] مدينتان
        [few] { $n } مدن
        [many] { $n } مدينةً
       *[other] { $n } مدينة
    }
phase5-mosque-minutes-unit =
    { $n ->
        [one] دقيقة
        [two] دقيقتان
        [few] { $n } دقائق
        [many] { $n } دقيقةً
       *[other] { $n } دقيقة
    }
phase5-highlat-variance =
    { $minutes ->
        [one] قد تختلف المصادر بنحو دقيقة واحدة. يمكنك ضبط أي صلاة أدناه.
        [two] قد تختلف المصادر بنحو دقيقتين. يمكنك ضبط أي صلاة أدناه.
        [few] قد تختلف المصادر بنحو { $minutes } دقائق. يمكنك ضبط أي صلاة أدناه.
        [many] قد تختلف المصادر بنحو { $minutes } دقيقةً. يمكنك ضبط أي صلاة أدناه.
       *[other] قد تختلف المصادر بنحو { $minutes } دقيقة. يمكنك ضبط أي صلاة أدناه.
    }
```

`few` is 3–10, `many` is 11–99, `other` is 0 and 100+. For one and two the
numeral is dropped, which is correct Arabic.

---

## 7. Gregorian month names — deliberately **not** catalogue keys

Atlas's brief asked for both calendars. The twelve Hijri names above are keys
because they are already hardcoded in `crates/tawadu-core/src/hijri.rs`
(`month_name_ar`) and must be pinned. The **Gregorian** names are not keys: the
engine already provides them in both languages from one table
(`hijridate::Gregorian::month_name()` and `month_name_in(Language::Arabic)`), and
a second copy in a catalogue would be a second source of truth that can drift.

Reference values (must match the engine, and the engine wins):

| # | English | Arabic |
|---|---|---|
| 1 | January | يناير |
| 2 | February | فبراير |
| 3 | March | مارس |
| 4 | April | أبريل |
| 5 | May | مايو |
| 6 | June | يونيو |
| 7 | July | يوليو |
| 8 | August | أغسطس |
| 9 | September | سبتمبر |
| 10 | October | أكتوبر |
| 11 | November | نوفمبر |
| 12 | December | ديسمبر |

---

## 8. Fluent notes (Phase 6)

- **File layout:** `i18n/en/phase5.ftl`, `i18n/ar/phase5.ftl`. Keys identical in
  both, `phase5-` prefix, lower kebab-case.
- **Plurals:** `phase5-city-results-count`, `phase5-mosque-minutes-unit`,
  `phase5-highlat-variance`. CLDR Arabic categories only (`one`, `two`, `few`,
  `many`, `other`).
- **Bidi:** never concatenate an Arabic catalogue value with a Latin value in
  the UI. Pass the Latin as an argument and render it in its own run (§5).
- **Composed facts stay in Rust.** The polar window sentence, the offsets
  summary, the "how computed" list and the fine-tune summary are composed from
  engine values and passed as whole runs. The catalogue supplies the template
  and the labels, never the arithmetic.

---

## 9. Attribution

1. **Picker footer** — `phase5-city-attribution-label` + `phase5-city-attribution-credit`,
   the latter read verbatim from `assets/licenses/geonames-cities1000-cc-by-4.0.txt`.
2. **Settings LOCATION band** — the same caption (Prism §3).
3. **Phase 6 About/licences page** — the full entry, including the "changes were
   made" sentence Scout requires.

If Scout's ledger changes the licence, only `phase5-city-attribution-credit`
changes — the label stays.

---

## 10. Counts

| Group | Keys |
|---|---|
| 4.1 City search & selection | 33 |
| 4.2 Mosque profiles | 24 |
| 4.3 High-latitude honesty | 31 |
| 4.4 Calendar & Ramadan mode | 30 |
| 4.5 Shared actions & errors | 6 |
| 4.6 Notification titles | 2 |
| **Total** | **126** |

126 catalogue keys = 126 English strings + 126 Arabic strings (three of which are
Fluent plural blocks). The N6 pass added 14 keys on 2026-11-20 (112 → 126).

**AR-run coverage (N6).** 18 of the 111 pre-N6 catalog values are rendered in
Arabic in the shipped app — the EN-primary-until-Phase-6 convention. Prism §5.2
explicitly requires the AR `estimated` caption (مُقدَّر) and §7.4 the
caption/privacy AR runs; those values are catalogued now. Closing the AR gap is
Phase 6 catalog work, not a Phase 5 code change.

---

## 11. Connectivity audit (Atlas's constraint)

No string in §4 implies a network. Checked:

- search says **offline** and **bundled** (`phase5-city-search-offline`);
- the only database error is **bundled** and tells the user to **reinstall**
  (`phase5-error-db-missing-body`) — never "check your connection";
- GPS copy talks about **permission**, **availability** and **device**, never
  about network or sync (`phase5-city-use-location-*`);
- the attribution is a static caption, not a fetch.

If any future copy needs to mention connectivity it must be an explicit, opt-in
feature — never a dependency of the bundled path.

---

## 12. Native-review flags

| Key | Why it needs a check |
|---|---|
| `phase5-city-results-cap` | "نتيجة" as the counted noun; a native may prefer "مدخل" or a noun-free phrasing. |
| `phase5-highlat-variance` | The "about N minutes" phrasing and the plural table; confirm it reads naturally. |
| `phase5-highlat-rule-*-desc` | Verified against `adhan`'s code, but the plain-language wording should be read aloud by a native. |
| `phase5-highlat-explain-body-*` | The polar sentences omit facts the engine cannot supply; confirm the remaining wording still reads as a complete sentence. |
| `phase5-ramadan-banner-title` | "رمضان" alone is deliberately neutral (the proposal suggested "Ramadan Mubarak"); confirm a bare label is acceptable for the banner. |
| `phase5-mosque-done` | "إتمام" for the editor's Done action; confirm it does not read as "completion of worship". |
| Hijri month 04 | The app ships **ربيع الآخر**; `hijridate`'s Arabic table says **ربيع الثاني**. Both are used; the shipped form is kept — confirm. |
| `phase5-city-use-location-privacy` | Proposal said "No network"; this says "nothing leaves this device", which is true even if assisted GPS is used. Confirm the wording. |
| `phase5-highlat-strip-rule-estimated` / `-plain-estimated` | New N6 keys. The `{is\|are}` / `{has\|have}` copula is composed in Rust; confirm the Arabic verbal forms (يُقدَّر …) read naturally for one and for two events. |
| `phase5-highlat-strip-undefined` | `{ $events }: لا وقت محدد.` — a colon construction, chosen to avoid dual/plural copula agreement; confirm it reads as a complete sentence. |
| `phase5-highlat-moonsighting-substitution` | New N6 key; the Arabic re-orders the sentence (تقدير موسمي بطريقة لجنة رؤية الهلال). Confirm the meaning survives. |
| `phase5-highlat-sun-never-reaches` / `-sun-computed` | New N6 keys; the trailing `{ $estimated }` is a composed run, so the Arabic connector "لذا" must agree with whatever follows. Confirm. |
| `phase5-notify-suhur` / `-iftar` | New N6 keys; "تذكير بالسحور · …" / "تذكير بالإفطار · …" — confirm the · separator and the base-title hand-off. |
| `phase5-city-tz-search-placeholder` | AR deliberately drops the "IANA" acronym (rule 1); confirm "ابحث عن منطقة زمنية" is clear without it. |
| `phase5-city-manual-error-both` | Two sentences in one string; confirm the repeated `{ $min }`/`{ $max }` values read cleanly in Arabic. |
