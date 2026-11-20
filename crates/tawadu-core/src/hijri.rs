//! Hijri conversion using the Umm al-Qura calendar, via the `hijridate` crate.
//!
//! Umm al-Qura is a sighting-based civil calendar, so it is table-driven rather
//! than arithmetic. `hijridate` carries the published month starts for Hijri
//! years 1343–1500 (Gregorian 1924–2077); outside that range it falls back to a
//! tabular calendar and can be a day or two off.

use chrono::{Datelike, NaiveDate};
use hijridate::Gregorian;

/// A Hijri (Umm al-Qura) date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HijriDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl HijriDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if month == 0 || month > 12 {
            return None;
        }
        let hijri = hijridate::Hijri::new(year, month, day).ok()?;
        let (y, m, d) = hijri.datetuple();
        Some(Self {
            year: y,
            month: m,
            day: d,
        })
    }

    /// English month name.
    pub fn month_name_en(self) -> &'static str {
        hijridate::Hijri::new(self.year, self.month, 1)
            .map(|h| h.month_name())
            .unwrap_or("")
    }

    /// Arabic month name.
    pub fn month_name_ar(self) -> &'static str {
        match self.month {
            1 => "محرم",
            2 => "صفر",
            3 => "ربيع الأول",
            4 => "ربيع الآخر",
            5 => "جمادى الأولى",
            6 => "جمادى الآخرة",
            7 => "رجب",
            8 => "شعبان",
            9 => "رمضان",
            10 => "شوال",
            11 => "ذو القعدة",
            12 => "ذو الحجة",
            _ => "",
        }
    }

    pub fn isoformat(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Published Umm al-Qura table bounds (hijridate::ummalqura FIRST/LAST
/// HIJRI_YEAR). Outside these the crate falls back to a tabular calendar
/// that can be a day or two off — navigation is disabled there with a
/// caption, never an extrapolated guess (Prism §6.1).
pub const TABLE_FIRST_YEAR: i32 = 1343;
pub const TABLE_LAST_YEAR: i32 = 1500;

impl HijriDate {
    /// Back to Gregorian (engine value; None if the date is invalid).
    pub fn to_gregorian(self) -> Option<NaiveDate> {
        let h = hijridate::Hijri::new(self.year, self.month, self.day).ok()?;
        let (y, m, d) = h.to_gregorian().datetuple();
        NaiveDate::from_ymd_opt(y, m as u32, d as u32)
    }

    /// Days in this Hijri month (29 or 30, from the table).
    pub fn month_len(self) -> u8 {
        hijridate::Hijri::new(self.year, self.month, 1)
            .map(|h| h.month_length())
            .unwrap_or(30)
    }

    pub fn is_ramadan(self) -> bool {
        self.month == 9
    }

    /// Inside the published table (vs the crate's tabular fallback).
    pub fn in_table(self) -> bool {
        (TABLE_FIRST_YEAR..=TABLE_LAST_YEAR).contains(&self.year)
    }
}

/// Convert a Gregorian date to its Umm al-Qura Hijri date.
pub fn from_gregorian(date: NaiveDate) -> Option<HijriDate> {
    let gregorian = Gregorian::new(date.year(), date.month() as u8, date.day() as u8).ok()?;
    let hijri = gregorian.to_hijri();
    let (year, month, day) = hijri.datetuple();
    Some(HijriDate { year, month, day })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn first_ramadan_1445_is_11_march_2024() {
        let h = from_gregorian(date(2024, 3, 11)).unwrap();
        assert_eq!((h.year, h.month, h.day), (1445, 9, 1));
        assert_eq!(h.month_name_en(), "Ramadan");
        assert_eq!(h.month_name_ar(), "رمضان");
    }

    #[test]
    fn known_date_matches_reference() {
        let h = from_gregorian(date(2026, 11, 17)).unwrap();
        assert_eq!((h.year, h.month, h.day), (1448, 6, 7));
        assert_eq!(h.isoformat(), "1448-06-07");
        assert_eq!(h.month_name_ar(), "جمادى الآخرة");
    }

    // ---- Phase 5 slice 5: calendar engine surface ----

    #[test]
    fn hijri_gregorian_round_trip() {
        let h = from_gregorian(date(2024, 3, 11)).unwrap();
        assert_eq!(h.to_gregorian(), Some(date(2024, 3, 11)));
        let h2 = HijriDate::new(1448, 6, 7).unwrap();
        assert_eq!(h2.to_gregorian(), Some(date(2026, 11, 17)));
    }

    #[test]
    fn month_lengths_come_from_the_table() {
        // Ramadan 1445 ran 11 Mar – 9 Apr 2024 = 30 days (Umm al-Qura).
        assert_eq!(HijriDate::new(1445, 9, 1).unwrap().month_len(), 30);
        let len = HijriDate::new(1448, 6, 1).unwrap().month_len();
        assert!(len == 29 || len == 30, "Hijri months are 29 or 30 days");
    }

    #[test]
    fn table_bounds_are_engine_values() {
        assert_eq!(TABLE_FIRST_YEAR, 1343);
        assert_eq!(TABLE_LAST_YEAR, 1500);
        assert!(HijriDate::new(1343, 1, 1).unwrap().in_table());
        assert!(HijriDate::new(1500, 12, 1).unwrap().in_table());
        assert!(!HijriDate::new(1501, 1, 1).unwrap().in_table());
        assert!(!HijriDate::new(1342, 12, 1).unwrap().in_table());
    }

    #[test]
    fn ramadan_is_month_nine() {
        assert!(HijriDate::new(1445, 9, 1).unwrap().is_ramadan());
        assert!(!HijriDate::new(1445, 8, 1).unwrap().is_ramadan());
        assert!(!HijriDate::new(1445, 10, 1).unwrap().is_ramadan());
    }

    #[test]
    fn rejects_invalid_month() {
        assert!(HijriDate::new(1448, 0, 1).is_none());
        assert!(HijriDate::new(1448, 13, 1).is_none());
    }
}
