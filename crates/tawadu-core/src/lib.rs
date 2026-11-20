//! Tawadu core — pure logic for prayer times, scheduling, locations and storage.
//!
//! This crate deliberately has **no UI and no platform types**, so the desktop
//! shell and (later) a mobile shell can share it. Everything here should be
//! unit-testable without a window, a network or an audio device.

// The explicit `Default` impls on the config types document the chosen
// defaults; clippy would derive them instead.
#![allow(clippy::derivable_impls)]

pub mod athkar;
pub mod cities;
pub mod hijri;
pub mod locations;
pub mod mosques;
pub mod notify;
pub mod prayer;
pub mod qibla;
pub mod scheduler;
pub mod settings;
pub mod storage;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
