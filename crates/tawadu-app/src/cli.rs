//! Command-line interface — headless use, scripting and quick checks.
//!
//! The GUI runs when no subcommand is given; `tawadu times …` runs headless.

use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use clap::{Parser, Subcommand};
use tawadu_core::hijri::from_gregorian;
use tawadu_core::locations::{find_city, Location};
use tawadu_core::prayer::{self, Method, Prayer, PrayerConfig};

#[derive(Debug, Parser)]
#[command(
    name = "tawadu",
    version,
    about = "Tawadu — a calm, offline-first prayer companion",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print prayer times for a city
    Times {
        /// City name, English or Arabic
        #[arg(long, default_value = "Riyadh")]
        city: String,
        /// Calculation method
        #[arg(long, default_value = "umm-al-qura")]
        method: String,
        /// Date as YYYY-MM-DD (defaults to today, UTC)
        #[arg(long)]
        date: Option<String>,
        /// IANA timezone override, e.g. Asia/Riyadh
        #[arg(long)]
        timezone: Option<String>,
    },
    /// List the bundled cities
    Cities,
}

/// Parse a method name; accepts kebab-case, snake_case and a few aliases.
pub fn parse_method(input: &str) -> Option<Method> {
    let key = input.trim().to_ascii_lowercase();
    let key = key.replace(['-', ' ', '.'], "_");
    Some(match key.as_str() {
        "mwl" | "muslim_world_league" => Method::MuslimWorldLeague,
        "egypt" | "egyptian" => Method::Egyptian,
        "karachi" => Method::Karachi,
        "umm_al_qura" | "makkah" | "um_al_qura" => Method::UmmAlQura,
        "dubai" => Method::Dubai,
        "moonsighting" | "moonsighting_committee" => Method::MoonsightingCommittee,
        "isna" | "north_america" => Method::NorthAmerica,
        "kuwait" => Method::Kuwait,
        "qatar" => Method::Qatar,
        "singapore" => Method::Singapore,
        "tehran" => Method::Tehran,
        "turkey" | "diyanet" => Method::Turkey,
        _ => return None,
    })
}

fn run_times(
    city: &str,
    method: &str,
    date: Option<&str>,
    timezone: Option<&str>,
) -> Result<(), String> {
    let location = find_city(city).ok_or_else(|| format!("unknown city: {city}"))?;
    let location = match timezone {
        Some(name) => {
            let tz: Tz = name
                .parse()
                .map_err(|_| format!("unknown timezone: {name}"))?;
            Location {
                timezone: tz,
                ..location
            }
        }
        None => location,
    };
    let method = parse_method(method).ok_or_else(|| format!("unknown method: {method}"))?;
    let date = match date {
        Some(text) => NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map_err(|error| format!("bad date '{text}': {error}"))?,
        None => Utc::now().date_naive(),
    };

    let config = PrayerConfig {
        method,
        ..Default::default()
    };
    let times = prayer::compute(date, location.coordinates, &config);

    println!(
        "{} · {} · {}",
        location.name,
        location.name_ar,
        location.timezone_name()
    );
    if let Some(hijri) = from_gregorian(date) {
        println!(
            "Hijri: {} {} {} AH",
            hijri.day,
            hijri.month_name_ar(),
            hijri.year
        );
    }
    println!();

    let order = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];
    for prayer in order {
        let local = location.local(times.time(prayer));
        println!(
            "{:<10} {:>8}   {}",
            prayer.name_en(),
            local.format("%H:%M"),
            prayer.name_ar()
        );
    }
    Ok(())
}

/// Run the CLI. Returns `Err` with a user-facing message on failure.
pub fn run(cli: &Cli) -> Result<(), String> {
    match &cli.command {
        Some(Command::Times {
            city,
            method,
            date,
            timezone,
        }) => run_times(city, method, date.as_deref(), timezone.as_deref()),
        Some(Command::Cities) => {
            for city in tawadu_core::locations::CITIES {
                println!(
                    "{:<16} {:<22} {:>8}  {}",
                    city.name, city.name_ar, city.timezone, city.country
                );
            }
            Ok(())
        }
        None => Err("no subcommand given — try `tawadu times --help`".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_method_aliases() {
        assert_eq!(parse_method("umm-al-qura"), Some(Method::UmmAlQura));
        assert_eq!(parse_method("Umm Al Qura"), Some(Method::UmmAlQura));
        assert_eq!(parse_method("isna"), Some(Method::NorthAmerica));
        assert_eq!(parse_method("mwl"), Some(Method::MuslimWorldLeague));
        assert_eq!(parse_method("nonsense"), None);
    }

    #[test]
    fn prints_times_for_riyadh() {
        run_times("Riyadh", "umm-al-qura", Some("2026-11-17"), None).unwrap();
    }

    #[test]
    fn rejects_an_unknown_city() {
        assert!(run_times("Atlantis", "umm-al-qura", None, None).is_err());
    }
}
