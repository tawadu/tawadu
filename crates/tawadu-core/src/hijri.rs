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

    #[test]
    fn rejects_invalid_month() {
        assert!(HijriDate::new(1448, 0, 1).is_none());
        assert!(HijriDate::new(1448, 13, 1).is_none());
    }
}
