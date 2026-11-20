//! Typed settings model, persisted in SQLite.

use crate::locations::{find_city, Location};
use crate::notify::NotificationSettings;
use crate::prayer::Coordinates;
use crate::prayer::PrayerConfig;
use crate::scheduler::IqamahOffsets;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Playback sequence mode. Pure data: lives in core so the settings blob
/// stays one serialisable shape; the audio engine consumes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    Sequential,
    Shuffle,
}

impl Default for PlaybackMode {
    fn default() -> Self {
        PlaybackMode::Sequential
    }
}

/// The user's per-event voice choice. `pack: None` means "Default" (the
/// marked default pack, else the first installed pack — never a guess).
/// `mode_override: None` means the pack manifest's sequence mode applies;
/// an override always wins over the manifest (Phase 3 decision 7).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EventVoice {
    #[serde(default)]
    pub pack: Option<String>,
    #[serde(default)]
    pub mode_override: Option<PlaybackMode>,
}

/// Audio settings, persisted inside the settings blob. `serde(default)` on
/// the whole struct keeps pre-audio blobs loading unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSettings {
    /// The pack the user marked default. `None` = first installed pack.
    #[serde(default)]
    pub default_pack: Option<String>,
    /// Per-event choices, keyed `athan` / `iqamah` / `athkar` / `chime`.
    /// A missing key is "Default" — never a fabricated choice.
    #[serde(default)]
    pub event_voices: std::collections::BTreeMap<String, EventVoice>,
    /// Stop the athan after N seconds; 0 = play the full recording.
    /// The user's original ask. Applied as a fade-out, never a hard click.
    #[serde(default)]
    pub athan_stop_after_s: u32,
    /// Seed for deterministic shuffle: same seed + same clips = same order.
    #[serde(default = "default_shuffle_seed")]
    pub shuffle_seed: u64,
}

fn default_shuffle_seed() -> u64 {
    // A fixed default keeps playback reproducible across restarts; the user
    // never sees this number, but tests and bug reports can rely on it.
    0x7461_7761_6475_0001
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            default_pack: None,
            event_voices: std::collections::BTreeMap::new(),
            athan_stop_after_s: 0,
            shuffle_seed: default_shuffle_seed(),
        }
    }
}

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
    /// `serde(default)` so a settings blob written before autostart existed
    /// upgrades to OFF instead of falling back to defaults wholesale.
    #[serde(default)]
    pub start_at_login: bool,
    pub athan_volume: f32,
    /// `serde(default)` so a settings blob written before notifications existed
    /// still loads instead of falling back to defaults.
    #[serde(default)]
    pub notifications: NotificationSettings,
    /// `serde(default)` so a settings blob written before audio existed
    /// still loads: no packs chosen, no trim, fixed shuffle seed.
    #[serde(default)]
    pub audio: AudioSettings,
    /// Phase 5: most-recent-first picked places (<= 5, deduped).
    /// `serde(default)` so pre-Phase-5 blobs upgrade with empty recents and
    /// the CHOSEN LOCATION PRESERVED — never a wholesale defaults fallback.
    #[serde(default)]
    pub recent_cities: Vec<StoredLocation>,
    /// Phase 5: the active mosque profile id (None = no active mosque;
    /// legacy settings.iqamah applies). `serde(default)` so pre-Phase-5
    /// blobs upgrade with NO active mosque — never a silent assignment.
    #[serde(default)]
    pub active_mosque: Option<String>,
    /// Phase 5 §6.2: Ramadan mode (opt-in, never auto-applied). Exactly
    /// three effects: theme suggestion, suhur/iftar pre-alert copy,
    /// calendar emphasis. Nothing invented.
    #[serde(default)]
    pub ramadan_mode: bool,
    /// The theme to restore when Ramadan mode turns off (remembered).
    #[serde(default)]
    pub pre_ramadan_theme: Option<Theme>,
    /// Calendar toggle: which numbering is primary (dual always shown).
    #[serde(default)]
    pub calendar_hijri_primary: bool,
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
            notifications: NotificationSettings::default(),
            audio: AudioSettings::default(),
            recent_cities: Vec::new(),
            active_mosque: None,
            ramadan_mode: false,
            pre_ramadan_theme: None,
            calendar_hijri_primary: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_phase5_blob_upgrades_with_location_preserved_and_empty_recents() {
        let mut blob = serde_json::to_value(Settings::default()).unwrap();
        let obj = blob.as_object_mut().unwrap();
        obj.remove("recent_cities");
        obj.remove("active_mosque");
        obj.remove("ramadan_mode");
        obj.remove("pre_ramadan_theme");
        obj.remove("calendar_hijri_primary");
        obj["location"]["name"] = serde_json::json!("Oslo");
        let upgraded: Settings = serde_json::from_value(blob).unwrap();
        assert_eq!(upgraded.location.name, "Oslo", "chosen city preserved");
        assert!(upgraded.recent_cities.is_empty());
        assert!(
            upgraded.active_mosque.is_none(),
            "no silent mosque assignment"
        );
        assert!(
            !upgraded.ramadan_mode,
            "Ramadan mode is opt-in, never auto-on"
        );
        assert!(upgraded.pre_ramadan_theme.is_none());
        assert!(!upgraded.calendar_hijri_primary);
    }

    #[test]
    fn pre_autostart_blob_upgrades_with_autostart_off() {
        // A blob written before start_at_login existed must load (not fall
        // back to defaults wholesale) and the new flag must default OFF.
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("start_at_login");
        let upgraded: Settings = serde_json::from_value(value).expect("old blob must load");
        assert!(!upgraded.start_at_login);
        assert_eq!(upgraded.location.name, Settings::default().location.name);
    }

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
    fn a_blob_without_notifications_still_loads() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("notifications");
        let back: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(back.notifications, NotificationSettings::default());
    }

    #[test]
    fn a_blob_without_audio_still_loads() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("audio");
        let back: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(back.audio, AudioSettings::default());
        assert_eq!(back.location, Settings::default().location);
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
