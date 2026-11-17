# ADR 0001 — Use Slint for the interface

- Status: accepted
- Date: 2026-11-17

## Context

Tawadu is a cross-platform desktop prayer app (Windows 10/11 first, then macOS and
Linux). It must run well on older hardware, be offline-first, and have a calm,
modern, Arabic-first interface. The interface layer also has to be reusable for a
possible mobile companion later.

## Options considered

| Option | Verdict |
|---|---|
| Tauri 2 | Best system-integration plugins, but needs WebView2 and ~100–200 MB RAM; a poor fit for the target Windows 10 machine |
| **Slint** | Pure Rust, compiled, GPU-rendered, no WebView2, small binary/RAM, cross-platform; mobile support exists (less mature) |
| egui | Fast to prototype, but feels tool-like and RTL layout is manual work |
| Iced | Clean architecture, smaller ecosystem |

## Decision

Use **Slint** with a Rust core. The interface is a thin layer over `tawadu-core`;
all prayer maths, scheduling, audio and storage live in pure Rust crates.

## Consequences

- No WebView2 dependency → runs on older Windows 10 PCs.
- Small binary and idle cost → meets the performance budget.
- System integration (tray, notifications, autostart) comes from separate crates
  (`tray-icon`, `notify-rust`, `auto-launch`, `single-instance`).
- **Risk:** Arabic shaping / RTL must be verified on all three platforms early.
- **Risk:** Slint mobile is less mature → mobile stays a P2 decision.

## Follow-ups

- Verify Arabic shaping on Windows/macOS/Linux in week 1.
- Port the tokens in `design/brand.md` into `.slint` theme files before building screens.
