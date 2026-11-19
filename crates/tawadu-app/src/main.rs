//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! With a subcommand (`tawadu times …`) it runs headless and exits; with none it
//! opens the window.
//!
//! Rust owns all state. The interface reports edits back through callbacks and
//! nothing in `.slint` persists anything by itself.

mod autostart;
mod cli;
mod restore;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveTime, Utc};
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use tawadu_core::hijri::from_gregorian;
use tawadu_core::locations::{find_city, Location, CITIES};
use tawadu_core::notify::{self, NotificationKind};
use tawadu_core::prayer::{self, HighLatitudeRule, Madhab, Method, Prayer, PrayerTimes, Rounding};
use tawadu_core::scheduler::{self, EventKind, IqamahOffsets, Timeline};
use tawadu_core::settings::{Language, Settings, Theme};
use tawadu_core::storage::{LoadOutcome, Storage};

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

/// Debounce decision for save-on-change: a save is due once `delay` has
/// passed since the LAST change; every change restarts the window. Pure
/// logic (injectable instants) so the policy is unit-testable; the UI thread
/// drains it on a short tick.
#[derive(Debug)]
struct SaveDebounce {
    delay: Duration,
    pending_since: Option<Instant>,
}

impl SaveDebounce {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            pending_since: None,
        }
    }

    /// A setting changed: (re)start the debounce window.
    fn touch(&mut self, now: Instant) {
        self.pending_since = Some(now);
    }

    /// An immediate save (حفظ) supersedes any pending debounce.
    fn cancel(&mut self) {
        self.pending_since = None;
    }

    /// Force the pending write out (quit / hide / exit): true when a save
    /// was pending. Survives abrupt exit before the debounce timer fires.
    fn flush(&mut self) -> bool {
        self.pending_since.take().is_some()
    }

    /// True exactly once when the window has elapsed; clears the pending
    /// save so the next tick does not write again.
    fn take_if_due(&mut self, now: Instant) -> bool {
        match self.pending_since {
            Some(since) if now.duration_since(since) >= self.delay => {
                self.pending_since = None;
                true
            }
            _ => false,
        }
    }
}

/// Everything the interface can change, plus where it is kept.
struct AppState {
    settings: Settings,
    storage: Option<Storage>,
    status: String,
    /// Tells the scheduler to recompute now rather than at the end of its slice.
    wake: Option<Arc<mpsc::Sender<()>>>,
    /// Save-on-change debounce (800 ms after the last mutation). RefCell so
    /// `changed(&self)` can mark it without widening every call site; the
    /// AppState Mutex already serializes access.
    debounce: std::cell::RefCell<SaveDebounce>,
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

/// One theme's tokens, from `design/brand.md`.
struct ThemeColors {
    bg: (u8, u8, u8),
    surface: (u8, u8, u8),
    surface_secondary: (u8, u8, u8),
    ink: (u8, u8, u8),
    ink_muted: (u8, u8, u8),
    border: (u8, u8, u8),
    accent: (u8, u8, u8),
    accent_soft: (u8, u8, u8),
    accent_ink: (u8, u8, u8),
    gold: (u8, u8, u8),
}

fn colors(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Plaster => ThemeColors {
            bg: (0xF6, 0xF1, 0xE8),
            surface: (0xFF, 0xFD, 0xF8),
            surface_secondary: (0xEF, 0xE8, 0xDB),
            ink: (0x21, 0x1C, 0x15),
            ink_muted: (0x6E, 0x66, 0x5A),
            border: (0xE2, 0xD9, 0xC9),
            accent: (0x2E, 0x5C, 0x4E),
            accent_soft: (0xE3, 0xEE, 0xE9),
            accent_ink: (0xFF, 0xFF, 0xFF),
            gold: (0xB0, 0x8A, 0x4A),
        },
        Theme::Ink => ThemeColors {
            bg: (0x14, 0x11, 0x0D),
            surface: (0x1D, 0x19, 0x15),
            surface_secondary: (0x27, 0x22, 0x1C),
            ink: (0xEF, 0xE7, 0xDA),
            ink_muted: (0xB0, 0xA6, 0x97),
            border: (0x33, 0x2C, 0x24),
            accent: (0x8F, 0xC3, 0xAE),
            accent_soft: (0x24, 0x35, 0x2E),
            accent_ink: (0x0F, 0x1A, 0x15),
            gold: (0xD0, 0xA4, 0x5C),
        },
        Theme::Ramadan => ThemeColors {
            bg: (0x17, 0x12, 0x09),
            surface: (0x22, 0x1A, 0x10),
            surface_secondary: (0x2C, 0x22, 0x14),
            ink: (0xF3, 0xE7, 0xD3),
            ink_muted: (0xC2, 0xB3, 0x99),
            border: (0x3A, 0x2E, 0x1C),
            accent: (0xD9, 0xA4, 0x41),
            accent_soft: (0x33, 0x27, 0x0F),
            accent_ink: (0x1A, 0x12, 0x06),
            gold: (0xD0, 0xA4, 0x5C),
        },
    }
}

/// Push a theme into the `Palette` global. The global is declared in `main.slint`
/// rather than an imported file precisely so the Rust side can reach it — an
/// imported global gets inlined and cannot be set at runtime.
fn apply_theme(ui: &MainWindow, theme: Theme) {
    use slint::Color;
    let c = colors(theme);
    let palette = ui.global::<Palette>();
    palette.set_bg(Color::from_rgb_u8(c.bg.0, c.bg.1, c.bg.2));
    palette.set_surface(Color::from_rgb_u8(c.surface.0, c.surface.1, c.surface.2));
    palette.set_surface_secondary(Color::from_rgb_u8(
        c.surface_secondary.0,
        c.surface_secondary.1,
        c.surface_secondary.2,
    ));
    palette.set_ink(Color::from_rgb_u8(c.ink.0, c.ink.1, c.ink.2));
    palette.set_ink_muted(Color::from_rgb_u8(
        c.ink_muted.0,
        c.ink_muted.1,
        c.ink_muted.2,
    ));
    palette.set_border(Color::from_rgb_u8(c.border.0, c.border.1, c.border.2));
    palette.set_accent(Color::from_rgb_u8(c.accent.0, c.accent.1, c.accent.2));
    palette.set_accent_soft(Color::from_rgb_u8(
        c.accent_soft.0,
        c.accent_soft.1,
        c.accent_soft.2,
    ));
    palette.set_accent_ink(Color::from_rgb_u8(
        c.accent_ink.0,
        c.accent_ink.1,
        c.accent_ink.2,
    ));
    palette.set_gold(Color::from_rgb_u8(c.gold.0, c.gold.1, c.gold.2));
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
    let remaining = to.signed_duration_since(now());
    let total = remaining.num_seconds().max(0);
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

/// Compute today's times plus the day's timeline for a location and settings.
///
/// Yesterday's events are merged in because a late Isha iqamah, or the sleep
/// athkar window at Isha+30, can fall after local midnight — and today's timeline
/// does not contain them. That only happens at high latitude, which is exactly
/// where the app promises not to invent or drop a time.
fn day(location: &Location, settings: &Settings) -> (PrayerTimes, Timeline) {
    let date = location.local(now()).date_naive();
    let iqamah = IqamahOffsets {
        fajr: settings.iqamah.fajr,
        dhuhr: settings.iqamah.dhuhr,
        asr: settings.iqamah.asr,
        maghrib: settings.iqamah.maghrib,
        isha: settings.iqamah.isha,
    };

    let times = prayer::compute(date, location.coordinates, &settings.config);
    let mut events = scheduler::build(times.clone(), &iqamah, settings.pre_alert_minutes).events;

    let yesterday = date - chrono::Duration::days(1);
    let ytimes = prayer::compute(yesterday, location.coordinates, &settings.config);
    events.extend(scheduler::build(ytimes, &iqamah, settings.pre_alert_minutes).events);

    events.sort_by(|a, b| {
        a.at.cmp(&b.at)
            .then_with(|| a.kind.priority().cmp(&b.kind.priority()))
    });

    (times.clone(), Timeline { times, events })
}

/// The next obligatory prayer, rolling into tomorrow when today is done.
fn next_prayer(
    times: &PrayerTimes,
    location: &Location,
    settings: &Settings,
) -> Option<(Prayer, chrono::DateTime<Utc>)> {
    if let Some(next) = times.next_obligatory(now()) {
        return Some(next);
    }
    let tomorrow = location.local(now()).date_naive() + chrono::Duration::days(1);
    let times = prayer::compute(tomorrow, location.coordinates, &settings.config);
    times.fajr.map(|at| (Prayer::Fajr, at))
}

/// Push the computed day into the home surface.
fn apply_home(ui: &MainWindow, state: &AppState) {
    let location = state.location();
    let (times, timeline) = day(&location, &state.settings);
    let next = next_prayer(&times, &location, &state.settings);
    let now = now();

    ui.set_location_name(location.name.clone().into());
    ui.set_location_name_ar(location.name_ar.clone().into());
    let hijri = from_gregorian(location.local(now).date_naive())
        .map(|h| format!("{} {} {} هـ", h.day, h.month_name_ar(), h.year))
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

    // A real dhikr from Hisn al-Muslim, shown statically until the athkar engine
    // lands (Phase 4). It deliberately carries no progress counter: a count next
    // to "one hundred times" that invents its own number is a visible lie.
    ui.set_athkar_text("سُبْحَانَ اللَّهِ وَبِحَمْدِهِ · مِائَةَ مَرَّةٍ".into());

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

    ui.set_settings_location_name(location.name.clone().into());
    ui.set_settings_location_name_ar(location.name_ar.clone().into());

    let n = &settings.notifications;
    ui.set_notif_pre_alert(n.pre_alert);
    ui.set_notif_athan(n.athan);
    ui.set_notif_iqamah(n.iqamah);
    ui.set_notif_athkar(n.athkar);
    ui.set_notif_system(n.system);
    ui.set_quiet_hours_enabled(n.quiet_hours_enabled);
    ui.set_start_at_login(settings.start_at_login);
    ui.set_quiet_from(n.quiet_from.format("%H:%M").to_string().into());
    ui.set_quiet_to(n.quiet_to.format("%H:%M").to_string().into());
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

    /// Flush a pending debounced save immediately (quit / hide / exit), so
    /// the trailing write survives an abrupt exit mid-debounce.
    fn flush_save(&mut self) {
        if self.debounce.borrow_mut().flush() {
            self.save();
        }
    }

    /// A setting changed: wake the scheduler (the timeline may have changed)
    /// and (re)start the debounced save-on-change window.
    fn changed(&self) {
        if let Some(wake) = &self.wake {
            let _ = wake.send(());
        }
        self.debounce.borrow_mut().touch(Instant::now());
    }
}

/// Where "now" comes from.
///
/// `TAWADU_DEBUG_NOW` (an RFC 3339 instant) starts the clock at that moment and
/// then lets it advance in real time, so a notification can be exercised without
/// waiting for a real prayer. Unset, this is the system clock.
///
/// The rest of the app reads `now()` rather than the clock directly: a frozen
/// clock would make the scheduler wait forever for an event that is already due.
static DEBUG_NOW: OnceLock<Option<(DateTime<Utc>, std::time::Instant)>> = OnceLock::new();

/// Multiplies real elapsed time. `12` makes a 60-second wait look like twelve
/// minutes to the clock, which is how a machine that slept past an event is
/// exercised without putting the PC to sleep. Unset = 1.0.
static DEBUG_SCALE: OnceLock<f64> = OnceLock::new();

fn now() -> DateTime<Utc> {
    let started = DEBUG_NOW.get_or_init(|| {
        std::env::var("TAWADU_DEBUG_NOW")
            .ok()
            .and_then(|value| value.parse::<DateTime<Utc>>().ok())
            .map(|base| (base, std::time::Instant::now()))
    });
    match started {
        Some((base, at)) => {
            let scale = *DEBUG_SCALE.get_or_init(|| {
                std::env::var("TAWADU_DEBUG_TIME_SCALE")
                    .ok()
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(1.0)
            });
            let elapsed = Duration::from_secs_f64(at.elapsed().as_secs_f64() * scale);
            *base + chrono::Duration::from_std(elapsed).unwrap_or_default()
        }
        None => Utc::now(),
    }
}

/// The scheduler never sleeps longer than this, so a clock change, a wake from
/// sleep or a settings edit is picked up within a minute.
const MAX_SLICE: Duration = Duration::from_secs(60);

/// Parse the debug slice override. Unset, invalid or zero = the production
/// 60 s — a zero slice would busy-loop the scheduler thread.
fn parse_slice_ms(value: Option<&str>) -> Duration {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(MAX_SLICE)
}

/// The scheduler's max sleep slice, read once. `TAWADU_DEBUG_MAX_SLICE_MS`
/// overrides it for the exit test: the slice is REAL time, so at debug scale
/// 12 a 60 s slice advances the debug clock 720 s — past the 120 s late
/// tolerance — and every event is silently dropped. A 5 s slice (60 debug-s)
/// keeps the scale-12 exit test runnable in ~2 minutes. Off by default.
fn max_slice() -> Duration {
    static SLICE: OnceLock<Duration> = OnceLock::new();
    *SLICE
        .get_or_init(|| parse_slice_ms(std::env::var("TAWADU_DEBUG_MAX_SLICE_MS").ok().as_deref()))
}
/// An event the machine slept through is still worth announcing, but only just.
const LATE_TOLERANCE_SECONDS: i64 = 120;
/// How long the in-app banner stays up.
const BANNER_MS: u64 = 8000;

/// How long the in-app banner stays up. `TAWADU_DEBUG_BANNER_MS` overrides it so a
/// screenshot can be taken without racing the auto-hide.
fn banner_hold() -> Duration {
    static HOLD: OnceLock<u64> = OnceLock::new();
    let ms = *HOLD.get_or_init(|| {
        std::env::var("TAWADU_DEBUG_BANNER_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(BANNER_MS)
    });
    Duration::from_millis(ms)
}

/// What the scheduler thread hands to the UI thread.
enum Notice {
    Banner { title: String, body: String },
}

fn event_kind(kind: EventKind) -> Option<NotificationKind> {
    match kind {
        EventKind::PreAlert => Some(NotificationKind::PreAlert),
        EventKind::Prayer => Some(NotificationKind::Athan),
        EventKind::Iqamah => Some(NotificationKind::Iqamah),
        EventKind::AthkarWindow => Some(NotificationKind::Athkar),
    }
}

/// Decide whether to announce an event, and with what words. `None` means the
/// user's switches say stay quiet.
fn announce(
    event: &scheduler::Event,
    location: &Location,
    settings: &Settings,
    now: DateTime<Utc>,
) -> Option<(String, String)> {
    let kind = event_kind(event.kind)?;
    // Quiet hours are judged at the EVENT's local time, not the fire time:
    // a late fire (sleep/wake) must not sneak a quiet-hours alert past the
    // window, nor silence an alert whose event fell outside it. The athan
    // is essential and bypasses quiet hours entirely (should_notify).
    let event_local = location.local(event.at);
    if !notify::should_notify(kind, &settings.notifications, event_local.time()) {
        return None;
    }
    let at = event_local.format("%H:%M").to_string();
    let (title, body) = match kind {
        NotificationKind::PreAlert => {
            // The alert fires pre_alert_minutes BEFORE the prayer; show the
            // prayer's own time (exact by construction: scheduler::build
            // places the alert at prayer_at - pre_alert_minutes), and floor
            // the LIVE remaining minutes so a late fire never contradicts
            // the countdown ("in 10 min" next to 00:09:50).
            let prayer_at =
                event.at + chrono::Duration::minutes(i64::from(settings.pre_alert_minutes));
            let minutes = (prayer_at - now).num_minutes().max(0);
            let title = if minutes == 0 {
                format!("{} now", event.label)
            } else {
                format!("{} in {} min", event.label, minutes)
            };
            let prayer_time = location.local(prayer_at).format("%H:%M");
            (
                title,
                format!("{} at {} · {}", event.label, prayer_time, location.name),
            )
        }
        NotificationKind::Athan => (
            format!("Athan · {}", event.label),
            format!("It is time for {} prayer · {}", event.label, at),
        ),
        NotificationKind::Iqamah => (
            format!("Iqamah · {}", event.label),
            "The prayer has started at the mosque".to_string(),
        ),
        NotificationKind::Athkar => (
            event.label.to_string(),
            "Your athkar window is open".to_string(),
        ),
        NotificationKind::System => return None,
    };
    Some((title, body))
}

/// Send an OS notification. Returns `false` when the platform has no usable
/// notification service; the in-app banner then carries the message.
fn os_notify(title: &str, body: &str) -> bool {
    let mut notification = notify_rust::Notification::new();
    notification.appname("Tawadu").summary(title).body(body);
    // Windows: attribute the toast to our registered AUMID instead of
    // notify-rust's PowerShell default (ADR 0002 condition 3). `app_id` is a
    // Windows-only method in notify-rust 4 — it does not exist on macOS.
    #[cfg(target_os = "windows")]
    notification.app_id(tray::AUMID);
    match notification.show() {
        Ok(_) => true,
        Err(error) => {
            log::warn!("OS notification unavailable ({error}); the in-app banner carried it");
            false
        }
    }
}

/// Sleep until the next event, fire it, advance.
///
/// The loop recomputes the timeline after every slice, which is what makes a
/// clock change, a wake from sleep and a settings edit all correct without a
/// platform-specific wake-up signal.
fn spawn_scheduler(
    state: Arc<Mutex<AppState>>,
    wake: mpsc::Receiver<()>,
    notices: mpsc::Sender<Notice>,
) {
    std::thread::Builder::new()
        .name("tawadu-scheduler".into())
        .spawn(move || {
            // Read once: the slice is a debug-clock testing knob, not a
            // setting that changes mid-run.
            let max_slice = max_slice();
            // The instant up to which every event has been considered. It only
            // advances after a sweep, which is what stops a machine that slept past
            // several events from hiding all but the one it was waiting on.
            let mut watermark = now();
            loop {
                let (location, settings) = {
                    let guard = state.lock().unwrap();
                    (guard.location(), guard.settings.clone())
                };
                let (_, timeline) = day(&location, &settings);
                let next = timeline.next_after(watermark).cloned();

                let wait = next
                    .as_ref()
                    .map(|event| (event.at - watermark).to_std().unwrap_or(Duration::ZERO))
                    .unwrap_or(max_slice)
                    .min(max_slice);

                match wake.recv_timeout(wait) {
                    Ok(()) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }

                let fired_at = now();
                let due = timeline.due(
                    watermark,
                    fired_at,
                    chrono::Duration::seconds(LATE_TOLERANCE_SECONDS),
                );
                // Ledger filter: per-identity skip of already-announced
                // events (restart inside the tolerance). Read errors fail
                // open (None -> fire): a broken ledger must never silence.
                let fireable = scheduler::fireable(due, |event| {
                    let guard = state.lock().unwrap();
                    let last = guard.storage.as_ref().and_then(|storage| {
                        storage
                            .last_fired(event.kind.key(), event.label)
                            .ok()
                            .flatten()
                    });
                    if scheduler::already_fired(last, event.at) {
                        log::info!("skipping already-fired {} · {}", event.kind.key(), event.at);
                    }
                    last
                });
                for event in fireable {
                    if let Some((title, body)) = announce(&event, &location, &settings, now()) {
                        // Transactional order: persist the fire BEFORE the
                        // announcement, so a crash after announcing can never
                        // re-announce on restart. Write failure is logged and
                        // fails open (the alert still goes out).
                        {
                            let guard = state.lock().unwrap();
                            if let Some(storage) = guard.storage.as_ref() {
                                if let Err(error) =
                                    storage.record_fired(event.kind.key(), event.label, event.at)
                                {
                                    log::warn!("fired-ledger write failed: {error}");
                                }
                            }
                        }
                        log::info!("firing {} · {} — {}", event.kind.key(), event.at, title);
                        let _ = notices.send(Notice::Banner {
                            title: title.clone(),
                            body: body.clone(),
                        });
                        // OS delivery must NEVER block the scheduler:
                        // notify-rust's macOS backend waits for a click by
                        // default (nsusernotifications.rs wait_for_click),
                        // which froze the loop after the first alert on CI.
                        // Toasts are secondary — the banner above is the
                        // correctness floor. Events are rare; a thread per
                        // toast costs nothing.
                        std::thread::spawn(move || {
                            os_notify(&title, &body);
                        });
                    }
                }
                watermark = fired_at;
            }
        })
        .expect("spawn the scheduler thread");
}

/// A second instance would run a second scheduler and notify twice, so only the
/// GUI claims the name; the CLI stays free to run alongside it.
///
/// The handle is deliberately leaked: dropping it releases the lock, and a
/// dropped lock is exactly the bug this is here to prevent.
fn claim_single_instance() -> bool {
    match single_instance::SingleInstance::new("tawadu-app") {
        Ok(instance) => {
            let single = instance.is_single();
            Box::leak(Box::new(instance));
            single
        }
        Err(error) => {
            log::warn!("could not check for another instance ({error}); continuing");
            true
        }
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

    if !claim_single_instance() {
        // ADR 0002 condition 5: restore the existing (possibly hidden) window
        // instead of printing into the void.
        if restore::signal_existing() {
            eprintln!("Tawadu is already running — restoring its window.");
        } else {
            eprintln!("Tawadu is already running.");
        }
        return Ok(());
    }

    env_logger::init();
    // Before any toast: attribute notifications to Tawadu, not PowerShell.
    tray::register_aumid();
    log::info!("Tawadu starting (core v{})", tawadu_core::VERSION);

    let storage = Storage::open_default();
    let mut settings = Settings::default();
    let mut warning: Option<String> = None;
    if let Some(store) = storage.as_ref() {
        match store.load_settings() {
            LoadOutcome::Fresh => {}
            LoadOutcome::Loaded(loaded) => settings = loaded,
            // Never silent: a corrupt blob or a failed read falls back to
            // defaults, but the user is told, because defaults are Riyadh and
            // the app will otherwise announce the wrong times with confidence.
            LoadOutcome::Corrupt(raw) => {
                log::error!("settings blob unreadable, using defaults: {raw}");
                warning = Some(
                    "Your saved settings could not be read — using defaults. Press حفظ to reset."
                        .to_string(),
                );
            }
            LoadOutcome::Failed(message) => {
                log::error!("could not read settings: {message}");
                warning = Some(format!(
                    "Could not read your saved settings ({message}). Using defaults — press حفظ to retry."
                ));
            }
        }
    }

    let state = Arc::new(Mutex::new(AppState {
        settings,
        storage,
        status: String::new(),
        wake: None,
        debounce: std::cell::RefCell::new(SaveDebounce::new(Duration::from_millis(800))),
    }));

    let (wake_tx, wake_rx) = mpsc::channel::<()>();
    state.lock().unwrap().wake = Some(Arc::new(wake_tx));

    // Opt-in autostart: reconcile the registration against the setting on
    // every launch (on-but-missing → register; off-but-stale → unregister).
    autostart::reconcile_at_startup(state.lock().unwrap().settings.start_at_login);

    let ui = MainWindow::new()?;
    // ADR 0002: hide() drops the window's keepalive; without this held
    // KeepAlive the event loop would quit on the first close-to-tray. The
    // type is not nameable from the facade — a main() local for the app
    // lifetime, dropped only when main returns after Quit.
    let _keepalive = tray::hold_event_loop()?;
    apply_theme(&ui, state.lock().unwrap().settings.theme);
    apply_home(&ui, &state.lock().unwrap());
    apply_settings(&ui, &state.lock().unwrap());
    if let Some(message) = &warning {
        ui.set_status_message(message.clone().into());
    }

    // ---- tray (ADR 0002) ----
    let hidden = Arc::new(AtomicBool::new(false));
    let initial_target = {
        let guard = state.lock().unwrap();
        let location = guard.location();
        let (times, _) = day(&location, &guard.settings);
        next_prayer(&times, &location, &guard.settings)
    };
    let initial_tooltip = match initial_target {
        Some((prayer, at)) => {
            let location = state.lock().unwrap().location();
            tray::tooltip_text(Some((prayer.name_en(), &hhmm(&location, at))))
        }
        None => tray::tooltip_text(None),
    };
    let tray = match tray::build_tray(&initial_tooltip) {
        Ok(tray) => Some(tray),
        Err(error) => {
            log::warn!("tray unavailable ({error}); closing will keep the window shown");
            None
        }
    };
    let tray_ok = tray.is_some();
    {
        let hidden = Arc::clone(&hidden);
        let flush_state = Arc::clone(&state);
        ui.window().on_close_requested(move || {
            if tray_ok {
                // Flush the trailing debounced write before hiding: the
                // process may stay hidden for days, or be killed outright.
                flush_state.lock().unwrap().flush_save();
                hidden.store(true, Ordering::SeqCst);
                log::info!("hidden to tray");
            }
            tray::close_response(tray_ok)
        });
    }
    let show_window = {
        let weak = ui.as_weak();
        let hidden = Arc::clone(&hidden);
        move || {
            if let Some(ui) = weak.upgrade() {
                if let Err(error) = ui.window().show() {
                    log::warn!("could not show window ({error})");
                }
                hidden.store(false, Ordering::SeqCst);
            }
        }
    };
    {
        let show_window = show_window.clone();
        restore::listen(move || {
            let show_window = show_window.clone();
            let _ = slint::invoke_from_event_loop(move || {
                show_window();
                log::info!("restored window (second launch)");
            });
        });
    }
    // Tray/menu events: muda delivers them on a global channel; poll it on
    // the UI thread (the winit backend's own muda feature stays dormant).
    let menu_timer = Timer::default();
    {
        let show_window = show_window.clone();
        let quit_state = Arc::clone(&state);
        menu_timer.start(TimerMode::Repeated, Duration::from_millis(250), move || {
            while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                match event.id().0.as_str() {
                    "show" => show_window(),
                    "quit" => {
                        log::info!("quitting (tray menu)");
                        quit_state.lock().unwrap().flush_save();
                        let _ = slint::quit_event_loop();
                    }
                    _ => {}
                }
            }
        });
    }

    // ---- callbacks ----
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_open_settings(move || {
            let state = state.lock().unwrap();
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                apply_theme(&ui, state.settings.theme);
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
                apply_settings(&ui, &state);
                state.changed();
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
                state.changed();
            }
            // The immediate save supersedes any pending debounce write.
            state.debounce.borrow_mut().cancel();
        });
    }

    // Debounced save-on-change: every settings mutation marks the window via
    // state.changed(); this short tick drains it once 800 ms of quiet have
    // passed (worst-case latency 800 ms + one tick). حفظ stays immediate.
    // Main scope: dropping a Slint Timer stops it, so it must outlive ui.run().
    let save_timer = Timer::default();
    let save_state = Arc::clone(&state);
    let save_handle = ui.as_weak();
    save_timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(ui) = save_handle.upgrade() else {
            return;
        };
        let mut state = save_state.lock().unwrap();
        if state.debounce.borrow_mut().take_if_due(Instant::now()) {
            state.save();
            apply_settings(&ui, &state);
        }
    });

    // Once a second: the countdown text only. Recompute when it runs out.
    // The tray lives in this closure so its tooltip rides the same tick —
    // recomputed once per prayer change, never per tick. TrayIcon is !Send
    // (Rc<RefCell> inside), so it must stay on the UI thread.
    let handle = ui.as_weak();
    let timer = Timer::default();
    let timer_state = Arc::clone(&state);
    let mut target = initial_target.map(|(_, at)| at);
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        let Some(ui) = handle.upgrade() else {
            return;
        };
        let stale = match target {
            Some(at) => at <= now(),
            None => true,
        };
        if stale {
            let guard = timer_state.lock().unwrap();
            let location = guard.location();
            let (times, _) = day(&location, &guard.settings);
            match next_prayer(&times, &location, &guard.settings) {
                Some((prayer, at)) => {
                    ui.set_next_name(prayer.name_en().to_owned().into());
                    ui.set_next_name_ar(prayer.name_ar().to_owned().into());
                    if let Some(tray) = &tray {
                        let _ = tray.set_tooltip(Some(tray::tooltip_text(Some((
                            prayer.name_en(),
                            &hhmm(&location, at),
                        )))));
                    }
                    target = Some(at);
                }
                None => {
                    target = None;
                    if let Some(tray) = &tray {
                        let _ = tray.set_tooltip(Some(tray::tooltip_text(None)));
                    }
                }
            }
        }
        if let Some(at) = target {
            ui.set_countdown(countdown(at).into());
        }
    });

    // ---- notification switches ----
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_notif_kind(move |key: SharedString, on: bool| {
            let mut state = state.lock().unwrap();
            if let Some(kind) = decode::<NotificationKind>(&key) {
                state.settings.notifications.set_enabled(kind, on);
            }
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
                state.changed();
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_quiet_enabled(move |on: bool| {
            let mut state = state.lock().unwrap();
            state.settings.notifications.quiet_hours_enabled = on;
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
                state.changed();
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_start_at_login(move |on: bool| {
            let mut state = state.lock().unwrap();
            state.settings.start_at_login = on;
            // Apply the registration immediately; the flag itself persists
            // through the debounced save-on-change like every other setting.
            autostart::set(on);
            if let Some(ui) = weak.upgrade() {
                apply_settings(&ui, &state);
                state.changed();
            }
        });
    }
    {
        let state = Arc::clone(&state);
        ui.on_set_quiet_from(move |value: SharedString| {
            if let Ok(time) = NaiveTime::parse_from_str(value.as_str(), "%H:%M") {
                let mut state = state.lock().unwrap();
                state.settings.notifications.quiet_from = time;
                state.changed();
            }
        });
    }
    {
        let state = Arc::clone(&state);
        ui.on_set_quiet_to(move |value: SharedString| {
            if let Ok(time) = NaiveTime::parse_from_str(value.as_str(), "%H:%M") {
                let mut state = state.lock().unwrap();
                state.settings.notifications.quiet_to = time;
                state.changed();
            }
        });
    }

    // ---- notification engine ----
    let (notice_tx, notice_rx) = mpsc::channel::<Notice>();
    spawn_scheduler(Arc::clone(&state), wake_rx, notice_tx);

    let banner_handle = ui.as_weak();
    let banner_hidden = Arc::clone(&hidden);
    let mut shown_at: Option<std::time::Instant> = None;
    let banner_timer = Timer::default();
    banner_timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
        let Some(ui) = banner_handle.upgrade() else {
            return;
        };
        while let Ok(notice) = notice_rx.try_recv() {
            match notice {
                Notice::Banner { title, body } => {
                    // ADR 0002 condition 2: the banner is the correctness
                    // floor — while hidden, restore + show the window first
                    // so the alert is never invisible.
                    if banner_hidden.swap(false, Ordering::SeqCst) {
                        if let Err(error) = ui.window().show() {
                            log::warn!("could not restore window for banner ({error})");
                        }
                        log::info!("restored window for alert banner");
                    }
                    ui.set_banner_title(title.into());
                    ui.set_banner_body(body.into());
                    ui.set_banner_visible(true);
                    shown_at = Some(std::time::Instant::now());
                }
            }
        }
        if let Some(at) = shown_at {
            if at.elapsed() >= banner_hold() {
                ui.set_banner_visible(false);
                shown_at = None;
            }
        }
    });

    // macOS: Slint's winit backend creates no default app menu, so Cmd+Q is
    // inert without one. The muda app menu's Quit item shares the tray menu's
    // "quit" id, so the same poll handles both.
    #[cfg(target_os = "macos")]
    let _app_menu = tray::macos_app_menu();

    // Debug lifecycle hooks for the process-level regression test
    // (tests/tray_lifecycle.rs): dispatch the REAL close event / the REAL quit
    // call after N ms. Off by default, like the debug clock.
    let mut debug_timers = Vec::new();
    if let Some(ms) = std::env::var("TAWADU_DEBUG_AUTOCLOSE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        let weak = ui.as_weak();
        let timer = Timer::default();
        timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(ms),
            move || {
                if let Some(ui) = weak.upgrade() {
                    log::info!("debug: dispatching CloseRequested");
                    ui.window()
                        .dispatch_event(slint::platform::WindowEvent::CloseRequested);
                }
            },
        );
        debug_timers.push(timer);
    }
    if let Some(ms) = std::env::var("TAWADU_DEBUG_AUTOQUIT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        let timer = Timer::default();
        let quit_state = Arc::clone(&state);
        timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(ms),
            move || {
                log::info!("debug: quitting event loop");
                quit_state.lock().unwrap().flush_save();
                let _ = slint::quit_event_loop();
            },
        );
        debug_timers.push(timer);
    }
    if let Some(ms) = std::env::var("TAWADU_DEBUG_TOUCH_SETTING_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        let timer = Timer::default();
        let touch_state = Arc::clone(&state);
        timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(ms),
            move || {
                // Mutate through the same path as a UI handler: marks the
                // debounce window, no immediate save. Lets process tests
                // exercise the trailing-write durability headlessly.
                let mut state = touch_state.lock().unwrap();
                state.settings.pre_alert_minutes = 15;
                state.changed();
                log::info!("debug: touched setting (debounce pending)");
            },
        );
        debug_timers.push(timer);
    }

    // Sentinel hardening: a platform event-loop failure must be visible at
    // app level, not just propagate as main's Err.
    if let Err(error) = ui.run() {
        log::error!("event loop failed: {error}");
        state.lock().unwrap().flush_save();
        return Err(error.into());
    }
    // Catch-all exit flush (macOS Cmd+Q and any other loop exit): the
    // trailing debounced write must survive an abrupt exit mid-debounce.
    state.lock().unwrap().flush_save();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_override_defaults_to_production_sixty_seconds() {
        assert_eq!(parse_slice_ms(None), Duration::from_secs(60));
    }

    #[test]
    fn slice_override_accepts_milliseconds() {
        assert_eq!(parse_slice_ms(Some("5000")), Duration::from_secs(5));
        assert_eq!(parse_slice_ms(Some("250")), Duration::from_millis(250));
    }

    #[test]
    fn slice_override_rejects_garbage_and_zero() {
        assert_eq!(parse_slice_ms(Some("garbage")), Duration::from_secs(60));
        assert_eq!(parse_slice_ms(Some("")), Duration::from_secs(60));
        // A zero slice would busy-loop the scheduler thread.
        assert_eq!(parse_slice_ms(Some("0")), Duration::from_secs(60));
    }

    fn riyadh() -> tawadu_core::locations::Location {
        find_city("Riyadh").expect("Riyadh in the city DB")
    }

    fn pre_alert(at: DateTime<Utc>) -> scheduler::Event {
        scheduler::Event {
            kind: EventKind::PreAlert,
            label: "Fajr",
            at,
        }
    }

    #[test]
    fn pre_alert_body_shows_the_prayer_time_not_the_alert_time() {
        let settings = Settings::default(); // pre_alert_minutes = 10
        let event = pre_alert("2026-11-19T01:41:00Z".parse().unwrap());
        let (_, body) = announce(&event, &riyadh(), &settings, event.at).expect("announced");
        // The prayer is 01:51Z = 04:51 Riyadh; the alert itself is 04:41.
        assert_eq!(body, "Fajr at 04:51 · Riyadh");
    }

    #[test]
    fn pre_alert_headline_floors_the_live_remaining_minutes() {
        let settings = Settings::default();
        let event = pre_alert("2026-11-19T01:41:00Z".parse().unwrap());
        // Fired 90 s late: 8 min 30 s remain, not the static "10 min".
        let now = event.at + chrono::Duration::seconds(90);
        let (title, _) = announce(&event, &riyadh(), &settings, now).expect("announced");
        assert_eq!(title, "Fajr in 8 min");
    }

    #[test]
    fn pre_alert_headline_says_now_when_under_a_minute_remains() {
        let settings = Settings::default();
        let event = pre_alert("2026-11-19T01:41:00Z".parse().unwrap());
        let now = event.at + chrono::Duration::seconds(9 * 60 + 30);
        let (title, _) = announce(&event, &riyadh(), &settings, now).expect("announced");
        assert_eq!(title, "Fajr now");
    }

    #[test]
    fn athan_copy_names_the_prayer_and_its_time() {
        let settings = Settings::default();
        let event = scheduler::Event {
            kind: EventKind::Prayer,
            label: "Fajr",
            at: "2026-11-19T01:51:00Z".parse().unwrap(),
        };
        let (title, body) = announce(&event, &riyadh(), &settings, event.at).expect("announced");
        assert_eq!(title, "Athan · Fajr");
        assert_eq!(body, "It is time for Fajr prayer · 04:51");
    }

    fn quiet_settings() -> Settings {
        let mut settings = Settings::default();
        settings.notifications.quiet_hours_enabled = true;
        settings.notifications.quiet_from = "22:00".parse().unwrap();
        settings.notifications.quiet_to = "06:00".parse().unwrap();
        settings
    }

    #[test]
    fn quiet_hours_judged_at_event_time_suppress_a_late_fire() {
        // Event 23:00 Riyadh (inside quiet); fires 06:30 Riyadh (outside,
        // e.g. after sleep). Judging at fire time would wrongly notify.
        let event = pre_alert("2026-11-19T20:00:00Z".parse().unwrap());
        let fire = "2026-11-20T03:30:00Z".parse().unwrap();
        assert!(announce(&event, &riyadh(), &quiet_settings(), fire).is_none());
    }

    #[test]
    fn quiet_hours_judged_at_event_time_notify_when_the_event_was_outside() {
        // Event 07:00 Riyadh (outside quiet); fires 23:30 Riyadh (inside).
        // Judging at fire time would wrongly silence it.
        let event = pre_alert("2026-11-19T04:00:00Z".parse().unwrap());
        let fire = "2026-11-19T20:30:00Z".parse().unwrap();
        assert!(announce(&event, &riyadh(), &quiet_settings(), fire).is_some());
    }

    #[test]
    fn athan_is_never_suppressed_even_inside_quiet_hours() {
        let event = scheduler::Event {
            kind: EventKind::Prayer,
            label: "Fajr",
            at: "2026-11-19T20:00:00Z".parse().unwrap(), // 23:00 Riyadh
        };
        assert!(
            announce(&event, &riyadh(), &quiet_settings(), event.at).is_some(),
            "the athan is essential and bypasses quiet hours"
        );
    }

    #[test]
    fn debounce_is_not_due_before_the_delay() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        let t0 = Instant::now();
        d.touch(t0);
        assert!(!d.take_if_due(t0 + Duration::from_millis(799)));
    }

    #[test]
    fn debounce_is_due_after_the_delay_and_fires_once() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        let t0 = Instant::now();
        d.touch(t0);
        assert!(d.take_if_due(t0 + Duration::from_millis(800)));
        // Cleared: the next tick must not write again.
        assert!(!d.take_if_due(t0 + Duration::from_millis(2000)));
    }

    #[test]
    fn debounce_restarts_on_every_change() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        let t0 = Instant::now();
        d.touch(t0);
        d.touch(t0 + Duration::from_millis(600));
        // 800 ms after the FIRST touch, but only 200 after the last.
        assert!(!d.take_if_due(t0 + Duration::from_millis(800)));
        assert!(d.take_if_due(t0 + Duration::from_millis(1400)));
    }

    #[test]
    fn debounce_cancel_supersedes_pending_save() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        let t0 = Instant::now();
        d.touch(t0);
        d.cancel();
        assert!(!d.take_if_due(t0 + Duration::from_millis(5000)));
    }

    #[test]
    fn debounce_flush_forces_the_pending_write_once() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        d.touch(Instant::now());
        assert!(d.flush());
        assert!(!d.flush());
        assert!(!d.take_if_due(Instant::now() + Duration::from_secs(5)));
    }

    #[test]
    fn debounce_without_changes_never_fires() {
        let mut d = SaveDebounce::new(Duration::from_millis(800));
        assert!(!d.take_if_due(Instant::now() + Duration::from_secs(60)));
    }
}
