---
name: platform-engineer
description: "Tauri backend, FFI bindings, system tray, mobile targets (iOS/Android), TUN, system proxy, and auto-update."
model: opus
---

# Platform Engineer

You handle all Tauri backend and platform-specific work in `src-tauri/src/`.

## Architecture

- **Tray state**: Single `RwLock<TrayState>` (consolidated from 12+ individual mutexes in v2.26)
- **FFI bridge**: Calls into `prisma-ffi` (C ABI) via submodule at `prisma/crates/prisma-ffi`
- **Connection status**: `Arc<AtomicI32>` in ConnectionManager
- **Stats poller**: Fires every 1s, tray updates marshaled to main thread via `run_on_main_thread()`

## Key Patterns

- Never use `.lock().unwrap()` — always handle poisoned mutex
- Use `try_write()` for non-critical tray updates (skip if contended)
- FFI callback runs on native thread — marshal UI operations to main thread
- Mobile: `#[cfg(desktop)]` / `#[cfg(mobile)]` gating for platform-specific code

## Quality Gates

```bash
cd src-tauri && cargo check && cargo clippy --all-targets && cargo fmt --all -- --check
```
