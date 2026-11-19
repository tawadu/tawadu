//! Test-only helpers: real WAV fixtures and real pack folders on disk, so
//! the importer/probe/render tests exercise actual bytes, not mocks.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// A unique temp dir per call (process id + counter + nanos).
pub fn temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tawadu-audio-test-{}-{}-{}-{}",
        tag,
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Write a mono 16-bit WAV of `seconds` at `rate` Hz (a quiet 440 Hz tone).
pub fn write_wav(path: &Path, seconds: f32, rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    let count = (seconds * rate as f32) as usize;
    for i in 0..count {
        let t = i as f32 / rate as f32;
        let sample = (0.25 * (std::f32::consts::TAU * 440.0 * t).sin() * i16::MAX as f32) as i16;
        writer.write_sample(sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn sha256_file(path: &Path) -> String {
    sha256_hex(&std::fs::read(path).expect("read file for hashing"))
}

/// Build a valid one-clip pack folder on disk: manifest.json + athan/tone.wav
/// (1.5 s @ 8 kHz) + a licences entry. Returns the folder path.
pub fn build_pack(dir: &Path) -> PathBuf {
    let pack = dir.join("my-pack");
    std::fs::create_dir_all(pack.join("athan")).unwrap();
    std::fs::create_dir_all(pack.join("licenses")).unwrap();
    write_wav(&pack.join("athan/tone.wav"), 1.5, 8_000);
    let sha = sha256_file(&pack.join("athan/tone.wav"));
    let manifest = format!(
        r#"{{
            "format": "tawadu-pack/1",
            "name": "My Pack",
            "reciter": "A Reciter",
            "license": "CC0-1.0",
            "attribution": "Recorded by A Reciter",
            "files": {{
                "athan/tone.wav": {{ "sha256": "{sha}", "license": "CC0-1.0" }}
            }},
            "sequences": {{
                "fajr": {{ "mode": "sequential", "clips": ["athan/tone.wav"] }}
            }}
        }}"#
    );
    std::fs::write(pack.join("manifest.json"), manifest).unwrap();
    std::fs::write(pack.join("licenses/tone.txt"), "Licence: CC0-1.0\n").unwrap();
    pack
}

/// Path to a bundled starter asset in the repo (tests read the real bytes).
pub fn repo_asset(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}
