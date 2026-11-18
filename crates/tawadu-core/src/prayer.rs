//! Prayer-time calculation: methods, madhabs and manual adjustments.
//!
//! All astronomy is performed in UTC; the caller adds the local offset. Never
//! convert to local time inside this module.
//!
//! **Note:** the underlying `salah` crate does not publicly expose its
//! high-latitude rule or rounding options, so those are not configurable here
//! yet. They are tracked in `docs/architecture.md` as a candidate improvement.

use chrono::{DateTime, NaiveDate, Utc};
use salah::prelude::{
    Adjustment as SalahAdjustment, Configuration, Coordinates as SalahCoordinates,
    Madhab as SalahMadhab, Method as SalahMethod, Prayer as SalahPrayer,
    PrayerTimes as SalahPrayerTimes,
};
use serde::{Deserialize, Serialize};

/// A geographic point. Positive latitude is north, positive longitude is east.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    fn to_salah(self) -> SalahCoordinates {
        SalahCoordinates::new(self.latitude, self.longitude)
    }
}

impl Default for Coordinates {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// The obligatory prayers, plus sunrise and qiyam, in chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prayer {
    Fajr,
    Sunrise,
    Dhuhr,
    Asr,
    Maghrib,
    Isha,
    Qiyam,
}

impl Prayer {
    pub const fn all() -> [Prayer; 7] {
        [
            Prayer::Fajr,
            Prayer::Sunrise,
            Prayer::Dhuhr,
            Prayer::Asr,
            Prayer::Maghrib,
            Prayer::Isha,
            Prayer::Qiyam,
        ]
    }

    pub const fn name_en(self) -> &'static str {
        match self {
            Prayer::Fajr => "Fajr",
            Prayer::Sunrise => "Sunrise",
            Prayer::Dhuhr => "Dhuhr",
            Prayer::Asr => "Asr",
            Prayer::Maghrib => "Maghrib",
            Prayer::Isha => "Isha",
            Prayer::Qiyam => "Qiyam",
        }
    }

    pub const fn name_ar(self) -> &'static str {
        match self {
            Prayer::Fajr => "الفجر",
            Prayer::Sunrise => "الشروق",
            Prayer::Dhuhr => "الظهر",
            Prayer::Asr => "العصر",
            Prayer::Maghrib => "المغرب",
            Prayer::Isha => "العشاء",
            Prayer::Qiyam => "القيام",
        }
    }

    /// True for the five obligatory prayers (not sunrise or qiyam).
    pub const fn is_prayer(self) -> bool {
        !matches!(self, Prayer::Sunrise | Prayer::Qiyam)
    }
}

/// Calculation authority preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    MuslimWorldLeague,
    Egyptian,
    Karachi,
    UmmAlQura,
    Dubai,
    MoonsightingCommittee,
    NorthAmerica,
    Kuwait,
    Qatar,
    Singapore,
    Tehran,
    Turkey,
    /// Own Fajr/Isha angles in degrees.
    Custom {
        fajr_angle: f64,
        isha_angle: f64,
    },
}

impl Default for Method {
    fn default() -> Self {
        Method::UmmAlQura
    }
}

/// Asr shadow factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Madhab {
    Shafi,
    Hanafi,
}

impl Default for Madhab {
    fn default() -> Self {
        Madhab::Shafi
    }
}

/// Per-prayer manual adjustment in minutes (positive or negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjustments {
    pub fajr: i64,
    pub sunrise: i64,
    pub dhuhr: i64,
    pub asr: i64,
    pub maghrib: i64,
    pub isha: i64,
}

impl Default for Adjustments {
    fn default() -> Self {
        Self {
            fajr: 0,
            sunrise: 0,
            dhuhr: 0,
            asr: 0,
            maghrib: 0,
            isha: 0,
        }
    }
}

/// Everything needed to compute one day of prayer times.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrayerConfig {
    pub method: Method,
    pub madhab: Madhab,
    pub adjustments: Adjustments,
    /// Override the Isha interval (minutes after Maghrib), used by Umm al-Qura
    /// and Qatar. `None` keeps the method's own value.
    pub isha_interval: Option<i32>,
}

impl Default for PrayerConfig {
    fn default() -> Self {
        Self {
            method: Method::default(),
            madhab: Madhab::default(),
            adjustments: Adjustments::default(),
            isha_interval: None,
        }
    }
}

impl PrayerConfig {
    fn to_parameters(&self) -> salah::Parameters {
        let mut params = match self.method {
            Method::MuslimWorldLeague => {
                Configuration::with(SalahMethod::MuslimWorldLeague, self.madhab.into())
            }
            Method::Egyptian => Configuration::with(SalahMethod::Egyptian, self.madhab.into()),
            Method::Karachi => Configuration::with(SalahMethod::Karachi, self.madhab.into()),
            Method::UmmAlQura => Configuration::with(SalahMethod::UmmAlQura, self.madhab.into()),
            Method::Dubai => Configuration::with(SalahMethod::Dubai, self.madhab.into()),
            Method::MoonsightingCommittee => {
                Configuration::with(SalahMethod::MoonsightingCommittee, self.madhab.into())
            }
            Method::NorthAmerica => {
                Configuration::with(SalahMethod::NorthAmerica, self.madhab.into())
            }
            Method::Kuwait => Configuration::with(SalahMethod::Kuwait, self.madhab.into()),
            Method::Qatar => Configuration::with(SalahMethod::Qatar, self.madhab.into()),
            Method::Singapore => Configuration::with(SalahMethod::Singapore, self.madhab.into()),
            Method::Tehran => Configuration::with(SalahMethod::Tehran, self.madhab.into()),
            Method::Turkey => Configuration::with(SalahMethod::Turkey, self.madhab.into()),
            Method::Custom {
                fajr_angle,
                isha_angle,
            } => Configuration::new(fajr_angle, isha_angle)
                .madhab(self.madhab.into())
                .done(),
        };

        params.adjustments = SalahAdjustment::new()
            .fajr(self.adjustments.fajr)
            .sunrise(self.adjustments.sunrise)
            .dhuhr(self.adjustments.dhuhr)
            .asr(self.adjustments.asr)
            .maghrib(self.adjustments.maghrib)
            .isha(self.adjustments.isha)
            .done();
        if let Some(interval) = self.isha_interval {
            params.isha_interval = interval;
        }
        params
    }
}

impl From<Madhab> for SalahMadhab {
    fn from(value: Madhab) -> Self {
        match value {
            Madhab::Shafi => SalahMadhab::Shafi,
            Madhab::Hanafi => SalahMadhab::Hanafi,
        }
    }
}

/// One day of computed times, all in UTC.
#[derive(Debug, Clone, PartialEq)]
pub struct PrayerTimes {
    pub fajr: DateTime<Utc>,
    pub sunrise: DateTime<Utc>,
    pub dhuhr: DateTime<Utc>,
    pub asr: DateTime<Utc>,
    pub maghrib: DateTime<Utc>,
    pub isha: DateTime<Utc>,
    pub qiyam: DateTime<Utc>,
}

impl PrayerTimes {
    /// Time of a single event.
    pub fn time(&self, prayer: Prayer) -> DateTime<Utc> {
        match prayer {
            Prayer::Fajr => self.fajr,
            Prayer::Sunrise => self.sunrise,
            Prayer::Dhuhr => self.dhuhr,
            Prayer::Asr => self.asr,
            Prayer::Maghrib => self.maghrib,
            Prayer::Isha => self.isha,
            Prayer::Qiyam => self.qiyam,
        }
    }

    /// The prayer period that contains `now`, if `now` is inside today's span.
    pub fn current(&self, now: DateTime<Utc>) -> Option<Prayer> {
        let mut current = None;
        for prayer in Prayer::all() {
            if self.time(prayer) <= now {
                current = Some(prayer);
            } else {
                break;
            }
        }
        current
    }

    /// The next event strictly after `now`, if today still has one.
    pub fn next(&self, now: DateTime<Utc>) -> Option<(Prayer, DateTime<Utc>)> {
        Prayer::all()
            .into_iter()
            .map(|prayer| (prayer, self.time(prayer)))
            .find(|(_, time)| *time > now)
    }

    /// The next obligatory prayer strictly after `now` (skips sunrise and qiyam).
    pub fn next_obligatory(&self, now: DateTime<Utc>) -> Option<(Prayer, DateTime<Utc>)> {
        Prayer::all()
            .into_iter()
            .filter(|prayer| prayer.is_prayer())
            .map(|prayer| (prayer, self.time(prayer)))
            .find(|(_, time)| *time > now)
    }
}

/// Compute one day of prayer times for a place and a configuration.
pub fn compute(date: NaiveDate, coordinates: Coordinates, config: &PrayerConfig) -> PrayerTimes {
    let params = config.to_parameters();
    let times = SalahPrayerTimes::new(date, coordinates.to_salah(), params);
    PrayerTimes {
        fajr: times.time(SalahPrayer::Fajr),
        sunrise: times.time(SalahPrayer::Sunrise),
        dhuhr: times.time(SalahPrayer::Dhuhr),
        asr: times.time(SalahPrayer::Asr),
        maghrib: times.time(SalahPrayer::Maghrib),
        isha: times.time(SalahPrayer::Isha),
        qiyam: times.time(SalahPrayer::Qiyam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};

    fn mins(t: DateTime<Utc>, offset_hours: i64) -> i64 {
        let local = t + Duration::hours(offset_hours);
        i64::from(local.hour()) * 60 + i64::from(local.minute())
    }

    fn assert_close(actual: i64, expected_hhmm: &str) {
        let mut it = expected_hhmm.split(':');
        let h: i64 = it.next().unwrap().parse().unwrap();
        let m: i64 = it.next().unwrap().parse().unwrap();
        let expected = h * 60 + m;
        // Cross-source differences of up to ~2 minutes are normal (Asr in
        // particular); per-prayer adjustments exist for exactly this reason.
        assert!(
            (actual - expected).abs() <= 2,
            "got {actual} min past midnight, expected {expected} ({expected_hhmm})"
        );
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 17).unwrap()
    }

    #[test]
    fn riyadh_umm_al_qura_matches_reference() {
        let t = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        assert_close(mins(t.fajr, 3), "04:49");
        assert_close(mins(t.sunrise, 3), "06:11");
        assert_close(mins(t.dhuhr, 3), "11:38");
        assert_close(mins(t.asr, 3), "14:45");
        assert_close(mins(t.maghrib, 3), "17:06");
        assert_close(mins(t.isha, 3), "18:36");
    }

    #[test]
    fn makkah_umm_al_qura_matches_reference() {
        let t = compute(
            date(),
            Coordinates::new(21.4225, 39.8262),
            &PrayerConfig::default(),
        );
        assert_close(mins(t.fajr, 3), "05:13");
        assert_close(mins(t.dhuhr, 3), "12:06");
        assert_close(mins(t.asr, 3), "15:16");
        assert_close(mins(t.maghrib, 3), "17:38");
        assert_close(mins(t.isha, 3), "19:08");
    }

    #[test]
    fn london_isna_matches_reference() {
        let cfg = PrayerConfig {
            method: Method::NorthAmerica,
            ..Default::default()
        };
        let t = compute(date(), Coordinates::new(51.5074, -0.1278), &cfg);
        assert_close(mins(t.fajr, 0), "05:44");
        assert_close(mins(t.dhuhr, 0), "11:45");
        assert_close(mins(t.asr, 0), "13:48");
        assert_close(mins(t.maghrib, 0), "16:08");
        assert_close(mins(t.isha, 0), "17:47");
    }

    #[test]
    fn hanafi_asr_is_later_than_shafi() {
        let shafi = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        let hanafi = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig {
                madhab: Madhab::Hanafi,
                ..Default::default()
            },
        );
        assert!(hanafi.asr > shafi.asr, "Hanafi Asr must be later");
    }

    #[test]
    fn manual_adjustment_shifts_only_the_target_prayer() {
        let base = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        let adjusted = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig {
                adjustments: Adjustments {
                    dhuhr: 5,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!((adjusted.dhuhr - base.dhuhr).num_minutes(), 5);
        assert_eq!(adjusted.fajr, base.fajr);
        assert_eq!(adjusted.isha, base.isha);
    }

    #[test]
    fn next_and_current_pick_the_right_prayer() {
        let t = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        // 2026-11-17 08:00 UTC == 11:00 in Riyadh: after sunrise, before Dhuhr.
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 8, 0, 0).unwrap();
        assert_eq!(t.current(now), Some(Prayer::Sunrise));
        assert_eq!(t.next(now), Some((Prayer::Dhuhr, t.dhuhr)));
    }

    #[test]
    fn next_obligatory_skips_sunrise_and_qiyam() {
        let t = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        // 2026-11-17 13:20 UTC == 16:20 in Riyadh: after Asr, before Maghrib.
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 13, 20, 0).unwrap();
        assert_eq!(t.next(now).map(|(p, _)| p), Some(Prayer::Maghrib));
        assert_eq!(
            t.next_obligatory(now).map(|(p, _)| p),
            Some(Prayer::Maghrib)
        );

        // 2026-11-17 15:10 UTC == 18:10 in Riyadh: after Maghrib, before Isha.
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 15, 10, 0).unwrap();
        assert_eq!(t.next_obligatory(now).map(|(p, _)| p), Some(Prayer::Isha));
    }

    #[test]
    fn prayer_names_are_bilingual() {
        assert_eq!(Prayer::Fajr.name_en(), "Fajr");
        assert_eq!(Prayer::Fajr.name_ar(), "الفجر");
        assert_eq!(Prayer::Isha.name_ar(), "العشاء");
        assert!(!Prayer::Sunrise.is_prayer());
        assert!(Prayer::Dhuhr.is_prayer());
    }
}
