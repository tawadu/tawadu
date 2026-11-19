//! Athkar corpus model, context timing and real progress (Phase 4).
//!
//! Corpus: MIT Hisn al-Muslim TEXT, pinned d793023e — bundled under
//! assets/athkar (provenance: docs/athkar-corpus.md, scripts/verify_athkar_corpus.py).
//! Platform-free: parsing, context selection and progress math are pure;
//! persistence lives in storage (schema v3). Recitation audio is NOT bundled.

use chrono::NaiveTime;
use serde::Deserialize;
use std::collections::HashMap;

// Bundled at compile time: offline-first, zero runtime network, no file IO.
// Provenance: docs/athkar-corpus.md (pinned d793023e, per-file SHA-256).
const AR_JSON: &str = include_str!("../../../assets/athkar/ar.json");
const EN_JSON: &str = include_str!("../../../assets/athkar/en.json");

#[derive(Deserialize)]
struct RawItem {
    id: u32,
    text: String,
    count: u32,
    transliteration: Option<String>,
}

#[derive(Deserialize)]
struct RawCategory {
    id: u32,
    category: String,
    array: Vec<RawItem>,
}

/// One dhikr: Arabic primary, translation/transliteration only where the
/// corpus provides them (never fabricated), repetition target from `count`.
pub struct Item {
    pub id: u32,
    pub arabic: String,
    pub translation: Option<String>,
    pub transliteration: Option<String>,
    pub count: u32,
}

/// A corpus category (Hisn al-Muslim chapter), AR + EN names aligned by id.
pub struct Category {
    pub id: u32,
    pub name_ar: String,
    pub name_en: String,
    pub items: Vec<Item>,
}

impl Category {
    /// Source reference for an item. The corpus provides category-level
    /// provenance (Hisn al-Muslim), not per-item hadith grading — say exactly
    /// that much, no more.
    pub fn reference(&self, item_id: u32) -> String {
        format!("Hisn al-Muslim · {} · #{}", self.name_en, item_id)
    }
}

/// The full bundled corpus (132 categories, 267 items at the pinned commit).
pub struct Corpus {
    pub categories: Vec<Category>,
}

impl Corpus {
    /// Parse the bundled AR/EN JSON, zipping categories and items by id.
    /// Any structural mismatch is an error, never a silent partial import.
    pub fn bundled() -> Result<Corpus, String> {
        let ar: Vec<RawCategory> =
            serde_json::from_str(AR_JSON).map_err(|e| format!("ar.json: {e}"))?;
        let en: Vec<RawCategory> =
            serde_json::from_str(EN_JSON).map_err(|e| format!("en.json: {e}"))?;
        let mut categories = Vec::with_capacity(ar.len());
        for a in ar {
            let e = en
                .iter()
                .find(|c| c.id == a.id)
                .ok_or_else(|| format!("EN category {} missing", a.id))?;
            if a.array.len() != e.array.len() {
                return Err(format!("category {} item count mismatch", a.id));
            }
            let mut items = Vec::with_capacity(a.array.len());
            for (ai, ei) in a.array.into_iter().zip(&e.array) {
                if ai.id != ei.id {
                    return Err(format!("category {} item id mismatch", a.id));
                }
                items.push(Item {
                    id: ai.id,
                    arabic: ai.text.trim().to_string(),
                    translation: Some(ei.text.trim().to_string()).filter(|s| !s.is_empty()),
                    transliteration: ei.transliteration.clone().filter(|s| !s.is_empty()),
                    count: ai.count.max(1),
                });
            }
            categories.push(Category {
                id: a.id,
                name_ar: a.category,
                name_en: e.category.clone(),
                items,
            });
        }
        Ok(Corpus { categories })
    }

    pub fn category(&self, id: u32) -> Option<&Category> {
        self.categories.iter().find(|c| c.id == id)
    }
}

/// The athkar contexts. Morning and Evening share the corpus's COMBINED
/// morning-and-evening category (27) — the corpus's own structure — with
/// separate per-window progress. Labels reuse the scheduler's existing
/// window strings; AfterPrayer's label is flagged for Quill's catalog.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Context {
    Morning,
    Evening,
    Sleep,
    AfterPrayer,
}

impl Context {
    pub const ALL: [Context; 4] = [
        Context::Morning,
        Context::Evening,
        Context::Sleep,
        Context::AfterPrayer,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Context::Morning => "morning",
            Context::Evening => "evening",
            Context::Sleep => "sleep",
            Context::AfterPrayer => "after_prayer",
        }
    }

    pub fn from_key(key: &str) -> Option<Context> {
        Context::ALL.into_iter().find(|c| c.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Context::Morning => "Morning athkar",
            Context::Evening => "Evening athkar",
            Context::Sleep => "Sleep athkar",
            Context::AfterPrayer => "After-prayer athkar",
        }
    }

    pub fn category_ids(self) -> &'static [u32] {
        match self {
            Context::Morning | Context::Evening => &[27],
            Context::Sleep => &[28],
            Context::AfterPrayer => &[25],
        }
    }
}

/// Local prayer boundary times for context selection (platform-free: the
/// app converts its computed UTC times to local before calling).
pub struct ContextTimes {
    pub fajr: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub isha: NaiveTime,
}

/// The active context at a local time. Windows extend to the next boundary
/// (catch-up): morning Fajr→Dhuhr, after-prayer Dhuhr→Asr, evening Asr→Isha,
/// sleep Isha→Fajr. A late opener still gets the window's athkar — nothing
/// is silently dropped, and the day is fully covered.
pub fn active_context(now: NaiveTime, t: &ContextTimes) -> Context {
    if now >= t.fajr && now < t.dhuhr {
        Context::Morning
    } else if now >= t.dhuhr && now < t.asr {
        Context::AfterPrayer
    } else if now >= t.asr && now < t.isha {
        Context::Evening
    } else {
        Context::Sleep
    }
}

fn each_item(corpus: &Corpus, ctx: Context, mut f: impl FnMut(u32, &Item)) {
    for id in ctx.category_ids() {
        if let Some(cat) = corpus.category(*id) {
            for item in &cat.items {
                f(cat.id, item);
            }
        }
    }
}

/// (items, total repeats) for a context — the honest denominators.
pub fn context_totals(corpus: &Corpus, ctx: Context) -> (u32, u32) {
    let mut items = 0;
    let mut repeats = 0;
    each_item(corpus, ctx, |_, item| {
        items += 1;
        repeats += item.count;
    });
    (items, repeats)
}

/// (items fully read, repeats done) — repeats capped at each item's target.
pub fn context_done(corpus: &Corpus, ctx: Context, done: &HashMap<(u32, u32), u32>) -> (u32, u32) {
    let mut items_done = 0;
    let mut repeats_done = 0;
    each_item(corpus, ctx, |cat, item| {
        let n = done.get(&(cat, item.id)).copied().unwrap_or(0);
        repeats_done += n.min(item.count);
        if n >= item.count {
            items_done += 1;
        }
    });
    (items_done, repeats_done)
}

/// Complete only when EVERY item reached its full repeat target.
pub fn context_complete(corpus: &Corpus, ctx: Context, done: &HashMap<(u32, u32), u32>) -> bool {
    let (items, _) = context_totals(corpus, ctx);
    let (items_done, _) = context_done(corpus, ctx, done);
    items > 0 && items_done == items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use chrono::NaiveTime;
    use std::collections::HashMap;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn riyadh_times() -> ContextTimes {
        ContextTimes {
            fajr: t(4, 49),
            dhuhr: t(11, 55),
            asr: t(15, 10),
            isha: t(18, 40),
        }
    }

    // ---- corpus ----

    #[test]
    fn bundled_corpus_has_the_full_pinned_structure() {
        let corpus = Corpus::bundled().unwrap();
        assert_eq!(corpus.categories.len(), 132);
        let items: usize = corpus.categories.iter().map(|c| c.items.len()).sum();
        assert_eq!(items, 267);
        assert_eq!(corpus.category(27).unwrap().items.len(), 24);
        assert_eq!(corpus.category(28).unwrap().items.len(), 13);
        assert_eq!(corpus.category(25).unwrap().items.len(), 8);
        assert!(corpus.category(999).is_none());
    }

    #[test]
    fn items_carry_arabic_translation_and_repeat_counts() {
        let corpus = Corpus::bundled().unwrap();
        let morning = corpus.category(27).unwrap();
        let first = &morning.items[0];
        assert!(first.arabic.contains("اللّ"), "Arabic text must be primary");
        assert!(first.translation.as_deref().unwrap().contains("Allah"));
        assert_eq!(first.count, 1);
        // The corpus has repeated dhikr (e.g. 100x) — counts must survive import.
        assert!(morning.items.iter().any(|i| i.count == 100));
    }

    #[test]
    fn transliteration_is_sparse_and_never_fabricated() {
        let corpus = Corpus::bundled().unwrap();
        let present = corpus
            .categories
            .iter()
            .flat_map(|c| &c.items)
            .filter(|i| i.transliteration.is_some())
            .count();
        assert_eq!(present, 1, "the pinned corpus provides exactly one");
        assert!(corpus.category(27).unwrap().items[0]
            .transliteration
            .is_none());
    }

    #[test]
    fn reference_names_the_corpus_category_and_item() {
        let corpus = Corpus::bundled().unwrap();
        let cat = corpus.category(27).unwrap();
        let reference = cat.reference(cat.items[0].id);
        assert!(reference.contains("Hisn al-Muslim"));
        assert!(reference.contains(&cat.name_en));
        assert!(reference.contains("#1"));
    }

    // ---- contexts & timing ----

    #[test]
    fn contexts_map_to_the_corpus_own_structure() {
        // Category 27 is the COMBINED morning-and-evening set: both contexts
        // share it, with separate per-window progress.
        assert_eq!(Context::Morning.category_ids(), &[27]);
        assert_eq!(Context::Evening.category_ids(), &[27]);
        assert_eq!(Context::Sleep.category_ids(), &[28]);
        assert_eq!(Context::AfterPrayer.category_ids(), &[25]);
        let keys: Vec<_> = Context::ALL.iter().map(|c| c.key()).collect();
        assert_eq!(keys, ["morning", "evening", "sleep", "after_prayer"]);
    }

    #[test]
    fn active_context_covers_the_day_with_catch_up_windows() {
        let times = riyadh_times();
        // Windows extend to the next boundary (catch-up: a late opener still
        // gets the window's athkar, never a silently dropped context).
        assert_eq!(active_context(t(5, 0), &times), Context::Morning);
        assert_eq!(active_context(t(13, 0), &times), Context::AfterPrayer);
        assert_eq!(active_context(t(16, 0), &times), Context::Evening);
        assert_eq!(active_context(t(20, 0), &times), Context::Sleep);
        assert_eq!(active_context(t(3, 0), &times), Context::Sleep);
    }

    #[test]
    fn active_context_boundaries_are_exact() {
        let times = riyadh_times();
        assert_eq!(active_context(t(4, 49), &times), Context::Morning);
        assert_eq!(active_context(t(11, 55), &times), Context::AfterPrayer);
        assert_eq!(active_context(t(15, 10), &times), Context::Evening);
        assert_eq!(active_context(t(18, 40), &times), Context::Sleep);
    }

    // ---- progress math (pure) ----

    #[test]
    fn context_totals_count_items_and_repeats() {
        let corpus = Corpus::bundled().unwrap();
        let (items, repeats) = context_totals(&corpus, Context::Morning);
        assert_eq!(items, 24);
        let expected: u32 = corpus
            .category(27)
            .unwrap()
            .items
            .iter()
            .map(|i| i.count)
            .sum();
        assert_eq!(repeats, expected);
        assert!(
            repeats > items,
            "repeated dhikr must inflate the repeat total"
        );
    }

    #[test]
    fn completion_is_honest_and_per_item() {
        let corpus = Corpus::bundled().unwrap();
        let cat = corpus.category(27).unwrap();
        let mut done: HashMap<(u32, u32), u32> = HashMap::new();
        assert!(!context_complete(&corpus, Context::Morning, &done));
        // All but the last item fully read.
        for item in &cat.items[..cat.items.len() - 1] {
            done.insert((27, item.id), item.count);
        }
        assert!(!context_complete(&corpus, Context::Morning, &done));
        let last = cat.items.last().unwrap();
        // Partial repeats of the last item do NOT complete it.
        if last.count > 1 {
            done.insert((27, last.id), last.count - 1);
            assert!(!context_complete(&corpus, Context::Morning, &done));
        }
        done.insert((27, last.id), last.count);
        assert!(context_complete(&corpus, Context::Morning, &done));
        let (items_done, repeats_done) = context_done(&corpus, Context::Morning, &done);
        assert_eq!(items_done, 24);
        assert_eq!(repeats_done, context_totals(&corpus, Context::Morning).1);
    }

    // ---- persistence (storage schema v3) ----

    #[test]
    fn repeats_increment_and_cap_at_the_target() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(
            storage
                .record_athkar_repeat("morning", 27, 3, "2026-11-19", 3)
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .record_athkar_repeat("morning", 27, 3, "2026-11-19", 3)
                .unwrap(),
            2
        );
        assert_eq!(
            storage
                .record_athkar_repeat("morning", 27, 3, "2026-11-19", 3)
                .unwrap(),
            3
        );
        // Honest counter: never over-counts past the target.
        assert_eq!(
            storage
                .record_athkar_repeat("morning", 27, 3, "2026-11-19", 3)
                .unwrap(),
            3
        );
    }

    #[test]
    fn progress_is_isolated_per_context_and_day() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .record_athkar_repeat("morning", 27, 1, "2026-11-19", 1)
            .unwrap();
        storage
            .record_athkar_repeat("evening", 27, 1, "2026-11-19", 1)
            .unwrap();
        storage
            .record_athkar_repeat("morning", 27, 1, "2026-11-18", 1)
            .unwrap();
        let today = storage.athkar_progress("morning", "2026-11-19").unwrap();
        assert_eq!(today.get(&(27, 1)), Some(&1));
        assert_eq!(today.len(), 1, "other contexts/days must not leak in");
        // A new day starts honest: yesterday's reads are not today's progress.
        assert!(storage
            .athkar_progress("morning", "2026-11-20")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn reset_clears_only_the_named_context_and_day() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .record_athkar_repeat("morning", 27, 1, "2026-11-19", 1)
            .unwrap();
        storage
            .record_athkar_repeat("evening", 27, 1, "2026-11-19", 1)
            .unwrap();
        storage
            .reset_athkar_context("morning", "2026-11-19")
            .unwrap();
        assert!(storage
            .athkar_progress("morning", "2026-11-19")
            .unwrap()
            .is_empty());
        assert_eq!(
            storage
                .athkar_progress("evening", "2026-11-19")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn v2_databases_migrate_to_v3_with_data_intact() {
        let dir = std::env::temp_dir().join(format!("tawadu-athkar-mig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT NOT NULL, updated_at TEXT NOT NULL);
                 CREATE TABLE fired (kind TEXT NOT NULL, prayer TEXT NOT NULL, at TEXT NOT NULL, PRIMARY KEY (kind, prayer, at));
                 INSERT INTO meta VALUES ('schema_version', '2');
                 INSERT INTO fired VALUES ('prayer', 'Fajr', '2026-11-19T01:49:00+00:00');",
            ).unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 3);
        assert!(storage
            .has_fired(
                "prayer",
                "Fajr",
                "2026-11-19T01:49:00+00:00".parse().unwrap()
            )
            .unwrap());
        storage
            .record_athkar_repeat("morning", 27, 1, "2026-11-19", 1)
            .unwrap();
        drop(storage);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
