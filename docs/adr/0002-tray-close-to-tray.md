# ADR 0002 — Close-to-tray via Slint's public close hook + KeepAlive

- **Status:** Accepted — unanimous panel (Forge, Sentinel, Prism), 2026-11-19.
  Each voter independently verified the cited lines in the vendored crate sources
  before voting.
- **Supersedes:** the "tray blocked" premise in HANDOFF.md §4.1 / §5 Step 1 and
  ROADMAP.md §7.4 (a false negative: the previous team grepped only the `slint`
  facade crate).

## Context

Phase 2's exit criterion requires that closing the window leaves the app alerting
from the tray. Today, closing the window ends the process (verified empirically:
click X → process exits). The handoff recorded this as unfixable without
backend-specific code because "Slint 1.19's public API has no window-close hook".

## Decision — option (c)

Use the public close hook plus an explicit event-loop keepalive, and a normal
tray icon. No winit patching, no second window.

1. `ui.window().on_close_requested(|| slint::CloseRequestResponse::HideWindow)`
   — public, stable API (`slint` re-exports `i_slint_core::api::*`).
2. Hold a `KeepAlive` for the app lifetime, obtained via
   `slint::private_unstable_api::ensure_context()?.keep_alive()`.
   `KeepAlive` is **not nameable** from the facade, so hold it as a `main()`
   local (type inference), never a struct field. Drop it on Quit (or simply call
   `slint::quit_event_loop()` — the Exit path is not keepalive-gated).
3. Tray via `tray-icon` + an **own `muda` dependency**. Poll
   `muda::MenuEvent::receiver()` from a Slint `Timer` or a thread +
   `invoke_from_event_loop`. Menu: Show / Quit. Tooltip: next prayer + time.
   Do **not** enable the winit backend's dormant `muda` feature — it is not
   declared/enabled upstream and would change Slint's menu handling app-wide.
   On Windows, muda spawns its own threads and coexists with Slint's loop.
4. Pin `slint = "=1.19.0"` (was loose `"1"`). Isolate `ensure_context()` behind
   one adapter function with a regression test: close → hidden → loop alive →
   Quit → process exits.
5. Quit = `slint::quit_event_loop()` → `ui.run()` returns → clean process exit.

## Evidence (vendored sources, registry root `index.crates.io-1949cf8c6b5b557f`)

| Claim | Location |
|---|---|
| Facade re-exports core API | `slint-1.19.0/lib.rs:226` |
| `CloseRequestResponse{HideWindow,KeepWindowShown}`, `on_close_requested` | `i-slint-core-1.19.0/api.rs:458,530` |
| WM_CLOSE forwarded to core | `i-slint-backend-winit-1.19.0/winitwindowadapter.rs:1532` |
| Callback → `HideWindow` → `self.hide()` | `i-slint-core-1.19.0/api.rs:760-768`, `window.rs:2708-2713` |
| `hide()` drops the window's keepalive | `i-slint-core-1.19.0/window.rs:2149-2153` |
| Last keepalive drop quits the loop | `i-slint-core-1.19.0/context.rs:99-104` |
| `KeepAlive` doc + Drop semantics | `i-slint-core-1.19.0/context.rs:144-154` |
| `keep_alive()` public | `i-slint-core-1.19.0/context.rs:471` |
| `ensure_context()` public | `slint-1.19.0/private_unstable_api.rs:108` |
| Exit event is not keepalive-gated | `i-slint-backend-winit-1.19.0/event_loop.rs:178-190` |

## Conditions (panel requirements — part of Step 1, not deferrable)

1. **Component lifetime.** `KeepAlive` keeps the *event loop* alive, not the
   *component*. `main` must hold its own strong component handle for the app
   lifetime (the existing `let ui = …` does). The exit test must assert
   pre-alert / athan / iqamah **fire while hidden**, not merely that the process
   survives.
2. **Banner floor while hidden.** Athan, pre-alert and iqamah must restore +
   show the window **and** the in-app banner when hidden; OS toasts are
   secondary. The exit test must assert toast delivery or fall back to
   showing the window — never assume `notify-rust` `Ok` means a toast appeared.
3. **Windows AUMID.** Call `SetCurrentProcessExplicitAppUserModelID` (or
   equivalent registration); until solved, fall back to show-window + banner
   for all alerts.
4. **Linux tray honesty.** If tray creation fails (no StatusNotifier host),
   fall back to `KeepWindowShown` on close — never hide into nothing. Assert
   tray-or-fallback in the test. macOS: Cmd+Q quits (platform norm).
5. **Second launch while hidden** must restore/show the existing window
   (single-instance callback), not print "already running" into the void.

## Alternatives rejected

- **(a) Backend-specific close hook (winit/`windows` crate):** per-OS FFI
  subclass code against the cross-platform principle; unnecessary — the public
  hook exists.
- **(b) Hidden secondary window owning tray/scheduler:** splits scheduler/tray
  lifetime across two window owners; confuses focus/activation and taskbar
  semantics; a hack where a supported API exists.

## Exit test (Step 1)

With `TAWADU_DEBUG_NOW` / `TAWADU_DEBUG_TIME_SCALE`: close the window → tray
icon remains → a pre-alert, an athan and an iqamah each fire **while hidden**
(window restored + banner shown), including after a simulated sleep/wake sweep →
Quit exits the process cleanly → relaunch works. Green gate + CI on all three
OSes + visual check on a real desktop.
