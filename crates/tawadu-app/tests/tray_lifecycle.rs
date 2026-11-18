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
//! All waits are marker-driven with generous deadlines, not fixed sleeps:
//! slow runners (macOS CI) can take seconds just to reach the first timer.
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

/// Poll the app's log until `needle` appears (or the deadline passes).
fn wait_for_log(log: &Arc<Mutex<String>>, needle: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if log.lock().unwrap().contains(needle) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Poll for whichever of two markers appears first; returns its index.
/// The tray path logs at close time, the fallback path at STARTUP — waiting
/// for the tray marker alone would burn the whole deadline on the fallback
/// path, past the auto-quit, and misread a clean exit as a keepalive failure.
fn wait_for_either(
    log: &Arc<Mutex<String>>,
    first: &str,
    second: &str,
    timeout: Duration,
) -> Option<usize> {
    let deadline = Instant::now() + timeout;
    loop {
        {
            let logs = log.lock().unwrap();
            if logs.contains(first) {
                return Some(0);
            }
            if logs.contains(second) {
                return Some(1);
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
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

#[cfg(windows)]
fn wait_for_visibility(visible: bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if window_visible() == visible {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
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

    // Wait for whichever path ran (slow runners start late; the fallback
    // path logs at startup, the tray path at close time).
    let path = wait_for_either(
        &log,
        "hidden to tray",
        "tray unavailable",
        Duration::from_secs(30),
    );
    let closed = path == Some(0);
    assert!(
        path.is_some(),
        "neither tray path ran; logs:\n{}",
        log.lock().unwrap()
    );

    // The close has fired; without the held KeepAlive the loop would have
    // quit when hide() dropped the window's last keepalive. Quit is 3.5 s
    // behind the close, so there is margin to observe the alive state.
    assert!(
        app.try_wait().expect("try_wait").is_none(),
        "process exited after close — KeepAlive is not holding the event loop"
    );
    #[cfg(windows)]
    {
        if closed {
            assert!(
                wait_for_visibility(false, Duration::from_secs(5)),
                "window must be hidden after close-to-tray"
            );
        } else {
            assert!(window_visible(), "fallback must keep the window shown");
        }
    }

    let status = wait_exit(&mut app, Duration::from_secs(30))
        .expect("Quit must exit the process (quit_event_loop is not keepalive-gated)");
    assert!(status.success(), "Quit must exit cleanly, got {status:?}");

    // ---- phase 2: second launch while hidden restores the window ----
    let (mut first, log) = spawn_app(&[("TAWADU_DEBUG_AUTOCLOSE_MS", "1500")]);
    let first_hidden = wait_for_either(
        &log,
        "hidden to tray",
        "tray unavailable",
        Duration::from_secs(30),
    ) == Some(0);
    assert!(
        first.try_wait().expect("try_wait").is_none(),
        "first instance died"
    );
    #[cfg(windows)]
    if first_hidden {
        assert!(
            wait_for_visibility(false, Duration::from_secs(5)),
            "first instance should be hidden before relaunch"
        );
    }

    let (mut second, _second_log) = spawn_app(&[]);
    let second_status = wait_exit(&mut second, Duration::from_secs(15))
        .expect("second launch must exit promptly (single instance)");
    assert!(
        second_status.success(),
        "second launch must exit 0, got {second_status:?}"
    );

    assert!(
        first.try_wait().expect("try_wait").is_none(),
        "first instance must keep running after a second launch"
    );
    if first_hidden {
        assert!(
            wait_for_log(
                &log,
                "restored window (second launch)",
                Duration::from_secs(15)
            ),
            "second launch must restore the hidden window (ADR condition 5); logs:\n{}",
            log.lock().unwrap()
        );
        #[cfg(windows)]
        assert!(
            wait_for_visibility(true, Duration::from_secs(5)),
            "restored window must actually be visible"
        );
    }

    let _ = first.kill();
    let _ = first.wait();
}
