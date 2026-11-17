//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! Phase 0 wires only the window. Later phases attach the day timeline, the
//! scheduler, the audio engine, tray/autostart and notifications.

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    env_logger::init();
    log::info!("Tawadu starting (core v{})", tawadu_core::VERSION);

    let ui = MainWindow::new()?;
    // TODO(phase-1): build the day timeline, start the scheduler, attach tray +
    // notifications, then run the event loop.
    ui.run()
}
