//! Audio engine for Tawadu.
//!
//! Responsibilities:
//! - the offline voice-pack importer: manifest validation, sha256
//!   verification, mandatory duration probing, licence enforcement;
//! - sequence ordering (sequential / seeded deterministic shuffle), trim
//!   and fade math, per-event voice routing and mode-override precedence;
//! - a small playback engine (rodio) with a built-in GENERATED calm chime;
//! - the trim/fade render cache: edits are applied non-destructively and
//!   rendered to cache files, never written back into the source pack.
//!
//! Everything that touches a device or the filesystem runs on background
//! threads; the UI thread never blocks on audio. The pure logic (manifest,
//! sequence, edit, routing, chime) is unit-testable without a device.
//!
//! Offline-first: nothing in this crate opens a network connection.

pub mod bundled;
pub mod chime;
pub mod edit;
pub mod engine;
pub mod error;
pub mod import;
pub mod library;
pub mod manifest;
pub mod probe;
pub mod render;
pub mod routing;
pub mod sequence;

#[cfg(test)]
pub mod testutil;
