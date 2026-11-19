//! The built-in GENERATED calm chime (Phase 3 decision 5): licence-free,
//! no asset, deterministic. A soft two-partial bell with a gentle attack
//! and an exponential decay — synthesised, never downloaded.

/// Chime length in seconds.
pub const CHIME_SECONDS: f32 = 1.6;

/// Generate the chime as mono f32 samples at `sample_rate`. Deterministic:
/// the same rate always produces the same samples.
pub fn chime_samples(sample_rate: u32) -> Vec<f32> {
    let count = (sample_rate as f32 * CHIME_SECONDS) as usize;
    let dt = 1.0 / sample_rate as f32;
    let two_pi = std::f32::consts::TAU;
    // A soft bell: E5 fundamental with a quiet B5 and E6 partial.
    let partials = [(659.25f32, 0.6f32), (987.77, 0.25), (1318.51, 0.1)];
    (0..count)
        .map(|i| {
            let t = i as f32 * dt;
            // 10 ms raised-cosine attack: no click at the start.
            let attack = if t < 0.010 {
                0.5 * (1.0 - (std::f32::consts::PI * t / 0.010).cos())
            } else {
                1.0
            };
            // Exponential decay (tau 0.35 s): the tail is silent well
            // before the buffer ends, so stopping is never a click.
            let decay = (-t / 0.35).exp();
            let tone: f32 = partials
                .iter()
                .map(|(freq, amp)| amp * (two_pi * freq * t).sin())
                .sum();
            // 0.5 master keeps the peak under 0.5 — calm by construction.
            attack * decay * tone * 0.5
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chime_has_the_advertised_length() {
        let samples = chime_samples(44_100);
        let expected = (44_100.0 * CHIME_SECONDS) as usize;
        assert_eq!(samples.len(), expected);
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(chime_samples(48_000), chime_samples(48_000));
    }

    #[test]
    fn the_chime_is_calm_bounded_and_audible() {
        let samples = chime_samples(44_100);
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.05, "the chime must be audible, peak {peak}");
        assert!(peak <= 0.7, "the chime must stay calm, peak {peak}");
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn the_chime_starts_soft_and_ends_silent() {
        let samples = chime_samples(44_100);
        // Gentle attack: the first millisecond stays near silence.
        let head_peak = samples[..44].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(head_peak < 0.05, "attack too abrupt: {head_peak}");
        // Decay: the last 50 ms are effectively silent (no click at the end).
        let tail_start = samples.len() - 2_205;
        let tail_peak = samples[tail_start..]
            .iter()
            .fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(tail_peak < 0.01, "tail not decayed: {tail_peak}");
    }

    #[test]
    fn other_sample_rates_work_too() {
        for rate in [8_000, 22_050, 48_000, 96_000] {
            let samples = chime_samples(rate);
            assert_eq!(samples.len(), (rate as f32 * CHIME_SECONDS) as usize);
        }
    }
}
