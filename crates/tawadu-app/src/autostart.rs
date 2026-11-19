//! Opt-in start-at-login (default OFF). The reconcile decision is pure and
//! headless-testable; platform registration goes through `auto-launch`
//! (Windows: HKCU Run key, no admin; macOS: launch agent; Linux: autostart
//! .desktop). Registration itself is verified by CI compile on all three
//! OSes + a manual desktop check — unit tests must never touch the user's
//! real registry/autostart entries.
//!
//! `current_exe` caveat: the registered path is the RUNNING executable, so a
//! debug build registers `target/debug/tawadu-app.exe`. Packaging (Phase 6)
//! revisits this with the installed path.

use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};

/// What startup reconciliation should do about the registration.
#[derive(Debug, PartialEq, Eq)]
pub enum Reconcile {
    Register,
    Unregister,
    None,
}

/// Pure decision: the user's setting vs the actual registration state.
/// Setting true but unregistered → register; false but registered →
/// unregister; agreement → nothing.
pub fn reconcile(want: bool, registered: bool) -> Reconcile {
    match (want, registered) {
        (true, false) => Reconcile::Register,
        (false, true) => Reconcile::Unregister,
        _ => Reconcile::None,
    }
}

/// Build the platform launcher for the running executable. `None` (logged)
/// when the path or the platform backend is unavailable — autostart must
/// never take the app down.
fn launcher() -> Option<AutoLaunch> {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            log::warn!("autostart: cannot resolve the executable path: {error}");
            return None;
        }
    };
    match AutoLaunchBuilder::new()
        .set_app_name("Tawadu")
        .set_app_path(&exe.to_string_lossy())
        // Current user only: no admin elevation on Windows.
        .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
        .build()
    {
        Ok(launcher) => Some(launcher),
        Err(error) => {
            log::warn!("autostart: launcher unavailable: {error}");
            None
        }
    }
}

/// Startup reconciliation. An unreadable registration state is skipped
/// (logged) rather than guessed — never unregister on uncertainty.
pub fn reconcile_at_startup(want: bool) {
    let Some(launcher) = launcher() else {
        return;
    };
    let registered = match launcher.is_enabled() {
        Ok(registered) => registered,
        Err(error) => {
            log::warn!("autostart: cannot read registration state: {error}");
            return;
        }
    };
    match reconcile(want, registered) {
        Reconcile::Register => match launcher.enable() {
            Ok(()) => log::info!("autostart: registered (setting was on, registration missing)"),
            Err(error) => log::warn!("autostart: registration failed: {error}"),
        },
        Reconcile::Unregister => match launcher.disable() {
            Ok(()) => log::info!("autostart: unregistered (setting was off, registration stale)"),
            Err(error) => log::warn!("autostart: unregistration failed: {error}"),
        },
        Reconcile::None => {}
    }
}

/// The settings toggle: apply the user's choice immediately (both calls are
/// idempotent). The flag itself persists via the debounced save-on-change.
pub fn set(want: bool) {
    let Some(launcher) = launcher() else {
        return;
    };
    let result = if want {
        launcher.enable()
    } else {
        launcher.disable()
    };
    match result {
        Ok(()) => log::info!("autostart: {}", if want { "enabled" } else { "disabled" }),
        Err(error) => log::warn!("autostart: change failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_registers_when_wanted_but_missing() {
        assert_eq!(reconcile(true, false), Reconcile::Register);
    }

    #[test]
    fn reconcile_unregisters_when_stale() {
        assert_eq!(reconcile(false, true), Reconcile::Unregister);
    }

    #[test]
    fn reconcile_leaves_agreement_alone() {
        assert_eq!(reconcile(true, true), Reconcile::None);
        assert_eq!(reconcile(false, false), Reconcile::None);
    }
}
