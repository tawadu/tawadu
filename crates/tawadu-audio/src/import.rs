//! The voice-pack importer: all-or-nothing validation, sha256 verification
//! against the manifest, MANDATORY duration probing, then a local copy into
//! the library. Offline by construction — the source is a local folder or a
//! `.tawadupack` zip, never a URL.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::ImportError;
use crate::manifest::Manifest;
use crate::probe::probe_duration_ms;

/// Everything the library persists about one installed pack (`pack.json`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackMeta {
    pub id: String,
    pub manifest: Manifest,
    /// Probed at import (decision 8). `None` = unknown → غير متاح.
    pub durations_ms: BTreeMap<String, Option<u64>>,
    pub bundled: bool,
    /// RFC 3339 install time; keeps "first imported pack" well-defined.
    pub installed_at: String,
}

/// Validate + checksum-verify + probe a pack folder without installing it.
pub fn inspect_folder(
    src: &Path,
) -> Result<(Manifest, BTreeMap<String, Option<u64>>), ImportError> {
    let manifest_path = src.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(ImportError::ManifestMissing);
    }
    let json =
        std::fs::read_to_string(&manifest_path).map_err(|_| ImportError::ManifestInvalid {
            file: "manifest.json".to_string(),
        })?;
    let manifest = Manifest::parse(&json)?;
    manifest.validate()?;
    let mut durations = BTreeMap::new();
    for (rel, entry) in manifest.audio_files() {
        let path = src.join(rel);
        // A missing listed file is a checksum failure: the recorded bytes
        // are not there. Never a silent skip.
        let sha = file_sha256(&path).map_err(|_| ImportError::ChecksumMismatch {
            file: rel.to_string(),
        })?;
        if !sha.eq_ignore_ascii_case(entry.sha256.trim()) {
            return Err(ImportError::ChecksumMismatch {
                file: rel.to_string(),
            });
        }
        let ms = probe_duration_ms(&path).map_err(|error| match error {
            ImportError::UnsupportedAudio { .. } => ImportError::UnsupportedAudio {
                file: rel.to_string(),
            },
            other => other,
        })?;
        durations.insert(rel.to_string(), ms);
    }
    Ok((manifest, durations))
}

/// Import a pack folder into `library_dir`: inspect, copy the manifest,
/// audio files and licences, probe durations, write `pack.json`.
pub fn import_folder(src: &Path, library_dir: &Path) -> Result<PackMeta, ImportError> {
    // All-or-nothing: everything is validated BEFORE the first byte is
    // copied, so a rejected pack leaves the library untouched.
    let (manifest, durations) = inspect_folder(src)?;
    std::fs::create_dir_all(library_dir).map_err(|_| ImportError::Generic)?;
    let base = slug(&manifest.name);
    let mut id = base.clone();
    let mut suffix = 2;
    while library_dir.join(&id).exists() {
        id = format!("{base}-{suffix}");
        suffix += 1;
    }
    let dest = library_dir.join(&id);
    let copy = || -> Result<(), ImportError> {
        std::fs::create_dir_all(&dest).map_err(|_| ImportError::Generic)?;
        std::fs::copy(src.join("manifest.json"), dest.join("manifest.json"))
            .map_err(|_| ImportError::Generic)?;
        for (rel, _) in manifest.audio_files() {
            let to = dest.join(rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|_| ImportError::Generic)?;
            }
            std::fs::copy(src.join(rel), &to).map_err(|_| ImportError::Generic)?;
        }
        let licences = src.join("licenses");
        if licences.is_dir() {
            copy_dir(&licences, &dest.join("licenses"))?;
        }
        Ok(())
    };
    if let Err(error) = copy() {
        // A half-copied pack is not a pack: roll the directory back.
        std::fs::remove_dir_all(&dest).ok();
        return Err(error);
    }
    let meta = PackMeta {
        id,
        manifest,
        durations_ms: durations,
        bundled: false,
        installed_at: install_stamp(),
    };
    write_pack_meta(&dest, &meta)?;
    Ok(meta)
}

/// Import a `.tawadupack` zip: extract to a private temp dir (rejecting any
/// entry that would escape it), then run the folder importer.
pub fn import_pack_file(src: &Path, library_dir: &Path) -> Result<PackMeta, ImportError> {
    let file = std::fs::File::open(src).map_err(|_| ImportError::Generic)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| ImportError::Generic)?;
    let dir = extract_dir();
    let result = extract_and_import(&mut archive, &dir, library_dir);
    std::fs::remove_dir_all(&dir).ok();
    result
}

fn extract_and_import(
    archive: &mut zip::ZipArchive<std::fs::File>,
    dir: &Path,
    library_dir: &Path,
) -> Result<PackMeta, ImportError> {
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| ImportError::Generic)?;
        // enclosed_name() refuses absolute paths and `..` traversal: the
        // entry can never write outside the private extract dir.
        let Some(rel) = entry.enclosed_name() else {
            return Err(ImportError::Generic);
        };
        let out = dir.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|_| ImportError::Generic)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|_| ImportError::Generic)?;
        }
        let mut file = std::fs::File::create(&out).map_err(|_| ImportError::Generic)?;
        std::io::copy(&mut entry, &mut file).map_err(|_| ImportError::Generic)?;
    }
    // The manifest sits at the zip root, or inside a single top-level folder.
    let root = if dir.join("manifest.json").is_file() {
        dir.to_path_buf()
    } else {
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("manifest.json").is_file() {
                    found = Some(path);
                    break;
                }
            }
        }
        found.unwrap_or_else(|| dir.to_path_buf())
    };
    import_folder(&root, library_dir)
}

/// Copy a directory tree (the pack's `licenses/` entries) file by file.
fn copy_dir(src: &Path, dest: &Path) -> Result<(), ImportError> {
    std::fs::create_dir_all(dest).map_err(|_| ImportError::Generic)?;
    let entries = std::fs::read_dir(src).map_err(|_| ImportError::Generic)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let to = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &to)?;
        } else {
            std::fs::copy(&path, &to).map_err(|_| ImportError::Generic)?;
        }
    }
    Ok(())
}

/// RFC 3339 UTC with milliseconds — lexicographically sortable, so "first
/// imported pack" (decision 2) is stable across restarts.
pub(crate) fn install_stamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub(crate) fn write_pack_meta(dest: &Path, meta: &PackMeta) -> Result<(), ImportError> {
    let json = serde_json::to_vec_pretty(meta).map_err(|_| ImportError::Generic)?;
    std::fs::write(dest.join("pack.json"), json).map_err(|_| ImportError::Generic)
}

/// sha256 of a file's bytes, lowercase hex.
pub(crate) fn file_sha256(path: &Path) -> Result<String, ImportError> {
    let bytes = std::fs::read(path).map_err(|_| ImportError::Generic)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// A filesystem-safe pack id from the manifest name.
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch == ' ' || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "pack".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The extracted temp dir for a zip import (unique per call).
pub(crate) fn extract_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tawadu-pack-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create extract dir");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{build_pack, sha256_file, temp_dir};

    #[test]
    fn a_folder_without_a_manifest_is_rejected() {
        let dir = temp_dir("no-manifest");
        let err = inspect_folder(&dir).unwrap_err();
        assert!(matches!(err, ImportError::ManifestMissing));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_valid_pack_inspects_with_a_probed_duration() {
        let dir = temp_dir("inspect-ok");
        let pack = build_pack(&dir);
        let (manifest, durations) = inspect_folder(&pack).expect("valid pack");
        assert_eq!(manifest.name, "My Pack");
        assert_eq!(durations.get("athan/tone.wav"), Some(&Some(1_500)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_sha256_mismatch_rejects_the_pack() {
        let dir = temp_dir("sha-mismatch");
        let pack = build_pack(&dir);
        // Tamper with the audio AFTER the manifest recorded its checksum.
        let mut bytes = std::fs::read(pack.join("athan/tone.wav")).unwrap();
        bytes.push(0);
        std::fs::write(pack.join("athan/tone.wav"), bytes).unwrap();
        match inspect_folder(&pack) {
            Err(ImportError::ChecksumMismatch { file }) => assert_eq!(file, "athan/tone.wav"),
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_listed_file_rejects_the_pack() {
        let dir = temp_dir("missing-file");
        let pack = build_pack(&dir);
        std::fs::remove_file(pack.join("athan/tone.wav")).unwrap();
        assert!(matches!(
            inspect_folder(&pack),
            Err(ImportError::ChecksumMismatch { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_licence_rejects_the_pack_before_any_copy() {
        let dir = temp_dir("no-licence");
        let pack = build_pack(&dir);
        let manifest = std::fs::read_to_string(pack.join("manifest.json"))
            .unwrap()
            .replace("\"license\": \"CC0-1.0\" }", "\"license\": \"\" }");
        std::fs::write(pack.join("manifest.json"), manifest).unwrap();
        let lib = dir.join("library");
        match import_folder(&pack, &lib) {
            Err(ImportError::LicenseMissing { file }) => assert_eq!(file, "athan/tone.wav"),
            other => panic!("expected LicenseMissing, got {other:?}"),
        }
        // Nothing was installed.
        assert!(!lib.exists() || std::fs::read_dir(&lib).unwrap().count() == 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undecodable_audio_rejects_the_pack() {
        let dir = temp_dir("bad-audio");
        let pack = build_pack(&dir);
        let path = pack.join("athan/tone.wav");
        let manifest = std::fs::read_to_string(pack.join("manifest.json")).unwrap();
        let old_sha = serde_json::from_str::<serde_json::Value>(&manifest).unwrap()["files"]
            ["athan/tone.wav"]["sha256"]
            .as_str()
            .unwrap()
            .to_string();
        // Replace the audio with garbage, then re-record its checksum so
        // the failure is the CODEC, not the hash.
        std::fs::write(&path, b"not audio").unwrap();
        let manifest = manifest.replace(&old_sha, &sha256_file(&path));
        std::fs::write(pack.join("manifest.json"), manifest).unwrap();
        assert!(matches!(
            inspect_folder(&pack),
            Err(ImportError::UnsupportedAudio { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_copies_files_and_records_metadata() {
        let dir = temp_dir("import-ok");
        let pack = build_pack(&dir);
        let lib = dir.join("library");
        let meta = import_folder(&pack, &lib).expect("import must succeed");
        assert_eq!(meta.id, "my-pack");
        assert!(!meta.bundled);
        assert!(lib.join("my-pack/manifest.json").exists());
        assert!(lib.join("my-pack/athan/tone.wav").exists());
        assert!(lib.join("my-pack/licenses/tone.txt").exists());
        assert!(lib.join("my-pack/pack.json").exists());
        assert_eq!(meta.durations_ms.get("athan/tone.wav"), Some(&Some(1_500)));
        // The copied bytes still match the manifest checksum.
        assert_eq!(
            sha256_file(&lib.join("my-pack/athan/tone.wav")),
            meta.manifest.files["athan/tone.wav"].sha256
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn importing_the_same_name_twice_does_not_overwrite() {
        let dir = temp_dir("import-twice");
        let pack = build_pack(&dir);
        let lib = dir.join("library");
        let first = import_folder(&pack, &lib).unwrap();
        let second = import_folder(&pack, &lib).unwrap();
        assert_ne!(first.id, second.id);
        assert!(lib.join(&first.id).exists());
        assert!(lib.join(&second.id).exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_tawadupack_zip_imports_like_a_folder() {
        let dir = temp_dir("zip-import");
        let pack = build_pack(&dir);
        let zip_path = dir.join("my.tawadupack");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            for rel in ["manifest.json", "athan/tone.wav", "licenses/tone.txt"] {
                zip.start_file(rel, options).unwrap();
                std::io::Write::write_all(&mut zip, &std::fs::read(pack.join(rel)).unwrap())
                    .unwrap();
            }
            zip.finish().unwrap();
        }
        let lib = dir.join("library");
        let meta = import_pack_file(&zip_path, &lib).expect("zip import");
        assert_eq!(meta.manifest.name, "My Pack");
        assert!(lib.join(&meta.id).join("athan/tone.wav").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_zip_with_path_traversal_is_rejected() {
        let dir = temp_dir("zip-evil");
        let zip_path = dir.join("evil.tawadupack");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.start_file("../escape.txt", options).unwrap();
            std::io::Write::write_all(&mut zip, b"no").unwrap();
            zip.finish().unwrap();
        }
        let lib = dir.join("library");
        assert!(import_pack_file(&zip_path, &lib).is_err());
        assert!(!dir.join("escape.txt").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn slugs_are_filesystem_safe() {
        assert_eq!(slug("My Pack"), "my-pack");
        assert_eq!(slug("  Weird / Name:  "), "weird-name");
        assert_eq!(slug("حزمة"), "pack"); // non-ASCII falls back, never empty
    }
}
