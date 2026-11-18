//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! With a subcommand (`tawadu times …`) it runs headless and exits; with none it
//! opens the window.
//!
//! Rust owns all state. The interface reports edits back through callbacks and
//! nothing in `.slint` persists anything by itself.

mod cli;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use tawadu_core::hijri::from_gregorian;
use tawadu_core::locations::{find_city, Location, CITIES};
use tawadu_core::prayer::{self, HighLatitudeRule, Madhab, Method, Prayer, PrayerTimes, Rounding};
use tawadu_core::scheduler::{self, EventKind, IqamahOffsets, Timeline};
use tawadu_core::settings::{Language, Settings, Theme};
use tawadu_core::storage::Storage;

slint::include_modules!();

/// The named methods offered in the interface. `Method::Custom` needs angle
/// inputs, so it arrives with the manual-angle field later.
const METHODS: [(&str, &str, &str); 12] = [
    (
        "muslim_world_league",
        "Muslim World League",
        "رابطة العالم الإسلامي",
    ),
    ("egyptian", "Egyptian", "الهيئة المصرية العامة للمساحة"),
    ("karachi", "Karachi", "جامعة كراتشي"),
    ("umm_al_qura", "Umm al-Qura", "أم القرى"),
    ("dubai", "Dubai", "دبي"),
    ("moonsighting_committee", "Moonsighting", "لجنة رؤية الهلال"),
    ("north_america", "ISNA", "أمريكا الشمالية"),
    ("kuwait", "Kuwait", "الكويت"),
    ("qatar", "Qatar", "قطر"),
    ("singapore", "Singapore", "سنغافورة"),
    ("tehran", "Tehran", "طهران"),
    ("turkey", "Turkey", "تركيا"),
];

/// Everything the interface can change, plus where it is kept.
struct AppState {
    settings: Settings,
    storage: Option<Storage>,
    status: String,
}

/// Decode a serde snake_case key back into a config value.
fn decode<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(key.to_string())).ok()
}

fn option_list(pairs: &[(&str, &str)]) -> Vec<OptionRow> {
    pairs
        .iter()
        .map(|(value, label)| OptionRow {
            value: (*value).into(),
            label: (*label).into(),
        })
        .collect()
}

fn city_rows(query: &str) -> Vec<CityRow> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    CITIES
        .iter()
        .filter(|city| {
            city.name.to_ascii_lowercase().contains(&needle) || city.name_ar.contains(&needle)
        })
        .take(4)
        .map(|city| CityRow {
            name: city.name.into(),
            name_ar: city.name_ar.into(),
            country: city.country.into(),
        })
        .collect()
}

fn method_rows() -> (Vec<MethodRow>, Vec<MethodRow>) {
    let all: Vec<MethodRow> = METHODS
        .iter()
        .map(|(key, label, label_ar)| MethodRow {
            key: (*key).into(),
            label: (*label).into(),
            label_ar: (*label_ar).into(),
        })
        .collect();
    let half = all.len() / 2;
    (all[..half].to_vec(), all[half..].to_vec())
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
    let times = prayer::compute(date, location.coordinates, &settings.config);
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

/// The next obligatory prayer, rolling into tomorrow when today is done.
fn next_prayer(
    times: &PrayerTimes,
    location: &Location,
    settings: &Settings,
) -> Option<(Prayer, chrono::DateTime<Utc>)> {
    if let Some(next) = times.next_obligatory(Utc::now()) {
        return Some(next);
    }
    let tomorrow = location.local(Utc::now()).date_naive() + chrono::Duration::days(1);
    let times = prayer::compute(tomorrow, location.coordinates, &settings.config);
    times.fajr.map(|at| (Prayer::Fajr, at))
}

/// Push the computed day into the home surface.
fn apply_home(ui: &MainWindow, state: &AppState) {
    let location = state.location();
    let (times, timeline) = day(&location, &state.settings);
    let next = next_prayer(&times, &location, &state.settings);
    let now = Utc::now();

    ui.set_location_name(location.name.clone().into());
    ui.set_location_name_ar(location.name_ar.clone().into());
    let hijri = from_gregorian(location.local(now).date_naive())
        .map(|h| format!("{} {} {} AH", h.day, h.month_name_ar(), h.year))
        .unwrap_or_default();
    ui.set_hijri_date(hijri.into());

    match next {
        Some((prayer, at)) => {
            ui.set_next_name(prayer.name_en().to_owned().into());
            ui.set_next_name_ar(prayer.name_ar().to_owned().into());
            ui.set_countdown(countdown(at).into());
        }
        None => {
            ui.set_next_name("—".into());
            ui.set_next_name_ar("—".into());
            ui.set_countdown("--:--:--".into());
        }
    }

    let iqamah_text = timeline
        .events
        .iter()
        .find(|event| event.kind == EventKind::Iqamah && event.at > now)
        .map(|event| format!("{} · {}", event.label, hhmm(&location, event.at)))
        .unwrap_or_else(|| "Set your mosque's iqamah offsets".to_string());
    ui.set_next_iqamah(iqamah_text.into());

    // Placeholder until the athkar engine lands (Phase 4).
    ui.set_athkar_text("سُبْحَانَ اللَّهِ وَبِحَمْدِهِ · مِائَةَ مَرَّةٍ".into());
    ui.set_athkar_progress("4 / 9".into());

    let order = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];
    let next_name = next.map(|(prayer, _)| prayer);
    let rows: Vec<Row> = order
        .into_iter()
        .map(|prayer| {
            let time = times
                .time(prayer)
                .map(|instant| hhmm(&location, instant))
                .unwrap_or_else(|| "--:--".to_string());
            row(
                prayer.name_en(),
                prayer.name_ar(),
                time,
                Some(prayer) == next_name,
                prayer.is_prayer(),
            )
        })
        .collect();
    ui.set_rows(ModelRc::new(VecModel::from(rows)));
}

fn row(name: &str, name_ar: &str, time: String, is_next: bool, is_prayer: bool) -> Row {
    Row {
        name: name.into(),
        name_ar: name_ar.into(),
        time: time.into(),
        is_next,
        is_prayer,
    }
}

/// Push the settings into the settings surface.
fn apply_settings(ui: &MainWindow, state: &AppState) {
    let settings = &state.settings;
    let location = state.location();

    ui.set_settings_location(format!("{} · {}", location.name, location.name_ar).into());
    ui.set_location_query(location.name.clone().into());
    ui.set_city_results(ModelRc::new(VecModel::from(Vec::<CityRow>::new())));

    let (left, right) = method_rows();
    ui.set_methods_left(ModelRc::new(VecModel::from(left)));
    ui.set_methods_right(ModelRc::new(VecModel::from(right)));

    ui.set_selected_method(serde_key(&settings.config.method).into());
    ui.set_madhab_options(ModelRc::new(VecModel::from(option_list(&[
        ("shafi", "Shafi"),
        ("hanafi", "Hanafi"),
    ]))));
    ui.set_selected_madhab(
        match settings.config.madhab {
            Madhab::Shafi => "shafi",
            Madhab::Hanafi => "hanafi",
        }
        .into(),
    );
    ui.set_high_latitude_options(ModelRc::new(VecModel::from(option_list(&[
        ("middle_of_the_night", "Middle of night"),
        ("seventh_of_the_night", "Seventh of night"),
        ("twilight_angle", "Twilight angle"),
    ]))));
    ui.set_selected_high_latitude(
        match settings.config.high_latitude_rule {
            HighLatitudeRule::MiddleOfTheNight => "middle_of_the_night",
            HighLatitudeRule::SeventhOfTheNight => "seventh_of_the_night",
            HighLatitudeRule::TwilightAngle => "twilight_angle",
        }
        .into(),
    );
    ui.set_rounding_options(ModelRc::new(VecModel::from(option_list(&[
        ("nearest", "Nearest minute"),
        ("up", "Up"),
        ("none", "Seconds"),
    ]))));
    ui.set_selected_rounding(
        match settings.config.rounding {
            Rounding::Nearest => "nearest",
            Rounding::Up => "up",
            Rounding::None => "none",
        }
        .into(),
    );
    ui.set_theme_options(ModelRc::new(VecModel::from(option_list(&[
        ("plaster", "Plaster"),
        ("ink", "Ink"),
        ("ramadan", "Ramadan"),
    ]))));
    ui.set_selected_theme(
        match settings.theme {
            Theme::Plaster => "plaster",
            Theme::Ink => "ink",
            Theme::Ramadan => "ramadan",
        }
        .into(),
    );
    ui.set_language_options(ModelRc::new(VecModel::from(option_list(&[
        ("arabic", "العربية"),
        ("english", "English"),
    ]))));
    ui.set_selected_language(
        match settings.language {
            Language::Arabic => "arabic",
            Language::English => "english",
        }
        .into(),
    );

    ui.set_pre_alert_minutes(settings.pre_alert_minutes as i32);
    ui.set_iqamah_fajr(settings.iqamah.fajr as i32);
    ui.set_iqamah_dhuhr(settings.iqamah.dhuhr as i32);
    ui.set_iqamah_asr(settings.iqamah.asr as i32);
    ui.set_iqamah_maghrib(settings.iqamah.maghrib as i32);
    ui.set_iqamah_isha(settings.iqamah.isha as i32);

    ui.set_status_message(state.status.clone().into());
}

/// The serde key for a method, for highlighting the current row.
fn serde_key(method: &Method) -> String {
    serde_json::to_value(method)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

impl AppState {
    fn location(&self) -> Location {
        find_city(&self.settings.location.name).unwrap_or_else(|| self.settings.location.resolve())
    }

    /// Persist the settings; records whether it worked.
    fn save(&mut self) {
        self.status = match &self.storage {
            Some(storage) => match storage.save_settings(&self.settings) {
                Ok(()) => "Saved ✓".to_string(),
                Err(error) => format!("Could not save: {error}"),
            },
            None => "Saving is unavailable".to_string(),
        };
    }
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

    let storage = Storage::open_default();
    let settings = storage
        .as_ref()
        .and_then(|s| s.load_settings().ok().flatten())
        .unwrap_or_default();

    let state = Arc::new(Mutex::new(AppState {
        settings,
        storage,
        status: String::new(),
    }));

    let ui = MainWindow::new()?;
    apply_home(&ui, &state.lock().unwrap());
    apply_settings(&ui, &state.lock().unwrap());

    // ---- callbacks ----
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_open_settings(move || {
            let state = state.lock().unwrap();
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_close_settings(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("home".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_search_city(move |query: SharedString| {
            let rows = city_rows(&query);
            if let Some(ui) = weak.upgrade() {
                ui.set_city_results(ModelRc::new(VecModel::from(rows)));
            }
            let _ = state;
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_select_city(move |name: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(location) = find_city(&name) {
                state.settings.location =
                    tawadu_core::settings::StoredLocation::from_location(&location);
                state.status = format!("Location: {}", location.name);
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_method(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(method) = decode::<Method>(&key) {
                state.settings.config.method = method;
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_madhab(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(madhab) = decode::<Madhab>(&key) {
                state.settings.config.madhab = madhab;
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_high_latitude(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(rule) = decode::<HighLatitudeRule>(&key) {
                state.settings.config.high_latitude_rule = rule;
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_rounding(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(rounding) = decode::<Rounding>(&key) {
                state.settings.config.rounding = rounding;
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_theme(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(theme) = decode::<Theme>(&key) {
                state.settings.theme = theme;
            }
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_language(move |key: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(language) = decode::<Language>(&key) {
                state.settings.language = language;
            }
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_pre_alert(move |minutes: i32| {
            let mut state = state.lock().unwrap();
            state.settings.pre_alert_minutes = minutes.max(0) as u32;
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_iqamah(move |prayer: SharedString, minutes: i32| {
            let mut state = state.lock().unwrap();
            let minutes = minutes.clamp(0, 240) as i64;
            match prayer.as_str() {
                "fajr" => state.settings.iqamah.fajr = minutes,
                "dhuhr" => state.settings.iqamah.dhuhr = minutes,
                "asr" => state.settings.iqamah.asr = minutes,
                "maghrib" => state.settings.iqamah.maghrib = minutes,
                "isha" => state.settings.iqamah.isha = minutes,
                _ => {}
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &state);
                apply_settings(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_save_settings(move || {
            let mut state = state.lock().unwrap();
            state.save();
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
            }
        });
    }

    // Once a second: the countdown text only. Recompute when it runs out.
    let handle = ui.as_weak();
    let timer = Timer::default();
    let initial_target = {
        let guard = state.lock().unwrap();
        let location = guard.location();
        let (times, _) = day(&location, &guard.settings);
        next_prayer(&times, &location, &guard.settings)
    };
    let mut target = initial_target.map(|(_, at)| at);
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        let Some(ui) = handle.upgrade() else {
            return;
        };
        let stale = match target {
            Some(at) => at <= Utc::now(),
            None => true,
        };
        if stale {
            let guard = state.lock().unwrap();
            let location = guard.location();
            let (times, _) = day(&location, &guard.settings);
            match next_prayer(&times, &location, &guard.settings) {
                Some((prayer, at)) => {
                    ui.set_next_name(prayer.name_en().to_owned().into());
                    ui.set_next_name_ar(prayer.name_ar().to_owned().into());
                    target = Some(at);
                }
                None => target = None,
            }
        }
        if let Some(at) = target {
            ui.set_countdown(countdown(at).into());
        }
    });

    ui.run()?;
    Ok(())
}
