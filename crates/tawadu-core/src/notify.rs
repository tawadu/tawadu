//! What the app is allowed to tell the user, and when.
//!
//! The decision is pure: `should_notify` takes the kind, the user's switches and
//! the local time, and answers yes or no. Nothing here touches an OS, so the
//! rules are testable without a notification service.

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

/// The five things the app can announce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    /// "Fajr is in 10 minutes."
    PreAlert,
    /// The call to prayer itself.
    Athan,
    /// The mosque has started prayer.
    Iqamah,
    /// A morning / evening / sleep athkar window opened.
    Athkar,
    /// Storage trouble, a missing licence, and other housekeeping.
    System,
}

impl NotificationKind {
    pub const ALL: [NotificationKind; 5] = [
        NotificationKind::PreAlert,
        NotificationKind::Athan,
        NotificationKind::Iqamah,
        NotificationKind::Athkar,
        NotificationKind::System,
    ];

    /// The athan is never suppressed. Quiet hours keep the app quiet; they do not
    /// stop the call to prayer.
    pub fn is_essential(self) -> bool {
        matches!(self, NotificationKind::Athan)
    }

    /// Stable key for the interface and for logs.
    pub fn key(self) -> &'static str {
        match self {
            NotificationKind::PreAlert => "pre_alert",
            NotificationKind::Athan => "athan",
            NotificationKind::Iqamah => "iqamah",
            NotificationKind::Athkar => "athkar",
            NotificationKind::System => "system",
        }
    }
}

/// Per-kind switches plus quiet hours. One blob, persisted with the settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub pre_alert: bool,
    pub athan: bool,
    pub iqamah: bool,
    pub athkar: bool,
    pub system: bool,
    pub quiet_hours_enabled: bool,
    /// Local wall-clock time, not UTC.
    pub quiet_from: NaiveTime,
    pub quiet_to: NaiveTime,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            pre_alert: true,
            athan: true,
            iqamah: true,
            athkar: true,
            system: true,
            quiet_hours_enabled: false,
            quiet_from: NaiveTime::from_hms_opt(22, 0, 0).expect("valid time"),
            quiet_to: NaiveTime::from_hms_opt(6, 0, 0).expect("valid time"),
        }
    }
}

impl NotificationSettings {
    pub fn enabled(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::PreAlert => self.pre_alert,
            NotificationKind::Athan => self.athan,
            NotificationKind::Iqamah => self.iqamah,
            NotificationKind::Athkar => self.athkar,
            NotificationKind::System => self.system,
        }
    }

    pub fn set_enabled(&mut self, kind: NotificationKind, enabled: bool) {
        match kind {
            NotificationKind::PreAlert => self.pre_alert = enabled,
            NotificationKind::Athan => self.athan = enabled,
            NotificationKind::Iqamah => self.iqamah = enabled,
            NotificationKind::Athkar => self.athkar = enabled,
            NotificationKind::System => self.system = enabled,
        }
    }
}

/// Is `now` inside the quiet window? An overnight window (22:00–06:00) wraps.
pub fn in_quiet_hours(now: NaiveTime, from: NaiveTime, to: NaiveTime) -> bool {
    if from == to {
        return false;
    }
    if from < to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

/// The one decision the scheduler and the UI both use.
pub fn should_notify(
    kind: NotificationKind,
    settings: &NotificationSettings,
    local_time: NaiveTime,
) -> bool {
    if !settings.enabled(kind) {
        return false;
    }
    if kind.is_essential() {
        return true;
    }
    if settings.quiet_hours_enabled
        && in_quiet_hours(local_time, settings.quiet_from, settings.quiet_to)
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> NotificationSettings {
        NotificationSettings::default()
    }

    fn at(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    #[test]
    fn quiet_window_is_the_interval_between_from_and_to() {
        assert!(in_quiet_hours(at(23, 0), at(22, 0), at(6, 0)));
        assert!(in_quiet_hours(at(2, 0), at(22, 0), at(6, 0)));
        assert!(!in_quiet_hours(at(12, 0), at(22, 0), at(6, 0)));
        assert!(!in_quiet_hours(at(6, 0), at(22, 0), at(6, 0)));
    }

    #[test]
    fn a_daytime_window_does_not_wrap() {
        assert!(in_quiet_hours(at(9, 0), at(8, 0), at(17, 0)));
        assert!(!in_quiet_hours(at(18, 0), at(8, 0), at(17, 0)));
    }

    #[test]
    fn equal_from_and_to_means_no_quiet_hours() {
        assert!(!in_quiet_hours(at(12, 0), at(8, 0), at(8, 0)));
    }

    #[test]
    fn athan_is_never_suppressed() {
        let mut s = settings();
        s.quiet_hours_enabled = true;
        assert!(should_notify(NotificationKind::Athan, &s, at(3, 0)));
    }

    #[test]
    fn quiet_hours_suppress_the_non_essential_kinds() {
        let mut s = settings();
        s.quiet_hours_enabled = true;
        assert!(!should_notify(NotificationKind::PreAlert, &s, at(3, 0)));
        assert!(!should_notify(NotificationKind::Iqamah, &s, at(3, 0)));
        assert!(!should_notify(NotificationKind::Athkar, &s, at(3, 0)));
        assert!(should_notify(NotificationKind::PreAlert, &s, at(12, 0)));
    }

    #[test]
    fn a_disabled_kind_never_notifies() {
        let mut s = settings();
        s.set_enabled(NotificationKind::Iqamah, false);
        assert!(!should_notify(NotificationKind::Iqamah, &s, at(12, 0)));
        // Disabling it does not disable the athan.
        assert!(should_notify(NotificationKind::Athan, &s, at(12, 0)));
    }

    #[test]
    fn quiet_hours_off_notifies_through_the_night() {
        let s = settings();
        assert!(should_notify(NotificationKind::Iqamah, &s, at(3, 0)));
    }

    #[test]
    fn notification_settings_round_trip_through_json() {
        let s = NotificationSettings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: NotificationSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn every_kind_has_a_stable_key() {
        let keys: Vec<_> = NotificationKind::ALL.iter().map(|k| k.key()).collect();
        assert_eq!(keys, ["pre_alert", "athan", "iqamah", "athkar", "system"]);
        assert_eq!(
            keys.len(),
            keys.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }
}
