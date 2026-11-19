//! Mandatory duration probing at import (Phase 3 decision 8). Uses
//! symphonia — the same decoder that plays the files — so a duration is
//! either a real probed value or honestly unknown (`None` → غير متاح).
//! Never a guess, never the file size, never a manifest claim.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::ImportError;

/// A whole file decoded to interleaved f32 samples — the render cache's
/// input. Decoding happens on background threads, never the UI thread.
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

fn unsupported(path: &Path) -> ImportError {
    ImportError::UnsupportedAudio {
        file: path.display().to_string(),
    }
}

/// Open the container with symphonia's probe (extension hint + content
/// sniffing). Unreadable file → `Generic`; undecodable format →
/// `UnsupportedAudio`.
fn probe_format(path: &Path) -> Result<symphonia::core::probe::ProbeResult, ImportError> {
    let file = File::open(path).map_err(|_| ImportError::Generic)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }
    symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| unsupported(path))
}

/// Decode every packet of the first decodable track into interleaved f32.
/// Corrupt individual packets are skipped (the container already validated);
/// a file that yields no audio at all is `UnsupportedAudio`.
pub fn decode_file(path: &Path) -> Result<DecodedAudio, ImportError> {
    let probed = probe_format(path)?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| unsupported(path))?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| unsupported(path))?;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| unsupported(path))?;
    let mut samples: Vec<f32> = Vec::new();
    let mut channels: u16 = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => break,
        };
        if packet.track_id() != track.id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        };
        if channels == 0 {
            channels = buf.spec().channels.count() as u16;
        }
        let mut sample_buf = SampleBuffer::<f32>::new(buf.capacity() as u64, *buf.spec());
        sample_buf.copy_interleaved_ref(buf);
        samples.extend_from_slice(sample_buf.samples());
    }
    if samples.is_empty() || channels == 0 {
        return Err(unsupported(path));
    }
    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Probe a file's playing time in milliseconds.
/// - `Ok(Some(ms))` — probed from the container's frame count, or by
///   decoding when the container does not record one (Ogg Vorbis).
/// - `Ok(None)` — decodable, but no duration could be determined.
/// - `Err(UnsupportedAudio)` — the format/codec cannot be decoded.
/// - `Err(Generic)` — the file could not be read at all.
pub fn probe_duration_ms(path: &Path) -> Result<Option<u64>, ImportError> {
    let probed = probe_format(path)?;
    let format = probed.format;
    if let Some(track) = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
    {
        if let (Some(n_frames), Some(time_base)) =
            (track.codec_params.n_frames, track.codec_params.time_base)
        {
            let time = time_base.calc_time(n_frames);
            return Ok(Some(
                time.seconds * 1000 + (time.frac * 1000.0).round() as u64,
            ));
        }
    }
    drop(format);
    // No recorded frame count (Ogg Vorbis): decode and count. This is a real
    // probed value — never the file size, never a manifest claim.
    let decoded = decode_file(path)?;
    let frames = decoded.samples.len() as u64 / u64::from(decoded.channels);
    if frames == 0 {
        return Ok(None);
    }
    Ok(Some(frames * 1000 / u64::from(decoded.sample_rate)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{repo_asset, temp_dir, write_wav};

    #[test]
    fn a_wav_duration_is_probed_exactly() {
        let dir = temp_dir("probe-wav");
        let path = dir.join("tone.wav");
        write_wav(&path, 1.5, 8_000);
        assert_eq!(probe_duration_ms(&path).unwrap(), Some(1_500));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_bundled_ogg_athan_probes_to_its_ledger_duration() {
        // A2: 42.098 s per docs/audio-sources.md. Ogg records no frame
        // count up front, so this exercises the decode-count fallback.
        let path = repo_asset("assets/voices/starter-adhan-announcement/athan/adhan.ogg");
        let ms = probe_duration_ms(&path)
            .expect("the bundled ogg must decode")
            .expect("a duration must be probed");
        assert!(
            (41_900..=42_300).contains(&ms),
            "probed {ms} ms, expected ~42098"
        );
    }

    #[test]
    fn the_full_bundled_athan_probes_too() {
        // A1: 154.018 s per the ledger.
        let path = repo_asset("assets/voices/starter-beautiful-adhan/athan/beautiful_adhan.ogg");
        let ms = probe_duration_ms(&path)
            .expect("the bundled ogg must decode")
            .expect("a duration must be probed");
        assert!(
            (153_800..=154_300).contains(&ms),
            "probed {ms} ms, expected ~154018"
        );
    }

    #[test]
    fn garbage_bytes_are_unsupported_audio() {
        let dir = temp_dir("probe-garbage");
        let path = dir.join("broken.wav");
        std::fs::write(&path, b"this is not audio at all").unwrap();
        assert!(matches!(
            probe_duration_ms(&path),
            Err(ImportError::UnsupportedAudio { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_is_generic_not_a_guess() {
        let dir = temp_dir("probe-missing");
        let err = probe_duration_ms(&dir.join("nope.wav")).unwrap_err();
        assert!(matches!(err, ImportError::Generic));
        std::fs::remove_dir_all(&dir).ok();
    }
}
