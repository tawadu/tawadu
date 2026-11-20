//! Opt-in device location (Phase 5 slice 6). Platform layer lives in the
//! APP crate — tawadu-core stays platform-free. Read ONCE per user click;
//! nothing is stored beyond the chosen location, nothing leaves the device
//! (the privacy caption's claim is true even for assisted GPS).
//!
//! Three honest outcomes + timeout (Quill §4.1): Denied / Unavailable /
//! Timeout / Found. A fix is never fabricated: out-of-bounds readings are
//! discarded by core::locations::validate_fix before the confirm card.

/// The outcome of one opt-in location read.
#[derive(Debug, Clone, PartialEq)]
pub enum GpsFix {
    Found { lat: f64, lon: f64 },
    Denied,
    Unavailable,
    Timeout,
}

/// Blocking read with a hard 5 s ceiling (Quill: "> 5 s, no endless
/// spinner"). The worker may finish later; its result is then discarded —
/// the UI already showed the honest timeout outcome.
pub fn read_fix() -> GpsFix {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_fix_inner());
    });
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or(GpsFix::Timeout)
}

/// Block on a WinRT IAsyncOperation by polling Status/GetResults (the
/// blocking `.get()` is not available with our feature set), with a hard
/// deadline so a stuck provider can never hang the read.
#[cfg(target_os = "windows")]
fn wait_op<T: windows::core::RuntimeType + 'static>(
    op: &windows_future::IAsyncOperation<T>,
) -> windows::core::Result<T> {
    use windows_future::AsyncStatus;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status = op.Status()?;
        if status == AsyncStatus::Completed {
            return op.GetResults();
        }
        if status == AsyncStatus::Error {
            let hr = op.ErrorCode()?;
            return Err(windows::core::Error::from_hresult(hr));
        }
        if status == AsyncStatus::Canceled {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x8007_04C7_u32 as i32,
            )));
        }
        // Started (or any future value): poll until the hard deadline.
        if std::time::Instant::now() >= deadline {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x8007_05B4_u32 as i32,
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[cfg(target_os = "windows")]
fn read_fix_inner() -> GpsFix {
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator, PositionAccuracy};
    // Permission first — Windows privacy settings can deny location to
    // desktop apps; that is outcome 1, reported honestly.
    let access = match Geolocator::RequestAccessAsync().and_then(|op| wait_op(&op)) {
        Ok(a) => a,
        Err(_) => return GpsFix::Unavailable,
    };
    if access != GeolocationAccessStatus::Allowed {
        return GpsFix::Denied;
    }
    let locator = match Geolocator::new() {
        Ok(l) => l,
        Err(_) => return GpsFix::Unavailable,
    };
    let _ = locator.SetDesiredAccuracy(PositionAccuracy::High);
    let pos = match locator.GetGeopositionAsync().and_then(|op| wait_op(&op)) {
        Ok(p) => p,
        Err(e) => {
            // E_ACCESSDENIED surfaced at read time is still "denied".
            return if e.code().0 == 0x8007_0005_u32 as i32 {
                GpsFix::Denied
            } else {
                GpsFix::Unavailable
            };
        }
    };
    let point = match pos.Coordinate().and_then(|c| c.Point()) {
        Ok(p) => p,
        Err(_) => return GpsFix::Unavailable,
    };
    let position = match point.Position() {
        Ok(p) => p,
        Err(_) => return GpsFix::Unavailable,
    };
    // Core validates the reading like manual entry — never fabricate.
    match tawadu_core::locations::validate_fix(position.Latitude, position.Longitude) {
        Some((lat, lon)) => GpsFix::Found { lat, lon },
        None => GpsFix::Unavailable,
    }
}

#[cfg(not(target_os = "windows"))]
fn read_fix_inner() -> GpsFix {
    // No platform geolocation backend wired on this OS yet — the honest
    // outcome, with the manual path always available.
    GpsFix::Unavailable
}
