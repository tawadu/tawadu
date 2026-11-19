//! Per-event voice routing and mode-override precedence — the routing table
//! the scheduler and the settings UI both read. Pure logic over the installed
//! pack list and the user's [`AudioSettings`]; no IO, no device.
//!
//! Decisions encoded here (PIPELINE.md Phase 3):
//! - 2: default = the marked default pack, else the FIRST installed pack,
//!   else the honest empty state (`None`) — never a guessed pack.
//! - 5: the v1 chime is built-in and licence-free; it is the chime default.
//! - 7: a user mode override wins over the manifest; "Reset to pack default"
//!   exists only while an override exists.

use std::collections::BTreeMap;

use tawadu_core::settings::{AudioSettings, PlaybackMode};

use crate::manifest::Manifest;

/// Event keys, matching `core::notify::NotificationKind::key()` for the four
/// voiced kinds (the `system` kind has no voice).
pub const EVENT_ATHAN: &str = "athan";
pub const EVENT_IQAMAH: &str = "iqamah";
pub const EVENT_ATHKAR: &str = "athkar";
pub const EVENT_CHIME: &str = "chime";

/// The built-in generated chime's voice id (not a pack).
pub const BUILTIN_CHIME_ID: &str = "builtin:chime";

/// An installed pack: its manifest plus the durations PROBED at import
/// (decision 8 — `None` means unknown and renders غير متاح, never a guess).
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledPack {
    pub id: String,
    pub manifest: Manifest,
    /// Per-file probed duration in milliseconds; `None` = probe unknown.
    pub durations_ms: BTreeMap<String, Option<u64>>,
    /// Bundled packs are the offline floor: they cannot be removed.
    pub bundled: bool,
}

/// What actually plays for an event.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedVoice<'a> {
    /// The built-in generated chime (decision 5).
    BuiltinChime,
    /// A pack's clips, in the effective mode.
    Pack {
        pack: &'a InstalledPack,
        mode: PlaybackMode,
    },
}

/// Resolve the voice for one event. `None` is the honest empty state: no
/// pack can voice this event (no packs installed, or the chosen pack has no
/// clips for it). A chosen pack that was removed falls back to the default
/// chain — the UI reports the fallback, never silent.
pub fn resolve_voice<'a>(
    event: &str,
    audio: &AudioSettings,
    packs: &'a [InstalledPack],
) -> Option<ResolvedVoice<'a>> {
    let choice = audio
        .event_voices
        .get(event)
        .and_then(|voice| voice.pack.as_deref());

    // The chime has a licence-free floor: the built-in generated chime.
    if event == EVENT_CHIME && (choice.is_none() || choice == Some(BUILTIN_CHIME_ID)) {
        return Some(ResolvedVoice::BuiltinChime);
    }

    // Candidate chain: the explicit per-event choice, then the marked
    // default, then the first installed pack. A removed pack simply falls
    // through to the next candidate — the UI reports the fallback.
    let candidates = [
        choice,
        audio.default_pack.as_deref(),
        packs.first().map(|pack| pack.id.as_str()),
    ];
    for id in candidates.into_iter().flatten() {
        let Some(pack) = packs.iter().find(|pack| pack.id == id) else {
            continue;
        };
        if !pack_has_content(event, pack) {
            continue;
        }
        return Some(ResolvedVoice::Pack {
            pack,
            mode: effective_mode(event, pack, audio),
        });
    }

    // No pack can voice this event. For the chime the built-in is still
    // available; for pack events this is the honest empty state.
    if event == EVENT_CHIME {
        return Some(ResolvedVoice::BuiltinChime);
    }
    None
}

/// Does the pack carry clips for this event kind? Athan/iqamah/chime voice
/// from the pack's primary athan sequence; athkar needs athkar clips.
fn pack_has_content(event: &str, pack: &InstalledPack) -> bool {
    match event {
        EVENT_ATHKAR => pack.manifest.has_athkar(),
        _ => pack.manifest.primary_sequence().is_some(),
    }
}

/// The effective playback mode: user override first, then the manifest's
/// sequence default, then sequential.
pub fn effective_mode(event: &str, pack: &InstalledPack, audio: &AudioSettings) -> PlaybackMode {
    if let Some(mode) = audio
        .event_voices
        .get(event)
        .and_then(|voice| voice.mode_override)
    {
        return mode;
    }
    manifest_mode(event, pack)
}

/// The manifest's own default for an event: the named sequence's mode when
/// one exists (athkar may define its own), else the primary sequence's.
fn manifest_mode(event: &str, pack: &InstalledPack) -> PlaybackMode {
    if event == EVENT_ATHKAR {
        if let Some(sequence) = pack.manifest.sequences.get(EVENT_ATHKAR) {
            return sequence.mode;
        }
    }
    pack.manifest
        .primary_sequence()
        .map(|sequence| sequence.mode)
        .unwrap_or(PlaybackMode::Sequential)
}

/// True only while a user override exists — the sole condition under which
/// "Reset to pack default" is offered (decision 7).
pub fn has_mode_override(event: &str, audio: &AudioSettings) -> bool {
    audio
        .event_voices
        .get(event)
        .and_then(|voice| voice.mode_override)
        .is_some()
}

/// Drop the user's override; the manifest default applies again.
pub fn reset_mode_override(event: &str, audio: &mut AudioSettings) {
    if let Some(voice) = audio.event_voices.get_mut(event) {
        voice.mode_override = None;
    }
}

/// The pack card's duration: total playing time of the PRIMARY athan
/// sequence (decision 1). If any clip's probed duration is unknown the
/// total is unknown — `None`, rendered غير متاح, never a partial guess.
pub fn pack_duration_ms(pack: &InstalledPack) -> Option<u64> {
    let sequence = pack.manifest.primary_sequence()?;
    let mut total = 0u64;
    for clip in &sequence.clips {
        total = total.checked_add(pack.durations_ms.get(clip).copied().flatten()?)?;
    }
    Some(total)
}

/// `m:ss` for a known duration (Western numerals, Quill §2 rule 3).
pub fn format_duration_ms(ms: u64) -> String {
    let seconds = ms / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{FileEntry, Sequence};

    fn pack(id: &str, mode: PlaybackMode, with_athkar: bool) -> InstalledPack {
        let mut files = BTreeMap::new();
        files.insert(
            "athan/full.ogg".to_string(),
            FileEntry {
                sha256: "aa".into(),
                license: "CC0-1.0".into(),
            },
        );
        let mut sequences = BTreeMap::new();
        for prayer in crate::manifest::PRAYERS {
            sequences.insert(
                prayer.to_string(),
                Sequence {
                    mode,
                    clips: vec!["athan/full.ogg".into()],
                },
            );
        }
        let mut durations_ms = BTreeMap::new();
        durations_ms.insert("athan/full.ogg".to_string(), Some(154_018));
        if with_athkar {
            files.insert(
                "athkar/morning.ogg".to_string(),
                FileEntry {
                    sha256: "bb".into(),
                    license: "CC0-1.0".into(),
                },
            );
            durations_ms.insert("athkar/morning.ogg".to_string(), Some(60_000));
        }
        InstalledPack {
            id: id.into(),
            manifest: Manifest {
                format: crate::manifest::FORMAT_ID.into(),
                name: id.into(),
                name_ar: None,
                reciter: "Reciter".into(),
                reciter_ar: None,
                description: String::new(),
                license: "CC0-1.0".into(),
                attribution: String::new(),
                source_url: String::new(),
                version: String::new(),
                updated: String::new(),
                files,
                sequences,
                edits: BTreeMap::new(),
            },
            durations_ms,
            bundled: false,
        }
    }

    fn audio_with(event: &str, choice: Option<&str>, mode: Option<PlaybackMode>) -> AudioSettings {
        let mut audio = AudioSettings::default();
        audio.event_voices.insert(
            event.into(),
            tawadu_core::settings::EventVoice {
                pack: choice.map(str::to_string),
                mode_override: mode,
            },
        );
        audio
    }

    // ---- the routing table ----

    #[test]
    fn no_packs_means_no_voice_for_pack_events() {
        let audio = AudioSettings::default();
        assert!(resolve_voice(EVENT_ATHAN, &audio, &[]).is_none());
        assert!(resolve_voice(EVENT_IQAMAH, &audio, &[]).is_none());
        assert!(resolve_voice(EVENT_ATHKAR, &audio, &[]).is_none());
    }

    #[test]
    fn the_builtin_chime_needs_no_pack() {
        let audio = AudioSettings::default();
        assert_eq!(
            resolve_voice(EVENT_CHIME, &audio, &[]),
            Some(ResolvedVoice::BuiltinChime)
        );
    }

    #[test]
    fn default_is_the_first_installed_pack_when_none_marked() {
        let packs = [
            pack("first", PlaybackMode::Sequential, false),
            pack("second", PlaybackMode::Sequential, false),
        ];
        let audio = AudioSettings::default();
        match resolve_voice(EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "first"),
            other => panic!("expected the first pack, got {other:?}"),
        }
    }

    #[test]
    fn a_marked_default_wins_over_install_order() {
        let packs = [
            pack("first", PlaybackMode::Sequential, false),
            pack("second", PlaybackMode::Sequential, false),
        ];
        let audio = AudioSettings {
            default_pack: Some("second".into()),
            ..AudioSettings::default()
        };
        match resolve_voice(EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "second"),
            other => panic!("expected the marked default, got {other:?}"),
        }
    }

    #[test]
    fn a_per_event_choice_wins_over_the_default() {
        let packs = [
            pack("first", PlaybackMode::Sequential, false),
            pack("second", PlaybackMode::Sequential, false),
        ];
        let mut audio = audio_with(EVENT_ATHAN, Some("second"), None);
        audio.default_pack = Some("first".into());
        match resolve_voice(EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "second"),
            other => panic!("expected the per-event choice, got {other:?}"),
        }
        // Other events still use the default.
        match resolve_voice(EVENT_IQAMAH, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "first"),
            other => panic!("expected the default for iqamah, got {other:?}"),
        }
    }

    #[test]
    fn a_removed_choice_falls_back_to_the_default_chain() {
        let packs = [pack("first", PlaybackMode::Sequential, false)];
        let audio = audio_with(EVENT_ATHAN, Some("gone"), None);
        match resolve_voice(EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "first"),
            other => panic!("expected fallback to the first pack, got {other:?}"),
        }
    }

    #[test]
    fn a_marked_default_that_was_removed_falls_back_to_the_first_pack() {
        let packs = [pack("first", PlaybackMode::Sequential, false)];
        let audio = AudioSettings {
            default_pack: Some("gone".into()),
            ..AudioSettings::default()
        };
        match resolve_voice(EVENT_ATHAN, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "first"),
            other => panic!("expected fallback, got {other:?}"),
        }
    }

    #[test]
    fn the_chime_can_be_given_a_pack_voice() {
        let packs = [pack("announce", PlaybackMode::Sequential, false)];
        let audio = audio_with(EVENT_CHIME, Some("announce"), None);
        match resolve_voice(EVENT_CHIME, &audio, &packs) {
            Some(ResolvedVoice::Pack { pack, .. }) => assert_eq!(pack.id, "announce"),
            other => panic!("expected the chosen pack for the chime, got {other:?}"),
        }
    }

    #[test]
    fn the_explicit_builtin_chime_id_resolves_without_packs() {
        let audio = audio_with(EVENT_CHIME, Some(BUILTIN_CHIME_ID), None);
        assert_eq!(
            resolve_voice(EVENT_CHIME, &audio, &[]),
            Some(ResolvedVoice::BuiltinChime)
        );
    }

    #[test]
    fn athkar_needs_athkar_clips() {
        let packs = [pack("athan-only", PlaybackMode::Sequential, false)];
        let audio = AudioSettings::default();
        assert!(resolve_voice(EVENT_ATHKAR, &audio, &packs).is_none());

        let packs = [pack("with-athkar", PlaybackMode::Sequential, true)];
        assert!(matches!(
            resolve_voice(EVENT_ATHKAR, &audio, &packs),
            Some(ResolvedVoice::Pack { .. })
        ));
    }

    // ---- mode-override precedence (decision 7) ----

    #[test]
    fn the_manifest_mode_applies_without_an_override() {
        let packs = [pack("shuffler", PlaybackMode::Shuffle, false)];
        let audio = AudioSettings::default();
        assert_eq!(
            effective_mode(EVENT_ATHAN, &packs[0], &audio),
            PlaybackMode::Shuffle
        );
        assert!(!has_mode_override(EVENT_ATHAN, &audio));
    }

    #[test]
    fn a_user_override_wins_over_the_manifest() {
        let packs = [pack("shuffler", PlaybackMode::Shuffle, false)];
        let audio = audio_with(EVENT_ATHAN, None, Some(PlaybackMode::Sequential));
        assert_eq!(
            effective_mode(EVENT_ATHAN, &packs[0], &audio),
            PlaybackMode::Sequential
        );
        assert!(has_mode_override(EVENT_ATHAN, &audio));
    }

    #[test]
    fn reset_removes_the_override_and_the_manifest_default_returns() {
        let packs = [pack("shuffler", PlaybackMode::Shuffle, false)];
        let mut audio = audio_with(EVENT_ATHAN, None, Some(PlaybackMode::Sequential));
        assert!(has_mode_override(EVENT_ATHAN, &audio));
        reset_mode_override(EVENT_ATHAN, &mut audio);
        assert!(!has_mode_override(EVENT_ATHAN, &audio));
        assert_eq!(
            effective_mode(EVENT_ATHAN, &packs[0], &audio),
            PlaybackMode::Shuffle
        );
    }

    #[test]
    fn an_override_on_one_event_does_not_leak_to_another() {
        let packs = [pack("p", PlaybackMode::Sequential, false)];
        let audio = audio_with(EVENT_ATHAN, None, Some(PlaybackMode::Shuffle));
        assert!(has_mode_override(EVENT_ATHAN, &audio));
        assert!(!has_mode_override(EVENT_IQAMAH, &audio));
        assert_eq!(
            effective_mode(EVENT_IQAMAH, &packs[0], &audio),
            PlaybackMode::Sequential
        );
    }

    // ---- duration (decisions 1 + 8) ----

    #[test]
    fn pack_duration_is_the_primary_sequence_total() {
        let p = pack("p", PlaybackMode::Sequential, true);
        // Primary athan sequence = one 154.018 s clip; the athkar clip is
        // NOT part of the athan total.
        assert_eq!(pack_duration_ms(&p), Some(154_018));
    }

    #[test]
    fn an_unknown_clip_duration_makes_the_total_unknown() {
        let mut p = pack("p", PlaybackMode::Sequential, false);
        p.durations_ms.insert("athan/full.ogg".into(), None);
        assert_eq!(pack_duration_ms(&p), None);
    }

    #[test]
    fn durations_format_as_m_ss() {
        assert_eq!(format_duration_ms(154_018), "2:34");
        assert_eq!(format_duration_ms(42_098), "0:42");
        assert_eq!(format_duration_ms(0), "0:00");
        assert_eq!(format_duration_ms(61_000), "1:01");
    }
}
