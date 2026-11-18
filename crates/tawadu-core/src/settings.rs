//! Typed settings model, persisted in SQLite.

use crate::locations::{find_city, Location};
use crate::prayer::Coordinates;
use crate::prayer::PrayerConfig;
use crate::scheduler::IqamahOffsets;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Interface theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Plaster,
    Ink,
    Ramadan,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Plaster
    }
}

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Arabic,
    English,
}

impl Default for Language {
    fn default() -> Self {
        Language::Arabic
    }
}

/// A location in a form that can be serialised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredLocation {
    pub name: String,
    pub name_ar: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

impl StoredLocation {
    pub fn from_location(location: &Location) -> Self {
        Self {
            name: location.name.clone(),
            name_ar: location.name_ar.clone(),
            latitude: location.coordinates.latitude,
            longitude: location.coordinates.longitude,
            timezone: location.timezone_name().to_string(),
        }
    }

    pub fn resolve(&self) -> Location {
        let timezone: Tz = self.timezone.parse().unwrap_or(Tz::UTC);
        Location::new(
            &self.name,
            &self.name_ar,
            Coordinates::new(self.latitude, self.longitude),
            timezone,
        )
    }
}

/// Everything the user can change, in one serialisable blob.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub location: StoredLocation,
    pub config: PrayerConfig,
    pub iqamah: IqamahOffsets,
    pub pre_alert_minutes: u32,
    pub theme: Theme,
    pub language: Language,
    pub start_at_login: bool,
    pub athan_volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let location = find_city("Riyadh").unwrap_or_else(|| {
            Location::new(
                "Riyadh",
                "الرياض",
                Coordinates::new(24.7136, 46.6753),
                Tz::UTC,
            )
        });
        Self {
            location: StoredLocation::from_location(&location),
            config: PrayerConfig::default(),
            iqamah: IqamahOffsets::default(),
            pre_alert_minutes: 10,
            theme: Theme::default(),
            language: Language::default(),
            start_at_login: false,
            athan_volume: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_riyadh_umm_al_qura() {
        let s = Settings::default();
        assert_eq!(s.location.name, "Riyadh");
        assert_eq!(s.location.timezone, "Asia/Riyadh");
        assert_eq!(s.config.method, crate::prayer::Method::UmmAlQura);
        assert_eq!(s.pre_alert_minutes, 10);
    }

    #[test]
    fn settings_round_trip_through_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn stored_location_resolves_back() {
        let s = Settings::default();
        let loc = s.location.resolve();
        assert_eq!(loc.name, "Riyadh");
        assert_eq!(loc.timezone_name(), "Asia/Riyadh");
    }

    #[test]
    fn unknown_timezone_falls_back_to_utc() {
        let loc = StoredLocation {
            name: "X".into(),
            name_ar: "X".into(),
            latitude: 0.0,
            longitude: 0.0,
            timezone: "Nope/Nope".into(),
        };
        assert_eq!(loc.resolve().timezone_name(), "UTC");
    }
}
