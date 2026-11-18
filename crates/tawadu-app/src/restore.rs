//! Second-launch restore signal — ADR 0002 condition 5.
//!
//! `single-instance` 0.3 is a pure lock (mutex / abstract socket / flock) with
//! no callback API, so the "restore the existing window" signal is a tiny
//! channel of our own: a named auto-reset event on Windows, a unix socket on
//! Linux (abstract) and macOS (temp-dir path). The first instance listens; a
//! second instance signals and exits.

/// Listen for restore signals from later launches. Binds synchronously, then
/// services accepts/waits on a background thread for the app lifetime.
pub fn listen(on_restore: impl Fn() + Send + 'static) {
    listen_on("tawadu-app", on_restore);
}

/// Tell an already-running instance to restore its window. `false` when no
/// listener answered (no first instance, or the channel failed).
pub fn signal_existing() -> bool {
    signal_on("tawadu-app")
}

#[cfg(target_os = "windows")]
fn listen_on(name: &str, on_restore: impl Fn() + Send + 'static) {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

    let event_name = HSTRING::from(format!("Local\\tawadu-restore-{name}"));
    // Auto-reset, initially unsignalled: one SetEvent wakes one wait.
    let handle = match unsafe { CreateEventW(None, false, false, &event_name) } {
        Ok(handle) => handle,
        Err(error) => {
            log::warn!("restore channel unavailable ({error}); second launches will not restore");
            return;
        }
    };
    struct SendHandle(windows::Win32::Foundation::HANDLE);
    unsafe impl Send for SendHandle {}
    let handle = SendHandle(handle);
    std::thread::Builder::new()
        .name("tawadu-restore".into())
        .spawn(move || {
            let handle = handle;
            loop {
                let wait = unsafe { WaitForSingleObject(handle.0, INFINITE) };
                if wait == WAIT_OBJECT_0 {
                    on_restore();
                } else {
                    log::warn!("restore wait ended ({wait:?}); stopping the listener");
                    break;
                }
            }
            let _ = unsafe { CloseHandle(handle.0) };
        })
        .expect("spawn the restore listener thread");
}

#[cfg(target_os = "windows")]
fn signal_on(name: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};

    let event_name = HSTRING::from(format!("Local\\tawadu-restore-{name}"));
    match unsafe { OpenEventW(EVENT_MODIFY_STATE, false, &event_name) } {
        Ok(handle) => {
            let set = unsafe { SetEvent(handle) }.is_ok();
            let _ = unsafe { CloseHandle(handle) };
            set
        }
        Err(_) => false,
    }
}

#[cfg(unix)]
fn socket_addr(name: &str) -> std::io::Result<std::os::unix::net::SocketAddr> {
    #[cfg(target_os = "linux")]
    {
        // from_abstract_name lives on the SocketAddrExt trait, not SocketAddr.
        use std::os::linux::net::SocketAddrExt;
        std::os::unix::net::SocketAddr::from_abstract_name(format!("tawadu-restore-{name}"))
    }
    #[cfg(target_os = "macos")]
    {
        std::os::unix::net::SocketAddr::from_pathname(
            std::env::temp_dir().join(format!("tawadu-restore-{name}.sock")),
        )
    }
}

#[cfg(unix)]
fn listen_on(name: &str, on_restore: impl Fn() + Send + 'static) {
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    let addr = match socket_addr(name) {
        Ok(addr) => addr,
        Err(error) => {
            log::warn!("restore channel unavailable ({error}); second launches will not restore");
            return;
        }
    };
    let listener = match UnixListener::bind_addr(&addr) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            // macOS leaves a stale socket file behind after a crash. If nobody
            // answers on it, remove it and bind again; if somebody answers, a
            // listener already exists and we must not double-listen.
            if std::os::unix::net::UnixStream::connect_addr(&addr).is_ok() {
                return;
            }
            #[cfg(target_os = "macos")]
            if let Some(path) = addr.as_pathname() {
                let _ = std::fs::remove_file(path);
            }
            match UnixListener::bind_addr(&addr) {
                Ok(listener) => listener,
                Err(error) => {
                    log::warn!("restore channel unavailable ({error})");
                    return;
                }
            }
        }
        Err(error) => {
            log::warn!("restore channel unavailable ({error}); second launches will not restore");
            return;
        }
    };
    std::thread::Builder::new()
        .name("tawadu-restore".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut byte = [0u8; 1];
                if stream.read(&mut byte).is_ok() {
                    on_restore();
                }
            }
        })
        .expect("spawn the restore listener thread");
}

#[cfg(unix)]
fn signal_on(name: &str) -> bool {
    use std::io::Write;
    let Ok(addr) = socket_addr(name) else {
        return false;
    };
    match std::os::unix::net::UnixStream::connect_addr(&addr) {
        Ok(mut stream) => stream.write_all(b"1").is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// Unique per test so the suite stays parallel-safe.
    fn test_name(tag: &str) -> String {
        format!("tawadu-test-{}-{tag}", std::process::id())
    }

    #[test]
    fn signal_without_a_listener_reports_false() {
        assert!(!signal_on(&test_name("absent")));
    }

    #[test]
    fn signal_reaches_the_listener() {
        let name = test_name("roundtrip");
        let (tx, rx) = mpsc::channel();
        listen_on(&name, move || {
            let _ = tx.send(());
        });
        // The listener binds synchronously, but allow a scheduling margin.
        let deadline = Instant::now() + Duration::from_secs(2);
        while !signal_on(&name) {
            assert!(Instant::now() < deadline, "listener never came up");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(()));
    }

    #[test]
    fn every_signal_fires_the_callback() {
        let name = test_name("repeat");
        let (tx, rx) = mpsc::channel();
        listen_on(&name, move || {
            let _ = tx.send(());
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while !signal_on(&name) {
            assert!(Instant::now() < deadline, "listener never came up");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(()));
        assert!(signal_on(&name));
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(()));
    }
}
