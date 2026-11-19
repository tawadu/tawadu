//! The playback engine: a rodio output on a BACKGROUND thread, fail-soft
//! when the machine has no audio device. The UI thread never blocks on
//! audio — it sends commands; the audio thread owns the device, the player
//! and the render-cache writes.
//!
//! The pure half ([`plan_for_event`]) turns the routing table + settings
//! into an ordered, edited clip list and is unit-testable without a device.

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use tawadu_core::settings::AudioSettings;

use crate::edit::resolve_stop_after_ms;
use crate::manifest::Edit;
use crate::routing::{resolve_voice, InstalledPack, ResolvedVoice, EVENT_ATHAN, EVENT_ATHKAR};
use crate::sequence::ordered_clips;

/// One clip as it will actually play: where its bytes live, its checksum
/// (the render-cache key input), and the resolved edit + stop cap.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedClip {
    pub pack_id: String,
    pub rel: String,
    pub source_sha256: String,
    pub edit: Edit,
    pub stop_after_ms: u64,
}

/// What the engine plays for one event.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackPlan {
    /// The built-in generated chime (decision 5) — no pack, no licence.
    Chime,
    /// Pack clips in effective order, each with its resolved edit.
    Clips(Vec<PlannedClip>),
}

/// Build the playback plan for an event from the routing table + settings.
/// `None` is the honest empty state: no pack can voice this event.
/// The user's athan stop-cap wins over the manifest edit (same precedence
/// as mode overrides); other events use the manifest's own stop.
pub fn plan_for_event(
    event: &str,
    audio: &AudioSettings,
    packs: &[InstalledPack],
) -> Option<PlaybackPlan> {
    match resolve_voice(event, audio, packs)? {
        ResolvedVoice::BuiltinChime => Some(PlaybackPlan::Chime),
        ResolvedVoice::Pack { pack, mode } => {
            // Athkar has its own sequence when the pack defines one; the
            // voiced prayer events use the named prayer sequence if present,
            // else the primary athan sequence.
            let sequence = if event == EVENT_ATHKAR {
                pack.manifest
                    .sequences
                    .get(EVENT_ATHKAR)
                    .or_else(|| pack.manifest.primary_sequence())?
            } else {
                pack.manifest.primary_sequence()?
            };
            let ordered = ordered_clips(&sequence.clips, mode, audio.shuffle_seed);
            let clips = ordered
                .into_iter()
                .map(|rel| {
                    let edit = pack.manifest.edits.get(&rel).copied().unwrap_or_default();
                    // The user's stop-cap is the ATHAN ask (v1 scope); other
                    // events keep the manifest's own stop.
                    let user_stop_s = if event == EVENT_ATHAN {
                        audio.athan_stop_after_s
                    } else {
                        0
                    };
                    let stop_after_ms = resolve_stop_after_ms(user_stop_s, &edit);
                    let source_sha256 = pack
                        .manifest
                        .files
                        .get(&rel)
                        .map(|entry| entry.sha256.clone())
                        .unwrap_or_default();
                    PlannedClip {
                        pack_id: pack.id.clone(),
                        rel,
                        source_sha256,
                        edit,
                        stop_after_ms,
                    }
                })
                .collect();
            Some(PlaybackPlan::Clips(clips))
        }
    }
}

/// A play command: the plan plus where the library and render cache live.
#[derive(Debug, Clone)]
pub struct PlayRequest {
    pub plan: PlaybackPlan,
    pub volume: f32,
    pub library_dir: PathBuf,
    pub cache_dir: PathBuf,
}

/// The device thread's resolved state. `Starting` is honest: the device
/// has not answered yet. `Unavailable` means playback is disabled and the
/// app keeps running — banners and toasts carry on regardless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Starting,
    Ready,
    Unavailable,
}

enum Command {
    Play(PlayRequest),
    Stop,
    Volume(f32),
    Shutdown,
}

/// Handle to the background audio thread. Cheap to clone; every method is
/// non-blocking and never panics when the device is absent or the thread
/// has shut down.
#[derive(Clone)]
pub struct AudioEngine {
    tx: mpsc::Sender<Command>,
    status: Arc<Mutex<EngineStatus>>,
}

impl AudioEngine {
    /// Spawn the audio thread. The device is opened THERE, not here: a
    /// machine with no output device logs and goes `Unavailable`, never
    /// fatal.
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        let status = Arc::new(Mutex::new(EngineStatus::Starting));
        let thread_status = Arc::clone(&status);
        std::thread::Builder::new()
            .name("tawadu-audio".into())
            .spawn(move || audio_thread(rx, thread_status))
            .expect("spawn the audio thread");
        Self { tx, status }
    }

    pub fn play(&self, request: PlayRequest) {
        let _ = self.tx.send(Command::Play(request));
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.tx.send(Command::Volume(volume));
    }

    pub fn status(&self) -> EngineStatus {
        *self
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}

/// The audio thread: owns the device sink and the player for their whole
/// lifetime (dropping either stops playback). Commands arrive over the
/// channel; nothing here ever touches the UI thread.
fn audio_thread(rx: mpsc::Receiver<Command>, status: Arc<Mutex<EngineStatus>>) {
    let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(sink) => {
            *status.lock().unwrap_or_else(|p| p.into_inner()) = EngineStatus::Ready;
            Some(sink)
        }
        Err(error) => {
            // Fail-soft: no device (CI, headless, muted hardware) disables
            // playback only. Banners and toasts are the correctness floor.
            log::warn!("audio output unavailable ({error}); playback disabled");
            *status.lock().unwrap_or_else(|p| p.into_inner()) = EngineStatus::Unavailable;
            None
        }
    };
    let player = sink
        .as_ref()
        .map(|sink| rodio::Player::connect_new(sink.mixer()));
    for command in rx {
        match command {
            Command::Play(request) => {
                if let Some(player) = &player {
                    play_request(player, request);
                }
            }
            Command::Stop => {
                if let Some(player) = &player {
                    player.stop();
                }
            }
            Command::Volume(volume) => {
                if let Some(player) = &player {
                    player.set_volume(volume.clamp(0.0, 1.0));
                }
            }
            Command::Shutdown => break,
        }
    }
}

/// Play one request: stop whatever is playing, set the volume, then queue
/// the plan. Clip renders happen here (background thread) and are cached by
/// (source checksum + edit + stop cap) — a stale render is never played.
fn play_request(player: &rodio::Player, request: PlayRequest) {
    player.stop();
    player.set_volume(request.volume.clamp(0.0, 1.0));
    match request.plan {
        PlaybackPlan::Chime => {
            // The mixer resamples to the device rate; 44.1 kHz is the
            // chime's synthesis rate, not a claim about the device.
            let rate = 44_100u32;
            let samples = crate::chime::chime_samples(rate);
            player.append(rodio::buffer::SamplesBuffer::new(
                std::num::NonZeroU16::new(1).expect("mono"),
                std::num::NonZeroU32::new(rate).expect("nonzero rate"),
                samples,
            ));
        }
        PlaybackPlan::Clips(ref clips) => {
            for clip in clips {
                let Some(path) = rendered_clip_path(&request, clip) else {
                    continue;
                };
                let file = match std::fs::File::open(&path) {
                    Ok(file) => file,
                    Err(error) => {
                        log::warn!("cannot open clip {}: {error}", path.display());
                        continue;
                    }
                };
                match rodio::Decoder::new(std::io::BufReader::new(file)) {
                    Ok(decoder) => player.append(decoder),
                    Err(error) => {
                        log::warn!("cannot decode clip {}: {error}", path.display());
                    }
                }
            }
        }
    }
}

/// The cache file for a planned clip, rendering it on first use. A render
/// failure skips the clip (fail-soft) — the unedited source is NOT played
/// instead, because that would silently ignore the user's trim.
fn rendered_clip_path(request: &PlayRequest, clip: &PlannedClip) -> Option<PathBuf> {
    let key = crate::render::cache_key(&clip.source_sha256, &clip.edit, clip.stop_after_ms);
    let cache_file = request
        .cache_dir
        .join(&clip.pack_id)
        .join(format!("{key}.wav"));
    if cache_file.exists() {
        return Some(cache_file);
    }
    let source = request.library_dir.join(&clip.pack_id).join(&clip.rel);
    if let Some(parent) = cache_file.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            log::warn!("cannot create render cache dir: {error}");
            return None;
        }
    }
    match crate::render::render_clip(&source, &clip.edit, clip.stop_after_ms, &cache_file) {
        Ok(()) => Some(cache_file),
        Err(error) => {
            log::warn!("render failed for {}: {error}", source.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::routing::{EVENT_ATHAN, EVENT_CHIME, EVENT_IQAMAH};
    use std::collections::BTreeMap;
    use tawadu_core::settings::{EventVoice, PlaybackMode};

    /// A pack with three athan clips, a shuffle primary sequence, an
    /// athkar clip + sequence, and a manifest stop edit on clip a.
    fn pack(id: &str) -> InstalledPack {
        let json = r#"{
            "format": "tawadu-pack/1",
            "name": "Plan Pack",
            "reciter": "A Reciter",
            "license": "CC0-1.0",
            "files": {
                "athan/a.ogg": { "sha256": "aa", "license": "CC0-1.0" },
                "athan/b.ogg": { "sha256": "bb", "license": "CC0-1.0" },
                "athan/c.ogg": { "sha256": "cc", "license": "CC0-1.0" },
                "athkar/m.ogg": { "sha256": "dd", "license": "CC0-1.0" }
            },
            "sequences": {
                "fajr": { "mode": "shuffle", "clips": ["athan/a.ogg", "athan/b.ogg", "athan/c.ogg"] },
                "athkar": { "mode": "sequential", "clips": ["athkar/m.ogg"] }
            },
            "edits": {
                "athan/a.ogg": { "fade_in_ms": 120, "stop_after_ms": 10000 }
            }
        }"#;
        let manifest = Manifest::parse(json).expect("valid manifest");
        let mut durations_ms = BTreeMap::new();
        for rel in manifest.files.keys() {
            durations_ms.insert(rel.clone(), Some(60_000));
        }
        InstalledPack {
            id: id.into(),
            manifest,
            durations_ms,
            bundled: false,
        }
    }

    fn clips(plan: Option<PlaybackPlan>) -> Vec<PlannedClip> {
        match plan.expect("a plan") {
            PlaybackPlan::Clips(clips) => clips,
            other => panic!("expected clips, got {other:?}"),
        }
    }

    #[test]
    fn no_packs_means_no_plan_for_pack_events() {
        let audio = AudioSettings::default();
        assert!(plan_for_event(EVENT_ATHAN, &audio, &[]).is_none());
        assert!(plan_for_event(EVENT_IQAMAH, &audio, &[]).is_none());
        assert!(plan_for_event(EVENT_ATHKAR, &audio, &[]).is_none());
    }

    #[test]
    fn the_chime_plans_without_any_pack() {
        let audio = AudioSettings::default();
        assert_eq!(
            plan_for_event(EVENT_CHIME, &audio, &[]),
            Some(PlaybackPlan::Chime)
        );
    }

    #[test]
    fn the_user_stop_cap_wins_over_the_manifest_edit() {
        let packs = [pack("p")];
        let audio = AudioSettings {
            athan_stop_after_s: 30,
            ..AudioSettings::default()
        };
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        let a = planned.iter().find(|c| c.rel == "athan/a.ogg").unwrap();
        assert_eq!(a.stop_after_ms, 30_000);
        assert_eq!(a.edit.fade_in_ms, 120, "the manifest edit still travels");
        assert_eq!(a.source_sha256, "aa");
    }

    #[test]
    fn without_a_user_trim_the_manifest_stop_applies() {
        let packs = [pack("p")];
        let audio = AudioSettings::default();
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        let a = planned.iter().find(|c| c.rel == "athan/a.ogg").unwrap();
        assert_eq!(a.stop_after_ms, 10_000);
        let b = planned.iter().find(|c| c.rel == "athan/b.ogg").unwrap();
        assert_eq!(b.stop_after_ms, 0, "no edit, no cap");
        assert_eq!(b.edit, Edit::default());
    }

    #[test]
    fn the_stop_cap_is_athan_only_in_v1() {
        let packs = [pack("p")];
        let audio = AudioSettings {
            athan_stop_after_s: 30,
            ..AudioSettings::default()
        };
        // Iqamah voices the same primary sequence but keeps the manifest
        // stop — the user's ask was "stop the ATHAN after N seconds".
        let planned = clips(plan_for_event(EVENT_IQAMAH, &audio, &packs));
        let a = planned.iter().find(|c| c.rel == "athan/a.ogg").unwrap();
        assert_eq!(a.stop_after_ms, 10_000);
    }

    #[test]
    fn shuffle_order_is_the_seeded_order() {
        let packs = [pack("p")];
        let audio = AudioSettings::default();
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        let rels: Vec<&str> = planned.iter().map(|c| c.rel.as_str()).collect();
        let source = vec![
            "athan/a.ogg".to_string(),
            "athan/b.ogg".to_string(),
            "athan/c.ogg".to_string(),
        ];
        let expected = ordered_clips(&source, PlaybackMode::Shuffle, audio.shuffle_seed);
        assert_eq!(rels, expected);
    }

    #[test]
    fn a_mode_override_changes_the_order() {
        let packs = [pack("p")];
        let mut audio = AudioSettings::default();
        audio.event_voices.insert(
            EVENT_ATHAN.into(),
            EventVoice {
                pack: None,
                mode_override: Some(PlaybackMode::Sequential),
            },
        );
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        let rels: Vec<&str> = planned.iter().map(|c| c.rel.as_str()).collect();
        assert_eq!(rels, ["athan/a.ogg", "athan/b.ogg", "athan/c.ogg"]);
    }

    #[test]
    fn a_per_event_choice_plans_that_pack() {
        let packs = [pack("first"), pack("second")];
        let mut audio = AudioSettings::default();
        audio.event_voices.insert(
            EVENT_ATHAN.into(),
            EventVoice {
                pack: Some("second".into()),
                mode_override: None,
            },
        );
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        assert!(planned.iter().all(|c| c.pack_id == "second"));
    }

    #[test]
    fn athkar_plans_the_athkar_sequence() {
        let packs = [pack("p")];
        let audio = AudioSettings::default();
        let planned = clips(plan_for_event(EVENT_ATHKAR, &audio, &packs));
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].rel, "athkar/m.ogg");
        assert_eq!(planned[0].source_sha256, "dd");
    }

    #[test]
    fn a_removed_choice_falls_back_to_the_default_chain() {
        let packs = [pack("first")];
        let mut audio = AudioSettings::default();
        audio.event_voices.insert(
            EVENT_ATHAN.into(),
            EventVoice {
                pack: Some("gone".into()),
                mode_override: None,
            },
        );
        let planned = clips(plan_for_event(EVENT_ATHAN, &audio, &packs));
        assert!(planned.iter().all(|c| c.pack_id == "first"));
    }

    #[test]
    fn the_engine_is_fail_soft_with_or_without_a_device() {
        // CI runners have no audio device; a dev machine may have one.
        // Either way: no panic, no hang, an honest resolved status.
        let engine = AudioEngine::spawn();
        engine.set_volume(0.5);
        engine.play(PlayRequest {
            plan: PlaybackPlan::Chime,
            volume: 0.5,
            library_dir: PathBuf::from("does-not-exist"),
            cache_dir: PathBuf::from("does-not-exist-either"),
        });
        engine.stop();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let status = loop {
            let status = engine.status();
            if status != EngineStatus::Starting {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "engine status never resolved"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(matches!(
            status,
            EngineStatus::Ready | EngineStatus::Unavailable
        ));
        engine.shutdown();
        // Commands after shutdown are dropped, never panics.
        engine.stop();
        engine.set_volume(0.2);
    }
}
