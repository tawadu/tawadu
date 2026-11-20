//! Full city database (GeoNames cities1000, CC BY 4.0) + search core.
//!
//! Phase 5 slice 1. Offline-first: the derived SQLite DB is bundled
//! (assets/cities.db, built by scripts/build_city_db.py — provenance and
//! hashes in assets/licenses/geonames-cities1000-cc-by-4.0.txt). Zero
//! runtime network; nothing leaves the device.
//!
//! Search contract (ledger §5.3, binding §8.5–8.6):
//! - prefix matching on normalized name/asciiname + alias keys (Arabic
//!   aliases included; curated Arabic display names are also search keys);
//! - Arabic normalization for SEARCH ONLY (diacritics stripped, alef/ya/ta
//!   marbuta unified) — display names are never altered;
//! - population ranking with country/admin1 on every hit — duplicate names
//!   (8,131 groups) are disambiguated by the UI, NEVER silently picked;
//! - timezone = the dump's IANA id (column 18), parsed to chrono_tz::Tz.
//!   The build pre-computes normalized columns and indexes, so runtime
//!   queries are indexed range scans — no index build on the UI thread.

use crate::prayer::Coordinates;
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// One search hit. `name_ar` is the CURATED display name ("" when the city
/// has none — GeoNames Arabic script is a search alias, never a display
/// fabrication). `country`/`admin1` ride along for disambiguation.
#[derive(Debug, Clone, PartialEq)]
pub struct CityHit {
    pub geonameid: u32,
    pub name: String,
    pub name_ar: String,
    pub country: String,
    pub admin1: String,
    pub population: u64,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: Tz,
}

impl CityHit {
    /// Resolve into the app's `Location` (coordinates + zone, DST-correct).
    pub fn location(&self) -> crate::locations::Location {
        crate::locations::Location::new(
            &self.name,
            &self.name_ar,
            Coordinates::new(self.latitude, self.longitude),
            self.timezone,
        )
    }
}

/// Normalize a search key or query: ASCII casefold + Arabic search
/// normalization (ledger §5.3). MUST mirror scripts/build_city_db.py's
/// `normalize()` exactly — the build pre-computes keys with the same rules.
/// Search only; never applied to display text.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        // Arabic diacritics (incl. superscript alef) + tatweel: drop.
        if ('\u{064B}'..='\u{065F}').contains(&ch) || ch == '\u{0670}' || ch == '\u{0640}' {
            continue;
        }
        let ch = match ch {
            'أ' | 'إ' | 'آ' => 'ا',
            'ى' => 'ي',
            'ة' => 'ه',
            _ => ch,
        };
        out.extend(ch.to_lowercase());
    }
    out
}

/// The bundled city database (read-only at runtime).
pub struct CityDb {
    conn: Connection,
}

impl CityDb {
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self { conn })
    }

    /// Prefix search over normalized name/asciiname/alias keys, ranked by
    /// population (desc). Every hit carries country/admin1 so duplicates
    /// are never silently resolved. Empty query → the most populous cities
    /// (a useful, honest default list).
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<CityHit>, rusqlite::Error> {
        let q = normalize(query.trim());
        let mut ids: Vec<u32> = Vec::new();
        if q.is_empty() {
            let mut stmt = self
                .conn
                .prepare("SELECT geonameid FROM cities ORDER BY population DESC LIMIT ?1")?;
            let rows = stmt.query_map(params![limit as i64], |r| r.get::<_, u32>(0))?;
            for row in rows {
                ids.push(row?);
            }
        } else {
            // Indexed range scans: [q, q + max-char) is the prefix range.
            let upper = format!("{q}\u{10FFFF}");
            for sql in [
                "SELECT geonameid FROM cities WHERE name_norm >= ?1 AND name_norm < ?2 LIMIT 400",
                "SELECT geonameid FROM cities WHERE ascii_norm >= ?1 AND ascii_norm < ?2 LIMIT 400",
                "SELECT geonameid FROM aliases WHERE alias_norm >= ?1 AND alias_norm < ?2 LIMIT 400",
            ] {
                let mut stmt = self.conn.prepare(sql)?;
                let rows = stmt.query_map(params![q, upper], |r| r.get::<_, u32>(0))?;
                for row in rows {
                    ids.push(row?);
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        let mut hits = Vec::new();
        for id in ids {
            let hit = self.conn.query_row(
                "SELECT geonameid, name, name_ar, country, admin1, population,
                        latitude, longitude, timezone
                 FROM cities WHERE geonameid = ?1",
                params![id],
                |r| {
                    let tz: String = r.get(8)?;
                    Ok(CityHit {
                        geonameid: r.get(0)?,
                        name: r.get(1)?,
                        name_ar: r.get(2)?,
                        country: r.get(3)?,
                        admin1: r.get(4)?,
                        population: r.get::<_, i64>(5)?.max(0) as u64,
                        latitude: r.get(6)?,
                        longitude: r.get(7)?,
                        // The build gates every zone name against the app's
                        // chrono-tz (unparseable_timezones() test asserts the
                        // empty set); a parse failure here would be a data
                        // bug — UTC fallback keeps the app alive, never silent
                        // in the suite.
                        timezone: tz.parse().unwrap_or(Tz::UTC),
                    })
                },
            )?;
            hits.push(hit);
        }
        hits.sort_by(|a, b| {
            b.population
                .cmp(&a.population)
                .then(a.geonameid.cmp(&b.geonameid))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    /// Exact lookup by stable GeoNames id (settings persistence uses ids).
    pub fn by_id(&self, geonameid: u32) -> Result<Option<CityHit>, rusqlite::Error> {
        let hit = self
            .conn
            .query_row(
                "SELECT geonameid, name, name_ar, country, admin1, population,
                        latitude, longitude, timezone
                 FROM cities WHERE geonameid = ?1",
                params![geonameid],
                |r| {
                    let tz: String = r.get(8)?;
                    Ok(CityHit {
                        geonameid: r.get(0)?,
                        name: r.get(1)?,
                        name_ar: r.get(2)?,
                        country: r.get(3)?,
                        admin1: r.get(4)?,
                        population: r.get::<_, i64>(5)?.max(0) as u64,
                        latitude: r.get(6)?,
                        longitude: r.get(7)?,
                        timezone: tz.parse().unwrap_or(Tz::UTC),
                    })
                },
            )
            .optional()?;
        Ok(hit)
    }

    /// Data-quality gate (ledger §3 caveat 1): every distinct timezone in
    /// the derived DB must parse with the app's own chrono-tz. Returns the
    /// offenders (empty = pass).
    pub fn unparseable_timezones(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT timezone FROM cities")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut bad = Vec::new();
        for row in rows {
            let tz = row?;
            if tz.parse::<Tz>().is_err() {
                bad.push(tz);
            }
        }
        Ok(bad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> CityDb {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/cities.db");
        CityDb::open(&path).expect("derived city DB must be bundled (scripts/build_city_db.py)")
    }

    #[test]
    fn normalization_strips_diacritics_and_unifies_forms() {
        assert_eq!(normalize("مَكَّةُ الْمُكَرَّمَةُ"), "مكه المكرمه");
        assert_eq!(normalize("أحمد إبراهيم"), "احمد ابراهيم");
        assert_eq!(normalize("موسى"), "موسي");
        assert_eq!(normalize("Riyadh"), "riyadh");
        assert_eq!(normalize("Sulţānah"), "sulţānah".to_lowercase());
    }

    #[test]
    fn db_opens_with_the_full_pinned_record_count() {
        let db = db();
        let n: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM cities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 172_038, "the pinned dump's record count");
    }

    #[test]
    fn every_timezone_parses_with_the_apps_chrono_tz() {
        assert!(db().unparseable_timezones().unwrap().is_empty());
    }

    #[test]
    fn riyadh_search_finds_the_capital_first_with_correct_zone() {
        let hits = db().search("Riyadh", 10).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].geonameid, 108410);
        assert_eq!(hits[0].name, "Riyadh");
        assert_eq!(hits[0].name_ar, "الرياض", "curated display preserved");
        assert_eq!(hits[0].timezone, Tz::Asia__Riyadh);
        assert_eq!(hits[0].country, "SA");
    }

    #[test]
    fn arabic_query_matches_diacritized_curated_names() {
        // مكة must match مكة المكرمة (curated name_ar is a search key).
        let hits = db().search("مكة", 10).unwrap();
        assert!(hits
            .iter()
            .any(|h| h.geonameid == 108410 || h.name == "Makkah"));
        let hits = db().search("الرياض", 10).unwrap();
        assert_eq!(hits[0].geonameid, 108410);
    }

    #[test]
    fn exit_criterion_cities_resolve_their_real_zones() {
        let db = db();
        for (query, id, tz) in [
            ("Oslo", 3143244, Tz::Europe__Oslo),
            ("Auckland", 2193733, Tz::Pacific__Auckland),
            ("Tromsø", 3133895, Tz::Europe__Oslo),
        ] {
            let hits = db.search(query, 5).unwrap();
            let hit = hits.iter().find(|h| h.geonameid == id).unwrap_or_else(|| {
                panic!(
                    "{query}: expected geonameid {id} among {:?}",
                    hits.iter()
                        .map(|h| (h.geonameid, &h.name))
                        .collect::<Vec<_>>()
                )
            });
            assert_eq!(hit.timezone, tz, "{query} zone");
        }
    }

    #[test]
    fn duplicate_names_are_ranked_and_disambiguated_never_silent() {
        // "Springfield" exists in many US states — every hit must carry
        // country + admin1, ranked by population, no silent single pick.
        let hits = db().search("Springfield", 20).unwrap();
        assert!(hits.len() > 1, "duplicates must surface as a list");
        assert!(hits.iter().all(|h| !h.country.is_empty()));
        assert!(hits.windows(2).all(|w| w[0].population >= w[1].population));
        let first = &hits[0];
        assert!(first.population > 0);
    }

    #[test]
    fn sultanah_counter_example_has_no_fabricated_arabic_display() {
        // geonameid 101760: pop 946,697, NO Arabic alias in the dump —
        // name_ar must stay empty (aliases are search keys, not display).
        let hit = db().by_id(101760).unwrap().expect("Sulţānah present");
        assert_eq!(hit.name_ar, "");
        assert!(hit.population > 900_000);
        // Latin search still finds it.
        let hits = db().search("Sultanah", 10).unwrap();
        assert!(hits.iter().any(|h| h.geonameid == 101760));
    }

    #[test]
    fn empty_query_returns_populous_defaults_and_by_id_round_trips() {
        let db = db();
        let top = db.search("", 5).unwrap();
        assert_eq!(top.len(), 5);
        assert!(top.windows(2).all(|w| w[0].population >= w[1].population));
        let riyadh = db.by_id(108410).unwrap().unwrap();
        assert_eq!(riyadh.name, "Riyadh");
        assert!(db.by_id(999_999_999).unwrap().is_none());
        let loc = riyadh.location();
        assert_eq!(loc.timezone_name(), "Asia/Riyadh");
    }
}
