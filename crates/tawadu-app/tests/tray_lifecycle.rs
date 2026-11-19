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

/// A fresh, isolated config home per spawn: `Storage::default_db_path` is
/// env-driven (APPDATA / HOME / XDG_CONFIG_HOME), so the suite never reads or
/// writes the user's real settings — quiet hours or iqamah offsets on the dev
/// machine must not change what the test asserts.
fn fresh_config_home() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tawadu-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).expect("create test config home");
    dir
}

/// Kills the app on drop — including during a panic unwind, so a failed
/// phase never orphans an instance that holds the single-instance lock
/// against the next run.
struct AppGuard(Child);

impl Drop for AppGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl std::ops::Deref for AppGuard {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}

impl std::ops::DerefMut for AppGuard {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

fn spawn_app(extra: &[(&str, &str)]) -> (AppGuard, Arc<Mutex<String>>) {
    spawn_app_in(&fresh_config_home(), extra)
}

fn spawn_app_in(home: &std::path::Path, extra: &[(&str, &str)]) -> (AppGuard, Arc<Mutex<String>>) {
    let mut cmd = Command::new(BIN);
    cmd.env("RUST_LOG", "info")
        .env("TAWADU_DEBUG_NOW", "2026-11-19T09:00:00Z")
        .env("TAWADU_DEBUG_TIME_SCALE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.env("APPDATA", home);
    #[cfg(target_os = "macos")]
    cmd.env("HOME", home);
    #[cfg(all(unix, not(target_os = "macos")))]
    cmd.env("XDG_CONFIG_HOME", home);
    for (key, value) in extra {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().expect("launch tawadu-app");
    let stderr = child.stderr.take().expect("piped stderr");
    (AppGuard(child), drain(stderr))
}

/// The app's settings DB path inside an isolated config home, per OS.
fn db_path(home: &std::path::Path) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    return home.join("Library/Application Support/tawadu/tawadu.db");
    #[cfg(not(target_os = "macos"))]
    return home.join("tawadu/tawadu.db");
}

/// Pre-seed the isolated settings DB (the app loads it at startup). The
/// subdir must mirror `Storage::default_db_path` per OS: macOS nests under
/// Library/Application Support, Windows (APPDATA) and Linux (XDG) do not.
fn write_settings(home: &std::path::Path, settings: &tawadu_core::settings::Settings) {
    #[cfg(target_os = "macos")]
    let dir = home.join("Library/Application Support/tawadu");
    #[cfg(not(target_os = "macos"))]
    let dir = home.join("tawadu");
    std::fs::create_dir_all(&dir).expect("create test db dir");
    let storage =
        tawadu_core::storage::Storage::open(&dir.join("tawadu.db")).expect("open test settings db");
    storage.save_settings(settings).expect("seed test settings");
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

fn wait_exit(child: &mut AppGuard, timeout: Duration) -> Option<std::process::ExitStatus> {
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
    // Pre-flight: the suite needs the global single-instance lock. Fail fast
    // with guidance instead of burning marker deadlines against an app that
    // exited "already running". The probe handle drops immediately.
    {
        let probe = single_instance::SingleInstance::new("tawadu-app").expect("probe the lock");
        assert!(
            probe.is_single(),
            "another Tawadu instance is running — wait for it to exit before \
             running this suite (do not blind-kill: it may be a teammate's \
             timed exit test)"
        );
    }

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
    #[cfg(windows)]
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

    // ---- phase 3: alerts FIRE while hidden (ADR conditions 1-2) ----
    // The regression Sentinel required: phases 1-2 would pass with the
    // notification engine dead. Scale 60 + a 1 s real slice keeps each wake
    // 60 debug-seconds ahead — inside the 120 s late tolerance. NOW 01:00Z =
    // 04:00 Riyadh: pre-alert ~01:41Z (~41 s real), athan ~01:51Z (~51 s),
    // iqamah ~02:01Z (~61 s; Fajr offset 10 seeded below). The close lands at
    // setup + 1.5 s, tens of seconds before the first alert even on slow CI.
    // One retry: on a shared desktop a teammate's second launch can signal
    // the restore channel mid-phase, legitimately showing the window and
    // clearing the hidden premise — detect that signature and rerun once.
    let mut phase3_done = false;
    for attempt in 0..2 {
        let home = fresh_config_home();
        let mut settings = tawadu_core::settings::Settings::default();
        settings.iqamah.fajr = 10;
        write_settings(&home, &settings);
        let (mut alerting, log) = spawn_app_in(
            &home,
            &[
                ("TAWADU_DEBUG_NOW", "2026-11-19T01:00:00Z"),
                ("TAWADU_DEBUG_TIME_SCALE", "60"),
                // 1 s real slice = 60 debug-seconds per wake, inside the 120 s
                // late tolerance. Without it the default 60 s slice jumps 3600
                // debug-seconds and drops every event (verified RED).
                ("TAWADU_DEBUG_MAX_SLICE_MS", "1000"),
                ("TAWADU_DEBUG_AUTOCLOSE_MS", "1500"),
                ("TAWADU_DEBUG_BANNER_MS", "60000"),
            ],
        );
        let tray_path = wait_for_either(
            &log,
            "hidden to tray",
            "tray unavailable",
            Duration::from_secs(30),
        ) == Some(0);
        #[cfg(windows)]
        if tray_path {
            assert!(
                wait_for_visibility(false, Duration::from_secs(5)),
                "window must be hidden before the alerts fire"
            );
        }
        for marker in ["firing pre_alert", "firing prayer", "firing iqamah"] {
            assert!(
                wait_for_log(&log, marker, Duration::from_secs(90)),
                "alert did not fire while hidden: {marker}; logs:\n{}",
                log.lock().unwrap()
            );
        }
        if tray_path {
            let restored = wait_for_log(
                &log,
                "restored window for alert banner",
                Duration::from_secs(10),
            );
            let interfered = log
                .lock()
                .unwrap()
                .contains("restored window (second launch)");
            if !restored && interfered && attempt == 0 {
                // An external second launch showed the window mid-phase;
                // the hidden premise was disturbed, not the banner floor.
                let _ = alerting.kill();
                let _ = alerting.wait();
                continue;
            }
            assert!(
                restored,
                "the banner floor must restore + show the window for alerts; logs:\n{}",
                log.lock().unwrap()
            );
            #[cfg(windows)]
            assert!(
                wait_for_visibility(true, Duration::from_secs(5)),
                "restored window must actually be visible"
            );
        }
        let _ = alerting.kill();
        let _ = alerting.wait();
        phase3_done = true;
        break;
    }
    assert!(phase3_done, "phase 3 did not complete");

    // ---- phase 4: fired-event ledger — no re-announce on restart ----
    // Fire the pre-alert, kill, restart with the SAME debug-clock anchor
    // (the clock resets to the anchor per process, so the event is due
    // again ~41 s in). The ledger must skip it; the athan — a different
    // identity — must still fire, proving the engine stayed alive.
    let home = fresh_config_home();
    let mut settings = tawadu_core::settings::Settings::default();
    settings.iqamah.fajr = 10;
    write_settings(&home, &settings);
    let ledger_env = [
        ("TAWADU_DEBUG_NOW", "2026-11-19T01:00:00Z"),
        ("TAWADU_DEBUG_TIME_SCALE", "60"),
        ("TAWADU_DEBUG_MAX_SLICE_MS", "1000"),
        ("TAWADU_DEBUG_BANNER_MS", "60000"),
    ];
    let (mut first, log) = spawn_app_in(&home, &ledger_env);
    assert!(
        wait_for_log(&log, "firing pre_alert", Duration::from_secs(90)),
        "pre-alert must fire before the restart; logs:\n{}",
        log.lock().unwrap()
    );
    let _ = first.kill();
    let _ = first.wait();
    let (mut second, log2) = spawn_app_in(&home, &ledger_env);
    assert!(
        wait_for_log(&log2, "skipping already-fired", Duration::from_secs(90)),
        "the ledger must skip the re-detected pre-alert after restart; logs:\n{}",
        log2.lock().unwrap()
    );
    assert!(
        wait_for_log(&log2, "firing prayer", Duration::from_secs(90)),
        "the athan (a different identity) must still fire after restart; logs:\n{}",
        log2.lock().unwrap()
    );
    assert!(
        !log2.lock().unwrap().contains("firing pre_alert"),
        "the pre-alert was re-announced after restart; logs:\n{}",
        log2.lock().unwrap()
    );
    let _ = second.kill();
    let _ = second.wait();

    // ---- phase 5: the trailing debounced write survives abrupt exit ----
    // Touch a setting at 2.0 s (800 ms debounce pending), quit at 2.6 s —
    // BEFORE the timer fires. The quit flush must persist the change.
    let home = fresh_config_home();
    let (mut quitter, _log) = spawn_app_in(
        &home,
        &[
            ("TAWADU_DEBUG_TOUCH_SETTING_MS", "2000"),
            ("TAWADU_DEBUG_AUTOQUIT_MS", "2600"),
        ],
    );
    assert!(
        wait_exit(&mut quitter, Duration::from_secs(20)).is_some_and(|s| s.success()),
        "app must exit cleanly via the quit hook"
    );
    let storage = tawadu_core::storage::Storage::open(&db_path(&home))
        .expect("settings DB must exist after the quit flush");
    match storage.load_settings() {
        tawadu_core::storage::LoadOutcome::Loaded(s) => assert_eq!(
            s.pre_alert_minutes, 15,
            "the trailing debounced write must survive quit mid-debounce"
        ),
        _ => panic!("settings must be persisted by the quit flush"),
    }

    // Hide variant: close-to-tray at 2.6 s flushes too; then a HARD kill.
    let home = fresh_config_home();
    let (mut hider, log) = spawn_app_in(
        &home,
        &[
            ("TAWADU_DEBUG_TOUCH_SETTING_MS", "2000"),
            ("TAWADU_DEBUG_AUTOCLOSE_MS", "2600"),
        ],
    );
    assert!(
        wait_for_log(&log, "hidden to tray", Duration::from_secs(20)),
        "app must hide to tray; logs:\n{}",
        log.lock().unwrap()
    );
    let _ = hider.kill();
    let _ = hider.wait();
    let storage = tawadu_core::storage::Storage::open(&db_path(&home))
        .expect("settings DB must exist after the hide flush");
    match storage.load_settings() {
        tawadu_core::storage::LoadOutcome::Loaded(s) => assert_eq!(
            s.pre_alert_minutes, 15,
            "the trailing debounced write must survive hide-then-kill"
        ),
        _ => panic!("settings must be persisted by the hide flush"),
    }
}
