//! The bundled starter packs (Atlas's v1 decision): A1 Beautiful Adhan
//! (full 2:34, generic athan for ALL prayers) and A2 Adhan Announcement
//! (0:42 short announcement). CC0, live-verified; the exact licence entries
//! travel with them. Embedded in the binary so the offline floor survives
//! even a wiped data directory; materialized into the library on first run.

use std::path::Path;

use crate::error::ImportError;
use crate::import::PackMeta;
use crate::manifest::Manifest;
use crate::probe::probe_duration_ms;

/// Stable ids; the UI shows a Bundled badge instead of Remove for these.
pub const BEAUTIFUL_ADHAN_ID: &str = "bundled-beautiful-adhan";
pub const ADHAN_ANNOUNCEMENT_ID: &str = "bundled-adhan-announcement";

struct BundledAsset {
    id: &'static str,
    manifest_json: &'static str,
    files: &'static [(&'static str, &'static [u8])],
}

const ASSETS: &[BundledAsset] = &[
    BundledAsset {
        id: BEAUTIFUL_ADHAN_ID,
        manifest_json: include_str!(
            "../../../assets/voices/starter-beautiful-adhan/manifest.json"
        ),
        files: &[
            (
                "athan/beautiful_adhan.ogg",
                include_bytes!(
                    "../../../assets/voices/starter-beautiful-adhan/athan/beautiful_adhan.ogg"
                ),
            ),
            (
                "licenses/commons-beautiful-adhan.txt",
                include_bytes!(
                    "../../../assets/voices/starter-beautiful-adhan/licenses/commons-beautiful-adhan.txt"
                ),
            ),
        ],
    },
    BundledAsset {
        id: ADHAN_ANNOUNCEMENT_ID,
        manifest_json: include_str!(
            "../../../assets/voices/starter-adhan-announcement/manifest.json"
        ),
        files: &[
            (
                "athan/adhan.ogg",
                include_bytes!(
                    "../../../assets/voices/starter-adhan-announcement/athan/adhan.ogg"
                ),
            ),
            (
                "licenses/commons-adhan-short.txt",
                include_bytes!(
                    "../../../assets/voices/starter-adhan-announcement/licenses/commons-adhan-short.txt"
                ),
            ),
        ],
    },
];

/// Ensure every bundled pack exists in `library_dir`, probing durations on
/// first materialization (decision 8 applies to bundled packs too). Already
/// installed packs are left untouched. Returns the bundled packs' metadata.
pub fn materialize(library_dir: &Path) -> Result<Vec<PackMeta>, ImportError> {
    std::fs::create_dir_all(library_dir).map_err(|_| ImportError::Generic)?;
    let mut metas = Vec::new();
    for asset in ASSETS {
        let dest = library_dir.join(asset.id);
        // Idempotent: an installed pack is read back, never re-written.
        if let Ok(json) = std::fs::read_to_string(dest.join("pack.json")) {
            if let Ok(meta) = serde_json::from_str::<PackMeta>(&json) {
                metas.push(meta);
                continue;
            }
        }
        std::fs::create_dir_all(&dest).map_err(|_| ImportError::Generic)?;
        std::fs::write(dest.join("manifest.json"), asset.manifest_json)
            .map_err(|_| ImportError::Generic)?;
        let manifest = Manifest::parse(asset.manifest_json)?;
        let mut durations = std::collections::BTreeMap::new();
        for (rel, bytes) in asset.files {
            let path = dest.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| ImportError::Generic)?;
            }
            std::fs::write(&path, bytes).map_err(|_| ImportError::Generic)?;
            if Manifest::is_v1_audio_path(rel) {
                let ms = probe_duration_ms(&path)?;
                durations.insert((*rel).to_string(), ms);
            }
        }
        let meta = PackMeta {
            id: asset.id.to_string(),
            manifest,
            durations_ms: durations,
            bundled: true,
            installed_at: crate::import::install_stamp(),
        };
        crate::import::write_pack_meta(&dest, &meta)?;
        metas.push(meta);
    }
    Ok(metas)
}

/// The bundled pack's position in the v1 starter order (A1 before A2), or
/// `None` for a non-bundled id. The library sorts bundled packs by this rank
/// so the offline floor's order is stable across restarts.
pub fn rank(id: &str) -> Option<usize> {
    ASSETS.iter().position(|asset| asset.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    #[test]
    fn both_bundled_packs_materialize_with_probed_durations() {
        let dir = temp_dir("bundled");
        let metas = materialize(&dir).expect("materialize");
        assert_eq!(metas.len(), 2);
        assert!(metas.iter().all(|m| m.bundled));
        let a1 = metas.iter().find(|m| m.id == BEAUTIFUL_ADHAN_ID).unwrap();
        let a2 = metas
            .iter()
            .find(|m| m.id == ADHAN_ANNOUNCEMENT_ID)
            .unwrap();
        // Ledger durations: 154.018 s and 42.098 s (probed, not claimed).
        let d1 = a1.durations_ms["athan/beautiful_adhan.ogg"].unwrap();
        let d2 = a2.durations_ms["athan/adhan.ogg"].unwrap();
        assert!((153_800..=154_300).contains(&d1), "A1 probed {d1}");
        assert!((41_900..=42_300).contains(&d2), "A2 probed {d2}");
        // Files + licence entries are on disk.
        assert!(dir
            .join(BEAUTIFUL_ADHAN_ID)
            .join("athan/beautiful_adhan.ogg")
            .exists());
        assert!(dir
            .join(BEAUTIFUL_ADHAN_ID)
            .join("licenses/commons-beautiful-adhan.txt")
            .exists());
        assert!(dir
            .join(ADHAN_ANNOUNCEMENT_ID)
            .join("athan/adhan.ogg")
            .exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn materialization_is_idempotent() {
        let dir = temp_dir("bundled-twice");
        let first = materialize(&dir).unwrap();
        let second = materialize(&dir).unwrap();
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].id, second[0].id);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            2,
            "no duplicate pack dirs"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_embedded_manifests_are_valid_v1_packs() {
        for asset in ASSETS {
            let manifest = Manifest::parse(asset.manifest_json).expect("parse");
            manifest.validate().expect("validate");
            assert_eq!(manifest.license, "CC0-1.0");
            assert!(!manifest.attribution.is_empty());
            // Every embedded file is listed in the manifest with its sha.
            for (rel, bytes) in asset.files {
                if rel.starts_with("athan/") {
                    let entry = &manifest.files[*rel];
                    let mut hasher = sha2::Sha256::new();
                    use sha2::Digest;
                    hasher.update(bytes);
                    assert_eq!(format!("{:x}", hasher.finalize()), entry.sha256);
                }
            }
        }
    }
}
