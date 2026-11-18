//! Great-circle bearing to the Kaaba (Makkah), in degrees clockwise from true
//! north. Pure trigonometry — no network, no services.

use crate::prayer::Coordinates;

const KAABA: Coordinates = Coordinates {
    latitude: 21.4225_241,
    longitude: 39.8261_818,
};

/// Initial bearing from `from` to the Kaaba, normalised to `[0, 360)`.
pub fn bearing_to_kaaba(from: Coordinates) -> f64 {
    let phi1 = from.latitude.to_radians();
    let phi2 = KAABA.latitude.to_radians();
    let delta_lambda = (KAABA.longitude - from.longitude).to_radians();

    let y = delta_lambda.sin() * phi2.cos();
    let x = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * delta_lambda.cos();

    y.atan2(x).to_degrees().rem_euclid(360.0)
}

/// Cardinal direction for a bearing, in English.
pub fn cardinal(bearing: f64) -> &'static str {
    const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((bearing + 22.5) / 45.0).floor() as usize % 8;
    POINTS[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_bearing(from: Coordinates, expected: f64) {
        let actual = bearing_to_kaaba(from);
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn new_york_bearing() {
        assert_bearing(Coordinates::new(40.7128, -74.0059), 58.4818);
    }

    #[test]
    fn london_bearing() {
        assert_bearing(Coordinates::new(51.5074, -0.1278), 118.9872);
    }

    #[test]
    fn sydney_bearing() {
        assert_bearing(Coordinates::new(-33.8688, 151.2093), 277.4996);
    }

    #[test]
    fn bearing_is_due_north_from_due_south() {
        let b = bearing_to_kaaba(Coordinates::new(21.0, KAABA.longitude));
        assert!(!(0.01..=359.99).contains(&b), "expected ~0, got {b}");
    }

    #[test]
    fn bearing_is_due_east_from_due_west() {
        let b = bearing_to_kaaba(Coordinates::new(KAABA.latitude, 39.0));
        assert!(
            (b - 90.0).abs() < 0.5 || (b - 270.0).abs() < 0.5,
            "expected ~90 or ~270, got {b}"
        );
    }

    #[test]
    fn cardinal_directions() {
        assert_eq!(cardinal(0.0), "N");
        assert_eq!(cardinal(90.0), "E");
        assert_eq!(cardinal(180.0), "S");
        assert_eq!(cardinal(270.0), "W");
        assert_eq!(cardinal(45.0), "NE");
    }
}
