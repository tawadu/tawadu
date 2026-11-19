//! Sequence ordering: sequential passthrough and SEEDED, deterministic
//! shuffle. Same seed + same clip list = same order, on every machine and
//! every run — a shuffle the user can reproduce in a bug report.

use tawadu_core::settings::PlaybackMode;

/// Order the clips for playback. Sequential keeps the manifest order;
/// shuffle is a Fisher–Yates permutation driven by a splitmix64 stream
/// seeded with `seed` (no platform RNG, no wall-clock entropy).
pub fn ordered_clips(clips: &[String], mode: PlaybackMode, seed: u64) -> Vec<String> {
    let mut ordered = clips.to_vec();
    if mode == PlaybackMode::Sequential || ordered.len() < 2 {
        return ordered;
    }
    // Fisher–Yates over a splitmix64 stream: deterministic for a given seed,
    // uniform enough for clip ordering, and identical on every platform.
    let mut state = seed;
    for i in (1..ordered.len()).rev() {
        let j = (splitmix64(&mut state) % (i as u64 + 1)) as usize;
        ordered.swap(i, j);
    }
    ordered
}

/// splitmix64 — a small, fast, well-distributed PRNG step. Not cryptographic;
/// it only needs to make shuffle reproducible, not unpredictable.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clips() -> Vec<String> {
        ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn sequential_keeps_manifest_order() {
        assert_eq!(
            ordered_clips(&clips(), PlaybackMode::Sequential, 42),
            clips()
        );
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut got = ordered_clips(&clips(), PlaybackMode::Shuffle, 42);
        got.sort();
        assert_eq!(got, clips());
    }

    #[test]
    fn same_seed_gives_the_same_order_every_time() {
        let first = ordered_clips(&clips(), PlaybackMode::Shuffle, 7);
        for _ in 0..10 {
            assert_eq!(ordered_clips(&clips(), PlaybackMode::Shuffle, 7), first);
        }
    }

    #[test]
    fn different_seeds_give_different_orders() {
        let a = ordered_clips(&clips(), PlaybackMode::Shuffle, 1);
        let b = ordered_clips(&clips(), PlaybackMode::Shuffle, 2);
        assert_ne!(a, b);
    }

    #[test]
    fn shuffle_actually_shuffles() {
        // Across a spread of seeds, at least one order differs from the
        // sequential order (a no-op "shuffle" would fail this).
        let base = clips();
        assert!((0..32).any(|seed| ordered_clips(&base, PlaybackMode::Shuffle, seed) != base));
    }

    #[test]
    fn empty_and_single_clip_lists_are_safe() {
        assert!(ordered_clips(&[], PlaybackMode::Shuffle, 9).is_empty());
        let one = vec!["only".to_string()];
        assert_eq!(ordered_clips(&one, PlaybackMode::Shuffle, 9), one);
    }

    #[test]
    fn seed_zero_is_valid() {
        let got = ordered_clips(&clips(), PlaybackMode::Shuffle, 0);
        let mut sorted = got.clone();
        sorted.sort();
        assert_eq!(sorted, clips());
    }
}
