//! SQLite schema and migrations. Holds settings, prayer history, athkar
//! progress, voice-pack metadata and mosque profiles.
//!
//! Only settings exist so far; the remaining tables arrive with their features.

use crate::settings::Settings;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Current schema version. Bump on every migration.
pub const SCHEMA_VERSION: i32 = 1;

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

    /// Load the settings blob, or `None` if none has been saved.
    pub fn load_settings(&self) -> Result<Option<Settings>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM settings WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => {
                let json: String = row.get(0)?;
                let settings = serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Some(settings))
            }
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
        assert!(storage.load_settings().unwrap().is_none());
    }

    #[test]
    fn settings_round_trip_through_sqlite() {
        let storage = Storage::open_in_memory().unwrap();
        let settings = Settings::default();
        storage.save_settings(&settings).unwrap();
        let loaded = storage.load_settings().unwrap().expect("settings");
        assert_eq!(loaded, settings);
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

        let loaded = storage.load_settings().unwrap().unwrap();
        assert_eq!(loaded.pre_alert_minutes, 25);

        let count: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn records_the_schema_version() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.schema_version().unwrap(), SCHEMA_VERSION);
    }
}
