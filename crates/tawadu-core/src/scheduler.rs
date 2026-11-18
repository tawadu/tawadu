//! Builds the day's event timeline — prayers, iqamah offsets, athkar windows and
//! pre-alerts — and answers "what happens next?".
//!
//! Recompute on: start-up, location/method change, settings change, system clock
//! change, wake from sleep, and every midnight.

use crate::prayer::{Prayer, PrayerTimes};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// What kind of thing happens at a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Prayer,
    Iqamah,
    AthkarWindow,
    PreAlert,
}

impl EventKind {
    /// Order when two events share a timestamp.
    pub fn priority(self) -> u8 {
        match self {
            EventKind::PreAlert => 0,
            EventKind::Prayer => 1,
            EventKind::Iqamah => 2,
            EventKind::AthkarWindow => 3,
        }
    }

    /// Stable key for logs.
    pub fn key(self) -> &'static str {
        match self {
            EventKind::PreAlert => "pre_alert",
            EventKind::Prayer => "prayer",
            EventKind::Iqamah => "iqamah",
            EventKind::AthkarWindow => "athkar_window",
        }
    }
}

/// One thing that happens at a moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub label: &'static str,
    pub at: DateTime<Utc>,
}

/// Iqamah offsets in minutes after each prayer's adhan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IqamahOffsets {
    pub fajr: i64,
    pub dhuhr: i64,
    pub asr: i64,
    pub maghrib: i64,
    pub isha: i64,
}

impl IqamahOffsets {
    /// Offset for a prayer, or `None` for sunrise/qiyam.
    pub fn for_prayer(&self, prayer: Prayer) -> Option<i64> {
        match prayer {
            Prayer::Fajr => Some(self.fajr),
            Prayer::Dhuhr => Some(self.dhuhr),
            Prayer::Asr => Some(self.asr),
            Prayer::Maghrib => Some(self.maghrib),
            Prayer::Isha => Some(self.isha),
            Prayer::Sunrise | Prayer::Qiyam => None,
        }
    }
}

const OBLIGATORY: [Prayer; 5] = [
    Prayer::Fajr,
    Prayer::Dhuhr,
    Prayer::Asr,
    Prayer::Maghrib,
    Prayer::Isha,
];

/// A day's worth of ordered events.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub times: PrayerTimes,
    pub events: Vec<Event>,
}

/// Build the timeline for one computed day.
pub fn build(times: PrayerTimes, iqamah: &IqamahOffsets, pre_alert_minutes: u32) -> Timeline {
    let mut events = Vec::new();

    for prayer in OBLIGATORY {
        let Some(at) = times.time(prayer) else {
            continue;
        };

        if pre_alert_minutes > 0 {
            events.push(Event {
                kind: EventKind::PreAlert,
                label: prayer.name_en(),
                at: at - Duration::minutes(i64::from(pre_alert_minutes)),
            });
        }

        events.push(Event {
            kind: EventKind::Prayer,
            label: prayer.name_en(),
            at,
        });

        if let Some(offset) = iqamah.for_prayer(prayer) {
            if offset > 0 {
                events.push(Event {
                    kind: EventKind::Iqamah,
                    label: prayer.name_en(),
                    at: at + Duration::minutes(offset),
                });
            }
        }
    }

    if let Some(fajr) = times.fajr {
        events.push(Event {
            kind: EventKind::AthkarWindow,
            label: "Morning athkar",
            at: fajr + Duration::minutes(30),
        });
    }
    if let Some(asr) = times.asr {
        events.push(Event {
            kind: EventKind::AthkarWindow,
            label: "Evening athkar",
            at: asr + Duration::minutes(30),
        });
    }
    if let Some(isha) = times.isha {
        events.push(Event {
            kind: EventKind::AthkarWindow,
            label: "Sleep athkar",
            at: isha + Duration::minutes(30),
        });
    }

    events.sort_by(|a, b| {
        a.at.cmp(&b.at)
            .then_with(|| a.kind.priority().cmp(&b.kind.priority()))
    });

    Timeline { times, events }
}

impl Timeline {
    /// The first event strictly after `now`.
    pub fn next_after(&self, now: DateTime<Utc>) -> Option<&Event> {
        self.events.iter().find(|event| event.at > now)
    }

    /// Every event that became due in `(after, now]`, in order, provided it is not
    /// older than `tolerance`.
    ///
    /// `next_after` only looks forward, so a machine that slept past several events
    /// would hide all but the one the scheduler was waiting on — and that one is
    /// often the *pre-alert*, leaving the athan itself unannounced. The scheduler
    /// sweeps this range after every wake instead of advancing by `next_after`.
    pub fn due(&self, after: DateTime<Utc>, now: DateTime<Utc>, tolerance: Duration) -> Vec<Event> {
        self.events
            .iter()
            .filter(|event| event.at > after && event.at <= now)
            .filter(|event| now.signed_duration_since(event.at) <= tolerance)
            .cloned()
            .collect()
    }

    /// Only the prayer events, in order.
    pub fn prayers(&self) -> impl Iterator<Item = &Event> {
        self.events
            .iter()
            .filter(|event| event.kind == EventKind::Prayer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prayer::{compute, Coordinates, PrayerConfig};
    use chrono::{NaiveDate, TimeZone};

    fn timeline() -> Timeline {
        let date = NaiveDate::from_ymd_opt(2026, 11, 17).unwrap();
        let times = compute(
            date,
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        build(
            times,
            &IqamahOffsets {
                fajr: 20,
                dhuhr: 18,
                asr: 14,
                maghrib: 8,
                isha: 16,
            },
            10,
        )
    }

    #[test]
    fn events_are_sorted_chronologically() {
        let t = timeline();
        for pair in t.events.windows(2) {
            assert!(pair[0].at <= pair[1].at, "events must be ordered");
        }
    }

    #[test]
    fn contains_five_prayers() {
        assert_eq!(timeline().prayers().count(), 5);
    }

    #[test]
    fn iqamah_is_offset_from_the_prayer() {
        let t = timeline();
        let dhuhr = t
            .prayers()
            .find(|e| e.label == "Dhuhr")
            .expect("dhuhr event");
        let iqamah = t
            .events
            .iter()
            .find(|e| e.kind == EventKind::Iqamah && e.label == "Dhuhr")
            .expect("dhuhr iqamah");
        assert_eq!((iqamah.at - dhuhr.at).num_minutes(), 18);
    }

    #[test]
    fn pre_alert_precedes_the_prayer() {
        let t = timeline();
        let fajr = t.prayers().find(|e| e.label == "Fajr").unwrap();
        let alert = t
            .events
            .iter()
            .find(|e| e.kind == EventKind::PreAlert && e.label == "Fajr")
            .unwrap();
        assert_eq!((fajr.at - alert.at).num_minutes(), 10);
    }

    #[test]
    fn next_after_picks_the_earliest_future_event() {
        let t = timeline();
        // 2026-11-17 01:30 UTC: before the Fajr pre-alert (01:39) and Fajr (01:49).
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 1, 30, 0).unwrap();
        let next = t.next_after(now).expect("an event");
        assert_eq!(next.kind, EventKind::PreAlert);
        assert_eq!(next.label, "Fajr");
    }

    #[test]
    fn athkar_windows_are_present() {
        let t = timeline();
        let labels: Vec<_> = t
            .events
            .iter()
            .filter(|e| e.kind == EventKind::AthkarWindow)
            .map(|e| e.label)
            .collect();
        assert_eq!(labels, ["Morning athkar", "Evening athkar", "Sleep athkar"]);
    }

    #[test]
    fn due_finds_the_athan_that_slept_through_the_pre_alert() {
        // Asleep across the Fajr pre-alert (01:39) and Fajr (01:49), awake at 01:50.
        // The pre-alert is ~11 min stale and must be dropped; the athan is 1 min
        // late and must still fire. `next_after` alone would drop both.
        let t = timeline();
        let after = Utc.with_ymd_and_hms(2026, 11, 17, 1, 38, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 1, 50, 0).unwrap();
        let fired = t.due(after, now, chrono::Duration::seconds(120));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, EventKind::Prayer);
        assert_eq!(fired[0].label, "Fajr");
    }

    #[test]
    fn due_is_empty_when_nothing_has_passed() {
        let t = timeline();
        let at = Utc.with_ymd_and_hms(2026, 11, 17, 1, 30, 0).unwrap();
        assert!(t.due(at, at, chrono::Duration::seconds(120)).is_empty());
    }

    #[test]
    fn due_keeps_every_event_within_tolerance_in_order() {
        let t = timeline();
        let after = Utc.with_ymd_and_hms(2026, 11, 17, 1, 30, 0).unwrap();
        // 01:49:30 is after the Fajr pre-alert (01:39) but the pre-alert is stale.
        let now = Utc.with_ymd_and_hms(2026, 11, 17, 1, 49, 30).unwrap();
        let fired = t.due(after, now, chrono::Duration::seconds(120));
        let labels: Vec<_> = fired.iter().map(|e| e.label).collect();
        assert_eq!(labels, ["Fajr"]);
    }

    #[test]
    fn zero_offset_omits_iqamah() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 17).unwrap();
        let times = compute(
            date,
            Coordinates::new(24.7136, 46.6753),
            &PrayerConfig::default(),
        );
        let t = build(times, &IqamahOffsets::default(), 0);
        assert_eq!(
            t.events
                .iter()
                .filter(|e| e.kind == EventKind::Iqamah)
                .count(),
            0
        );
        assert_eq!(
            t.events
                .iter()
                .filter(|e| e.kind == EventKind::PreAlert)
                .count(),
            0
        );
    }
}
