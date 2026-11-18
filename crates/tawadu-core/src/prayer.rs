//! Prayer-time calculation: methods, madhabs, high-latitude rules, rounding and
//! manual adjustments.
//!
//! All astronomy is performed in UTC; the caller adds the local offset. Never
//! convert to local time inside this module.
//!
//! Backed by the `adhan` crate, which exposes every configuration knob —
//! including the high-latitude rule and rounding, which `salah` keeps private.

use adhan::{
    HighLatitudeRule as AdhanHighLatitudeRule, Madhab as AdhanMadhab, Method as AdhanMethod,
    Parameters as AdhanParameters, Rounding as AdhanRounding,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
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

/// How Fajr and Isha are bounded where twilight never ends (above ~48°).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighLatitudeRule {
    /// Split the night in half.
    MiddleOfTheNight,
    /// Use the last seventh of the night.
    SeventhOfTheNight,
    /// Use a fixed fraction of the twilight angle.
    TwilightAngle,
}

impl Default for HighLatitudeRule {
    fn default() -> Self {
        HighLatitudeRule::MiddleOfTheNight
    }
}

/// How seconds are handled when a time is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    /// To the nearest minute.
    Nearest,
    /// Up to the next minute.
    Up,
    /// No rounding; seconds are kept.
    None,
}

impl Default for Rounding {
    fn default() -> Self {
        Rounding::Nearest
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
    /// How Fajr and Isha are bounded at high latitude.
    pub high_latitude_rule: HighLatitudeRule,
    /// How seconds are rounded when reporting a time.
    pub rounding: Rounding,
}

impl Default for PrayerConfig {
    fn default() -> Self {
        Self {
            method: Method::default(),
            madhab: Madhab::default(),
            adjustments: Adjustments::default(),
            isha_interval: None,
            high_latitude_rule: HighLatitudeRule::default(),
            rounding: Rounding::default(),
        }
    }
}

impl PrayerConfig {
    fn to_parameters(&self) -> AdhanParameters {
        let method = match self.method {
            Method::MuslimWorldLeague => AdhanMethod::MuslimWorldLeague,
            Method::Egyptian => AdhanMethod::Egyptian,
            Method::Karachi => AdhanMethod::Karachi,
            Method::UmmAlQura => AdhanMethod::UmmAlQura,
            Method::Dubai => AdhanMethod::Dubai,
            Method::MoonsightingCommittee => AdhanMethod::MoonsightingCommittee,
            Method::NorthAmerica => AdhanMethod::NorthAmerica,
            Method::Kuwait => AdhanMethod::Kuwait,
            Method::Qatar => AdhanMethod::Qatar,
            Method::Singapore => AdhanMethod::Singapore,
            Method::Tehran => AdhanMethod::Tehran,
            Method::Turkey => AdhanMethod::Turkey,
            Method::Custom { .. } => AdhanMethod::Other,
        };

        let mut params = AdhanParameters::for_method(method);

        if let Method::Custom {
            fajr_angle,
            isha_angle,
        } = self.method
        {
            params.fajr_angle = fajr_angle;
            params.isha_angle = isha_angle;
            params.isha_interval = 0.0;
        }

        params.madhab = match self.madhab {
            Madhab::Shafi => AdhanMadhab::Shafi,
            Madhab::Hanafi => AdhanMadhab::Hanafi,
        };
        params.high_latitude_rule = match self.high_latitude_rule {
            HighLatitudeRule::MiddleOfTheNight => AdhanHighLatitudeRule::MiddleOfTheNight,
            HighLatitudeRule::SeventhOfTheNight => AdhanHighLatitudeRule::SeventhOfTheNight,
            HighLatitudeRule::TwilightAngle => AdhanHighLatitudeRule::TwilightAngle,
        };
        params.rounding = match self.rounding {
            Rounding::Nearest => AdhanRounding::Nearest,
            Rounding::Up => AdhanRounding::Up,
            Rounding::None => AdhanRounding::None,
        };

        params.fajr_minutes = self.adjustments.fajr as f64;
        params.sunrise_minutes = self.adjustments.sunrise as f64;
        params.dhuhr_minutes = self.adjustments.dhuhr as f64;
        params.asr_minutes = self.adjustments.asr as f64;
        params.maghrib_minutes = self.adjustments.maghrib as f64;
        params.isha_minutes = self.adjustments.isha as f64;

        if let Some(interval) = self.isha_interval {
            params.isha_interval = f64::from(interval);
            params.isha_angle = 0.0;
        }

        params
    }
}

/// One day of computed times, all in UTC.
///
/// A time is `None` when the sun never reaches the position it needs, which
/// happens inside the polar circles. That is the honest answer rather than an
/// invented one; the caller decides what to show.
#[derive(Debug, Clone, PartialEq)]
pub struct PrayerTimes {
    pub fajr: Option<DateTime<Utc>>,
    pub sunrise: Option<DateTime<Utc>>,
    pub dhuhr: Option<DateTime<Utc>>,
    pub asr: Option<DateTime<Utc>>,
    pub maghrib: Option<DateTime<Utc>>,
    pub isha: Option<DateTime<Utc>>,
    pub qiyam: Option<DateTime<Utc>>,
}

impl PrayerTimes {
    /// Time of a single event.
    pub fn time(&self, prayer: Prayer) -> Option<DateTime<Utc>> {
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

    /// The prayer period that contains `now`.
    pub fn current(&self, now: DateTime<Utc>) -> Option<Prayer> {
        Prayer::all()
            .into_iter()
            .filter_map(|prayer| self.time(prayer).map(|time| (prayer, time)))
            .filter(|(_, time)| *time <= now)
            .map(|(prayer, _)| prayer)
            .next_back()
    }

    /// The next event strictly after `now`, if today still has one.
    pub fn next(&self, now: DateTime<Utc>) -> Option<(Prayer, DateTime<Utc>)> {
        Prayer::all()
            .into_iter()
            .filter_map(|prayer| self.time(prayer).map(|time| (prayer, time)))
            .find(|(_, time)| *time > now)
    }

    /// The next obligatory prayer strictly after `now` (skips sunrise and qiyam).
    pub fn next_obligatory(&self, now: DateTime<Utc>) -> Option<(Prayer, DateTime<Utc>)> {
        Prayer::all()
            .into_iter()
            .filter(|prayer| prayer.is_prayer())
            .filter_map(|prayer| self.time(prayer).map(|time| (prayer, time)))
            .find(|(_, time)| *time > now)
    }
}

/// A clock reading on a given date, as an absolute instant.
fn at(date: NaiveDate, reading: (i64, u32, u32)) -> DateTime<Utc> {
    date.and_hms_opt(reading.0 as u32, reading.1, reading.2)
        .expect("a normalised clock reading is a valid time")
        .and_utc()
}

/// A reading that falls before the day's transit. Far enough east, that instant
/// is on the previous UTC day, so step back when the candidate lands after it.
fn before(date: NaiveDate, reading: (i64, u32, u32), anchor: DateTime<Utc>) -> DateTime<Utc> {
    let candidate = at(date, reading);
    if candidate > anchor {
        candidate - Duration::days(1)
    } else {
        candidate
    }
}

/// A reading that falls after the day's transit. Far enough west, that instant
/// is on the next UTC day, so step forward when the candidate lands before it.
fn after(date: NaiveDate, reading: (i64, u32, u32), anchor: DateTime<Utc>) -> DateTime<Utc> {
    let candidate = at(date, reading);
    if candidate < anchor {
        candidate + Duration::days(1)
    } else {
        candidate
    }
}

/// Compute one day of prayer times for a place and a configuration.
pub fn compute(date: NaiveDate, coordinates: Coordinates, config: &PrayerConfig) -> PrayerTimes {
    let params = config.to_parameters();
    let raw = adhan::prayer_times(
        date.year(),
        date.month(),
        date.day(),
        coordinates.latitude,
        coordinates.longitude,
        &params,
    );

    // The crate returns UTC clock readings with no day attached. Anchor them to
    // the day's transit so a reading that belongs to a neighbouring UTC day
    // lands on the right date.
    let anchor = raw
        .dhuhr
        .map(|reading| at(date, reading))
        .unwrap_or_else(|| at(date, (12, 0, 0)));

    let fajr = raw.fajr.map(|reading| before(date, reading, anchor));
    let sunrise = raw.sunrise.map(|reading| before(date, reading, anchor));
    let dhuhr = raw.dhuhr.map(|reading| at(date, reading));
    let asr = raw.asr.map(|reading| after(date, reading, anchor));
    let maghrib = raw.maghrib.map(|reading| after(date, reading, anchor));
    let isha = raw.isha.map(|reading| after(date, reading, anchor));

    let qiyam = adhan::sunnah_times(
        date.year(),
        date.month(),
        date.day(),
        coordinates.latitude,
        coordinates.longitude,
        &params,
    )
    .last_third_of_the_night
    .and_then(|reading| maghrib.map(|sunset| after(date, reading, sunset)));

    PrayerTimes {
        fajr,
        sunrise,
        dhuhr,
        asr,
        maghrib,
        isha,
        qiyam,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};

    fn mins(time: Option<DateTime<Utc>>, offset_hours: i64) -> i64 {
        let local = time.unwrap() + Duration::hours(offset_hours);
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
        assert_eq!(
            (adjusted.dhuhr.unwrap() - base.dhuhr.unwrap()).num_minutes(),
            5
        );
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
        assert_eq!(t.next(now), Some((Prayer::Dhuhr, t.dhuhr.unwrap())));
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
    fn times_are_in_order() {
        let t = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        let order = [
            t.fajr, t.sunrise, t.dhuhr, t.asr, t.maghrib, t.isha, t.qiyam,
        ];
        for pair in order.windows(2) {
            assert!(pair[0] < pair[1], "the day must run in order");
        }
    }

    #[test]
    fn far_east_city_keeps_the_right_utc_day() {
        // Auckland is UTC+13: its local 17 Nov times fall on 16 Nov UTC.
        let t = compute(
            date(),
            Coordinates::new(-36.8485, 174.7633),
            &PrayerConfig {
                method: Method::MuslimWorldLeague,
                ..Default::default()
            },
        );
        let local_minutes = |time: Option<DateTime<Utc>>| {
            let local = time.unwrap() + Duration::hours(13);
            i64::from(local.hour()) * 60 + i64::from(local.minute())
        };
        let close = |actual: i64, expected: &str| {
            let mut it = expected.split(':');
            let h: i64 = it.next().unwrap().parse().unwrap();
            let m: i64 = it.next().unwrap().parse().unwrap();
            assert!(
                (actual - (h * 60 + m)).abs() <= 1,
                "got {actual} min, expected {h}:{m:02}"
            );
        };
        close(local_minutes(t.fajr), "04:19");
        close(local_minutes(t.sunrise), "06:02");
        close(local_minutes(t.dhuhr), "13:06");
        close(local_minutes(t.maghrib), "20:10");
        close(local_minutes(t.isha), "21:46");
        // And they really are on the previous UTC day.
        assert_eq!(
            t.fajr.unwrap().date_naive(),
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
    }

    #[test]
    fn high_latitude_rule_changes_fajr() {
        // Oslo in late November: the twilight angle still resolves, but the two
        // rules disagree, which is what makes the setting real.
        let middle = compute(
            date(),
            Coordinates::new(59.9139, 10.7522),
            &PrayerConfig {
                method: Method::MuslimWorldLeague,
                high_latitude_rule: HighLatitudeRule::MiddleOfTheNight,
                ..Default::default()
            },
        );
        let seventh = compute(
            date(),
            Coordinates::new(59.9139, 10.7522),
            &PrayerConfig {
                method: Method::MuslimWorldLeague,
                high_latitude_rule: HighLatitudeRule::SeventhOfTheNight,
                ..Default::default()
            },
        );
        assert_ne!(middle.fajr, seventh.fajr, "the rule must change Fajr");
    }

    #[test]
    fn rounding_up_is_never_earlier_than_nearest() {
        let nearest = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig {
                rounding: Rounding::Nearest,
                ..Default::default()
            },
        );
        let up = compute(
            date(),
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig {
                rounding: Rounding::Up,
                ..Default::default()
            },
        );
        assert!(up.dhuhr >= nearest.dhuhr);
        assert!(up.fajr >= nearest.fajr);
    }

    #[test]
    fn polar_day_has_no_sunrise_or_sunset() {
        // Tromsø, midsummer: the sun never sets, so the honest answer is `None`.
        let t = compute(
            NaiveDate::from_ymd_opt(2026, 6, 21).unwrap(),
            Coordinates::new(69.6492, 18.9553),
            &PrayerConfig::default(),
        );
        assert!(t.sunrise.is_none(), "the midnight sun has no sunrise");
        assert!(t.maghrib.is_none(), "the midnight sun has no sunset");
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
