//! Tawadu desktop shell — Slint UI, tray, notifications and scheduler wiring.
//!
//! With a subcommand (`tawadu times …`) it runs headless and exits; with none it
//! opens the window.

mod cli;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = <cli::Cli as clap::Parser>::parse();

    if cli.command.is_some() {
        if let Err(message) = cli::run(&cli) {
            eprintln!("tawadu: {message}");
            std::process::exit(2);
        }
        return Ok(());
    }

    env_logger::init();
    log::info!("Tawadu starting (core v{})", tawadu_core::VERSION);

    let ui = MainWindow::new()?;
    // TODO(phase-1): build the day timeline, start the scheduler, attach tray +
    // notifications, then run the event loop.
    ui.run()?;
    Ok(())
}
