//! SQLite schema and migrations. Holds settings, prayer history, athkar
//! progress, voice-pack metadata and mosque profiles.
//!
//! Only settings exist so far; the remaining tables arrive with their features.

use crate::settings::Settings;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Current schema version. Bump on every migration.
pub const SCHEMA_VERSION: i32 = 1;

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
    /// The platform's default database path (`…/tawadu/tawadu.db`).
    pub fn default_db_path() -> Option<PathBuf> {
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
        Some(dir.join("tawadu.db"))
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
}
