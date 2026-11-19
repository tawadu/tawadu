//! SQLite schema and migrations. Holds settings, prayer history, athkar
//! progress, voice-pack metadata and mosque profiles.
//!
//! Only settings exist so far; the remaining tables arrive with their features.

use crate::settings::Settings;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Current schema version. Bump on every migration.
pub const SCHEMA_VERSION: i32 = 2;

/// The result of loading settings, with every case kept distinct.
///
/// Only [`LoadOutcome::Fresh`] justifies falling back to
/// [`Settings::default()`]. A corrupt blob or a database failure must never
/// silently become factory defaults: announcing prayer times for the wrong
/// city with full confidence is worse than showing an error.
#[derive(Debug, Clone, PartialEq)]
// Settings is ~256 bytes, but this enum is built once per load and matched
// immediately; boxing would add an allocation and caller friction for nothing.
#[allow(clippy::large_enum_variant)]
pub enum LoadOutcome {
    /// No settings row exists: a brand-new install. Defaults are correct here.
    Fresh,
    /// A settings row exists and parsed successfully.
    Loaded(Settings),
    /// A row exists but its JSON will not deserialise. Carries the raw JSON
    /// verbatim so the UI can offer a reset (or recovery) instead of guessing.
    Corrupt(String),
    /// SQLite itself failed (I/O, locked database, missing table, …).
    /// Carries the error message for display/logging.
    Failed(String),
}

/// A SQLite-backed store.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// The platform's default data directory (`…/tawadu/`), created if
    /// missing. Settings DB, voice-pack library and audio render cache all
    /// live under it.
    pub fn default_data_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let base = PathBuf::from(std::env::var_os("APPDATA")?);
        #[cfg(target_os = "macos")]
        let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
        #[cfg(all(unix, not(target_os = "macos")))]
        let base = match std::env::var_os("XDG_CONFIG_HOME") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
        };

        let dir = base.join("tawadu");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// The platform's default database path (`…/tawadu/tawadu.db`).
    pub fn default_db_path() -> Option<PathBuf> {
        Self::default_data_dir().map(|dir| dir.join("tawadu.db"))
    }

    /// Open the default database, or `None` if it cannot be created.
    pub fn open_default() -> Option<Self> {
        Self::default_db_path().and_then(|path| Self::open(&path).ok())
    }

    /// Open (and migrate) a database file.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    /// Open an in-memory database (used by tests and the CLI's dry runs).
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 json TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS fired (
                 kind TEXT NOT NULL,
                 prayer TEXT NOT NULL,
                 at TEXT NOT NULL,
                 PRIMARY KEY (kind, prayer, at)
             );",
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION.to_string()],
        )?;
        Ok(())
    }

    /// Schema version recorded in the database.
    pub fn schema_version(&self) -> Result<i32, rusqlite::Error> {
        let value: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )?;
        Ok(value.parse().unwrap_or(0))
    }

    /// Record that an event was actually announced. Identity is the EXACT
    /// (kind, prayer, scheduled instant) triple — a schedule recomputed to a
    /// different instant (method/location change) is a NEW event, never
    /// suppressed by an older fire. Idempotent insert; growth is a few rows
    /// per day.
    pub fn record_fired(
        &self,
        kind: &str,
        prayer: &str,
        at: DateTime<Utc>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR IGNORE INTO fired (kind, prayer, at) VALUES (?1, ?2, ?3)",
            params![kind, prayer, at.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Whether this exact event identity was already announced. Existence
    /// only — no range comparison, so a shifted schedule can never be
    /// silently skipped.
    pub fn has_fired(
        &self,
        kind: &str,
        prayer: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, rusqlite::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM fired WHERE kind = ?1 AND prayer = ?2 AND at = ?3",
            params![kind, prayer, at.to_rfc3339()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Save the settings blob (single row, id = 1).
    pub fn save_settings(&self, settings: &Settings) -> Result<(), rusqlite::Error> {
        let json = serde_json::to_string(settings)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO settings (id, json, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
            params![json, updated_at],
        )?;
        Ok(())
    }

    /// Load the settings blob, distinguishing a fresh install (`Fresh`) from
    /// a corrupt blob (`Corrupt`, raw JSON preserved) and a database failure
    /// (`Failed`). Only `Fresh` should fall back to `Settings::default()`.
    pub fn load_settings(&self) -> LoadOutcome {
        let raw = self.read_settings_json();
        match raw {
            Err(e) => LoadOutcome::Failed(e.to_string()),
            Ok(None) => LoadOutcome::Fresh,
            Ok(Some(json)) => match serde_json::from_str::<Settings>(&json) {
                Ok(settings) => LoadOutcome::Loaded(settings),
                // Keep the raw blob: the UI may offer reset/recovery, and a
                // log can show exactly what was on disk.
                Err(_) => LoadOutcome::Corrupt(json),
            },
        }
    }

    /// Fetch the raw settings JSON, or `None` if no row exists. All SQLite
    /// errors (prepare, query, row read) surface here as `Err`.
    fn read_settings_json(&self) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM settings WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_database_has_no_settings() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.load_settings(), LoadOutcome::Fresh);
    }

    #[test]
    fn settings_round_trip_through_sqlite() {
        let storage = Storage::open_in_memory().unwrap();
        let settings = Settings::default();
        storage.save_settings(&settings).unwrap();
        assert_eq!(storage.load_settings(), LoadOutcome::Loaded(settings));
    }

    #[test]
    fn saving_twice_updates_the_same_row() {
        let storage = Storage::open_in_memory().unwrap();
        storage.save_settings(&Settings::default()).unwrap();
        let changed = Settings {
            pre_alert_minutes: 25,
            ..Settings::default()
        };
        storage.save_settings(&changed).unwrap();

        let LoadOutcome::Loaded(loaded) = storage.load_settings() else {
            panic!("expected Loaded after saving twice");
        };
        assert_eq!(loaded.pre_alert_minutes, 25);

        let count: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    /// Write a raw blob straight into the settings row, bypassing
    /// `save_settings`, to simulate corruption or an old schema on disk.
    fn write_raw_blob(storage: &Storage, json: &str) {
        storage
            .conn
            .execute(
                "INSERT INTO settings (id, json, updated_at) VALUES (1, ?1, 'test')
                 ON CONFLICT(id) DO UPDATE SET json = excluded.json",
                params![json],
            )
            .unwrap();
    }

    #[test]
    fn a_broken_json_blob_is_corrupt_not_fresh() {
        let storage = Storage::open_in_memory().unwrap();
        let broken = "{\"location\": {\"name\": \"Riyadh\", ";
        write_raw_blob(&storage, broken);
        // The raw blob must survive verbatim so the UI can offer a reset.
        assert_eq!(storage.load_settings(), LoadOutcome::Corrupt(broken.into()));
    }

    #[test]
    fn valid_json_of_the_wrong_shape_is_corrupt_too() {
        let storage = Storage::open_in_memory().unwrap();
        // Parses as JSON, but is missing every required Settings field.
        write_raw_blob(&storage, "{}");
        assert_eq!(storage.load_settings(), LoadOutcome::Corrupt("{}".into()));
    }

    #[test]
    fn a_blob_missing_newer_fields_still_loads() {
        let storage = Storage::open_in_memory().unwrap();
        let settings = Settings::default();
        let mut value = serde_json::to_value(&settings).unwrap();
        value.as_object_mut().unwrap().remove("notifications");
        write_raw_blob(&storage, &value.to_string());

        let LoadOutcome::Loaded(loaded) = storage.load_settings() else {
            panic!("a pre-notifications blob must still load");
        };
        assert_eq!(
            loaded.notifications,
            crate::notify::NotificationSettings::default()
        );
        assert_eq!(loaded.location, settings.location);
    }

    #[test]
    fn a_database_failure_is_failed_not_fresh() {
        let storage = Storage::open_in_memory().unwrap();
        // Simulate a real SQLite error: the table itself is gone.
        storage.conn.execute_batch("DROP TABLE settings").unwrap();
        let LoadOutcome::Failed(message) = storage.load_settings() else {
            panic!("a SQLite error must surface as Failed, never Fresh");
        };
        assert!(!message.is_empty());
    }

    #[test]
    fn corrupt_settings_can_be_recovered_by_saving() {
        let storage = Storage::open_in_memory().unwrap();
        write_raw_blob(&storage, "not json at all");
        assert!(matches!(storage.load_settings(), LoadOutcome::Corrupt(_)));
        // The UI's "reset" path: save fresh defaults over the corrupt row.
        storage.save_settings(&Settings::default()).unwrap();
        assert_eq!(
            storage.load_settings(),
            LoadOutcome::Loaded(Settings::default())
        );
    }

    #[test]
    fn records_the_schema_version() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.schema_version().unwrap(), SCHEMA_VERSION);
    }

    fn instant(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn ledger_round_trips_a_fired_event() {
        let storage = Storage::open_in_memory().unwrap();
        let at = instant("2026-11-19T01:41:00Z");
        storage.record_fired("pre_alert", "Fajr", at).unwrap();
        assert!(storage.has_fired("pre_alert", "Fajr", at).unwrap());
        // Exact identity: a different instant is a different event.
        assert!(!storage
            .has_fired("pre_alert", "Fajr", at + chrono::Duration::seconds(1))
            .unwrap());
    }

    #[test]
    fn ledger_is_empty_before_any_fire() {
        let storage = Storage::open_in_memory().unwrap();
        let at = instant("2026-11-19T01:41:00Z");
        assert!(!storage.has_fired("prayer", "Fajr", at).unwrap());
    }

    #[test]
    fn ledger_records_each_instant_and_repeats_are_idempotent() {
        let storage = Storage::open_in_memory().unwrap();
        let first = instant("2026-11-19T01:41:00Z");
        let second = instant("2026-11-20T01:42:00Z");
        storage.record_fired("pre_alert", "Fajr", first).unwrap();
        storage.record_fired("pre_alert", "Fajr", first).unwrap();
        storage.record_fired("pre_alert", "Fajr", second).unwrap();
        assert!(storage.has_fired("pre_alert", "Fajr", first).unwrap());
        assert!(storage.has_fired("pre_alert", "Fajr", second).unwrap());
    }

    #[test]
    fn ledger_identities_are_independent() {
        let storage = Storage::open_in_memory().unwrap();
        let at = instant("2026-11-19T01:41:00Z");
        storage.record_fired("pre_alert", "Fajr", at).unwrap();
        // Same prayer, different kind; same kind, different prayer.
        assert!(!storage.has_fired("prayer", "Fajr", at).unwrap());
        assert!(!storage.has_fired("pre_alert", "Dhuhr", at).unwrap());
    }

    #[test]
    fn ledger_schedule_shift_is_a_new_event() {
        // Sentinel's missed-fire path: user switches method/location mid-day
        // and Dhuhr's pre-alert recomputes EARLIER (11:28 -> 11:26 Riyadh).
        // The stored 11:28 must NOT suppress the new 11:26 event.
        let storage = Storage::open_in_memory().unwrap();
        let old = instant("2026-11-19T08:28:00Z");
        storage.record_fired("pre_alert", "Dhuhr", old).unwrap();
        let shifted = instant("2026-11-19T08:26:00Z");
        assert!(storage.has_fired("pre_alert", "Dhuhr", old).unwrap());
        assert!(
            !storage.has_fired("pre_alert", "Dhuhr", shifted).unwrap(),
            "a shifted schedule must read as a NEW event, not be suppressed"
        );
    }

    #[test]
    fn ledger_survives_reopen_and_v1_databases_migrate() {
        // A schema-v1 file (settings only) must gain the ledger on open
        // without losing the saved settings.
        let path = std::env::temp_dir().join(format!(
            "tawadu-ledger-migration-{}-{}.db",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_file(&path);
        {
            let old = Connection::open(&path).unwrap();
            old.execute_batch(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE settings (
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );",
            )
            .unwrap();
            old.execute(
                "INSERT INTO meta (key, value) VALUES ('schema_version', '1')",
                [],
            )
            .unwrap();
            let json = serde_json::to_string(&Settings::default()).unwrap();
            old.execute(
                "INSERT INTO settings (id, json, updated_at) VALUES (1, ?1, 'then')",
                params![json],
            )
            .unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(
            storage.load_settings(),
            LoadOutcome::Loaded(Settings::default())
        );
        let at = instant("2026-11-19T01:41:00Z");
        storage.record_fired("prayer", "Fajr", at).unwrap();
        drop(storage);
        let reopened = Storage::open(&path).unwrap();
        assert!(reopened.has_fired("prayer", "Fajr", at).unwrap());
        let _ = std::fs::remove_file(&path);
    }
}
