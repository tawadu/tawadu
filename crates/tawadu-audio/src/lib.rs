//! Audio engine for Tawadu.
//!
//! Responsibilities:
//! - a small mixer with channels for `athan`, `athkar` and notification chimes,
//!   each with its own volume, fade envelopes and optional ducking;
//! - the offline **voice-pack editor**: decode → trim → fade → (resample) →
//!   concatenate → encode, so users can cut, reorder and shuffle recordings
//!   without any cloud processing.
//!
//! Everything here runs on its own thread; the UI thread never blocks on audio.
