//! The trim/fade render cache. Edits are NON-DESTRUCTIVE: the source file
//! is never modified; the edit is rendered to a WAV cache file keyed by
//! (source checksum + edit parameters + stop cap). Removing a pack deletes
//! its cache (decision 4), so "removed from this device" stays true.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::edit::{fade_gain, DEFAULT_FADE_OUT_MS};
use crate::error::ImportError;
use crate::manifest::Edit;
use crate::probe::decode_file;

/// Stable cache key: same source bytes + same edits + same cap = same key.
/// Any change produces a different key, so a stale render is never played.
pub fn cache_key(source_sha256: &str, edit: &Edit, stop_after_ms: u64) -> String {
    let material = format!(
        "{source_sha256}|{}|{}|{}|{}|{stop_after_ms}",
        edit.trim_start_ms, edit.trim_end_ms, edit.fade_in_ms, edit.fade_out_ms
    );
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Render `src` with `edit` and the resolved `stop_after_ms` into `dest`
/// (WAV, source sample rate and channel count, f32). The fade-out defaults
/// to [`DEFAULT_FADE_OUT_MS`] when the manifest sets none — a cut is always
/// a fade, never a hard click.
pub fn render_clip(
    src: &Path,
    edit: &Edit,
    stop_after_ms: u64,
    dest: &Path,
) -> Result<(), ImportError> {
    let decoded = decode_file(src)?;
    let rate = u64::from(decoded.sample_rate);
    let channels = usize::from(decoded.channels);
    let total_frames = decoded.samples.len() / channels;
    let ms_to_frames = |ms: u64| (ms * rate / 1000) as usize;

    let start = ms_to_frames(edit.trim_start_ms).min(total_frames);
    let mut len = total_frames
        .saturating_sub(start)
        .saturating_sub(ms_to_frames(edit.trim_end_ms));
    if stop_after_ms > 0 {
        len = len.min(ms_to_frames(stop_after_ms));
    }

    let fade_in = ms_to_frames(edit.fade_in_ms) as u64;
    // A cut (stop cap or tail trim) always fades; an untouched full recording
    // keeps its natural ending. An explicit manifest fade always applies.
    let fade_out_ms = if edit.fade_out_ms > 0 {
        edit.fade_out_ms
    } else if stop_after_ms > 0 || edit.trim_end_ms > 0 {
        DEFAULT_FADE_OUT_MS
    } else {
        0
    };
    let fade_out = ms_to_frames(fade_out_ms) as u64;

    let spec = hound::WavSpec {
        channels: decoded.channels,
        sample_rate: decoded.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(dest, spec).map_err(|_| ImportError::Generic)?;
    for frame in 0..len {
        let gain = fade_gain(frame as u64, len as u64, fade_in, fade_out);
        for channel in 0..channels {
            let sample = decoded.samples[(start + frame) * channels + channel] * gain;
            writer
                .write_sample(sample)
                .map_err(|_| ImportError::Generic)?;
        }
    }
    writer.finalize().map_err(|_| ImportError::Generic)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{sha256_file, temp_dir, write_wav};

    fn read_frames(path: &Path) -> (Vec<f32>, u32, u16) {
        let mut reader = hound::WavReader::open(path).expect("rendered wav");
        let spec = reader.spec();
        let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
        (samples, spec.sample_rate, spec.channels)
    }

    #[test]
    fn an_unedited_render_preserves_length_and_rate() {
        let dir = temp_dir("render-plain");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let dest = dir.join("out.wav");
        render_clip(&src, &Edit::default(), 0, &dest).expect("render");
        let (samples, rate, channels) = read_frames(&dest);
        assert_eq!(rate, 8_000);
        assert_eq!(channels, 1);
        assert_eq!(samples.len(), 8_000);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stop_after_cuts_the_render_to_the_cap() {
        let dir = temp_dir("render-stop");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let dest = dir.join("out.wav");
        render_clip(&src, &Edit::default(), 500, &dest).expect("render");
        let (samples, _, _) = read_frames(&dest);
        assert_eq!(samples.len(), 4_000);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn head_and_tail_trims_are_applied() {
        let dir = temp_dir("render-trim");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let dest = dir.join("out.wav");
        let edit = Edit {
            trim_start_ms: 200,
            trim_end_ms: 300,
            ..Edit::default()
        };
        render_clip(&src, &edit, 0, &dest).expect("render");
        let (samples, _, _) = read_frames(&dest);
        assert_eq!(samples.len(), 4_000); // 1000 − 200 − 300 = 500 ms
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_render_fades_in_and_out() {
        let dir = temp_dir("render-fade");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let dest = dir.join("out.wav");
        let edit = Edit {
            fade_in_ms: 100,
            fade_out_ms: 400,
            ..Edit::default()
        };
        render_clip(&src, &edit, 0, &dest).expect("render");
        let (samples, _, _) = read_frames(&dest);
        assert_eq!(samples[0], 0.0, "fade-in must start at silence");
        let last = samples[samples.len() - 1].abs();
        assert!(last < 0.01, "fade-out must end near silence, got {last}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_cut_gets_the_default_fade_out_even_without_a_manifest_fade() {
        let dir = temp_dir("render-default-fade");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let dest = dir.join("out.wav");
        render_clip(&src, &Edit::default(), 500, &dest).expect("render");
        let (samples, _, _) = read_frames(&dest);
        let last = samples[samples.len() - 1].abs();
        assert!(last < 0.01, "the cut must fade, got {last}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cache_keys_are_stable_and_sensitive() {
        let edit = Edit {
            fade_out_ms: 400,
            ..Edit::default()
        };
        let a = cache_key("abcd", &edit, 30_000);
        assert_eq!(a, cache_key("abcd", &edit, 30_000));
        assert_ne!(a, cache_key("abcd", &edit, 0));
        assert_ne!(a, cache_key("abce", &edit, 30_000));
        let other_edit = Edit {
            fade_out_ms: 200,
            ..edit
        };
        assert_ne!(a, cache_key("abcd", &other_edit, 30_000));
    }

    #[test]
    fn the_source_file_is_never_modified() {
        let dir = temp_dir("render-nondestructive");
        let src = dir.join("src.wav");
        write_wav(&src, 1.0, 8_000);
        let before = sha256_file(&src);
        let dest = dir.join("out.wav");
        render_clip(
            &src,
            &Edit {
                trim_start_ms: 200,
                ..Edit::default()
            },
            500,
            &dest,
        )
        .expect("render");
        assert_eq!(sha256_file(&src), before);
        std::fs::remove_dir_all(&dir).ok();
    }
}
