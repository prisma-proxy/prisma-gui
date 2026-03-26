# Prisma GUI

Desktop + mobile client for the Prisma encrypted proxy system. Built with Tauri 2 + React 19.

Version: 2.26.0

## Architecture

```
src/                   React 19 frontend (TypeScript)
  pages/               Route pages (Home, Profiles, Connections, Logs, etc.)
  components/          Reusable UI components (shadcn-style + custom)
  hooks/               Custom hooks (useConnection, usePrismaEvents, useStatusSync, etc.)
  store/               Zustand stores (main, settings, connections, analytics, etc.)
  i18n/                Translations (en.json, zh-CN.json)
  lib/                 Utilities (commands.ts for Tauri IPC, format.ts, types.ts)

src-tauri/             Tauri 2 Rust backend
  src/
    lib.rs             App setup, FFI callback, lifecycle
    commands.rs        IPC command handlers
    tray.rs            System tray menu + status updates
    state.rs           Global app state
    mobile.rs          iOS/Android specific handlers
  gen/
    android/           Generated Gradle project
    apple/             Generated Xcode project

prisma/                Git submodule → github.com/prisma-proxy/prisma
  crates/prisma-ffi/   C FFI shared library (direct Cargo dependency)
  crates/prisma-core/  Shared types, crypto, protocol (direct Cargo dependency)
```

## Setup

```bash
git clone --recurse-submodules https://github.com/prisma-proxy/prisma-gui.git
cd prisma-gui
npm install
```

## Key Commands

```bash
npm run dev                              # Dev mode (desktop)
npm run build                            # Production build (frontend only)
npx tsc --noEmit                         # TypeScript check
cargo check                              # Rust check (run from src-tauri/)
cargo clippy --all-targets               # Lint Rust (run from src-tauri/)
cargo fmt --all -- --check               # Format check Rust
npx tauri dev                            # Full Tauri dev (frontend + backend)
npx tauri build                          # Full production build
npx tauri ios dev                        # iOS development
npx tauri android dev                    # Android development
```

## Dependencies

The Tauri backend depends on two crates from the main Prisma repo via git submodule:

```toml
# src-tauri/Cargo.toml
prisma-ffi  = { path = "prisma/crates/prisma-ffi" }
prisma-core = { path = "prisma/crates/prisma-core" }
```

To update the submodule to latest:
```bash
cd prisma && git pull origin master && cd ..
git add prisma && git commit -m "chore: update prisma submodule"
```

## Key Patterns

- **State**: Zustand stores in `src/store/`. Use `getState()` for non-reactive reads. Never create objects inside selectors.
- **IPC**: Tauri commands in `src/lib/commands.ts` → Rust handlers in `src-tauri/src/commands.rs`
- **Events**: Backend fires `prisma://event` → frontend listens in `src/hooks/usePrismaEvents.ts`
- **Stats**: Backend sends stats every 1s → batched via `requestAnimationFrame` before store update
- **Persist**: Stores using `persist()` middleware write to localStorage. Use debounced manual persist for frequently-updated stores.
- **i18n**: All user-facing strings via i18next. Files: `src/i18n/locales/{en,zh-CN}.json`
- **Dark mode**: Tailwind class-based (`.dark` on html). Canvas components detect via `document.documentElement.classList`.

## Agent System

Use `gui-orchestrator` for any project work. It coordinates specialized agents:

| Agent | Scope |
|-------|-------|
| `gui-orchestrator` | Primary entry point for all tasks |
| `frontend-engineer` | React pages, components, hooks, stores, i18n |
| `platform-engineer` | Tauri backend, FFI, tray, mobile, system proxy |
| `version-sync` | Atomic version bump across 3 files |
