//! Trim and fade math — pure functions, unit-testable without an audio
//! device. The user's original ask lives here: stop the athan after N
//! seconds, as a fade-out, never a hard click.

use crate::manifest::Edit;

/// The format doc's default fade-out (and the number the UI caption quotes).
pub const DEFAULT_FADE_OUT_MS: u64 = 400;

/// Resolve the effective stop cap: the USER's trim wins over the manifest
/// edit (same precedence as mode overrides — the user's choice is final).
/// Returns milliseconds; 0 = no cap (full recording).
pub fn resolve_stop_after_ms(user_stop_s: u32, edit: &Edit) -> u64 {
    if user_stop_s > 0 {
        u64::from(user_stop_s) * 1000
    } else {
        edit.stop_after_ms
    }
}

/// How long a clip actually plays: source duration minus the manifest's
/// head/tail trims, capped by the resolved stop. All milliseconds; the
/// result saturates at 0 — an edit longer than the clip plays nothing,
/// never a negative or wrapped duration.
pub fn playable_ms(clip_ms: u64, edit: &Edit, stop_after_ms: u64) -> u64 {
    let trimmed = clip_ms
        .saturating_sub(edit.trim_start_ms)
        .saturating_sub(edit.trim_end_ms);
    if stop_after_ms > 0 {
        trimmed.min(stop_after_ms)
    } else {
        trimmed
    }
}

/// The fade envelope gain for one sample. Linear ramps: 0→1 over the first
/// `fade_in_samples`, 1→0 over the last `fade_out_samples`, 1 elsewhere.
/// Overlapping ramps take the quieter gain; the result is always 0.0..=1.0.
pub fn fade_gain(
    sample: u64,
    total_samples: u64,
    fade_in_samples: u64,
    fade_out_samples: u64,
) -> f32 {
    if total_samples == 0 || sample >= total_samples {
        return 0.0;
    }
    let mut gain = 1.0f32;
    if fade_in_samples > 0 && sample < fade_in_samples {
        gain = gain.min(sample as f32 / fade_in_samples as f32);
    }
    let fade_out_start = total_samples.saturating_sub(fade_out_samples);
    if fade_out_samples > 0 && sample >= fade_out_start {
        gain = gain.min((total_samples - sample) as f32 / fade_out_samples as f32);
    }
    gain.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(trim_start_ms: u64, trim_end_ms: u64, stop_after_ms: u64) -> Edit {
        Edit {
            trim_start_ms,
            trim_end_ms,
            stop_after_ms,
            ..Edit::default()
        }
    }

    // ---- stop-cap precedence ----

    #[test]
    fn user_trim_wins_over_the_manifest_edit() {
        assert_eq!(resolve_stop_after_ms(30, &edit(0, 0, 10_000)), 30_000);
    }

    #[test]
    fn zero_user_trim_falls_back_to_the_manifest_edit() {
        assert_eq!(resolve_stop_after_ms(0, &edit(0, 0, 10_000)), 10_000);
    }

    #[test]
    fn no_trim_anywhere_means_full_recording() {
        assert_eq!(resolve_stop_after_ms(0, &Edit::default()), 0);
    }

    // ---- playable duration ----

    #[test]
    fn unedited_uncapped_clip_plays_fully() {
        assert_eq!(playable_ms(154_018, &Edit::default(), 0), 154_018);
    }

    #[test]
    fn stop_after_thirty_seconds_cuts_a_full_athan() {
        assert_eq!(playable_ms(154_018, &Edit::default(), 30_000), 30_000);
    }

    #[test]
    fn a_cap_longer_than_the_clip_plays_the_whole_clip() {
        assert_eq!(playable_ms(42_098, &Edit::default(), 300_000), 42_098);
    }

    #[test]
    fn head_and_tail_trims_shorten_the_source() {
        assert_eq!(playable_ms(10_000, &edit(1_000, 2_000, 0), 0), 7_000);
    }

    #[test]
    fn trims_and_cap_compose() {
        // 10 s source − 1 s head − 2 s tail = 7 s, capped at 3 s.
        assert_eq!(playable_ms(10_000, &edit(1_000, 2_000, 0), 3_000), 3_000);
    }

    #[test]
    fn overlong_trims_saturate_at_zero() {
        assert_eq!(playable_ms(1_000, &edit(2_000, 2_000, 0), 0), 0);
    }

    // ---- fade envelope ----

    #[test]
    fn no_fades_means_unity_gain() {
        for sample in [0, 100, 9_999] {
            assert_eq!(fade_gain(sample, 10_000, 0, 0), 1.0);
        }
    }

    #[test]
    fn fade_in_ramps_from_silence() {
        assert_eq!(fade_gain(0, 10_000, 1_000, 0), 0.0);
        assert_eq!(fade_gain(500, 10_000, 1_000, 0), 0.5);
        assert_eq!(fade_gain(1_000, 10_000, 1_000, 0), 1.0);
    }

    #[test]
    fn fade_out_ramps_to_silence_without_a_click() {
        // The last sample is nearly silent, never an abrupt drop from 1.
        let last = fade_gain(9_999, 10_000, 0, 1_000);
        assert!(last <= 0.001, "last sample gain {last} must be ~0");
        assert_eq!(fade_gain(9_000, 10_000, 0, 1_000), 1.0);
        let mid = fade_gain(9_500, 10_000, 0, 1_000);
        assert!((mid - 0.5).abs() < 0.001);
    }

    #[test]
    fn overlapping_fades_take_the_quieter_gain_and_stay_in_range() {
        for sample in 0..100 {
            let g = fade_gain(sample, 100, 80, 80);
            assert!((0.0..=1.0).contains(&g), "gain {g} out of range");
        }
        assert_eq!(fade_gain(50, 100, 80, 80), 0.625);
    }

    #[test]
    fn an_empty_clip_has_zero_gain() {
        assert_eq!(fade_gain(0, 0, 10, 10), 0.0);
    }
}
