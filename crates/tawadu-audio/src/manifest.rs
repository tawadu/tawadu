//! Voice-pack manifest (`docs/voice-pack-format.md`, format `tawadu-pack/1`).
//!
//! Validation is all-or-nothing: every imported file needs a sha256 AND a
//! licence entry, or the whole pack is rejected. `name_ar` / `reciter_ar`
//! are optional (format addendum; Phase 3 decision 6) — the importer
//! tolerates their absence and falls back to `name` / `reciter`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tawadu_core::settings::PlaybackMode;

use crate::error::ImportError;

/// The only format id this importer understands.
pub const FORMAT_ID: &str = "tawadu-pack/1";

/// The five prayer sequence keys, in prayer order.
pub const PRAYERS: [&str; 5] = ["fajr", "dhuhr", "asr", "maghrib", "isha"];

/// One recorded file: its checksum and its licence. Both are mandatory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub sha256: String,
    pub license: String,
}

/// A named playback sequence: an ordered clip list plus its default mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence {
    pub mode: PlaybackMode,
    pub clips: Vec<String>,
}

/// Non-destructive per-file edits, rendered to a cache file on first play.
/// All values milliseconds; 0 = unset. `stop_after_ms` 0 = full recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Edit {
    #[serde(default)]
    pub trim_start_ms: u64,
    #[serde(default)]
    pub trim_end_ms: u64,
    #[serde(default)]
    pub fade_in_ms: u64,
    #[serde(default)]
    pub fade_out_ms: u64,
    #[serde(default)]
    pub stop_after_ms: u64,
}

/// The pack manifest. Unknown fields are tolerated (forward compatibility);
/// unknown folders are ignored by the v1 importer (a `chime/` folder is a
/// future format addendum).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub name: String,
    #[serde(default)]
    pub name_ar: Option<String>,
    pub reciter: String,
    #[serde(default)]
    pub reciter_ar: Option<String>,
    #[serde(default)]
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub attribution: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub updated: String,
    pub files: BTreeMap<String, FileEntry>,
    #[serde(default)]
    pub sequences: BTreeMap<String, Sequence>,
    #[serde(default)]
    pub edits: BTreeMap<String, Edit>,
}

impl Manifest {
    /// Parse `manifest.json` text. Unreadable JSON is `ManifestInvalid`.
    pub fn parse(json: &str) -> Result<Manifest, ImportError> {
        serde_json::from_str(json).map_err(|_| ImportError::ManifestInvalid {
            file: "manifest.json".to_string(),
        })
    }

    /// Structural validation. Does NOT touch the filesystem — checksum
    /// verification and duration probing happen in the importer.
    pub fn validate(&self) -> Result<(), ImportError> {
        let invalid = || ImportError::ManifestInvalid {
            file: "manifest.json".to_string(),
        };
        if self.format != FORMAT_ID {
            return Err(invalid());
        }
        if self.name.trim().is_empty() || self.reciter.trim().is_empty() {
            return Err(invalid());
        }
        // The pack-level licence is mandatory: no licence entry → rejected.
        if self.license.trim().is_empty() {
            return Err(ImportError::LicenseMissing {
                file: "manifest.json".to_string(),
            });
        }
        // Path safety: nothing may point outside the pack folder.
        let safe = |path: &str| {
            !path.contains("..")
                && !path.starts_with('/')
                && !path.starts_with('\\')
                && !path.contains(':')
        };
        for path in self.files.keys() {
            if !safe(path) {
                return Err(invalid());
            }
        }
        // Every v1 audio file needs a licence and a recorded checksum.
        // Unknown folders (a future `chime/`) are ignored, not validated.
        for (path, entry) in self.audio_files() {
            if entry.license.trim().is_empty() {
                return Err(ImportError::LicenseMissing {
                    file: path.to_string(),
                });
            }
            if entry.sha256.trim().is_empty() {
                return Err(invalid());
            }
        }
        for sequence in self.sequences.values() {
            if sequence.clips.is_empty() {
                return Err(invalid());
            }
            for clip in &sequence.clips {
                if !safe(clip) || !Self::is_v1_audio_path(clip) || !self.files.contains_key(clip) {
                    return Err(invalid());
                }
            }
        }
        for path in self.edits.keys() {
            if !safe(path) || !self.files.contains_key(path) {
                return Err(invalid());
            }
        }
        Ok(())
    }

    /// Is this path one the v1 importer handles (`athan/…` or `athkar/…`)?
    /// Unknown folders are ignored, per the format doc.
    pub fn is_v1_audio_path(path: &str) -> bool {
        (path.starts_with("athan/") || path.starts_with("athkar/"))
            && path.len() > "athan/".len()
            && !path.contains("..")
    }

    /// The files the v1 importer verifies and copies, in stable order.
    pub fn audio_files(&self) -> Vec<(&str, &FileEntry)> {
        self.files
            .iter()
            .filter(|(path, _)| Self::is_v1_audio_path(path))
            .map(|(path, entry)| (path.as_str(), entry))
            .collect()
    }

    /// The sequence for a prayer key, if the pack defines one.
    pub fn sequence_for(&self, prayer: &str) -> Option<&Sequence> {
        self.sequences.get(prayer)
    }

    /// The pack's primary athan sequence: the first prayer (in prayer order)
    /// that has one, else the first defined sequence. `None` for a pack with
    /// no sequences at all.
    pub fn primary_sequence(&self) -> Option<&Sequence> {
        PRAYERS
            .iter()
            .find_map(|prayer| self.sequences.get(*prayer))
            .or_else(|| self.sequences.values().next())
    }

    /// Does the pack carry any athkar clips?
    pub fn has_athkar(&self) -> bool {
        self.files.keys().any(|path| path.starts_with("athkar/"))
    }

    /// Arabic display name, falling back to `name` when absent (decision 6).
    pub fn name_ar_or_name(&self) -> &str {
        self.name_ar.as_deref().unwrap_or(&self.name)
    }

    /// Arabic reciter, falling back to `reciter` when absent (decision 6).
    pub fn reciter_ar_or_reciter(&self) -> &str {
        self.reciter_ar.as_deref().unwrap_or(&self.reciter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid manifest: one athan clip, one sequence, full licence.
    fn valid_json() -> String {
        r#"{
            "format": "tawadu-pack/1",
            "name": "Test Pack",
            "reciter": "A Reciter",
            "license": "CC0-1.0",
            "attribution": "Recorded by A Reciter",
            "files": {
                "athan/fajr.mp3": { "sha256": "ab12", "license": "CC0-1.0" }
            },
            "sequences": {
                "fajr": { "mode": "sequential", "clips": ["athan/fajr.mp3"] }
            }
        }"#
        .to_string()
    }

    fn manifest_from(json: &str) -> Manifest {
        Manifest::parse(json).expect("valid manifest must parse")
    }

    #[test]
    fn a_valid_manifest_parses_and_validates() {
        let m = manifest_from(&valid_json());
        assert_eq!(m.format, FORMAT_ID);
        assert_eq!(m.name, "Test Pack");
        assert!(m.validate().is_ok());
    }

    #[test]
    fn optional_arabic_fields_are_tolerated_and_fall_back() {
        let m = manifest_from(&valid_json());
        assert_eq!(m.name_ar, None);
        assert_eq!(m.reciter_ar, None);
        assert_eq!(m.name_ar_or_name(), "Test Pack");
        assert_eq!(m.reciter_ar_or_reciter(), "A Reciter");

        let with_ar = valid_json().replace(
            "\"name\": \"Test Pack\",",
            "\"name\": \"Test Pack\", \"name_ar\": \"حزمة اختبار\", \"reciter_ar\": \"قارئ\",",
        );
        let m = manifest_from(&with_ar);
        assert_eq!(m.name_ar_or_name(), "حزمة اختبار");
        assert_eq!(m.reciter_ar_or_reciter(), "قارئ");
        assert!(m.validate().is_ok());
    }

    #[test]
    fn broken_json_is_manifest_invalid() {
        let err = Manifest::parse("{ not json").unwrap_err();
        assert!(matches!(err, ImportError::ManifestInvalid { .. }));
    }

    #[test]
    fn a_foreign_format_id_is_rejected() {
        let json = valid_json().replace("tawadu-pack/1", "tawadu-pack/99");
        let m = manifest_from(&json);
        assert!(matches!(
            m.validate(),
            Err(ImportError::ManifestInvalid { .. })
        ));
    }

    #[test]
    fn a_missing_pack_licence_is_rejected_with_the_honest_error() {
        let json = valid_json().replace("\"license\": \"CC0-1.0\",", "\"license\": \"\",");
        let m = manifest_from(&json);
        match m.validate() {
            Err(ImportError::LicenseMissing { file }) => assert_eq!(file, "manifest.json"),
            other => panic!("expected LicenseMissing, got {other:?}"),
        }
    }

    #[test]
    fn a_file_without_a_licence_rejects_the_whole_pack() {
        let json = valid_json().replace(
            "{ \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" }",
            "{ \"sha256\": \"ab12\", \"license\": \"\" }",
        );
        let m = manifest_from(&json);
        match m.validate() {
            Err(ImportError::LicenseMissing { file }) => assert_eq!(file, "athan/fajr.mp3"),
            other => panic!("expected LicenseMissing for the clip, got {other:?}"),
        }
    }

    #[test]
    fn a_file_without_a_sha256_is_invalid() {
        let json = valid_json().replace(
            "{ \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" }",
            "{ \"license\": \"CC0-1.0\" }",
        );
        // Missing mandatory field: the manifest itself is invalid.
        assert!(
            Manifest::parse(&json).is_err() || {
                let m = manifest_from(&json);
                matches!(m.validate(), Err(ImportError::ManifestInvalid { .. }))
            }
        );
    }

    #[test]
    fn a_sequence_referencing_an_unlisted_clip_is_invalid() {
        let json = valid_json().replace(
            "\"clips\": [\"athan/fajr.mp3\"]",
            "\"clips\": [\"athan/ghost.mp3\"]",
        );
        let m = manifest_from(&json);
        assert!(matches!(
            m.validate(),
            Err(ImportError::ManifestInvalid { .. })
        ));
    }

    #[test]
    fn an_edit_referencing_an_unlisted_file_is_invalid() {
        let json = valid_json().replace(
            "\"sequences\": {",
            "\"edits\": { \"athan/ghost.mp3\": { \"fade_out_ms\": 400 } }, \"sequences\": {",
        );
        let m = manifest_from(&json);
        assert!(matches!(
            m.validate(),
            Err(ImportError::ManifestInvalid { .. })
        ));
    }

    #[test]
    fn path_traversal_is_invalid() {
        let json = valid_json().replace("athan/fajr.mp3", "../evil.mp3");
        // Either parse-level or validate-level rejection is fine; the pack
        // must never import a path outside itself.
        let rejected = match Manifest::parse(&json) {
            Err(_) => true,
            Ok(m) => m.validate().is_err(),
        };
        assert!(rejected);
    }

    #[test]
    fn unknown_folders_are_ignored_not_imported() {
        let json = valid_json().replace(
            "\"athan/fajr.mp3\": { \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" }",
            "\"athan/fajr.mp3\": { \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" },\n\
             \"chime/ding.ogg\": { \"sha256\": \"cd34\", \"license\": \"\" }",
        );
        let m = manifest_from(&json);
        // The chime/ entry has no licence, but v1 ignores unknown folders —
        // the pack still validates, and the chime file is not imported.
        assert!(m.validate().is_ok());
        let paths: Vec<_> = m.audio_files().iter().map(|(p, _)| *p).collect();
        assert_eq!(paths, ["athan/fajr.mp3"]);
    }

    #[test]
    fn primary_sequence_follows_prayer_order() {
        let json = valid_json().replace(
            "\"fajr\": { \"mode\": \"sequential\", \"clips\": [\"athan/fajr.mp3\"] }",
            "\"isha\": { \"mode\": \"shuffle\", \"clips\": [\"athan/fajr.mp3\"] },\n\
             \"asr\": { \"mode\": \"sequential\", \"clips\": [\"athan/fajr.mp3\"] }",
        );
        let m = manifest_from(&json);
        let seq = m.primary_sequence().expect("a primary sequence");
        assert_eq!(seq.mode, PlaybackMode::Sequential); // asr before isha
    }

    #[test]
    fn athkar_presence_tracks_athkar_files() {
        let m = manifest_from(&valid_json());
        assert!(!m.has_athkar());
        let json = valid_json().replace(
            "\"athan/fajr.mp3\": { \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" }",
            "\"athan/fajr.mp3\": { \"sha256\": \"ab12\", \"license\": \"CC0-1.0\" },\n\
             \"athkar/morning_01.mp3\": { \"sha256\": \"ef56\", \"license\": \"CC0-1.0\" }",
        );
        let m = manifest_from(&json);
        assert!(m.has_athkar());
    }

    #[test]
    fn edits_parse_with_defaults() {
        let json = valid_json().replace(
            "\"sequences\": {",
            "\"edits\": { \"athan/fajr.mp3\": { \"fade_out_ms\": 400 } }, \"sequences\": {",
        );
        let m = manifest_from(&json);
        let edit = m.edits.get("athan/fajr.mp3").expect("edit present");
        assert_eq!(edit.fade_out_ms, 400);
        assert_eq!(edit.trim_start_ms, 0);
        assert_eq!(edit.stop_after_ms, 0);
        assert!(m.validate().is_ok());
    }
}
