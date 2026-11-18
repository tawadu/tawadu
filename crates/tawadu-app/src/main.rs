//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! With a subcommand (`tawadu times …`) it runs headless and exits; with none it
//! opens the window.

mod cli;

use std::time::Duration;

use chrono::Utc;
use slint::{ModelRc, Timer, TimerMode, VecModel};
use tawadu_core::hijri::from_gregorian;
use tawadu_core::locations::{find_city, Location};
use tawadu_core::prayer::{self, Prayer, PrayerConfig, PrayerTimes};
use tawadu_core::scheduler::{self, EventKind, IqamahOffsets, Timeline};
use tawadu_core::settings::Settings;

slint::include_modules!();

/// Build one row of the day list.
fn row(name: &str, name_ar: &str, time: String, is_next: bool, is_prayer: bool) -> Row {
    Row {
        name: name.into(),
        name_ar: name_ar.into(),
        time: time.into(),
        is_next,
        is_prayer,
    }
}

fn hhmm(location: &Location, instant: chrono::DateTime<Utc>) -> String {
    location.local(instant).format("%H:%M").to_string()
}

fn countdown(to: chrono::DateTime<Utc>) -> String {
    let remaining = to.signed_duration_since(Utc::now());
    let total = remaining.num_seconds().max(0);
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

/// Compute today's times plus the day's timeline for a location and settings.
fn day(location: &Location, settings: &Settings) -> (PrayerTimes, Timeline) {
    let date = location.local(Utc::now()).date_naive();
    let config = PrayerConfig {
        method: settings.config.method,
        madhab: settings.config.madhab,
        adjustments: settings.config.adjustments,
        isha_interval: settings.config.isha_interval,
    };
    let times = prayer::compute(date, location.coordinates, &config);
    let iqamah = IqamahOffsets {
        fajr: settings.iqamah.fajr,
        dhuhr: settings.iqamah.dhuhr,
        asr: settings.iqamah.asr,
        maghrib: settings.iqamah.maghrib,
        isha: settings.iqamah.isha,
    };
    let timeline = scheduler::build(times, &iqamah, settings.pre_alert_minutes);
    (timeline.times.clone(), timeline)
}

/// The next prayer event, rolling into tomorrow when today is done.
fn next_prayer(
    times: &PrayerTimes,
    location: &Location,
    settings: &Settings,
) -> (Prayer, chrono::DateTime<Utc>) {
    if let Some(next) = times.next_obligatory(Utc::now()) {
        return next;
    }
    let tomorrow = location.local(Utc::now()).date_naive() + chrono::Duration::days(1);
    let config = PrayerConfig {
        method: settings.config.method,
        madhab: settings.config.madhab,
        adjustments: settings.config.adjustments,
        isha_interval: settings.config.isha_interval,
    };
    let times = prayer::compute(tomorrow, location.coordinates, &config);
    (Prayer::Fajr, times.fajr)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = <cli::Cli as clap::Parser>::parse();

    if cli.command.is_some() {
        if let Err(message) = cli::run(&cli) {
            eprintln!("tawadu: {message}");
            std::process::exit(2);
        }
        return Ok(());
    }

    env_logger::init();
    log::info!("Tawadu starting (core v{})", tawadu_core::VERSION);

    let settings = match tawadu_core::storage::Storage::open_default() {
        Some(storage) => storage.load_settings().ok().flatten().unwrap_or_default(),
        None => tawadu_core::storage::Storage::open_in_memory()
            .ok()
            .and_then(|storage| storage.load_settings().ok().flatten())
            .unwrap_or_default(),
    };
    let location =
        find_city(&settings.location.name).unwrap_or_else(|| settings.location.resolve());

    let ui = MainWindow::new()?;

    let (times, timeline) = day(&location, &settings);
    let now = Utc::now();

    // Header
    ui.set_location_name(location.name.clone().into());
    ui.set_location_name_ar(location.name_ar.clone().into());
    let hijri = from_gregorian(location.local(now).date_naive())
        .map(|h| format!("{} {} {} AH", h.day, h.month_name_ar(), h.year))
        .unwrap_or_default();
    ui.set_hijri_date(hijri.into());

    // Countdown
    let (next, at) = next_prayer(&times, &location, &settings);
    ui.set_next_name(next.name_en().to_owned().into());
    ui.set_next_name_ar(next.name_ar().to_owned().into());
    ui.set_countdown(countdown(at).into());

    // Next iqamah
    let iqamah_text = timeline
        .events
        .iter()
        .find(|event| event.kind == EventKind::Iqamah && event.at > now)
        .map(|event| format!("{} · {}", event.label, hhmm(&location, event.at)))
        .unwrap_or_else(|| "Set your mosque's iqamah offsets".to_string());
    ui.set_next_iqamah(iqamah_text.into());

    // Athkar card
    ui.set_athkar_text("سُبْحَانَ اللَّهِ وَبِحَمْدِهِ · مِائَةَ مَرَّةٍ".into());
    ui.set_athkar_progress("4 / 9".into());

    // Day list
    let order = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];
    let rows: Vec<Row> = order
        .into_iter()
        .map(|prayer| {
            row(
                prayer.name_en(),
                prayer.name_ar(),
                hhmm(&location, times.time(prayer)),
                prayer == next,
                prayer.is_prayer(),
            )
        })
        .collect();
    ui.set_rows(ModelRc::new(VecModel::from(rows)));

    // Tick the countdown once a second.
    let handle = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        if let Some(ui) = handle.upgrade() {
            let (next, at) = {
                let (times, _) = day(&location, &settings);
                next_prayer(&times, &location, &settings)
            };
            ui.set_next_name(next.name_en().to_owned().into());
            ui.set_countdown(countdown(at).into());
        }
    });

    ui.run()?;
    Ok(())
}
