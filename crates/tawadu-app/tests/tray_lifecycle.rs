//! Process-level regression for ADR 0002 (close-to-tray).
//!
//! Drives the REAL binary through the debug lifecycle hooks
//! (`TAWADU_DEBUG_AUTOCLOSE_MS`, `TAWADU_DEBUG_AUTOQUIT_MS`), which dispatch the
//! same `CloseRequested` event and the same `quit_event_loop()` call the tray
//! menu uses. Asserts: close → hidden → event loop alive → Quit → clean exit,
//! and second launch → existing window restored.
//!
//! Needs a GUI session: Windows/macOS runners have one; Linux CI runs the
//! suite under `xvfb-run`. On Linux without an SNI host the tray falls back to
//! `KeepWindowShown` (ADR condition 4) — the test asserts tray-or-fallback
//! from the app's own log, never assumes which path ran.
//!
//! NOTE: the single-instance lock is global. If a Tawadu instance is already
//! running on this machine, kill it first (`taskkill /F /IM tawadu-app.exe`).

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_tawadu-app");

fn spawn_app(extra: &[(&str, &str)]) -> (Child, Arc<Mutex<String>>) {
    let mut cmd = Command::new(BIN);
    cmd.env("RUST_LOG", "info")
        .env("TAWADU_DEBUG_NOW", "2026-11-19T09:00:00Z")
        .env("TAWADU_DEBUG_TIME_SCALE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().expect("launch tawadu-app");
    let stderr = child.stderr.take().expect("piped stderr");
    (child, drain(stderr))
}

/// Accumulate a pipe on a thread so reading logs never blocks the test.
fn drain(pipe: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let log = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&log);
    std::thread::spawn(move || {
        let mut pipe = pipe;
        let mut buf = [0u8; 4096];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => sink
                    .lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n])),
            }
        }
    });
    log
}

fn wait_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => return Some(status),
            None if Instant::now() >= deadline => return None,
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

#[cfg(windows)]
fn window_visible() -> bool {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};
    unsafe {
        FindWindowW(None, w!("Tawadu"))
            .map(|hwnd| IsWindowVisible(hwnd).as_bool())
            .unwrap_or(false)
    }
}

/// One test fn, sequential phases: the single-instance lock is process-global,
/// so two parallel app-launching tests would collide.
#[test]
fn tray_lifecycle() {
    // ---- phase 1: close → hidden → loop alive → Quit → clean exit ----
    let (mut app, log) = spawn_app(&[
        ("TAWADU_DEBUG_AUTOCLOSE_MS", "1500"),
        ("TAWADU_DEBUG_AUTOQUIT_MS", "5000"),
    ]);

    // Past the 1.5 s auto-close: without the held KeepAlive the loop would
    // have quit when hide() dropped the window's last keepalive.
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        app.try_wait().expect("try_wait").is_none(),
        "process exited after close — KeepAlive is not holding the event loop"
    );

    let logs = log.lock().unwrap().clone();
    let hidden_to_tray = logs.contains("hidden to tray");
    let tray_fallback = logs.contains("tray unavailable");
    assert!(
        hidden_to_tray || tray_fallback,
        "neither tray path ran; logs:\n{logs}"
    );
    #[cfg(windows)]
    {
        if hidden_to_tray {
            assert!(
                !window_visible(),
                "window must be hidden after close-to-tray"
            );
        } else {
            assert!(window_visible(), "fallback must keep the window shown");
        }
    }

    let status = wait_exit(&mut app, Duration::from_secs(10))
        .expect("Quit must exit the process (quit_event_loop is not keepalive-gated)");
    assert!(status.success(), "Quit must exit cleanly, got {status:?}");

    // ---- phase 2: second launch while hidden restores the window ----
    let (mut first, log) = spawn_app(&[("TAWADU_DEBUG_AUTOCLOSE_MS", "1500")]);
    std::thread::sleep(Duration::from_millis(2500));
    assert!(
        first.try_wait().expect("try_wait").is_none(),
        "first instance died"
    );
    let first_logs = log.lock().unwrap().clone();
    let first_hidden = first_logs.contains("hidden to tray");
    #[cfg(windows)]
    if first_hidden {
        assert!(
            !window_visible(),
            "first instance should be hidden before relaunch"
        );
    }

    let (mut second, _second_log) = spawn_app(&[]);
    let second_status = wait_exit(&mut second, Duration::from_secs(5))
        .expect("second launch must exit promptly (single instance)");
    assert!(
        second_status.success(),
        "second launch must exit 0, got {second_status:?}"
    );

    // Give the restore signal time to cross the event loop.
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        first.try_wait().expect("try_wait").is_none(),
        "first instance must keep running after a second launch"
    );
    #[cfg(windows)]
    if first_hidden {
        assert!(
            window_visible(),
            "second launch must restore/show the hidden window (ADR condition 5)"
        );
    }
    #[cfg(not(windows))]
    {
        // Visibility is not assertable headlessly here; the restore callback
        // path is logged, so assert it ran when the tray path was taken.
        if first_hidden {
            let logs = log.lock().unwrap().clone();
            assert!(
                logs.contains("restored window (second launch)"),
                "restore callback did not run; logs:\n{logs}"
            );
        }
    }

    let _ = first.kill();
    let _ = first.wait();
}
