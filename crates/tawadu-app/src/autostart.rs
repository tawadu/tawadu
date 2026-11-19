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
pub fn reconcile(want: bool, registered: bool, path_current: bool) -> Reconcile {
    match (want, registered) {
        // A registration pointing at a moved exe (debug -> release) is an
        // orphan: re-registering overwrites the path (enable is an upsert).
        (true, false) => Reconcile::Register,
        (true, true) if !path_current => Reconcile::Register,
        (false, true) => Reconcile::Unregister,
        _ => Reconcile::None,
    }
}

/// Whether the registered command points at the running executable.
/// Windows reads the HKCU Run value (the debug/release path-move trap);
/// macOS/Linux report true — their launch-agent/.desktop paths are stable
/// per user, and Phase 6 packaging revisits all platforms.
fn registration_path_current(launcher: &AutoLaunch) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_registry::CURRENT_USER;
        let Ok(key) = CURRENT_USER.open(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run") else {
            return true; // unreadable -> do not force a rewrite
        };
        let Ok(value) = key.get_value("Tawadu") else {
            return true;
        };
        // REG_SZ payload is UTF-16, NUL-terminated, quoted by auto-launch.
        let wide = value.as_wide();
        let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        let registered = String::from_utf16_lossy(&wide[..end]);
        registered
            .trim_matches('"')
            .eq_ignore_ascii_case(launcher.get_app_path())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = launcher;
        true
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
    let path_current = registration_path_current(&launcher);
    match reconcile(want, registered, path_current) {
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
        assert_eq!(reconcile(true, false, true), Reconcile::Register);
        assert_eq!(reconcile(true, false, false), Reconcile::Register);
    }

    #[test]
    fn reconcile_unregisters_when_stale() {
        assert_eq!(reconcile(false, true, true), Reconcile::Unregister);
        assert_eq!(reconcile(false, true, false), Reconcile::Unregister);
    }

    #[test]
    fn reconcile_leaves_agreement_alone() {
        // Idempotent: no repeated registry writes when everything agrees.
        assert_eq!(reconcile(true, true, true), Reconcile::None);
        assert_eq!(reconcile(false, false, true), Reconcile::None);
        assert_eq!(reconcile(false, false, false), Reconcile::None);
    }

    #[test]
    fn reconcile_reregisters_when_the_exe_path_moved() {
        // Orphan prevention: registered but pointing at an old exe path.
        assert_eq!(reconcile(true, true, false), Reconcile::Register);
    }
}
