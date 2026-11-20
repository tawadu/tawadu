//! Mosque profiles (Phase 5 slice 3, Atlas addendum: v1 = name + five
//! per-prayer iqamah offsets ONLY — per-profile LOCATION is deferred, no
//! per-profile timelines). The active profile binds the home NEXT IQAMAH
//! card and the scheduler offsets using the app's SINGLE location.
//! Platform-free; persistence in storage (schema v4).

use crate::scheduler::IqamahOffsets;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};

/// One mosque profile. `id` is stable (settings.active_mosque references
/// it); offsets are minutes after the athan, 0 = not set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MosqueProfile {
    pub id: String,
    pub name: String,
    pub name_ar: String,
    pub offsets: IqamahOffsets,
}

impl MosqueProfile {
    pub fn new(id: String) -> Self {
        Self {
            id,
            name: String::new(),
            name_ar: String::new(),
            offsets: IqamahOffsets {
                fajr: 0,
                dhuhr: 0,
                asr: 0,
                maghrib: 0,
                isha: 0,
            },
        }
    }

    /// Rust-composed caption (Prism: only non-zero offsets; the UI places
    /// it, never composes it). "" when all zero — the UI shows Quill's
    /// "No offsets set" string instead.
    pub fn offsets_summary(&self) -> String {
        let parts: Vec<String> = [
            ("Fajr", self.offsets.fajr),
            ("Dhuhr", self.offsets.dhuhr),
            ("Asr", self.offsets.asr),
            ("Maghrib", self.offsets.maghrib),
            ("Isha", self.offsets.isha),
        ]
        .iter()
        .filter(|(_, v)| *v > 0)
        .map(|(n, v)| format!("{n} +{v}"))
        .collect();
        parts.join(" · ")
    }
}

/// The offsets the scheduler and home card must use: the ACTIVE profile's
/// when one is set and found, else the legacy settings.iqamah. Removing
/// the active profile falls back honestly — never a silent reassignment.
pub fn effective_iqamah(settings: &Settings, mosques: &[MosqueProfile]) -> IqamahOffsets {
    if let Some(active) = &settings.active_mosque {
        if let Some(profile) = mosques.iter().find(|m| &m.id == active) {
            return profile.offsets;
        }
    }
    settings.iqamah
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, fajr: i64, dhuhr: i64) -> MosqueProfile {
        let mut m = MosqueProfile::new(id.into());
        m.name = "Al-Noor".into();
        m.offsets.fajr = fajr;
        m.offsets.dhuhr = dhuhr;
        m
    }

    #[test]
    fn summary_lists_only_nonzero_offsets_in_prayer_order() {
        let m = profile("m1", 15, 10);
        assert_eq!(m.offsets_summary(), "Fajr +15 · Dhuhr +10");
        assert_eq!(MosqueProfile::new("m0".into()).offsets_summary(), "");
    }

    #[test]
    fn mosque_crud_round_trips_and_deletes() {
        let storage = crate::storage::Storage::open_in_memory().unwrap();
        let mut m = profile("m1", 15, 10);
        m.name_ar = "مسجد النور".into();
        storage.save_mosque(&m).unwrap();
        m.offsets.fajr = 20;
        storage.save_mosque(&m).unwrap(); // upsert, not duplicate
        let all = storage.load_mosques().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].offsets.fajr, 20);
        assert_eq!(all[0].name_ar, "مسجد النور");
        storage.delete_mosque("m1").unwrap();
        assert!(storage.load_mosques().unwrap().is_empty());
    }

    #[test]
    fn v3_databases_migrate_to_v4_preserving_athkar_and_fired() {
        let dir = std::env::temp_dir().join(format!("tawadu-mosque-mig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT NOT NULL, updated_at TEXT NOT NULL);
                 CREATE TABLE fired (kind TEXT NOT NULL, prayer TEXT NOT NULL, at TEXT NOT NULL, PRIMARY KEY (kind, prayer, at));
                 CREATE TABLE athkar_progress (context TEXT NOT NULL, category INTEGER NOT NULL, item INTEGER NOT NULL, day TEXT NOT NULL, done INTEGER NOT NULL, PRIMARY KEY (context, category, item, day));
                 INSERT INTO meta VALUES ('schema_version', '3');
                 INSERT INTO fired VALUES ('prayer', 'Fajr', '2026-11-19T01:51:00+00:00');
                 INSERT INTO athkar_progress VALUES ('morning', 27, 1, '2026-11-19', 1);",
            ).unwrap();
        }
        let storage = crate::storage::Storage::open(&path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 4);
        assert!(storage
            .has_fired(
                "prayer",
                "Fajr",
                "2026-11-19T01:51:00+00:00".parse().unwrap()
            )
            .unwrap());
        assert_eq!(
            storage
                .athkar_progress("morning", "2026-11-19")
                .unwrap()
                .get(&(27, 1)),
            Some(&1)
        );
        storage.save_mosque(&profile("m1", 15, 10)).unwrap();
        assert_eq!(storage.load_mosques().unwrap().len(), 1);
        drop(storage);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn active_profile_binds_the_offsets() {
        let mut settings = Settings::default();
        settings.iqamah.fajr = 5;
        let mosques = vec![profile("m1", 15, 10), profile("m2", 20, 20)];
        // No active profile: legacy settings win.
        assert_eq!(effective_iqamah(&settings, &mosques).fajr, 5);
        settings.active_mosque = Some("m1".into());
        let eff = effective_iqamah(&settings, &mosques);
        assert_eq!(eff.fajr, 15);
        assert_eq!(eff.dhuhr, 10);
        assert_eq!(eff.asr, 0);
        // Active id no longer present (removed): honest fallback, never a
        // silent reassignment to another profile.
        settings.active_mosque = Some("gone".into());
        assert_eq!(effective_iqamah(&settings, &mosques).fajr, 5);
    }
}
