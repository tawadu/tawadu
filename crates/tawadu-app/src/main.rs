//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! With a subcommand (`tawadu times …`) it runs headless and exits; with none it
//! opens the window.
//!
//! Rust owns all state. The interface reports edits back through callbacks and
//! nothing in `.slint` persists anything by itself.

mod autostart;
mod cli;
mod gps;
mod restore;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use tawadu_core::athkar::{self, Context, ContextTimes, Corpus};
use tawadu_core::cities::{CityDb, CityHit};
use tawadu_core::hijri::{from_gregorian, HijriDate, TABLE_FIRST_YEAR, TABLE_LAST_YEAR};
use tawadu_core::locations::{find_city, Location, CITIES};
use tawadu_core::locations::{push_recent, timezone_list, validate_coordinates};
use tawadu_core::mosques::effective_iqamah;
use tawadu_core::notify::{self, NotificationKind};
use tawadu_core::prayer::{
    self, explain_strip, provenance, HighLatitudeRule, Madhab, Method, Prayer, PrayerTimes,
    Rounding, TimeSource,
};
use tawadu_core::scheduler::{self, EventKind, Timeline};
use tawadu_core::settings::{AudioSettings, EventVoice, Language, PlaybackMode, Settings, Theme};
use tawadu_core::storage::{LoadOutcome, Storage};

use tawadu_audio::edit::playable_ms;
use tawadu_audio::engine::{self, AudioEngine, PlayRequest, PlaybackPlan};
use tawadu_audio::library::PackLibrary;
use tawadu_audio::routing::{self, InstalledPack, ResolvedVoice};

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
/// Athkar UI state (Phase 4). RefCell for the same reason as debounce:
/// apply_home takes &AppState; the AppState Mutex serializes access.
/// City-picker UI state (Phase 5). RefCell like debounce/athkar: the
/// AppState Mutex serializes access; callbacks take &AppState.
struct PickerUi {
    pending_query: Option<String>,
    results: Vec<CityHit>,
    highlight: i32,
}

struct AthkarUi {
    ctx: Context,
    loaded_ctx: Option<Context>,
    manual: bool,
    day: String,
    done: std::collections::HashMap<(u32, u32), u32>,
    dirty: bool,
}

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
    /// The installed voice-pack library (shared with the scheduler thread).
    library: Option<Arc<Mutex<PackLibrary>>>,
    /// The background playback engine (fail-soft when no audio device).
    engine: Option<AudioEngine>,
    /// Bundled athkar corpus (Phase 4). None only if parsing fails (logged).
    corpus: Option<Corpus>,
    /// Athkar context/progress UI state.
    athkar: std::cell::RefCell<AthkarUi>,
    /// Bundled GeoNames city DB (Phase 5). None = fail-soft: the picker
    /// says so honestly and manual coordinates still work.
    city_db: Option<CityDb>,
    picker: std::cell::RefCell<PickerUi>,
    /// Mosque profiles (Phase 5 v1: name + offsets; storage v4 is the
    /// source of truth, this is the UI/scheduler cache).
    mosques: Vec<tawadu_core::mosques::MosqueProfile>,
    /// Two-step remove confirm: the id awaiting confirmation ("" = none).
    mosque_confirm: std::cell::RefCell<String>,
    /// Calendar view state (Phase 5): the viewed month + selected day.
    calendar: std::cell::RefCell<CalendarUi>,
}

struct CalendarUi {
    anchor: NaiveDate,
    selected: Option<NaiveDate>,
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
    palette.set_gold_is_accent(matches!(theme, Theme::Ramadan));
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
fn day(
    location: &Location,
    settings: &Settings,
    mosques: &[tawadu_core::mosques::MosqueProfile],
) -> (PrayerTimes, Timeline) {
    let date = location.local(now()).date_naive();
    // Phase 5: the ACTIVE mosque profile binds the scheduler offsets using
    // the app's single location; no active profile -> legacy settings.iqamah
    // (effective_iqamah never silently reassigns — core-tested).
    let iqamah = effective_iqamah(settings, mosques);

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
    let (times, timeline) = day(&location, &state.settings, &state.mosques);
    let next = next_prayer(&times, &location, &state.settings);
    let now = now();

    let display_name = if location.name.is_empty() {
        format!(
            "{:.4}, {:.4}",
            location.coordinates.latitude, location.coordinates.longitude
        )
    } else {
        location.name.clone()
    };
    ui.set_location_name(display_name.into());
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
            ui.set_next_name("\u{2014}".into());
            ui.set_next_name_ar("\u{2014}".into());
            ui.set_countdown("\u{2014}".into());
        }
    }

    let iqamah_text = timeline
        .events
        .iter()
        .find(|event| event.kind == EventKind::Iqamah && event.at > now)
        .map(|event| format!("{} · {}", event.label, hhmm(&location, event.at)))
        .unwrap_or_else(|| "Set your mosque's iqamah offsets".to_string());
    ui.set_next_iqamah(iqamah_text.into());
    // Phase 5: active-mosque caption (Quill home-caption, Rust-composed).
    let active = state
        .settings
        .active_mosque
        .as_ref()
        .and_then(|id| state.mosques.iter().find(|m| &m.id == id));
    let (caption, caption_ar) = match active {
        Some(m) => {
            // F4: activation is name-gated — the active profile always has
            // a real user-given name; no fabricated fallback.
            let name = m.name.as_str();
            let summary = m.offsets_summary();
            let cap = if summary.is_empty() {
                name.to_string()
            } else {
                format!("{name} · {summary}")
            };
            (cap, m.name_ar.clone())
        }
        None => (String::new(), String::new()),
    };
    ui.set_next_iqamah_caption(caption.into());
    ui.set_next_iqamah_caption_ar(caption_ar.into());

    // Phase 4: the real athkar engine — context-aware card, honest progress.
    apply_athkar(ui, state, &location, &times);

    let order = [
        Prayer::Fajr,
        Prayer::Sunrise,
        Prayer::Dhuhr,
        Prayer::Asr,
        Prayer::Maghrib,
        Prayer::Isha,
    ];
    let next_name = next.map(|(prayer, _)| prayer);
    // Phase 5: per-event provenance — estimated / undefined markers and the
    // explanation strip come from the ENGINE, never UI-invented facts.
    let prov = provenance(
        location.local(now).date_naive(),
        location.coordinates,
        &state.settings.config,
    );
    let source_of = |prayer: Prayer| match prayer {
        Prayer::Fajr => prov.fajr,
        Prayer::Sunrise => prov.sunrise,
        Prayer::Dhuhr => prov.dhuhr,
        Prayer::Asr => prov.asr,
        Prayer::Maghrib => prov.maghrib,
        Prayer::Isha => prov.isha,
        Prayer::Qiyam => TimeSource::Sun,
    };
    let rows: Vec<Row> = order
        .into_iter()
        .map(|prayer| {
            let instant = times.time(prayer);
            let time = instant
                .map(|i| hhmm(&location, i))
                .unwrap_or_else(|| "\u{2014}".to_string());
            let src = source_of(prayer);
            row(
                prayer.name_en(),
                prayer.name_ar(),
                time,
                Some(prayer) == next_name,
                prayer.is_prayer(),
                src == TimeSource::Rule,
                instant.is_none(),
            )
        })
        .collect();
    ui.set_rows(ModelRc::new(VecModel::from(rows)));

    // Explanation strip (Prism §5.3): polar state or any estimated event.
    // Facts the engine cannot supply are OMITTED, never filled in.
    let rule_label = match state.settings.config.high_latitude_rule {
        HighLatitudeRule::MiddleOfTheNight => "Middle of the night",
        HighLatitudeRule::SeventhOfTheNight => "Seventh of the night",
        HighLatitudeRule::TwilightAngle => "Twilight angle",
    };
    // Sentinel F1: the strip is composed PER EVENT by the core from the
    // provenance (engine-bound, unit-tested for both mixed exit-gate cases)
    // — never a template that names events the engine contradicts.
    let moonsighting_substituted = state.settings.config.method == Method::MoonsightingCommittee
        && location.coordinates.latitude.abs() >= 55.0;
    let (explain_visible, explain_title, explain_body) =
        match explain_strip(&location.name, rule_label, &prov, moonsighting_substituted) {
            Some((t, b)) => (true, t, b),
            None => (false, String::new(), String::new()),
        };
    // Sentinel F2: the variance figure is engine-derived (max Fajr/Isha
    // spread across the supported methods here today) — never a constant.
    let variance_line = match prayer::method_spread_minutes(
        location.local(now).date_naive(),
        location.coordinates,
        &state.settings.config,
    ) {
        Some(1) => "Different sources can differ by about 1 minute. Fine-tune any prayer below."
            .to_string(),
        Some(n) => format!(
            "Different sources can differ by about {n} minutes. Fine-tune any prayer below."
        ),
        None => {
            "Different sources can give different times. Fine-tune any prayer below.".to_string()
        }
    };
    ui.set_variance_line(variance_line.into());
    ui.set_explain_visible(explain_visible);
    ui.set_explain_title(explain_title.into());
    ui.set_explain_body(explain_body.into());
    ui.set_next_none_visible(next.is_none());
}

fn row(
    name: &str,
    name_ar: &str,
    time: String,
    is_next: bool,
    is_prayer: bool,
    estimated: bool,
    is_none: bool,
) -> Row {
    Row {
        name: name.into(),
        name_ar: name_ar.into(),
        time: time.into(),
        is_next,
        is_prayer,
        estimated,
        is_none,
    }
}

/// Push the settings into the settings surface.
/// Reload day-scoped progress for the displayed context (honest reset on a
/// new day or context change — yesterday's reads are never today's counts).
fn athkar_reload(state: &AppState, a: &mut AthkarUi) {
    a.done = state
        .storage
        .as_ref()
        .and_then(|s| s.athkar_progress(a.ctx.key(), &a.day).ok())
        .unwrap_or_default();
    a.loaded_ctx = Some(a.ctx);
    a.dirty = true;
}

/// Advance the athkar state to a local moment — the refresh the 1 s home
/// tick drives (Sentinel F1): Auto must re-follow window boundaries and the
/// day rollover while idle, with no user interaction. A manual pick overrides
/// until the user taps Auto. A new day or context reloads the persisted
/// progress — yesterday's reads are never today's counts. Pure state update
/// (no UI, injectable instant) so the tick's refresh path is unit-testable.
fn athkar_tick(state: &AppState, location: &Location, times: &PrayerTimes, now: DateTime<Utc>) {
    if state.corpus.is_none() {
        return;
    }
    let local = location.local(now);
    let day = local.format("%Y-%m-%d").to_string();
    let mut a = state.athkar.borrow_mut();
    if !a.manual {
        if let (Some(f), Some(d), Some(asr), Some(i)) =
            (times.fajr, times.dhuhr, times.asr, times.isha)
        {
            let ct = ContextTimes {
                fajr: location.local(f).time(),
                dhuhr: location.local(d).time(),
                asr: location.local(asr).time(),
                isha: location.local(i).time(),
            };
            a.ctx = athkar::active_context(local.time(), &ct);
        }
    }
    if a.day != day || a.loaded_ctx != Some(a.ctx) {
        if let Some(prev) = a.loaded_ctx {
            if prev != a.ctx {
                log::info!("athkar context: {} -> {}", prev.key(), a.ctx.key());
            }
        }
        if !a.day.is_empty() && a.day != day {
            log::info!("athkar day rollover: {} -> {}", a.day, day);
        }
        a.day = day;
        athkar_reload(state, &mut a);
    }
}

/// Context-aware athkar card + view refresh. Driven by the 1 s home tick
/// (Sentinel F1) AND every user callback: Auto re-follows window boundaries
/// while idle, the day rollover reloads honest counts, and a tray-restored
/// window shows the real current context because the tick runs while hidden.
/// A manual pick overrides until the user taps Auto. Progress is always the
/// persisted truth.
fn apply_athkar(ui: &MainWindow, state: &AppState, location: &Location, times: &PrayerTimes) {
    let Some(corpus) = &state.corpus else {
        return;
    };
    athkar_tick(state, location, times, now());
    let mut a = state.athkar.borrow_mut();
    // Card: the first UNREAD item of the context (else the first item).
    let mut current: Option<&athkar::Item> = None;
    let mut first: Option<&athkar::Item> = None;
    for id in a.ctx.category_ids() {
        if let Some(cat) = corpus.category(*id) {
            for item in &cat.items {
                if first.is_none() {
                    first = Some(item);
                }
                let done = a.done.get(&(cat.id, item.id)).copied().unwrap_or(0);
                if done < item.count && current.is_none() {
                    current = Some(item);
                }
            }
        }
    }
    let (items_total, _) = athkar::context_totals(corpus, a.ctx);
    let (items_done, _) = athkar::context_done(corpus, a.ctx, &a.done);
    ui.set_athkar_label(a.ctx.label().into());
    ui.set_athkar_progress(format!("{items_done}/{items_total}").into());
    ui.set_athkar_auto(!a.manual);
    ui.set_athkar_text(
        current
            .or(first)
            .map(|i| i.arabic.clone())
            .unwrap_or_default()
            .into(),
    );
    if ui.get_page() == "athkar" && a.dirty {
        build_athkar_view(ui, corpus, &a);
        a.dirty = false;
    }
}

/// Rebuild the athkar view model from the corpus + persisted progress.
fn build_athkar_view(ui: &MainWindow, corpus: &Corpus, a: &AthkarUi) {
    let mut items = Vec::new();
    for id in a.ctx.category_ids() {
        if let Some(cat) = corpus.category(*id) {
            for item in &cat.items {
                let done = a.done.get(&(cat.id, item.id)).copied().unwrap_or(0);
                items.push(AthkarItemView {
                    category: cat.id as i32,
                    item_id: item.id as i32,
                    arabic: item.arabic.clone().into(),
                    translation: item.translation.clone().unwrap_or_default().into(),
                    transliteration: item.transliteration.clone().unwrap_or_default().into(),
                    reference: cat.reference(item.id).into(),
                    counter: format!("{done}/{}", item.count).into(),
                    done: done as i32,
                    count: item.count as i32,
                    complete: done >= item.count,
                });
            }
        }
    }
    ui.set_athkar_items(ModelRc::new(VecModel::from(items)));
    let chips: Vec<ContextChip> = Context::ALL
        .iter()
        .map(|c| ContextChip {
            key: c.key().into(),
            label: c.label().into(),
            active: *c == a.ctx,
        })
        .collect();
    ui.set_athkar_contexts(ModelRc::new(VecModel::from(chips)));
    ui.set_athkar_context_key(a.ctx.key().into());
}

/// Locate the bundled derived city DB: next to the exe (installed layout,
/// Phase 6 revisits) or the workspace assets dir in dev. Opening is cheap —
/// indexes are PRE-BUILT by scripts/build_city_db.py (the off-UI-thread
/// index condition is satisfied at build time, not startup).
fn city_db_path() -> Option<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("assets/cities.db"), dir.join("cities.db")] {
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
    }
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/cities.db");
    if dev.exists() {
        return Some(dev);
    }
    None
}

fn recents_model(state: &AppState) -> ModelRc<CityResult> {
    let rows: Vec<CityResult> = state
        .settings
        .recent_cities
        .iter()
        .enumerate()
        .map(|(i, r)| CityResult {
            id: i.to_string().into(),
            name: if r.name.is_empty() {
                format!("{:.4}, {:.4}", r.latitude, r.longitude)
            } else {
                r.name.clone()
            }
            .into(),
            name_ar: r.name_ar.clone().into(),
            country: String::new().into(),
            region_line: r.timezone.clone().into(),
            timezone: r.timezone.clone().into(),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Run a picker search: prefix (+substring fallback) over the bundled DB,
/// population-ranked, capped 50 with the honest cap caption; duplicate
/// names get the disambiguation caption; every row carries country +
/// admin1 · IANA zone (Rust-composed runs, never Slint arithmetic).
fn run_picker_search(ui: &MainWindow, state: &AppState, query: &str) {
    let Some(db) = &state.city_db else {
        ui.set_picker_db_available(false);
        // N2: Quill phase5-error-db-missing-title/body verbatim.
        ui.set_picker_message("City database not found".into());
        ui.set_picker_message_body(
            "The app could not read its bundled city database. Reinstall the app to restore it."
                .into(),
        );
        return;
    };
    ui.set_picker_db_available(true);
    // N2: a DB ERROR is not "no results" — honest generic error copy.
    let hits = match db.search(query, 50) {
        Ok(h) => h,
        Err(e) => {
            log::error!("city search failed: {e}");
            ui.set_picker_message("Something went wrong".into());
            ui.set_picker_message_body(
                "Try again; if it keeps happening, reinstall the app.".into(),
            );
            return;
        }
    };
    let disamb = if hits.len() > 1 {
        let name = hits[0].name.to_lowercase();
        let n = hits
            .iter()
            .filter(|h| h.name.to_lowercase() == name)
            .count();
        if n > 1 {
            format!("{n} places share this name — choose by region and country.")
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let rows: Vec<CityResult> = hits
        .iter()
        .map(|h| CityResult {
            id: h.geonameid.to_string().into(),
            name: h.name.clone().into(),
            name_ar: h.name_ar.clone().into(),
            country: h.country.clone().into(),
            region_line: format!("{} · {}", h.admin1, h.timezone.name()).into(),
            timezone: h.timezone.name().into(),
        })
        .collect();
    let capped = hits.len() >= 50;
    let empty = hits.is_empty();
    let mut p = state.picker.borrow_mut();
    p.results = hits;
    p.highlight = if empty { -1 } else { 0 };
    ui.set_highlighted_result(p.highlight);
    drop(p);
    ui.set_picker_results(ModelRc::new(VecModel::from(rows)));
    ui.set_city_cap_visible(capped);
    ui.set_city_disambiguation(disamb.into());
    if empty && !query.trim().is_empty() {
        ui.set_picker_message(format!("No matches for “{}”.", query.trim()).into());
        ui.set_picker_message_body(
            "Try the Arabic name, enter coordinates manually, or use your location.".into(),
        );
    } else {
        ui.set_picker_message("".into());
        ui.set_picker_message_body("".into());
    }
}

/// Apply a chosen place: settings + recents ride the debounced save path;
/// the scheduler wakes; the status line confirms (Quill selected-message).
fn apply_location(ui: &MainWindow, state: &mut AppState, loc: tawadu_core::locations::Location) {
    let stored = tawadu_core::settings::StoredLocation::from_location(&loc);
    let label = if loc.name.is_empty() {
        format!(
            "{:.4}, {:.4}",
            loc.coordinates.latitude, loc.coordinates.longitude
        )
    } else {
        loc.name.clone()
    };
    state.settings.location = stored.clone();
    push_recent(&mut state.settings.recent_cities, stored);
    state.status = format!("Location set to {label}.");
    state.changed();
    if let Some(wake) = &state.wake {
        let _ = wake.send(());
    }
    apply_home(ui, state);
    apply_settings(ui, state);
}

/// F4: Add creates an in-memory DRAFT only — nothing touches storage until
/// the user gives a name (create-on-first-name). Dismissing an unnamed
/// draft discards it; a named profile is never discarded by this path.
fn add_mosque_draft(state: &mut AppState) -> String {
    let id = format!("m{}", now().timestamp_millis());
    state
        .mosques
        .push(tawadu_core::mosques::MosqueProfile::new(id.clone()));
    id
}

fn discard_unnamed_mosque(state: &mut AppState, id: &str) {
    let unnamed = state
        .mosques
        .iter()
        .find(|m| m.id == id)
        .map(|m| !m.is_named())
        .unwrap_or(false);
    if !unnamed {
        return;
    }
    if let Some(storage) = &state.storage {
        // No-op if the draft was never saved; also cleans a profile whose
        // name was later emptied — unnamed never stays persisted.
        let _ = storage.delete_mosque(id);
    }
    state.mosques.retain(|m| m.id != id);
    if state.settings.active_mosque.as_deref() == Some(id) {
        state.settings.active_mosque = None;
    }
}

fn mosques_model(state: &AppState) -> ModelRc<MosqueProfile> {
    let rows: Vec<MosqueProfile> = state
        .mosques
        .iter()
        .map(|m| MosqueProfile {
            id: m.id.clone().into(),
            name: m.name.clone().into(),
            name_ar: m.name_ar.clone().into(),
            offsets_summary: m.offsets_summary().into(),
            is_active: state.settings.active_mosque.as_deref() == Some(m.id.as_str()),
            fajr: m.offsets.fajr as i32,
            dhuhr: m.offsets.dhuhr as i32,
            asr: m.offsets.asr as i32,
            maghrib: m.offsets.maghrib as i32,
            isha: m.offsets.isha as i32,
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn refresh_mosques_ui(ui: &MainWindow, state: &AppState) {
    ui.set_mosques(mosques_model(state));
    ui.set_mosque_remove_confirm(state.mosque_confirm.borrow().clone().into());
}

/// Render the calendar month: dual numbering (both engine values), today
/// anchor, engine-bound navigation (Umm al-Qura table 1343–1500 AH — never
/// an extrapolated guess), selected-day times with the same —/estimated
/// honesty rules, and the Ramadan banner state (Prism §6, Quill §4.4).
fn build_calendar(ui: &MainWindow, state: &AppState, location: &tawadu_core::locations::Location) {
    let today = location.local(now()).date_naive();
    let cal = state.calendar.borrow();
    let first = cal.anchor.with_day(1).unwrap_or(today);
    let start = first - chrono::Duration::days(i64::from(first.weekday().num_days_from_monday()));
    let mut days = Vec::with_capacity(42);
    for i in 0..42 {
        let d = start + chrono::Duration::days(i);
        let h = from_gregorian(d);
        days.push(CalendarDay {
            iso: d.format("%Y-%m-%d").to_string().into(),
            gregorian: d.day() as i32,
            hijri: h.map(|x| i32::from(x.day)).unwrap_or(0),
            in_month: d.month() == first.month() && d.year() == first.year(),
            is_today: d == today,
            is_ramadan: h.map(|x| x.is_ramadan()).unwrap_or(false),
        });
    }
    ui.set_calendar_days(ModelRc::new(VecModel::from(days)));
    ui.set_calendar_month_label(first.format("%B %Y").to_string().into());
    let mid_h = from_gregorian(first + chrono::Duration::days(14));
    ui.set_calendar_month_label_ar(
        mid_h
            .map(|h| format!("{} {}", h.month_name_ar(), h.year))
            .unwrap_or_default()
            .into(),
    );

    // Engine bounds: the published table's first/last Gregorian days.
    let bound_first = HijriDate::new(TABLE_FIRST_YEAR, 1, 1)
        .and_then(|h| h.to_gregorian())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1924, 7, 31).unwrap());
    let bound_last = HijriDate::new(TABLE_LAST_YEAR, 12, 1)
        .map(|h| {
            let len = i64::from(h.month_len());
            h.to_gregorian()
                .map(|g| g + chrono::Duration::days(len - 1))
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2077, 12, 31).unwrap())
        })
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2077, 12, 31).unwrap());
    let prev_enabled = first > bound_first.with_day(1).unwrap_or(bound_first);
    let next_enabled = first < bound_last.with_day(1).unwrap_or(bound_last);
    ui.set_calendar_prev_enabled(prev_enabled);
    ui.set_calendar_next_enabled(next_enabled);
    let note = if !next_enabled {
        format!("Dates unavailable beyond {}", bound_last.format("%B %Y"))
    } else if !prev_enabled {
        format!("Dates unavailable before {}", bound_first.format("%B %Y"))
    } else {
        String::new()
    };
    ui.set_hijri_range_note(note.into());

    // Selected day: five times, same honesty rules as the Today panel.
    let sel = cal.selected.unwrap_or(today);
    let times = prayer::compute(sel, location.coordinates, &state.settings.config);
    let prov = provenance(sel, location.coordinates, &state.settings.config);
    let strip: Vec<DayTime> = [
        (Prayer::Fajr, prov.fajr),
        (Prayer::Dhuhr, prov.dhuhr),
        (Prayer::Asr, prov.asr),
        (Prayer::Maghrib, prov.maghrib),
        (Prayer::Isha, prov.isha),
    ]
    .iter()
    .map(|(p, src)| {
        let t = times.time(*p);
        DayTime {
            name: p.name_en().into(),
            name_ar: p.name_ar().into(),
            time: t
                .map(|i| hhmm(location, i))
                .unwrap_or_else(|| "\u{2014}".to_string())
                .into(),
            estimated: *src == TimeSource::Rule,
            is_none: t.is_none(),
        }
    })
    .collect();
    ui.set_selected_day_times(ModelRc::new(VecModel::from(strip)));
    ui.set_selected_day_label(format!("Times for this day · {}", sel.format("%a %d %b %Y")).into());

    // Ramadan banner: offer during Ramadan when off; gentle end notice.
    let cur_ramadan = from_gregorian(today)
        .map(|h| h.is_ramadan())
        .unwrap_or(false);
    let banner = if cur_ramadan && !state.settings.ramadan_mode {
        1
    } else if state.settings.ramadan_mode && !cur_ramadan {
        2
    } else {
        0
    };
    ui.set_ramadan_month(cur_ramadan);
    ui.set_ramadan_banner(banner);
    ui.set_ramadan_mode(state.settings.ramadan_mode);
    ui.set_calendar_primary(
        if state.settings.calendar_hijri_primary {
            "hijri"
        } else {
            "greg"
        }
        .into(),
    );
}

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
    ui.set_settings_location_tz(state.settings.location.timezone.clone().into());
    refresh_mosques_ui(ui, state);

    // Phase 5 §5.1: rule rows with Quill's verified descriptions (verbatim).
    ui.set_high_lat_rules(ModelRc::new(VecModel::from(vec![
        RuleOption {
            key: "middle_of_the_night".into(),
            label: "Middle of the night".into(),
            label_ar: "منتصف الليل".into(),
            desc: "Sets Fajr half the night before sunrise and Isha half the night after sunset, when the sun never reaches the twilight angle.".into(),
            desc_ar: "يقدّر الفجر قبل الشروق بنصف الليل، والعشاء بعد الغروب بنصف الليل، عندما لا تصل الشمس إلى زاوية الشفق.".into(),
        },
        RuleOption {
            key: "seventh_of_the_night".into(),
            label: "Seventh of the night".into(),
            label_ar: "سُبع الليل".into(),
            desc: "Sets Fajr one seventh of the night before sunrise and Isha one seventh of the night after sunset, when the sun never reaches the twilight angle.".into(),
            desc_ar: "يقدّر الفجر قبل الشروق بسُبع الليل، والعشاء بعد الغروب بسُبع الليل، عندما لا تصل الشمس إلى زاوية الشفق.".into(),
        },
        RuleOption {
            key: "twilight_angle".into(),
            label: "Twilight angle".into(),
            label_ar: "زاوية الشفق".into(),
            desc: "Places Fajr and Isha at a fraction of the night set by the method's twilight angles, when the sun never reaches those angles.".into(),
            desc_ar: "يضع الفجر والعشاء عند جزء من الليل تحدده زوايا الشفق للطريقة المختارة، عندما لا تصل الشمس إلى تلك الزوايا.".into(),
        },
    ])));
    ui.set_selected_high_lat_rule(
        match state.settings.config.high_latitude_rule {
            HighLatitudeRule::MiddleOfTheNight => "middle_of_the_night",
            HighLatitudeRule::SeventhOfTheNight => "seventh_of_the_night",
            HighLatitudeRule::TwilightAngle => "twilight_angle",
        }
        .into(),
    );

    // Phase 5 §5.4: fine-tune steppers + the never-invisible adjustment summary.
    let adj = &state.settings.config.adjustments;
    ui.set_fine_tune(ModelRc::new(VecModel::from(vec![
        adj.fajr as i32,
        adj.dhuhr as i32,
        adj.asr as i32,
        adj.maghrib as i32,
        adj.isha as i32,
    ])));
    let parts: Vec<String> = [
        ("Fajr", adj.fajr),
        ("Dhuhr", adj.dhuhr),
        ("Asr", adj.asr),
        ("Maghrib", adj.maghrib),
        ("Isha", adj.isha),
    ]
    .iter()
    .filter(|(_, v)| *v != 0)
    .map(|(n, v)| format!("{n} {v:+} min"))
    .collect();
    ui.set_fine_tune_summary(parts.join(" · ").into());

    // "How computed" expander: engine values only (Prism §5.4).
    let method_label = match state.settings.config.method {
        Method::MuslimWorldLeague => "Muslim World League",
        Method::Egyptian => "Egyptian",
        Method::Karachi => "Karachi",
        Method::UmmAlQura => "Umm al-Qura",
        Method::Dubai => "Dubai",
        Method::MoonsightingCommittee => "Moonsighting Committee",
        Method::NorthAmerica => "North America",
        Method::Kuwait => "Kuwait",
        Method::Qatar => "Qatar",
        Method::Singapore => "Singapore",
        Method::Tehran => "Tehran",
        Method::Turkey => "Turkey",
        Method::Custom { .. } => "Custom angles",
    };
    let rule_label = match state.settings.config.high_latitude_rule {
        HighLatitudeRule::MiddleOfTheNight => "Middle of the night",
        HighLatitudeRule::SeventhOfTheNight => "Seventh of the night",
        HighLatitudeRule::TwilightAngle => "Twilight angle",
    };
    let rounding_label = match state.settings.config.rounding {
        Rounding::Nearest => "nearest minute",
        Rounding::Up => "up to the minute",
        Rounding::None => "exact",
    };
    let mut summary = format!(
        "Method: {method_label} · Madhab: {} · High-latitude rule: {rule_label} · Rounding: {rounding_label}",
        match state.settings.config.madhab { Madhab::Shafi => "Shafi", Madhab::Hanafi => "Hanafi" }
    );
    if let Some(interval) = state.settings.config.isha_interval {
        summary.push_str(&format!(" · Isha interval: {interval} min after Maghrib"));
    }
    if !parts.is_empty() {
        summary.push_str(&format!(" · Adjustments: {}", parts.join(", ")));
    }
    ui.set_computed_summary(summary.into());
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
    apply_audio(ui, state);
}

/// Push the audio state into the AUDIO band. Lock order: AppState (held by
/// the caller) THEN library — the same order the scheduler uses.
fn apply_audio(ui: &MainWindow, state: &AppState) {
    let Some(library) = &state.library else {
        return;
    };
    let lib = library.lock().unwrap();
    let audio = &state.settings.audio;
    ui.set_audio_packs(ModelRc::new(VecModel::from(pack_rows(
        lib.packs(),
        audio.default_pack.as_deref(),
    ))));
    ui.set_audio_voices(ModelRc::new(VecModel::from(voice_options(
        lib.packs(),
        false,
    ))));
    ui.set_chime_voices(ModelRc::new(VecModel::from(voice_options(
        lib.packs(),
        true,
    ))));
    ui.set_event_voices(ModelRc::new(VecModel::from(event_voice_rows(
        lib.packs(),
        audio,
    ))));
    ui.set_audio_volume((state.settings.athan_volume * 100.0).round() as i32);
    ui.set_athan_stop_after_s(audio.athan_stop_after_s as i32);
    if audio.athan_stop_after_s == 0 {
        // The stepper's zero label carries the "no trim" copy.
        ui.set_trim_unit(SharedString::new());
        ui.set_trim_unit_ar(SharedString::new());
    } else {
        let (unit, unit_ar) = trim_unit_words(audio.athan_stop_after_s);
        ui.set_trim_unit(unit.into());
        ui.set_trim_unit_ar(unit_ar.into());
    }
}

/// The credits page: one card per installed pack with the EXACT licence
/// file text, plus the font and library entries from docs/attribution.md.
fn apply_credits(ui: &MainWindow, state: &AppState) {
    let packs: Vec<CreditPack> = match &state.library {
        Some(library) => {
            let lib = library.lock().unwrap();
            lib.packs()
                .iter()
                .map(|pack| {
                    let dir = lib.pack_dir(&pack.id).join("licenses");
                    let mut license_text = String::new();
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        let mut files: Vec<_> = entries
                            .flatten()
                            .map(|entry| entry.path())
                            .filter(|path| path.is_file())
                            .collect();
                        files.sort();
                        for path in files {
                            if let Ok(text) = std::fs::read_to_string(&path) {
                                if !license_text.is_empty() {
                                    license_text.push_str("\n\n");
                                }
                                license_text.push_str(text.trim_end());
                            }
                        }
                    }
                    CreditPack {
                        name: pack.manifest.name.clone().into(),
                        name_ar: pack.manifest.name_ar.clone().unwrap_or_default().into(),
                        reciter: pack.manifest.reciter.clone().into(),
                        source: pack.manifest.source_url.clone().into(),
                        license: pack.manifest.license.clone().into(),
                        attribution: pack.manifest.attribution.clone().into(),
                        license_text: license_text.into(),
                    }
                })
                .collect()
        }
        None => Vec::new(),
    };
    ui.set_credit_packs(ModelRc::new(VecModel::from(packs)));
    ui.set_credit_fonts(ModelRc::new(VecModel::from(vec![
        CreditRow {
            name: "IBM Plex Sans".into(),
            license: "SIL OFL 1.1".into(),
            role: "Latin UI".into(),
        },
        CreditRow {
            name: "IBM Plex Sans Arabic".into(),
            license: "SIL OFL 1.1".into(),
            role: "Arabic UI".into(),
        },
        CreditRow {
            name: "Amiri".into(),
            license: "SIL OFL 1.1".into(),
            role: "Scripture / athkar display".into(),
        },
    ])));
    ui.set_credit_libs(ModelRc::new(VecModel::from(vec![
        CreditRow {
            name: "auto-launch".into(),
            license: "MIT".into(),
            role: "Opt-in start-at-login".into(),
        },
        CreditRow {
            name: "windows-registry".into(),
            license: "MIT OR Apache-2.0".into(),
            role: "Windows Run-key detection".into(),
        },
        CreditRow {
            name: "tray-icon".into(),
            license: "MIT OR Apache-2.0".into(),
            role: "System tray icon".into(),
        },
        CreditRow {
            name: "muda".into(),
            license: "Apache-2.0 OR MIT".into(),
            role: "Tray/app menus".into(),
        },
        CreditRow {
            name: "Hisn al-Muslim · Islamic-Pro-azkar-API".into(),
            license: "MIT".into(),
            role: "Athkar text corpus (pinned d793023e)".into(),
        },
        CreditRow {
            name: "GeoNames Gazetteer · cities1000".into(),
            license: "CC BY 4.0".into(),
            role: "City database (derived; geonames.org, Unxos GmbH)".into(),
        },
        CreditRow {
            name: "ksni".into(),
            license: "Unlicense".into(),
            role: "Linux StatusNotifier tray".into(),
        },
        CreditRow {
            name: "zbus".into(),
            license: "MIT".into(),
            role: "SNI watcher probe (Linux)".into(),
        },
        CreditRow {
            name: "windows".into(),
            license: "MIT OR Apache-2.0".into(),
            role: "AUMID + restore event (Windows)".into(),
        },
        CreditRow {
            name: "single-instance".into(),
            license: "MIT".into(),
            role: "Single-instance lock".into(),
        },
        CreditRow {
            name: "rodio".into(),
            license: "MIT OR Apache-2.0".into(),
            role: "Audio playback".into(),
        },
        CreditRow {
            name: "symphonia".into(),
            license: "MPL-2.0".into(),
            role: "Audio decoding".into(),
        },
        CreditRow {
            name: "hound".into(),
            license: "Apache-2.0".into(),
            role: "WAV render cache".into(),
        },
        CreditRow {
            name: "zip".into(),
            license: "MIT".into(),
            role: ".tawadupack import".into(),
        },
        CreditRow {
            name: "sha2".into(),
            license: "MIT OR Apache-2.0".into(),
            role: "Checksum verification".into(),
        },
        CreditRow {
            name: "rfd".into(),
            license: "MIT".into(),
            role: "Native file pickers".into(),
        },
    ])));
    ui.set_code_license("MIT".into());
}

/// Import on a background thread: the picker AND the validation/probe/copy
/// never block the UI. Results return through the event loop.
fn spawn_import(state: Arc<Mutex<AppState>>, weak: slint::Weak<MainWindow>, folder: bool) {
    std::thread::spawn(move || {
        let picked = if folder {
            rfd::FileDialog::new().pick_folder()
        } else {
            rfd::FileDialog::new()
                .add_filter("Tawadu voice pack", &["tawadupack"])
                .pick_file()
        };
        let Some(path) = picked else {
            // Quill audio-import-cancelled.
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_audio_message("Import cancelled.".into());
                    ui.set_audio_message_ar("أُلغي الاستيراد.".into());
                    ui.set_audio_message_arg(SharedString::new());
                    ui.set_audio_message_ar_after(SharedString::new());
                }
            });
            return;
        };
        // Clone the library handle OUT of the state lock: the import's IO
        // and probing must not hold the AppState mutex.
        let library = state.lock().unwrap().library.clone();
        let result = library.map(|library| library.lock().unwrap().import(&path));
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.lock().unwrap();
            match result {
                Some(Ok(pack)) => {
                    state.status = format!("Added pack: {}", pack.manifest.name);
                    state.changed();
                    apply_audio(&ui, &state);
                    ui.set_status_message(state.status.clone().into());
                    // Quill audio-import-success, split around the name.
                    ui.set_audio_message(
                        format!("\u{201C}{}\u{201D} was added.", pack.manifest.name).into(),
                    );
                    ui.set_audio_message_ar("أُضيفت «".into());
                    ui.set_audio_message_arg(pack.manifest.name.clone().into());
                    ui.set_audio_message_ar_after("».".into());
                }
                Some(Err(error)) => {
                    // The validator's exact reason, Arabic pre-split so the
                    // offending path renders as its own run (Quill §2).
                    let message = error.message();
                    ui.set_audio_message(message.body_en.into());
                    ui.set_audio_message_ar(message.body_ar_before.into());
                    ui.set_audio_message_arg(message.arg.into());
                    ui.set_audio_message_ar_after(message.body_ar_after.into());
                }
                None => {}
            }
        });
    });
}

/// The serde key for a method, for highlighting the current row.
fn serde_key(method: &Method) -> String {
    serde_json::to_value(method)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

// ---- audio UI row builders (pure; the Slint surface only renders these) ----

/// Quill's `audio-trim-seconds` plural (CLDR Arabic categories), as two
/// separate runs — the numeral stays in the stepper, never in the word.
fn trim_unit_words(n: u32) -> (&'static str, &'static str) {
    let en = if n == 1 { "second" } else { "seconds" };
    let ar = match n {
        1 => "ثانية",
        2 => "ثانيتان",
        _ => {
            let hundred = n % 100;
            if (3..=10).contains(&hundred) {
                "ثوانٍ" // few
            } else if (11..=99).contains(&hundred) {
                "ثانيةً" // many
            } else {
                "ثانية" // zero / other
            }
        }
    };
    (en, ar)
}

/// Pack cards: probed duration (unknown → "" → غير متاح), the selected
/// (default) pack, Arabic name only when the manifest actually carries one.
fn pack_rows(packs: &[InstalledPack], default_pack: Option<&str>) -> Vec<PackRow> {
    let selected = default_pack
        .filter(|id| packs.iter().any(|pack| pack.id == *id))
        .or_else(|| packs.first().map(|pack| pack.id.as_str()));
    packs
        .iter()
        .map(|pack| PackRow {
            id: pack.id.clone().into(),
            name: pack.manifest.name.clone().into(),
            name_ar: pack.manifest.name_ar.clone().unwrap_or_default().into(),
            reciter: pack.manifest.reciter.clone().into(),
            license: pack.manifest.license.clone().into(),
            duration: routing::pack_duration_ms(pack)
                .map(routing::format_duration_ms)
                .unwrap_or_default()
                .into(),
            is_bundled: pack.bundled,
            is_selected: selected == Some(pack.id.as_str()),
        })
        .collect()
}

/// The voice picker options: Default first, then every installed pack.
/// The built-in chime appears only in the chime picker. An Arabic label
/// never carries Latin text — absent name_ar means an empty Arabic run.
fn voice_options(packs: &[InstalledPack], include_builtin_chime: bool) -> Vec<VoiceOption> {
    let mut options = vec![VoiceOption {
        id: SharedString::new(),
        label: "Default".into(),
        label_ar: "الافتراضي".into(),
        pack_name: SharedString::new(),
    }];
    if include_builtin_chime {
        options.push(VoiceOption {
            id: routing::BUILTIN_CHIME_ID.into(),
            label: "Alert chime".into(),
            label_ar: "نغمة التنبيه".into(),
            pack_name: SharedString::new(),
        });
    }
    for pack in packs {
        options.push(VoiceOption {
            id: pack.id.clone().into(),
            label: pack.manifest.name.clone().into(),
            label_ar: pack.manifest.name_ar.clone().unwrap_or_default().into(),
            pack_name: pack.manifest.reciter.clone().into(),
        });
    }
    options
}

/// One row per voiced event: the explicit choice (or Default), the
/// effective mode, whether an override exists, and whether any voice can
/// actually sound (the honest empty state).
fn event_voice_rows(packs: &[InstalledPack], audio: &AudioSettings) -> Vec<EventVoiceRow> {
    const EVENTS: [(&str, &str, &str); 4] = [
        (routing::EVENT_ATHAN, "Adhan", "الأذان"),
        (routing::EVENT_IQAMAH, "Iqamah", "الإقامة"),
        (routing::EVENT_ATHKAR, "Athkar", "الأذكار"),
        (routing::EVENT_CHIME, "Alert chime", "نغمة التنبيه"),
    ];
    EVENTS
        .iter()
        .map(|(kind, label, label_ar)| {
            let choice = audio
                .event_voices
                .get(*kind)
                .and_then(|voice| voice.pack.clone());
            let resolved = routing::resolve_voice(kind, audio, packs);
            // The pill names the explicit choice; a choice whose pack was
            // removed reports the fallback honestly (Default), never silent.
            let mut voice_id = choice.clone().unwrap_or_default();
            let (voice_label, voice_label_ar) = match choice.as_deref() {
                Some(routing::BUILTIN_CHIME_ID) => {
                    ("Alert chime".to_string(), "نغمة التنبيه".to_string())
                }
                Some(id) => match packs.iter().find(|pack| pack.id == id) {
                    Some(pack) => (
                        pack.manifest.name.clone(),
                        pack.manifest.name_ar.clone().unwrap_or_default(),
                    ),
                    None => {
                        voice_id = String::new();
                        ("Default".to_string(), "الافتراضي".to_string())
                    }
                },
                None => ("Default".to_string(), "الافتراضي".to_string()),
            };
            let mode = match &resolved {
                Some(ResolvedVoice::Pack { mode, .. }) => *mode,
                _ => PlaybackMode::Sequential,
            };
            EventVoiceRow {
                kind: (*kind).into(),
                label: (*label).into(),
                label_ar: (*label_ar).into(),
                voice_id: voice_id.into(),
                voice_label: voice_label.into(),
                voice_label_ar: voice_label_ar.into(),
                mode: match mode {
                    PlaybackMode::Sequential => "sequential",
                    PlaybackMode::Shuffle => "shuffle",
                }
                .into(),
                has_override: routing::has_mode_override(kind, audio),
                has_voice: resolved.is_some(),
            }
        })
        .collect()
}

/// How long a preview will play, when every clip's probed duration is
/// known — `None` means no auto-stop guess is possible (honest).
fn preview_duration_ms(plan: &PlaybackPlan, packs: &[InstalledPack]) -> Option<u64> {
    match plan {
        PlaybackPlan::Chime => Some((tawadu_audio::chime::CHIME_SECONDS * 1000.0) as u64),
        PlaybackPlan::Clips(clips) => {
            let mut total = 0u64;
            for clip in clips {
                let pack = packs.iter().find(|pack| pack.id == clip.pack_id)?;
                let duration = pack.durations_ms.get(&clip.rel).copied().flatten()?;
                total = total.checked_add(playable_ms(duration, &clip.edit, clip.stop_after_ms))?;
            }
            Some(total)
        }
    }
}

/// Drop every settings reference to a removed pack (Sentinel N3): a
/// re-import landing on the same slug id must not resurrect a stale
/// assignment. Returns true when anything referenced the pack — the
/// remove handler's "assigned events fall back" status condition.
fn forget_removed_pack(audio: &mut AudioSettings, id: &str) -> bool {
    let mut affected = audio.default_pack.as_deref() == Some(id);
    if affected {
        audio.default_pack = None;
    }
    audio.event_voices.retain(|_, voice| {
        let references = voice.pack.as_deref() == Some(id);
        affected |= references;
        !references
    });
    affected
}

/// The remove handler's status line (Prism §3: never silent).
fn remove_status(name: &str, affected: bool) -> String {
    if affected {
        format!("Removed {name}. Assigned events fall back to the default voice.")
    } else {
        format!("Removed {name}.")
    }
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

/// Which voice event an announcement maps to. The pre-alert is the short
/// chime moment; the athan/iqamah/athkar events voice from their packs.
fn voice_event(kind: EventKind) -> &'static str {
    match kind {
        EventKind::PreAlert => routing::EVENT_CHIME,
        EventKind::Prayer => routing::EVENT_ATHAN,
        EventKind::Iqamah => routing::EVENT_IQAMAH,
        EventKind::AthkarWindow => routing::EVENT_ATHKAR,
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
    //
    // SEMANTICS (intentional — do not "fix"): PreAlert's event.at is the
    // ALERT instant (prayer time minus pre_alert_minutes), so quiet hours
    // bind to the alert instant, not the prayer instant. A pre-alert that
    // falls inside quiet hours is suppressed even when the prayer itself
    // falls outside them; that is the user's quiet window speaking.
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
            // Ramadan mode (Phase 5 §6.2): renamed copy ONLY — same
            // pre-alert machinery, same event kinds, no new notifications.
            let title = if settings.ramadan_mode && event.label == "Fajr" {
                format!("Suhur reminder · {title}")
            } else if settings.ramadan_mode && event.label == "Maghrib" {
                format!("Iftar reminder · {title}")
            } else {
                title
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
                let (location, settings, mosques) = {
                    let guard = state.lock().unwrap();
                    (
                        guard.location(),
                        guard.settings.clone(),
                        guard.mosques.clone(),
                    )
                };
                let (_, timeline) = day(&location, &settings, &mosques);
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
                // Ledger filter: skip already-announced EXACT identities
                // (kind, prayer, scheduled instant). Read errors fail open
                // (false -> fire): a broken ledger must never silence.
                let fireable = scheduler::fireable(due, |event| {
                    let guard = state.lock().unwrap();
                    let fired = guard
                        .storage
                        .as_ref()
                        .and_then(|storage| {
                            storage
                                .has_fired(event.kind.key(), event.label, event.at)
                                .ok()
                        })
                        .unwrap_or(false);
                    if fired {
                        log::info!("skipping already-fired {} · {}", event.kind.key(), event.at);
                    }
                    fired
                });
                for event in fireable {
                    if let Some((title, body)) = announce(&event, &location, &settings, now()) {
                        // Transactional order: persist the fire BEFORE the
                        // announcement, so a crash after announcing can never
                        // re-announce on restart. Record event.at — the
                        // SCHEDULED instant, NEVER the fire wall-time: a late
                        // fire recorded as wall-time would widen the dedupe
                        // identity and suppress the true event later. Write
                        // failure is logged and fails open (alert still goes).
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
                        // Audio rides the SAME should_notify gate as the
                        // banner: announce() already judged quiet hours at
                        // the EVENT's local time and applied the
                        // athan-essential exemption. There is no separate
                        // audio gate to drift out of sync.
                        {
                            let guard = state.lock().unwrap();
                            if let (Some(engine), Some(library)) = (&guard.engine, &guard.library) {
                                let lib = library.lock().unwrap();
                                if let Some(plan) = engine::plan_for_event(
                                    voice_event(event.kind),
                                    &settings.audio,
                                    lib.packs(),
                                ) {
                                    engine.play(PlayRequest {
                                        plan,
                                        volume: settings.athan_volume,
                                        library_dir: lib.dir().to_path_buf(),
                                        cache_dir: lib.cache_root().to_path_buf(),
                                    });
                                }
                            }
                        }
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

    // ---- audio: library + engine ----
    // The device opens on the audio thread (fail-soft when absent); the
    // bundled floor materializes on a background thread because first-run
    // duration probing must never block the UI. Lock order everywhere:
    // AppState mutex, THEN library mutex.
    let (library, engine) = match Storage::default_data_dir() {
        Some(dir) => {
            let library = Arc::new(Mutex::new(PackLibrary::open(
                dir.join("voices"),
                dir.join("audio-cache"),
            )));
            (Some(library), Some(AudioEngine::spawn()))
        }
        None => {
            log::warn!("no data directory available; audio disabled");
            (None, None)
        }
    };

    let initial_mosques = storage
        .as_ref()
        .and_then(|s| s.load_mosques().ok())
        .unwrap_or_default();
    let state = Arc::new(Mutex::new(AppState {
        settings,
        storage,
        status: String::new(),
        wake: None,
        debounce: std::cell::RefCell::new(SaveDebounce::new(Duration::from_millis(800))),
        library,
        engine,
        corpus: match Corpus::bundled() {
            Ok(corpus) => Some(corpus),
            Err(e) => {
                log::error!("athkar corpus unavailable: {e}");
                None
            }
        },
        athkar: std::cell::RefCell::new(AthkarUi {
            ctx: Context::Morning,
            loaded_ctx: None,
            manual: false,
            day: String::new(),
            done: std::collections::HashMap::new(),
            dirty: true,
        }),
        city_db: match city_db_path().and_then(|p| CityDb::open(&p).ok()) {
            Some(db) => Some(db),
            None => {
                log::warn!("city database unavailable; picker falls back to manual coordinates");
                None
            }
        },
        picker: std::cell::RefCell::new(PickerUi {
            pending_query: None,
            results: Vec::new(),
            highlight: -1,
        }),
        mosques: initial_mosques,
        mosque_confirm: std::cell::RefCell::new(String::new()),
        calendar: std::cell::RefCell::new(CalendarUi {
            anchor: Utc::now().date_naive(),
            selected: None,
        }),
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
    ui.set_gps_available(cfg!(target_os = "windows"));
    apply_home(&ui, &state.lock().unwrap());
    apply_settings(&ui, &state.lock().unwrap());
    if let Some(message) = &warning {
        ui.set_status_message(message.clone().into());
    }

    // Materialize the bundled floor in the background, then refresh the
    // band when it lands: first-run duration probing never blocks the UI,
    // and the packs appear as soon as they are really installed.
    {
        let library = state.lock().unwrap().library.clone();
        if let Some(library) = library {
            let weak = ui.as_weak();
            let refresh_state = Arc::clone(&state);
            if let Err(error) = std::thread::Builder::new()
                .name("tawadu-audio-floor".into())
                .spawn(move || {
                    library.lock().unwrap().install_bundled();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            let state = refresh_state.lock().unwrap();
                            apply_audio(&ui, &state);
                        }
                    });
                })
            {
                log::warn!("could not spawn bundled-pack materialization: {error}");
            }
        }
    }

    // ---- tray (ADR 0002) ----
    let hidden = Arc::new(AtomicBool::new(false));
    let initial_target = {
        let guard = state.lock().unwrap();
        let location = guard.location();
        let (times, _) = day(&location, &guard.settings, &guard.mosques);
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
        ui.on_open_athkar(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                s.athkar.borrow_mut().dirty = true;
                ui.set_page("athkar".into());
                apply_home(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_close_athkar(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("home".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_athkar_switch_context(move |key| {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                if let Some(ctx) = Context::from_key(&key) {
                    let mut a = s.athkar.borrow_mut();
                    a.manual = true;
                    a.ctx = ctx;
                    athkar_reload(&s, &mut a);
                    drop(a);
                    apply_home(&ui, &s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_athkar_follow_window(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let mut a = s.athkar.borrow_mut();
                a.manual = false;
                a.loaded_ctx = None;
                drop(a);
                apply_home(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_athkar_repeat(move |category, item| {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let target = s
                    .corpus
                    .as_ref()
                    .and_then(|c| c.category(category as u32))
                    .and_then(|cat| cat.items.iter().find(|i| i.id == item as u32))
                    .map(|i| i.count);
                if let Some(target) = target {
                    let mut a = s.athkar.borrow_mut();
                    if let Some(storage) = &s.storage {
                        match storage.record_athkar_repeat(
                            a.ctx.key(),
                            category as u32,
                            item as u32,
                            &a.day,
                            target,
                        ) {
                            Ok(done) => {
                                a.done.insert((category as u32, item as u32), done);
                                a.dirty = true;
                            }
                            Err(e) => log::error!("athkar repeat write failed: {e}"),
                        }
                    }
                    drop(a);
                    apply_home(&ui, &s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_open_city_picker(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                ui.set_recent_cities(recents_model(&s));
                ui.set_city_query("".into());
                ui.set_manual_error("".into());
                run_picker_search(&ui, &s, "");
                ui.set_page("city-picker".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_open_calendar(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let location = s.location();
                let today = location.local(now()).date_naive();
                {
                    let mut cal = s.calendar.borrow_mut();
                    cal.anchor = today;
                    cal.selected = Some(today);
                }
                build_calendar(&ui, &s, &location);
                ui.set_page("calendar".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_calendar_back(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("home".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_calendar_nav(move |delta| {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let location = s.location();
                {
                    let mut cal = s.calendar.borrow_mut();
                    let months = chrono::Months::new(delta.unsigned_abs());
                    cal.anchor = if delta < 0 {
                        cal.anchor.checked_sub_months(months)
                    } else {
                        cal.anchor.checked_add_months(months)
                    }
                    .unwrap_or(cal.anchor);
                }
                build_calendar(&ui, &s, &location);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_calendar_today(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let location = s.location();
                let today = location.local(now()).date_naive();
                {
                    let mut cal = s.calendar.borrow_mut();
                    cal.anchor = today;
                    cal.selected = Some(today);
                }
                build_calendar(&ui, &s, &location);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_calendar_primary(move |key| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                s.settings.calendar_hijri_primary = key.as_str() == "hijri";
                s.changed();
                let location = s.location();
                build_calendar(&ui, &s, &location);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_pick_day(move |iso| {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                if let Ok(d) = NaiveDate::parse_from_str(iso.as_str(), "%Y-%m-%d") {
                    s.calendar.borrow_mut().selected = Some(d);
                    let location = s.location();
                    build_calendar(&ui, &s, &location);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_ramadan_mode(move |on| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                if on {
                    // Opt-in only. Remember the theme; suggest Ramadan's.
                    s.settings.pre_ramadan_theme = Some(s.settings.theme);
                    s.settings.theme = tawadu_core::settings::Theme::Ramadan;
                    s.settings.calendar_hijri_primary = true;
                } else {
                    // Restore the remembered theme — never a silent switch.
                    s.settings.theme = s
                        .settings
                        .pre_ramadan_theme
                        .take()
                        .unwrap_or(tawadu_core::settings::Theme::Plaster);
                }
                s.settings.ramadan_mode = on;
                s.changed();
                apply_theme(&ui, s.settings.theme);
                apply_home(&ui, &s);
                apply_settings(&ui, &s);
                let location = s.location();
                build_calendar(&ui, &s, &location);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_high_lat(move |key| {
            let mut s = state.lock().unwrap();
            if let Some(rule) = decode::<HighLatitudeRule>(&key) {
                s.settings.config.high_latitude_rule = rule;
                s.changed();
                if let Some(wake) = &s.wake {
                    let _ = wake.send(());
                }
                if let Some(ui) = weak.upgrade() {
                    apply_home(&ui, &s);
                    apply_settings(&ui, &s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_fine_tune(move |prayer, value| {
            let mut s = state.lock().unwrap();
            let v = value.clamp(-30, 30) as i64;
            let adj = &mut s.settings.config.adjustments;
            match prayer.as_str() {
                "fajr" => adj.fajr = v,
                "dhuhr" => adj.dhuhr = v,
                "asr" => adj.asr = v,
                "maghrib" => adj.maghrib = v,
                "isha" => adj.isha = v,
                _ => return,
            }
            s.changed();
            if let Some(wake) = &s.wake {
                let _ = wake.send(());
            }
            if let Some(ui) = weak.upgrade() {
                apply_home(&ui, &s);
                apply_settings(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_show_explain(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_explain_visible(true);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_explain_change_rule(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_explain_why_rules(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_picker_back(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_search_cities(move |text| {
            if weak.upgrade().is_some() {
                let s = state.lock().unwrap();
                s.picker.borrow_mut().pending_query = Some(text.to_string());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_pick_city(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                let hit = id
                    .parse::<u32>()
                    .ok()
                    .and_then(|gid| s.city_db.as_ref()?.by_id(gid).ok().flatten());
                if let Some(hit) = hit {
                    apply_location(&ui, &mut s, hit.location());
                    ui.set_page("settings".into());
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_pick_recent(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                let stored = id
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| s.settings.recent_cities.get(i).cloned());
                if let Some(stored) = stored {
                    apply_location(&ui, &mut s, stored.resolve());
                    ui.set_page("settings".into());
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_nav_result(move |delta| {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let mut p = s.picker.borrow_mut();
                let len = p.results.len() as i32;
                if len > 0 {
                    p.highlight = (p.highlight + delta).rem_euclid(len);
                    ui.set_highlighted_result(p.highlight);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_use_gps(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            if ui.get_gps_busy() {
                return;
            }
            ui.set_gps_busy(true);
            ui.set_gps_confirm_visible(false);
            // Privacy caption doubles as the busy line (existing Quill key).
            ui.set_picker_message(
                "Read once, stays on this device. Nothing leaves this device.".into(),
            );
            ui.set_picker_message_body("".into());
            let weak2 = ui.as_weak();
            std::thread::spawn(move || {
                let fix = gps::read_fix();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(ui) = weak2.upgrade() else {
                        return;
                    };
                    ui.set_gps_busy(false);
                    match fix {
                        gps::GpsFix::Found { lat, lon } => {
                            ui.set_gps_lat(format!("{lat:.4}").into());
                            ui.set_gps_lon(format!("{lon:.4}").into());
                            ui.set_gps_confirm_visible(true);
                            ui.set_picker_message("".into());
                        }
                        gps::GpsFix::Denied => {
                            ui.set_picker_message(
                                "Location permission denied — enter coordinates manually.".into(),
                            );
                        }
                        gps::GpsFix::Unavailable => {
                            ui.set_picker_message(
                                "Location is unavailable on this device — enter coordinates manually.".into(),
                            );
                        }
                        gps::GpsFix::Timeout => {
                            ui.set_picker_message(
                                "Taking too long — enter coordinates manually.".into(),
                            );
                        }
                    }
                });
            });
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_gps_confirm(move || {
            if let Some(ui) = weak.upgrade() {
                // CONFIRMED fix prefills the manual form; the timezone is
                // still chosen EXPLICITLY (never guessed from longitude).
                ui.set_manual_lat(ui.get_gps_lat());
                ui.set_manual_lon(ui.get_gps_lon());
                ui.set_coords_open(true);
                ui.set_gps_confirm_visible(false);
                ui.set_picker_message(
                    "Coordinates filled in — choose a timezone, then apply.".into(),
                );
                ui.set_picker_message_body("".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_gps_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_gps_confirm_visible(false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_apply_coords(move |lat, lon, tz, name| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                let lat = lat.trim().parse::<f64>();
                let lon = lon.trim().parse::<f64>();
                let (lat, lon) = match (lat, lon) {
                    (Ok(a), Ok(b)) => (a, b),
                    _ => {
                        ui.set_manual_error(
                            "Latitude must be between -90 and 90. Longitude must be between -180 and 180.".into(),
                        );
                        return;
                    }
                };
                if let Err(which) = validate_coordinates(lat, lon) {
                    ui.set_manual_error(
                        if which == "latitude" {
                            "Latitude must be between -90 and 90."
                        } else {
                            "Longitude must be between -180 and 180."
                        }
                        .into(),
                    );
                    return;
                }
                let zone: chrono_tz::Tz = match tz.trim().parse() {
                    Ok(z) => z,
                    Err(_) => {
                        ui.set_manual_error("Choose a timezone.".into());
                        return;
                    }
                };
                let loc = tawadu_core::locations::Location::new(
                    name.trim(),
                    "",
                    tawadu_core::prayer::Coordinates::new(lat, lon),
                    zone,
                );
                ui.set_manual_error("".into());
                apply_location(&ui, &mut s, loc);
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_search_tz(move |q| {
            if let Some(ui) = weak.upgrade() {
                let q = q.trim().to_lowercase();
                let rows: Vec<OptionRow> = if q.is_empty() {
                    Vec::new()
                } else {
                    timezone_list()
                        .into_iter()
                        .filter(|z| z.to_lowercase().contains(&q))
                        .take(30)
                        .map(|z| OptionRow {
                            value: z.into(),
                            label: z.into(),
                        })
                        .collect()
                };
                ui.set_tz_results(ModelRc::new(VecModel::from(rows)));
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_pick_tz(move |zone| {
            if let Some(ui) = weak.upgrade() {
                ui.set_manual_tz(zone.clone());
                ui.set_manual_tz_query("".into());
                ui.set_tz_results(ModelRc::new(VecModel::from(Vec::<OptionRow>::new())));
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_add_mosque(move || {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                // F4: draft only — persistence waits for a real name.
                let id = add_mosque_draft(&mut s);
                *s.mosque_confirm.borrow_mut() = String::new();
                ui.set_editing_mosque(id.into());
                ui.set_mosque_name_hint(false);
                refresh_mosques_ui(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_edit_mosque(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                // F4: switching away from an unnamed draft discards it.
                let current = ui.get_editing_mosque().to_string();
                if !current.is_empty() && current != id.as_str() {
                    discard_unnamed_mosque(&mut s, &current);
                    s.changed();
                }
                ui.set_editing_mosque(id.clone());
                ui.set_mosque_name_hint(false);
                refresh_mosques_ui(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_cancel_mosque_remove(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                *s.mosque_confirm.borrow_mut() = String::new();
                refresh_mosques_ui(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_activate_mosque(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                if s.mosques
                    .iter()
                    .any(|m| m.id == id.as_str() && m.is_named())
                {
                    s.settings.active_mosque = Some(id.to_string());
                    s.changed();
                    if let Some(wake) = &s.wake {
                        let _ = wake.send(());
                    }
                    apply_home(&ui, &s);
                    refresh_mosques_ui(&ui, &s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_mosque_name(move |id, name| {
            if let Some(ui) = weak.upgrade() {
                let mut guard = state.lock().unwrap();
                // Plain &mut AppState: MutexGuard deref would defeat the
                // field-disjoint borrows (mosques mut + storage imm).
                let s = &mut *guard;
                let updated = {
                    if let Some(m) = s.mosques.iter_mut().find(|m| m.id == id.as_str()) {
                        m.name = name.to_string();
                        if let Some(storage) = &s.storage {
                            if m.is_named() {
                                // Create-on-first-name (F4).
                                let _ = storage.save_mosque(m);
                            } else {
                                // Emptied again: unnamed never stays persisted.
                                let _ = storage.delete_mosque(&m.id);
                            }
                        }
                        true
                    } else {
                        false
                    }
                };
                if updated {
                    s.changed();
                    ui.set_mosque_name_hint(false);
                    apply_home(&ui, s);
                    refresh_mosques_ui(&ui, s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_mosque_name_ar(move |id, name| {
            if let Some(ui) = weak.upgrade() {
                let mut guard = state.lock().unwrap();
                // Plain &mut AppState: MutexGuard deref would defeat the
                // field-disjoint borrows (mosques mut + storage imm).
                let s = &mut *guard;
                let updated = {
                    if let Some(m) = s.mosques.iter_mut().find(|m| m.id == id.as_str()) {
                        m.name_ar = name.to_string();
                        if let Some(storage) = &s.storage {
                            let _ = storage.save_mosque(m);
                        }
                        true
                    } else {
                        false
                    }
                };
                if updated {
                    s.changed();
                    apply_home(&ui, s);
                    refresh_mosques_ui(&ui, s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_mosque_offset(move |id, prayer, value| {
            if let Some(ui) = weak.upgrade() {
                let mut guard = state.lock().unwrap();
                // Plain &mut AppState: MutexGuard deref would defeat the
                // field-disjoint borrows (mosques mut + storage imm).
                let s = &mut *guard;
                let value = value.clamp(0, 60) as i64;
                let updated = {
                    if let Some(m) = s.mosques.iter_mut().find(|m| m.id == id.as_str()) {
                        match prayer.as_str() {
                            "fajr" => m.offsets.fajr = value,
                            "dhuhr" => m.offsets.dhuhr = value,
                            "asr" => m.offsets.asr = value,
                            "maghrib" => m.offsets.maghrib = value,
                            "isha" => m.offsets.isha = value,
                            _ => return,
                        }
                        if let Some(storage) = &s.storage {
                            let _ = storage.save_mosque(m);
                        }
                        true
                    } else {
                        false
                    }
                };
                if updated {
                    s.changed();
                    if let Some(wake) = &s.wake {
                        let _ = wake.send(());
                    }
                    apply_home(&ui, s);
                    refresh_mosques_ui(&ui, s);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_remove_mosque(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.lock().unwrap();
                let confirming = s.mosque_confirm.borrow().clone();
                if confirming != id.as_str() {
                    // First click: arm the two-step confirm (Prism §4) with
                    // the Quill remove copy — the consequence is VISIBLE at
                    // confirm time (N4), never a silent reassignment.
                    *s.mosque_confirm.borrow_mut() = id.to_string();
                    let name = s
                        .mosques
                        .iter()
                        .find(|m| m.id == id.as_str())
                        .map(|m| m.name.clone())
                        .unwrap_or_default();
                    ui.set_mosque_remove_body(
                        format!("\u{201C}{name}\u{201D} will be removed. The app will use no active mosque.").into(),
                    );
                    ui.set_mosque_remove_active(
                        s.settings.active_mosque.as_deref() == Some(id.as_str()),
                    );
                    refresh_mosques_ui(&ui, &s);
                    return;
                }
                // Second click: remove. Active profile -> NO active mosque
                // (honest fallback, never a silent reassignment).
                if let Some(storage) = &s.storage {
                    let _ = storage.delete_mosque(id.as_str());
                }
                s.mosques.retain(|m| m.id != id.as_str());
                if s.settings.active_mosque.as_deref() == Some(id.as_str()) {
                    s.settings.active_mosque = None;
                    s.status = "Mosque removed — no active mosque.".into();
                }
                *s.mosque_confirm.borrow_mut() = String::new();
                if ui.get_editing_mosque().as_str() == id.as_str() {
                    ui.set_editing_mosque("".into());
                }
                s.changed();
                if let Some(wake) = &s.wake {
                    let _ = wake.send(());
                }
                apply_home(&ui, &s);
                apply_settings(&ui, &s);
                refresh_mosques_ui(&ui, &s);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_done_editing(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let editing = ui.get_editing_mosque().to_string();
                let unnamed = s
                    .mosques
                    .iter()
                    .find(|m| m.id == editing)
                    .map(|m| m.name.trim().is_empty())
                    .unwrap_or(false);
                if unnamed {
                    // Name required (Prism §4): Done on an unnamed draft
                    // DISCARDS it — nothing unnamed persists, no fabricated
                    // "Mosque" name (F4). The hint flashes via the row.
                    drop(s);
                    let mut s = state.lock().unwrap();
                    discard_unnamed_mosque(&mut s, &editing);
                    s.changed();
                    ui.set_editing_mosque("".into());
                    ui.set_mosque_name_hint(false);
                    refresh_mosques_ui(&ui, &s);
                } else {
                    ui.set_editing_mosque("".into());
                    ui.set_mosque_name_hint(false);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_athkar_reset(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.lock().unwrap();
                let mut a = s.athkar.borrow_mut();
                if let Some(storage) = &s.storage {
                    if let Err(e) = storage.reset_athkar_context(a.ctx.key(), &a.day) {
                        log::error!("athkar reset failed: {e}");
                    }
                }
                a.done.clear();
                a.dirty = true;
                drop(a);
                apply_home(&ui, &s);
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
            // Status feedback ONLY: a full apply_settings here could clobber
            // a location_query typed during the debounce window (Sentinel).
            ui.set_status_message(state.status.clone().into());
        }
    });

    // Once a second: the countdown text AND the athkar card/view refresh
    // (Sentinel F1). Auto must re-follow window boundaries while idle — the
    // Dhuhr Morning→AfterPrayer boundary has no scheduler event at all — and
    // the day rollover must reload honest counts without user interaction.
    // The tick runs while the window is hidden to tray, so a restored window
    // shows the real current context, never a stale one. The athkar refresh
    // is pure + cheap (one day's prayer compute + a card scan; ROADMAP debt
    // #7 measures the full home rebuild at ~165 µs). The tray lives in this
    // closure so its tooltip rides the same tick — recomputed once per prayer
    // change, never per tick. TrayIcon is !Send (Rc<RefCell> inside), so it
    // must stay on the UI thread.
    let handle = ui.as_weak();
    let timer = Timer::default();
    let timer_state = Arc::clone(&state);
    let mut target = initial_target.map(|(_, at)| at);
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        let Some(ui) = handle.upgrade() else {
            return;
        };
        let guard = timer_state.lock().unwrap();
        let location = guard.location();
        let times = prayer::compute(
            location.local(now()).date_naive(),
            location.coordinates,
            &guard.settings.config,
        );
        apply_athkar(&ui, &guard, &location, &times);
        let stale = match target {
            Some(at) => at <= now(),
            None => true,
        };
        if stale {
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
        // Athkar rides the same idle tick: Auto re-follows window boundaries
        // and the day rollover reloads progress without any interaction.
        apply_athkar(&ui, &guard, &location, &times);
    });

    // City-picker search debounce (~150 ms, Prism §3): the expensive scan
    // runs once per pause, never per keystroke.
    let picker_timer = Timer::default();
    {
        let handle = ui.as_weak();
        let picker_state = Arc::clone(&state);
        picker_timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
            let Some(ui) = handle.upgrade() else {
                return;
            };
            let s = picker_state.lock().unwrap();
            let pending = s.picker.borrow_mut().pending_query.take();
            if let Some(q) = pending {
                run_picker_search(&ui, &s, &q);
            }
        });
    }

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

    // ---- audio callbacks ----
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_import_pack_file(move || {
            spawn_import(Arc::clone(&state), weak.clone(), false);
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_import_pack_folder(move || {
            spawn_import(Arc::clone(&state), weak.clone(), true);
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_preview_pack(move |id: SharedString| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let state = state.lock().unwrap();
            // One preview at a time: clicking the active card stops it.
            if ui.get_previewing_pack_id() == id {
                if let Some(engine) = &state.engine {
                    engine.stop();
                }
                ui.set_previewing_pack_id(SharedString::new());
                return;
            }
            let (Some(engine), Some(library)) = (&state.engine, &state.library) else {
                return;
            };
            let lib = library.lock().unwrap();
            // Preview plays THIS pack with the current trim/fade/volume —
            // exactly what the event will sound like (Prism §3).
            let mut audio = state.settings.audio.clone();
            let mode_override = audio
                .event_voices
                .get(routing::EVENT_ATHAN)
                .and_then(|voice| voice.mode_override);
            audio.event_voices.insert(
                routing::EVENT_ATHAN.into(),
                EventVoice {
                    pack: Some(id.to_string()),
                    mode_override,
                },
            );
            let Some(plan) = engine::plan_for_event(routing::EVENT_ATHAN, &audio, lib.packs())
            else {
                return;
            };
            engine.play(PlayRequest {
                plan: plan.clone(),
                volume: state.settings.athan_volume,
                library_dir: lib.dir().to_path_buf(),
                cache_dir: lib.cache_root().to_path_buf(),
            });
            ui.set_previewing_pack_id(id.clone());
            // Auto-clear the Stop state when the preview's real probed
            // length is known; unknown length → the user stops it (honest).
            if let Some(ms) = preview_duration_ms(&plan, lib.packs()) {
                let weak = weak.clone();
                let id = id.to_string();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(ms + 300));
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            if ui.get_previewing_pack_id().as_str() == id {
                                ui.set_previewing_pack_id(SharedString::new());
                            }
                        }
                    });
                });
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_remove_pack(move |id: SharedString| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.lock().unwrap();
            let Some(library) = state.library.clone() else {
                return;
            };
            let name = library
                .lock()
                .unwrap()
                .packs()
                .iter()
                .find(|pack| pack.id == id.as_str())
                .map(|pack| pack.manifest.name.clone());
            let removed = library.lock().unwrap().remove(&id);
            match removed {
                Ok(true) => {
                    let name = name.unwrap_or_else(|| id.to_string());
                    // Never silent: assigned events fall back (routing),
                    // and the status line says so (Prism §3). The stale
                    // references are dropped here so a re-import on the
                    // same slug id cannot resurrect them (Sentinel N3);
                    // `changed()` below persists the cleanup.
                    let affected = forget_removed_pack(&mut state.settings.audio, id.as_str());
                    state.status = remove_status(&name, affected);
                    if ui.get_previewing_pack_id() == id {
                        if let Some(engine) = &state.engine {
                            engine.stop();
                        }
                        ui.set_previewing_pack_id(SharedString::new());
                    }
                    state.changed();
                    apply_audio(&ui, &state);
                    ui.set_status_message(state.status.clone().into());
                }
                Ok(false) => {}
                Err(error) => {
                    state.status = format!("Could not remove: {error}");
                    ui.set_status_message(state.status.clone().into());
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_show_attribution(move |_id: SharedString| {
            if let Some(ui) = weak.upgrade() {
                let state = state.lock().unwrap();
                apply_credits(&ui, &state);
                ui.set_page("credits".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_close_credits(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page("settings".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_pick_event_voice(move |kind: SharedString, id: SharedString| {
            let mut state = state.lock().unwrap();
            let voice = state
                .settings
                .audio
                .event_voices
                .entry(kind.to_string())
                .or_default();
            voice.pack = if id.is_empty() {
                None
            } else {
                Some(id.to_string())
            };
            if let Some(ui) = weak.upgrade() {
                state.changed();
                apply_audio(&ui, &state);
                ui.set_expanded_event(SharedString::new());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_event_mode(move |kind: SharedString, mode: SharedString| {
            let mut state = state.lock().unwrap();
            if let Some(mode) = decode::<PlaybackMode>(&mode) {
                state
                    .settings
                    .audio
                    .event_voices
                    .entry(kind.to_string())
                    .or_default()
                    .mode_override = Some(mode);
            }
            if let Some(ui) = weak.upgrade() {
                state.changed();
                apply_audio(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_reset_event_mode(move |kind: SharedString| {
            let mut state = state.lock().unwrap();
            routing::reset_mode_override(&kind, &mut state.settings.audio);
            if let Some(ui) = weak.upgrade() {
                state.changed();
                apply_audio(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_toggle_event_expanded(move |kind: SharedString| {
            if let Some(ui) = weak.upgrade() {
                // Only one row expanded at a time (Prism §4).
                let next = if ui.get_expanded_event() == kind {
                    SharedString::new()
                } else {
                    kind
                };
                ui.set_expanded_event(next);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_audio_volume(move |value: i32| {
            let mut state = state.lock().unwrap();
            let value = value.clamp(0, 100);
            state.settings.athan_volume = value as f32 / 100.0;
            if let Some(engine) = &state.engine {
                engine.set_volume(state.settings.athan_volume);
            }
            if let Some(ui) = weak.upgrade() {
                ui.set_audio_volume(value);
                state.changed();
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = Arc::clone(&state);
        ui.on_set_athan_trim(move |value: i32| {
            let mut state = state.lock().unwrap();
            state.settings.audio.athan_stop_after_s = value.clamp(0, 300) as u32;
            if let Some(ui) = weak.upgrade() {
                state.changed();
                apply_audio(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_dismiss_audio_message(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_audio_message(SharedString::new());
                ui.set_audio_message_ar(SharedString::new());
                ui.set_audio_message_arg(SharedString::new());
                ui.set_audio_message_ar_after(SharedString::new());
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
                // Contract with tray_lifecycle phase 5: default + 5 (= 15).
                state.settings.pre_alert_minutes =
                    tawadu_core::settings::Settings::default().pre_alert_minutes + 5;
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
    if let Some(engine) = &state.lock().unwrap().engine {
        engine.shutdown();
    }
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
    fn every_event_kind_maps_to_its_voice_event() {
        assert_eq!(voice_event(EventKind::PreAlert), routing::EVENT_CHIME);
        assert_eq!(voice_event(EventKind::Prayer), routing::EVENT_ATHAN);
        assert_eq!(voice_event(EventKind::Iqamah), routing::EVENT_IQAMAH);
        assert_eq!(voice_event(EventKind::AthkarWindow), routing::EVENT_ATHKAR);
    }

    // ---- audio UI row builders ----

    fn ui_pack(id: &str, name_ar: Option<&str>, duration: Option<u64>) -> InstalledPack {
        let ar = name_ar
            .map(|ar| format!(", \"name_ar\": \"{ar}\""))
            .unwrap_or_default();
        let json = format!(
            r#"{{
                "format": "tawadu-pack/1",
                "name": "{id}",
                "reciter": "A Reciter"{ar},
                "license": "CC0-1.0",
                "files": {{ "athan/a.ogg": {{ "sha256": "aa", "license": "CC0-1.0" }} }},
                "sequences": {{ "fajr": {{ "mode": "sequential", "clips": ["athan/a.ogg"] }} }}
            }}"#
        );
        let manifest = tawadu_audio::manifest::Manifest::parse(&json).expect("manifest");
        let mut durations_ms = std::collections::BTreeMap::new();
        durations_ms.insert("athan/a.ogg".to_string(), duration);
        InstalledPack {
            id: id.into(),
            manifest,
            durations_ms,
            bundled: false,
        }
    }

    #[test]
    fn trim_plural_follows_cldr_arabic_categories() {
        assert_eq!(trim_unit_words(1), ("second", "ثانية"));
        assert_eq!(trim_unit_words(2), ("seconds", "ثانيتان"));
        assert_eq!(trim_unit_words(5), ("seconds", "ثوانٍ")); // few: 3..10
        assert_eq!(trim_unit_words(10), ("seconds", "ثوانٍ"));
        assert_eq!(trim_unit_words(15), ("seconds", "ثانيةً")); // many: 11..99
        assert_eq!(trim_unit_words(95), ("seconds", "ثانيةً"));
        assert_eq!(trim_unit_words(100), ("seconds", "ثانية")); // other
        assert_eq!(trim_unit_words(105), ("seconds", "ثوانٍ")); // 105 % 100 = 5
        assert_eq!(trim_unit_words(0), ("seconds", "ثانية"));
    }

    #[test]
    fn pack_rows_show_probed_duration_or_nothing() {
        let packs = [
            ui_pack("known", None, Some(154_018)),
            ui_pack("unknown", None, None),
        ];
        let rows = pack_rows(&packs, None);
        assert_eq!(rows[0].duration.as_str(), "2:34");
        assert_eq!(rows[1].duration.as_str(), "", "unknown is never a guess");
        assert_eq!(rows[0].name_ar.as_str(), "", "no machine translation");
    }

    #[test]
    fn pack_rows_select_the_marked_default_else_the_first() {
        let packs = [
            ui_pack("first", None, Some(1000)),
            ui_pack("second", None, Some(1000)),
        ];
        let rows = pack_rows(&packs, None);
        assert!(rows[0].is_selected);
        assert!(!rows[1].is_selected);
        let rows = pack_rows(&packs, Some("second"));
        assert!(!rows[0].is_selected);
        assert!(rows[1].is_selected);
        // A marked default that no longer exists falls back to the first.
        let rows = pack_rows(&packs, Some("gone"));
        assert!(rows[0].is_selected);
    }

    #[test]
    fn voice_options_start_with_default_and_keep_scripts_separate() {
        let packs = [
            ui_pack("latin-only", None, Some(1000)),
            ui_pack("bilingual", Some("حزمة"), Some(1000)),
        ];
        let options = voice_options(&packs, false);
        assert_eq!(options[0].id.as_str(), "");
        assert_eq!(options[0].label.as_str(), "Default");
        assert_eq!(options[0].label_ar.as_str(), "الافتراضي");
        assert_eq!(
            options[1].label_ar.as_str(),
            "",
            "Latin never leaks into the Arabic run"
        );
        assert_eq!(options[2].label_ar.as_str(), "حزمة");
        assert!(!options
            .iter()
            .any(|o| o.id.as_str() == routing::BUILTIN_CHIME_ID));
        let chime = voice_options(&packs, true);
        assert!(chime
            .iter()
            .any(|o| o.id.as_str() == routing::BUILTIN_CHIME_ID));
    }

    #[test]
    fn event_rows_report_choice_mode_override_and_empty_state() {
        let packs = [ui_pack("p", None, Some(1000))];
        let mut audio = AudioSettings::default();
        let rows = event_voice_rows(&packs, &audio);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].voice_label.as_str(), "Default");
        assert!(rows[0].has_voice);
        // Athkar: the pack has no athkar clips → honest empty state.
        let athkar = rows.iter().find(|r| r.kind.as_str() == "athkar").unwrap();
        assert!(!athkar.has_voice);
        // The chime always has the built-in floor.
        let chime = rows.iter().find(|r| r.kind.as_str() == "chime").unwrap();
        assert!(chime.has_voice);

        audio.event_voices.insert(
            "athan".into(),
            EventVoice {
                pack: Some("p".into()),
                mode_override: Some(PlaybackMode::Shuffle),
            },
        );
        let rows = event_voice_rows(&packs, &audio);
        let athan = &rows[0];
        assert_eq!(athan.voice_id.as_str(), "p");
        assert_eq!(athan.voice_label.as_str(), "p");
        assert_eq!(athan.mode.as_str(), "shuffle");
        assert!(athan.has_override);

        // A removed choice reports the fallback honestly: Default.
        audio.event_voices.insert(
            "athan".into(),
            EventVoice {
                pack: Some("gone".into()),
                mode_override: None,
            },
        );
        let rows = event_voice_rows(&packs, &audio);
        assert_eq!(rows[0].voice_label.as_str(), "Default");
        assert_eq!(rows[0].voice_id.as_str(), "");
    }

    #[test]
    fn removing_a_pack_drops_its_settings_references() {
        // Sentinel N3: a re-import landing on the same slug id must not
        // resurrect a stale assignment — removal cleans both fields.
        let mut audio = AudioSettings {
            default_pack: Some("gone".into()),
            ..AudioSettings::default()
        };
        audio.event_voices.insert(
            "athan".into(),
            EventVoice {
                pack: Some("gone".into()),
                mode_override: None,
            },
        );
        audio.event_voices.insert(
            "iqamah".into(),
            EventVoice {
                pack: Some("gone".into()),
                mode_override: Some(PlaybackMode::Shuffle),
            },
        );
        audio.event_voices.insert(
            "athkar".into(),
            EventVoice {
                pack: Some("keep".into()),
                mode_override: None,
            },
        );
        audio.event_voices.insert(
            "chime".into(),
            EventVoice {
                pack: None,
                mode_override: Some(PlaybackMode::Sequential),
            },
        );

        // References existed → the "assigned events fall back" branch.
        assert!(forget_removed_pack(&mut audio, "gone"));
        assert_eq!(audio.default_pack, None);
        assert!(!audio.event_voices.contains_key("athan"));
        assert!(!audio.event_voices.contains_key("iqamah"));
        assert!(audio
            .event_voices
            .values()
            .all(|voice| voice.pack.as_deref() != Some("gone")));
        // Unrelated choices survive untouched (incl. a Default + override).
        assert_eq!(audio.event_voices["athkar"].pack.as_deref(), Some("keep"));
        assert_eq!(
            audio.event_voices["chime"].mode_override,
            Some(PlaybackMode::Sequential)
        );

        // The fallback still behaves: routing resolves to the remaining
        // pack and the UI row reports Default honestly.
        let packs = [ui_pack("keep", None, Some(1000))];
        match routing::resolve_voice(routing::EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "keep"),
            other => panic!("expected fallback to the remaining pack, got {other:?}"),
        }
        let rows = event_voice_rows(&packs, &audio);
        assert_eq!(rows[0].voice_label.as_str(), "Default");
        assert_eq!(rows[0].voice_id.as_str(), "");

        // Nothing left to drop → the plain "Removed {name}." branch.
        assert!(!forget_removed_pack(&mut audio, "gone"));
        assert_eq!(
            remove_status("Gone Pack", true),
            "Removed Gone Pack. Assigned events fall back to the default voice."
        );
        assert_eq!(remove_status("Gone Pack", false), "Removed Gone Pack.");
    }

    #[test]
    fn preview_duration_is_known_only_when_every_clip_is_known() {
        let packs = [ui_pack("p", None, Some(60_000))];
        let audio = AudioSettings {
            athan_stop_after_s: 30,
            ..AudioSettings::default()
        };
        let plan = engine::plan_for_event(routing::EVENT_ATHAN, &audio, &packs).unwrap();
        assert_eq!(preview_duration_ms(&plan, &packs), Some(30_000));
        assert_eq!(
            preview_duration_ms(&PlaybackPlan::Chime, &packs),
            Some(1_600)
        );
        let unknown = [ui_pack("p", None, None)];
        let plan = engine::plan_for_event(routing::EVENT_ATHAN, &audio, &unknown).unwrap();
        assert_eq!(preview_duration_ms(&plan, &unknown), None);
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

    // ---- athkar tick refresh (Sentinel F1: idle boundary-cross + day rollover) ----

    // ---- F4 (Sentinel): unnamed mosque drafts never persist ----

    #[test]
    fn add_then_dismiss_leaves_no_persisted_unnamed_profile() {
        let mut s = athkar_fixture("2026-11-19", Context::Morning, false);
        let id = add_mosque_draft(&mut s);
        assert_eq!(s.mosques.len(), 1, "draft visible for editing");
        assert!(
            s.storage
                .as_ref()
                .unwrap()
                .load_mosques()
                .unwrap()
                .is_empty(),
            "unnamed draft must NOT be persisted"
        );
        discard_unnamed_mosque(&mut s, &id);
        assert!(s.mosques.is_empty(), "dismiss discards the draft");
        assert!(s
            .storage
            .as_ref()
            .unwrap()
            .load_mosques()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn named_profile_persists_and_survives_dismiss() {
        let mut s = athkar_fixture("2026-11-19", Context::Morning, false);
        let id = add_mosque_draft(&mut s);
        if let Some(m) = s.mosques.iter_mut().find(|m| m.id == id) {
            m.name = "Al-Noor".into();
            if let Some(storage) = &s.storage {
                storage.save_mosque(m).unwrap();
            }
        }
        discard_unnamed_mosque(&mut s, &id);
        assert_eq!(s.mosques.len(), 1, "named profile stays");
        assert_eq!(s.storage.as_ref().unwrap().load_mosques().unwrap().len(), 1);
    }

    /// An AppState with the real bundled corpus + an in-memory store, parked
    /// at a known context/day — the wiring-level fixture for the tick refresh.
    fn athkar_fixture(day: &str, ctx: Context, manual: bool) -> AppState {
        AppState {
            settings: Settings::default(),
            storage: Some(Storage::open_in_memory().unwrap()),
            status: String::new(),
            wake: None,
            debounce: std::cell::RefCell::new(SaveDebounce::new(Duration::from_millis(800))),
            library: None,
            engine: None,
            corpus: Some(Corpus::bundled().unwrap()),
            city_db: None,
            picker: std::cell::RefCell::new(PickerUi {
                pending_query: None,
                results: Vec::new(),
                highlight: -1,
            }),
            mosques: Vec::new(),
            mosque_confirm: std::cell::RefCell::new(String::new()),
            calendar: std::cell::RefCell::new(CalendarUi {
                anchor: Utc::now().date_naive(),
                selected: None,
            }),
            athkar: std::cell::RefCell::new(AthkarUi {
                ctx,
                loaded_ctx: Some(ctx),
                manual,
                day: day.to_string(),
                done: std::collections::HashMap::new(),
                dirty: false,
            }),
        }
    }

    #[test]
    fn tick_refollows_the_window_boundary_while_idle() {
        // The Dhuhr Morning→AfterPrayer boundary has NO scheduler event —
        // only the 1 s tick can re-follow Auto while the app is idle.
        let state = athkar_fixture("2026-11-19", Context::Morning, false);
        let location = state.location();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let times = prayer::compute(date, location.coordinates, &state.settings.config);
        let dhuhr = times.dhuhr.expect("Riyadh Dhuhr is always defined");
        let storage = state.storage.as_ref().unwrap();
        storage
            .record_athkar_repeat("morning", 27, 1, "2026-11-19", 1)
            .unwrap();
        for _ in 0..3 {
            storage
                .record_athkar_repeat("after_prayer", 25, 4, "2026-11-19", 33)
                .unwrap();
        }
        // The UI loaded the morning window's counts at startup.
        state.athkar.borrow_mut().done = storage.athkar_progress("morning", "2026-11-19").unwrap();

        // One second before the boundary: still Morning — no premature switch.
        athkar_tick(
            &state,
            &location,
            &times,
            dhuhr - chrono::Duration::seconds(1),
        );
        assert_eq!(state.athkar.borrow().ctx, Context::Morning);

        // The boundary itself, with NO user interaction: Auto re-follows and
        // the progress becomes the NEW window's persisted truth.
        athkar_tick(&state, &location, &times, dhuhr);
        let a = state.athkar.borrow();
        assert_eq!(a.ctx, Context::AfterPrayer);
        assert_eq!(a.loaded_ctx, Some(Context::AfterPrayer));
        assert_eq!(a.day, "2026-11-19");
        assert_eq!(a.done.get(&(25, 4)), Some(&3));
        assert!(
            !a.done.contains_key(&(27, 1)),
            "morning counts never leak across contexts"
        );
        assert!(a.dirty, "the view must rebuild for the new context");
    }

    #[test]
    fn tick_never_clobbers_a_manual_pick() {
        let state = athkar_fixture("2026-11-19", Context::Sleep, true);
        let location = state.location();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let times = prayer::compute(date, location.coordinates, &state.settings.config);
        let dhuhr = times.dhuhr.unwrap();
        // The user picked Sleep mid-morning and is mid-count.
        state.athkar.borrow_mut().done.insert((28, 1), 2);

        athkar_tick(&state, &location, &times, dhuhr);
        {
            let a = state.athkar.borrow();
            assert_eq!(
                a.ctx,
                Context::Sleep,
                "the user's override wins over the window"
            );
            assert_eq!(
                a.done.get(&(28, 1)),
                Some(&2),
                "in-flight counts survive the tick"
            );
            assert_eq!(a.loaded_ctx, Some(Context::Sleep));
            assert!(!a.dirty, "no reload churn while the override holds");
        }

        // Tapping Auto (on_athkar_follow_window semantics) re-follows on the
        // very next tick — the override lasts only until the user re-follows.
        {
            let mut a = state.athkar.borrow_mut();
            a.manual = false;
            a.loaded_ctx = None;
        }
        athkar_tick(&state, &location, &times, dhuhr);
        let a = state.athkar.borrow();
        assert_eq!(a.ctx, Context::AfterPrayer);
        assert_eq!(a.loaded_ctx, Some(Context::AfterPrayer));
    }

    #[test]
    fn tick_rolls_the_day_over_without_interaction() {
        use chrono::TimeZone;
        let state = athkar_fixture("2026-11-19", Context::Sleep, false);
        let location = state.location();
        let storage = state.storage.as_ref().unwrap();
        // Yesterday's read is persisted AND loaded; a read already recorded
        // for the new day exists too (the user read past midnight).
        storage
            .record_athkar_repeat("sleep", 28, 1, "2026-11-19", 3)
            .unwrap();
        storage
            .record_athkar_repeat("sleep", 28, 2, "2026-11-20", 1)
            .unwrap();
        state.athkar.borrow_mut().done = storage.athkar_progress("sleep", "2026-11-19").unwrap();

        // 00:00:30 local on the 20th: still the Sleep window (Isha→Fajr spans
        // midnight), so ONLY the day changes. The tick computes the NEW day's
        // times, exactly as the wired 1 s tick does.
        let date20 = chrono::NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let times20 = prayer::compute(date20, location.coordinates, &state.settings.config);
        let just_after_midnight = location
            .timezone
            .with_ymd_and_hms(2026, 11, 20, 0, 0, 30)
            .unwrap();
        athkar_tick(
            &state,
            &location,
            &times20,
            just_after_midnight.with_timezone(&Utc),
        );

        let a = state.athkar.borrow();
        assert_eq!(
            a.ctx,
            Context::Sleep,
            "the window itself does not change at midnight"
        );
        assert_eq!(a.day, "2026-11-20");
        assert_eq!(a.loaded_ctx, Some(Context::Sleep));
        assert!(
            !a.done.contains_key(&(28, 1)),
            "yesterday's reads are never today's counts"
        );
        assert_eq!(
            a.done.get(&(28, 2)),
            Some(&1),
            "the new day's persisted truth is loaded, not a fabricated zero"
        );
        assert!(a.dirty);
    }

    #[test]
    fn tick_is_a_no_op_when_nothing_changed() {
        let state = athkar_fixture("2026-11-19", Context::Morning, false);
        let location = state.location();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let times = prayer::compute(date, location.coordinates, &state.settings.config);
        let fajr = times.fajr.unwrap();
        let storage = state.storage.as_ref().unwrap();
        storage
            .record_athkar_repeat("morning", 27, 3, "2026-11-19", 1)
            .unwrap();
        state.athkar.borrow_mut().done.insert((27, 3), 1);

        // Mid-window, same day, same context: the tick must not reload or
        // mark the view dirty — no churn, and in-flight counts are untouched.
        athkar_tick(&state, &location, &times, fajr + chrono::Duration::hours(1));
        let a = state.athkar.borrow();
        assert_eq!(a.ctx, Context::Morning);
        assert_eq!(a.day, "2026-11-19");
        assert_eq!(a.done.get(&(27, 3)), Some(&1));
        assert!(!a.dirty, "an unchanged tick never rebuilds the view");
    }
}
