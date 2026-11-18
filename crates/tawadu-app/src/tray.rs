//! System-tray integration — ADR 0002 (close-to-tray).
//!
//! All tray/platform code lives here in `tawadu-app`; `tawadu-core` stays free
//! of UI and platform types. The pure helpers (icon geometry, tooltip text,
//! close policy) are unit-tested headlessly; the live tray path is exercised
//! by the process-level lifecycle test and the desktop exit test.

/// Brand accent (`design/brand.md`) — the tray mark's fill.
const ACCENT: (u8, u8, u8) = (0x2E, 0x5C, 0x4E);

/// A rounded-square tray mark on transparency, generated at runtime so no
/// image asset (and no licence entry) is needed. Hard edges, no antialiasing:
/// deterministic and crisp at tray sizes.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let radius = (size / 5).max(1) as i64;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size as i64 {
        for x in 0..size as i64 {
            // Inside the rounded square = inside the full square, except the
            // four corner squares where a quarter-circle of `radius` applies.
            let cx = x.min(size as i64 - 1 - x);
            let cy = y.min(size as i64 - 1 - y);
            let inside = if cx < radius && cy < radius {
                let dx = radius - cx;
                let dy = radius - cy;
                dx * dx + dy * dy <= radius * radius
            } else {
                true
            };
            if inside {
                rgba.extend_from_slice(&[ACCENT.0, ACCENT.1, ACCENT.2, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    rgba
}

/// Tray tooltip: the next obligatory prayer and its local time. `None` (a
/// polar day with no answer) renders an em dash — never an invented time.
pub fn tooltip_text(next: Option<(&str, &str)>) -> String {
    match next {
        Some((name, time)) => format!("Next: {name} · {time}"),
        None => "Tawadu · —".to_string(),
    }
}

/// Close policy: hide to tray when a tray exists; otherwise keep the window
/// shown — never hide into nothing (ADR 0002 condition 4).
pub fn close_response(tray_ok: bool) -> slint::CloseRequestResponse {
    if tray_ok {
        slint::CloseRequestResponse::HideWindow
    } else {
        slint::CloseRequestResponse::KeepWindowShown
    }
}

/// The AUMID toasts are attributed to. Registered with the shell before any
/// toast (Windows) and passed to `notify-rust` via `app_id` — its default is
/// PowerShell's AUMID, which is why unpackaged-exe toasts silently vanish.
/// Windows-only: both uses are cfg-gated, so the constant is too (dead-code
/// lint on macOS/Linux otherwise).
#[cfg(target_os = "windows")]
pub const AUMID: &str = "org.tawadu.Tawadu";

/// ADR 0002 adapter: the ONE place that touches `private_unstable_api`.
/// `hide()` drops the window's keepalive; holding this `KeepAlive` for the
/// app lifetime keeps the event loop running while hidden. The type is not
/// nameable from the `slint` facade, so callers hold the returned value as a
/// `main()` local (type inference) — never a struct field.
pub fn hold_event_loop() -> Result<impl Sized, slint::PlatformError> {
    Ok(slint::private_unstable_api::ensure_context()?.keep_alive())
}

/// Register our AUMID so Windows attributes toasts to Tawadu (ADR 0002
/// condition 3). Until this succeeds the show-window + banner path is the
/// floor; a failed registration is logged, never fatal.
#[cfg(target_os = "windows")]
pub fn register_aumid() {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    unsafe {
        if let Err(error) = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(AUMID)) {
            log::warn!(
                "could not register AUMID ({error}); toasts may not appear — \
                 the in-app banner remains the floor"
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn register_aumid() {}

/// ksni's `spawn` assumes an SNI watcher exists (`assume_sni_available(true)`),
/// so probe the session bus ourselves: no watcher → no tray → `KeepWindowShown`
/// fallback. Never hide into nothing (ADR 0002 condition 4).
#[cfg(target_os = "linux")]
fn sni_watcher_present() -> bool {
    let Ok(connection) = zbus::blocking::Connection::session() else {
        return false;
    };
    let Ok(proxy) = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    ) else {
        return false;
    };
    proxy
        .call_method("NameHasOwner", &("org.kde.StatusNotifierWatcher",))
        .ok()
        .and_then(|message| message.body().deserialize::<bool>().ok())
        .unwrap_or(false)
}

/// Build the tray icon: generated rounded-square mark, Show/Quit menu,
/// next-prayer tooltip. Menu clicks arrive through `muda::MenuEvent::receiver()`
/// on every platform (ksni routes them via the muda snapshot's activate).
pub fn build_tray(tooltip: &str) -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    if !sni_watcher_present() {
        return Err("no StatusNotifier watcher on the session bus".into());
    }
    let menu = muda::Menu::new();
    menu.append(&muda::MenuItem::with_id("show", "Show Tawadu", true, None))?;
    menu.append(&muda::MenuItem::with_id("quit", "Quit", true, None))?;
    let icon = tray_icon::Icon::from_rgba(icon_rgba(32), 32, 32)?;
    Ok(tray_icon::TrayIcon::builder()
        .with_id("tawadu-tray")
        .with_icon(icon)
        .with_tooltip(tooltip)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(true)
        .build()?)
}

/// macOS app menu with a Cmd+Q Quit item. Slint 1.19's winit backend creates
/// no default app menu (verified in the vendored sources — only the dormant
/// muda menubar path exists), so without this Cmd+Q is inert. The Quit item
/// uses the same "quit" id as the tray menu, so one poll handles both.
#[cfg(target_os = "macos")]
pub fn macos_app_menu() -> muda::Menu {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    let menu = muda::Menu::new();
    let quit = muda::MenuItem::with_id(
        "quit",
        "Quit Tawadu",
        true,
        Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
    );
    if let Err(error) = menu.append(&quit) {
        log::warn!("could not build the macOS app menu ({error}); Cmd+Q may not quit");
    }
    menu.init_for_nsapp();
    menu
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    #[test]
    fn icon_has_exact_rgba_dimensions() {
        assert_eq!(icon_rgba(32).len(), 32 * 32 * 4);
        assert_eq!(icon_rgba(16).len(), 16 * 16 * 4);
    }

    #[test]
    fn icon_corners_are_transparent() {
        let rgba = icon_rgba(32);
        for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
            assert_eq!(pixel(&rgba, 32, x, y)[3], 0, "corner ({x},{y})");
        }
    }

    #[test]
    fn icon_center_and_edge_midpoints_are_brand_accent() {
        let rgba = icon_rgba(32);
        let accent = [ACCENT.0, ACCENT.1, ACCENT.2, 255];
        for (x, y) in [(16, 16), (16, 0), (0, 16), (31, 16), (16, 31)] {
            assert_eq!(pixel(&rgba, 32, x, y), accent, "pixel ({x},{y})");
        }
    }

    #[test]
    fn icon_contains_only_accent_or_transparency() {
        let rgba = icon_rgba(32);
        for i in (0..rgba.len()).step_by(4) {
            let px = &rgba[i..i + 4];
            assert!(
                px == [ACCENT.0, ACCENT.1, ACCENT.2, 255] || px == [0, 0, 0, 0],
                "stray pixel {px:?}"
            );
        }
    }

    #[test]
    fn tooltip_names_the_next_prayer_and_time() {
        assert_eq!(tooltip_text(Some(("Fajr", "04:49"))), "Next: Fajr · 04:49");
    }

    #[test]
    fn tooltip_never_invents_a_time() {
        assert_eq!(tooltip_text(None), "Tawadu · —");
    }

    #[test]
    fn close_hides_when_a_tray_exists() {
        assert_eq!(
            close_response(true),
            slint::CloseRequestResponse::HideWindow
        );
    }

    #[test]
    fn close_keeps_window_when_tray_creation_failed() {
        assert_eq!(
            close_response(false),
            slint::CloseRequestResponse::KeepWindowShown
        );
    }
}
