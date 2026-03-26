# Prisma GUI

Cross-platform desktop and mobile client for the [Prisma](https://github.com/prisma-proxy/prisma) encrypted proxy system.

Built with **Tauri 2** + **React 19** — a single codebase for Windows, macOS, Linux, iOS, and Android.

## Features

- One-click connect/disconnect with multiple server profiles
- Real-time speed graph, session stats, and connection quality indicator
- System tray with live speed stats, quick toggles, and recent connections
- SOCKS5, system proxy, TUN, and per-app proxy modes
- Subscription support with redeem codes and auto-update
- Profile import via QR code, clipboard, or subscription URL
- Routing rules with GeoSite/GeoIP and rule providers
- Real-time connection tracking with virtual scrolling
- Speed test (download + upload via Cloudflare)
- Network diagnostics (ping, DNS lookup, connectivity test)
- Traffic analytics by domain, daily trends, and rule stats
- Auto-reconnect with exponential backoff
- Light and dark mode
- i18n: English and Chinese (Simplified)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + 1` | Home |
| `Cmd/Ctrl + 2` | Profiles |
| `Cmd/Ctrl + 3` | Rules |
| `Cmd/Ctrl + 4` | Logs |
| `Cmd/Ctrl + 5` | Speed Test |
| `Cmd/Ctrl + 6` | Settings |
| `Cmd/Ctrl + K` | Toggle connection |
| `Cmd/Ctrl + N` | Go to Profiles |

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) stable toolchain
- [Tauri 2 CLI](https://v2.tauri.app/start/prerequisites/): `npm install -g @tauri-apps/cli@^2`
- Platform-specific dependencies:
  - **Linux**: `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libgtk-3-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools + WebView2

### Clone and Build

```bash
# Standalone repo (future)
git clone --recurse-submodules https://github.com/prisma-proxy/prisma-gui.git
cd prisma-gui
npm install

# Or from monorepo (current)
cd apps/prisma-gui
npm install
```

### Development

```bash
npx tauri dev          # Desktop dev mode with hot reload
```

### Production Build

```bash
npx tauri build        # Builds for current platform
```

### Mobile

```bash
npx tauri ios init     # First-time iOS setup
npx tauri ios dev      # iOS simulator

npx tauri android init # First-time Android setup
npx tauri android dev  # Android emulator
```

## Architecture

```
Frontend (React 19)
    |
    | Tauri IPC (invoke/listen)
    v
Tauri Backend (Rust)
    |
    | C FFI (prisma_connect, prisma_disconnect, ...)
    v
prisma-ffi -> prisma-client -> prisma-core
    |
    | Encrypted tunnel
    v
prisma-server
```

## Project Structure

```
src/                   React frontend
  pages/               Route pages (Home, Profiles, Connections, Logs, etc.)
  components/          UI components (Sidebar, StatusBar, SpeedGraph, etc.)
  hooks/               Custom hooks (useConnection, usePrismaEvents, etc.)
  store/               Zustand state management
  i18n/                Translations (en.json, zh-CN.json)
  lib/                 Utilities, Tauri command wrappers, types

src-tauri/             Tauri Rust backend
  src/                 App lifecycle, IPC commands, system tray
  gen/                 Generated mobile projects (iOS/Android)
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Tauri 2 |
| Frontend | React 19, TypeScript, Vite 6 |
| Styling | Tailwind CSS, Radix UI |
| State | Zustand (with persist middleware) |
| Virtualization | @tanstack/react-virtual |
| Routing | React Router v7 |
| i18n | i18next + react-i18next |
| Backend | Rust, prisma-ffi (C FFI) |

## License

MIT
