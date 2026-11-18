//! City database, geocoding and timezone resolution.
//!
//! Offline-first: a bundled city database is the default. Network lookups are a
//! later, optional enhancement and must degrade silently.

use crate::prayer::Coordinates;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;

/// A city in the bundled database.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct City {
    pub name: &'static str,
    pub name_ar: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: &'static str,
    pub country: &'static str,
}

/// A resolved place: coordinates plus a timezone.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub name: String,
    pub name_ar: String,
    pub coordinates: Coordinates,
    pub timezone: Tz,
}

impl Location {
    pub fn new(name: &str, name_ar: &str, coordinates: Coordinates, timezone: Tz) -> Self {
        Self {
            name: name.to_string(),
            name_ar: name_ar.to_string(),
            coordinates,
            timezone,
        }
    }

    /// Convert a UTC instant into this location's local time.
    pub fn local(&self, instant: DateTime<Utc>) -> DateTime<Tz> {
        instant.with_timezone(&self.timezone)
    }

    pub fn timezone_name(&self) -> &str {
        self.timezone.name()
    }
}

impl City {
    /// Resolve into a `Location`, falling back to UTC if the timezone is unknown.
    pub fn location(&self) -> Location {
        let timezone: Tz = self.timezone.parse().unwrap_or(Tz::UTC);
        Location::new(
            self.name,
            self.name_ar,
            Coordinates::new(self.latitude, self.longitude),
            timezone,
        )
    }
}

/// Bundled cities. Small on purpose: the full GeoNames set is a later addition.
pub const CITIES: &[City] = &[
    City {
        name: "Riyadh",
        name_ar: "الرياض",
        latitude: 24.7136,
        longitude: 46.6753,
        timezone: "Asia/Riyadh",
        country: "SA",
    },
    City {
        name: "Makkah",
        name_ar: "مكة المكرمة",
        latitude: 21.4225,
        longitude: 39.8262,
        timezone: "Asia/Riyadh",
        country: "SA",
    },
    City {
        name: "Madinah",
        name_ar: "المدينة المنورة",
        latitude: 24.5247,
        longitude: 39.5692,
        timezone: "Asia/Riyadh",
        country: "SA",
    },
    City {
        name: "Jeddah",
        name_ar: "جدة",
        latitude: 21.4858,
        longitude: 39.1925,
        timezone: "Asia/Riyadh",
        country: "SA",
    },
    City {
        name: "Dammam",
        name_ar: "الدمام",
        latitude: 26.4207,
        longitude: 50.0888,
        timezone: "Asia/Riyadh",
        country: "SA",
    },
    City {
        name: "Dubai",
        name_ar: "دبي",
        latitude: 25.2048,
        longitude: 55.2708,
        timezone: "Asia/Dubai",
        country: "AE",
    },
    City {
        name: "Doha",
        name_ar: "الدوحة",
        latitude: 25.2854,
        longitude: 51.5310,
        timezone: "Asia/Qatar",
        country: "QA",
    },
    City {
        name: "Kuwait City",
        name_ar: "مدينة الكويت",
        latitude: 29.3759,
        longitude: 47.9774,
        timezone: "Asia/Kuwait",
        country: "KW",
    },
    City {
        name: "Cairo",
        name_ar: "القاهرة",
        latitude: 30.0444,
        longitude: 31.2357,
        timezone: "Africa/Cairo",
        country: "EG",
    },
    City {
        name: "Istanbul",
        name_ar: "إسطنبول",
        latitude: 41.0082,
        longitude: 28.9784,
        timezone: "Europe/Istanbul",
        country: "TR",
    },
    City {
        name: "Amman",
        name_ar: "عمّان",
        latitude: 31.9454,
        longitude: 35.9284,
        timezone: "Asia/Amman",
        country: "JO",
    },
    City {
        name: "Baghdad",
        name_ar: "بغداد",
        latitude: 33.3152,
        longitude: 44.3661,
        timezone: "Asia/Baghdad",
        country: "IQ",
    },
    City {
        name: "Karachi",
        name_ar: "كراتشي",
        latitude: 24.8607,
        longitude: 67.0011,
        timezone: "Asia/Karachi",
        country: "PK",
    },
    City {
        name: "Lahore",
        name_ar: "لاهور",
        latitude: 31.5204,
        longitude: 74.3587,
        timezone: "Asia/Karachi",
        country: "PK",
    },
    City {
        name: "Dhaka",
        name_ar: "دكا",
        latitude: 23.8103,
        longitude: 90.4125,
        timezone: "Asia/Dhaka",
        country: "BD",
    },
    City {
        name: "Jakarta",
        name_ar: "جاكرتا",
        latitude: -6.2088,
        longitude: 106.8456,
        timezone: "Asia/Jakarta",
        country: "ID",
    },
    City {
        name: "Kuala Lumpur",
        name_ar: "كوالالمبور",
        latitude: 3.1390,
        longitude: 101.6869,
        timezone: "Asia/Kuala_Lumpur",
        country: "MY",
    },
    City {
        name: "Casablanca",
        name_ar: "الدار البيضاء",
        latitude: 33.5731,
        longitude: -7.5898,
        timezone: "Africa/Casablanca",
        country: "MA",
    },
    City {
        name: "Algiers",
        name_ar: "الجزائر",
        latitude: 36.7538,
        longitude: 3.0588,
        timezone: "Africa/Algiers",
        country: "DZ",
    },
    City {
        name: "London",
        name_ar: "لندن",
        latitude: 51.5074,
        longitude: -0.1278,
        timezone: "Europe/London",
        country: "GB",
    },
    City {
        name: "Paris",
        name_ar: "باريس",
        latitude: 48.8566,
        longitude: 2.3522,
        timezone: "Europe/Paris",
        country: "FR",
    },
    City {
        name: "Berlin",
        name_ar: "برلين",
        latitude: 52.5200,
        longitude: 13.4050,
        timezone: "Europe/Berlin",
        country: "DE",
    },
    City {
        name: "Oslo",
        name_ar: "أوسلو",
        latitude: 59.9139,
        longitude: 10.7522,
        timezone: "Europe/Oslo",
        country: "NO",
    },
    City {
        name: "New York",
        name_ar: "نيويورك",
        latitude: 40.7128,
        longitude: -74.0060,
        timezone: "America/New_York",
        country: "US",
    },
    City {
        name: "Los Angeles",
        name_ar: "لوس أنجلوس",
        latitude: 34.0522,
        longitude: -118.2437,
        timezone: "America/Los_Angeles",
        country: "US",
    },
    City {
        name: "Toronto",
        name_ar: "تورونتو",
        latitude: 43.6532,
        longitude: -79.3832,
        timezone: "America/Toronto",
        country: "CA",
    },
    City {
        name: "Sydney",
        name_ar: "سيدني",
        latitude: -33.8688,
        longitude: 151.2093,
        timezone: "Australia/Sydney",
        country: "AU",
    },
    City {
        name: "Auckland",
        name_ar: "أوكلاند",
        latitude: -36.8485,
        longitude: 174.7633,
        timezone: "Pacific/Auckland",
        country: "NZ",
    },
];

/// Case-insensitive lookup by English or Arabic name (exact, then prefix).
pub fn find_city(query: &str) -> Option<Location> {
    let q = query.trim();
    CITIES
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(q) || c.name_ar == q)
        .or_else(|| {
            CITIES.iter().find(|c| {
                c.name
                    .to_ascii_lowercase()
                    .starts_with(&q.to_ascii_lowercase())
            })
        })
        .map(City::location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn finds_riyadh_by_english_and_arabic_name() {
        let en = find_city("Riyadh").expect("Riyadh should be in the database");
        assert_eq!(en.timezone_name(), "Asia/Riyadh");
        assert!((en.coordinates.latitude - 24.7136).abs() < 1e-9);

        let ar = find_city("الرياض").expect("Arabic name should resolve");
        assert_eq!(ar.name, "Riyadh");
    }

    #[test]
    fn prefix_lookup_is_case_insensitive() {
        assert_eq!(find_city("mak").unwrap().name, "Makkah");
        assert_eq!(find_city("NEW").unwrap().name, "New York");
    }

    #[test]
    fn unknown_city_is_none() {
        assert!(find_city("Atlantis").is_none());
    }

    #[test]
    fn converts_utc_to_local_time() {
        let loc = find_city("Riyadh").unwrap();
        let instant = Utc.with_ymd_and_hms(2026, 11, 17, 1, 49, 0).unwrap();
        let local = loc.local(instant);
        assert_eq!(local.hour(), 4);
        assert_eq!(local.minute(), 49);
    }

    #[test]
    fn unknown_timezone_falls_back_to_utc() {
        let city = City {
            name: "Nowhere",
            name_ar: "لا مكان",
            latitude: 0.0,
            longitude: 0.0,
            timezone: "Not/AZone",
            country: "ZZ",
        };
        assert_eq!(city.location().timezone_name(), "UTC");
    }
}
