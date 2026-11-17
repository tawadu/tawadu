//! Builds the day's event timeline — prayers, iqamah offsets, athkar windows and
//! pre-alerts — then sleeps until the next event and fires it.
//!
//! Recompute on: start-up, location/method change, settings change, system clock
//! change, wake from sleep, and every midnight.
