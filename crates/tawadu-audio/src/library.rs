//! The installed-pack library: a directory of packs (`pack.json` metadata +
//! the pack's own files) plus a per-pack render-cache directory. Scanning is
//! tolerant — a damaged pack is logged and skipped, never fatal; the app
//! must still start (and the bundled floor is re-materialized).

use std::path::{Path, PathBuf};

use crate::bundled;
use crate::error::ImportError;
use crate::import::{self, PackMeta};
use crate::routing::InstalledPack;

/// The on-disk library and its render cache.
pub struct PackLibrary {
    dir: PathBuf,
    cache: PathBuf,
    packs: Vec<InstalledPack>,
}

impl PackLibrary {
    /// Open (scan) a library directory. Missing dirs are created. Pack
    /// order: bundled first, then imports by install time — "the first
    /// imported pack" (decision 2) is well-defined and stable.
    pub fn open(dir: PathBuf, cache: PathBuf) -> Self {
        if let Err(error) = std::fs::create_dir_all(&dir) {
            log::warn!("could not create pack library dir: {error}");
        }
        if let Err(error) = std::fs::create_dir_all(&cache) {
            log::warn!("could not create audio cache dir: {error}");
        }
        let mut library = Self {
            dir,
            cache,
            packs: Vec::new(),
        };
        library.reload();
        library
    }

    pub fn packs(&self) -> &[InstalledPack] {
        &self.packs
    }

    pub fn pack_dir(&self, id: &str) -> PathBuf {
        self.dir.join(id)
    }

    /// Absolute path of one clip inside an installed pack.
    pub fn clip_path(&self, pack_id: &str, rel: &str) -> PathBuf {
        self.dir.join(pack_id).join(rel)
    }

    /// The render-cache directory for one pack (deleted with the pack).
    pub fn cache_dir_for(&self, pack_id: &str) -> PathBuf {
        self.cache.join(pack_id)
    }

    /// Materialize the bundled starter packs if absent (the offline floor).
    pub fn install_bundled(&mut self) {
        match bundled::materialize(&self.dir) {
            Ok(_) => self.reload(),
            // Fail-soft: a broken floor must never stop the app starting.
            Err(error) => log::warn!("could not materialize bundled packs: {error}"),
        }
    }

    /// Import a local folder or `.tawadupack` file. All-or-nothing: a
    /// rejected pack leaves the library untouched.
    pub fn import(&mut self, src: &Path) -> Result<InstalledPack, ImportError> {
        let meta = if src.is_file() {
            import::import_pack_file(src, &self.dir)?
        } else {
            import::import_folder(src, &self.dir)?
        };
        self.reload();
        Ok(Self::to_installed(meta))
    }

    /// Remove an installed pack AND its render cache (decision 4). Bundled
    /// packs cannot be removed — returns `Ok(false)`. Unknown id: `Ok(false)`.
    pub fn remove(&mut self, id: &str) -> Result<bool, ImportError> {
        let Some(pack) = self.packs.iter().find(|pack| pack.id == id) else {
            return Ok(false);
        };
        if pack.bundled {
            return Ok(false);
        }
        std::fs::remove_dir_all(self.pack_dir(id)).map_err(|_| ImportError::Generic)?;
        let cache = self.cache_dir_for(id);
        if cache.exists() {
            std::fs::remove_dir_all(&cache).map_err(|_| ImportError::Generic)?;
        }
        self.reload();
        Ok(true)
    }

    fn reload(&mut self) {
        let mut metas: Vec<PackMeta> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let parsed = std::fs::read_to_string(path.join("pack.json"))
                    .map_err(|error| error.to_string())
                    .and_then(|json| {
                        serde_json::from_str::<PackMeta>(&json).map_err(|error| error.to_string())
                    });
                match parsed {
                    Ok(meta) => metas.push(meta),
                    // Tolerant scan: a damaged pack is logged and skipped.
                    Err(error) => {
                        log::warn!("skipping damaged pack {}: {error}", path.display());
                    }
                }
            }
        }
        // Bundled first in the stable starter order (A1 before A2), then
        // imports by install time — RFC 3339 UTC sorts lexicographically.
        metas.sort_by(|a, b| match (a.bundled, b.bundled) {
            (true, true) => bundled::rank(&a.id).cmp(&bundled::rank(&b.id)),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => a.installed_at.cmp(&b.installed_at).then(a.id.cmp(&b.id)),
        });
        self.packs = metas.into_iter().map(Self::to_installed).collect();
    }

    fn to_installed(meta: PackMeta) -> InstalledPack {
        InstalledPack {
            id: meta.id,
            manifest: meta.manifest,
            durations_ms: meta.durations_ms,
            bundled: meta.bundled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{build_pack, temp_dir};

    fn library(dir: &Path) -> PackLibrary {
        PackLibrary::open(dir.join("packs"), dir.join("cache"))
    }

    #[test]
    fn an_empty_library_has_no_packs() {
        let dir = temp_dir("lib-empty");
        let lib = library(&dir);
        assert!(lib.packs().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bundled_packs_install_first_and_survive_reopen() {
        let dir = temp_dir("lib-bundled");
        let mut lib = library(&dir);
        lib.install_bundled();
        assert_eq!(lib.packs().len(), 2);
        assert!(lib.packs().iter().all(|p| p.bundled));
        // Reopen: the scan finds them again, in the same order.
        let reopened = library(&dir);
        assert_eq!(reopened.packs().len(), 2);
        assert_eq!(reopened.packs()[0].id, lib.packs()[0].id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_imported_pack_lands_after_the_bundled_ones() {
        let dir = temp_dir("lib-import");
        let mut lib = library(&dir);
        lib.install_bundled();
        let src = build_pack(&dir);
        let installed = lib.import(&src).expect("import");
        assert!(!installed.bundled);
        assert_eq!(lib.packs().len(), 3);
        assert_eq!(lib.packs()[2].id, installed.id);
        assert_eq!(
            lib.packs()[2].durations_ms.get("athan/tone.wav"),
            Some(&Some(1_500))
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_rejected_import_leaves_the_library_untouched() {
        let dir = temp_dir("lib-reject");
        let mut lib = library(&dir);
        lib.install_bundled();
        let src = build_pack(&dir);
        // Tamper: checksum mismatch.
        let mut bytes = std::fs::read(src.join("athan/tone.wav")).unwrap();
        bytes.push(0);
        std::fs::write(src.join("athan/tone.wav"), bytes).unwrap();
        assert!(lib.import(&src).is_err());
        assert_eq!(lib.packs().len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_deletes_the_pack_and_its_cache() {
        let dir = temp_dir("lib-remove");
        let mut lib = library(&dir);
        lib.install_bundled();
        let src = build_pack(&dir);
        let installed = lib.import(&src).expect("import");
        // Simulate a render cache for the pack.
        let cache = lib.cache_dir_for(&installed.id);
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("render.wav"), b"cached").unwrap();

        assert!(lib.remove(&installed.id).unwrap());
        assert!(!lib.pack_dir(&installed.id).exists());
        assert!(!cache.exists(), "remove must delete the render cache");
        assert_eq!(lib.packs().len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bundled_packs_cannot_be_removed() {
        let dir = temp_dir("lib-bundled-remove");
        let mut lib = library(&dir);
        lib.install_bundled();
        let id = lib.packs()[0].id.clone();
        assert!(!lib.remove(&id).unwrap());
        assert_eq!(lib.packs().len(), 2);
        assert!(lib.pack_dir(&id).exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn removing_an_unknown_id_is_a_noop() {
        let dir = temp_dir("lib-unknown");
        let mut lib = library(&dir);
        assert!(!lib.remove("ghost").unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_damaged_pack_is_skipped_not_fatal() {
        let dir = temp_dir("lib-damaged");
        let mut lib = library(&dir);
        lib.install_bundled();
        // Corrupt one pack.json on disk, then reopen.
        let id = lib.packs()[0].id.clone();
        std::fs::write(lib.pack_dir(&id).join("pack.json"), b"{ broken").unwrap();
        let reopened = library(&dir);
        assert_eq!(reopened.packs().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
