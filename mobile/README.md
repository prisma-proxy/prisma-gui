# Mobile Native Code

Platform-specific native code for Android and iOS VPN services. These files are copied into the Tauri-generated `gen/` directory after running `tauri android init` / `tauri ios init`.

## Setup

```bash
# 1. Initialize platform projects (one-time)
npm run tauri android init
npm run tauri ios init

# 2. Copy native code into generated projects
# Android:
cp mobile/android/PrismaVpnService.kt gen/android/app/src/main/java/com/prisma/client/
# Then manually merge mobile/android/AndroidManifest.additions.xml into gen/android/app/src/main/AndroidManifest.xml

# iOS:
# Create a Network Extension target in Xcode, then copy:
cp mobile/ios/PacketTunnelProvider.swift gen/apple/prisma-tunnel/
cp mobile/ios/PrismaTunnel.entitlements gen/apple/prisma-tunnel/

# 3. Build and run
npm run tauri android dev
npm run tauri ios dev
```

## Architecture

```
Frontend (React)
    ↓ Tauri command
mobile.rs (Tauri backend)
    ↓ FFI call
prisma-ffi (android.rs / ios.rs)
    ↓ JNI / C bridge
PrismaVpnService.kt / PacketTunnelProvider.swift
    ↓ OS VPN API
TUN fd → Rust TUN handler → smoltcp → PrismaVeil tunnel
```

## Android

- `PrismaVpnService.kt` — VpnService with foreground notification
- `AndroidManifest.additions.xml` — Required permissions and service declaration
- Requires: `BIND_VPN_SERVICE`, `FOREGROUND_SERVICE`, `ACCESS_NETWORK_STATE`

## iOS

- `PacketTunnelProvider.swift` — NEPacketTunnelProvider implementation
- `PrismaTunnel.entitlements` — Network Extension + App Group entitlements
- Requires: Network Extension capability in Xcode, Apple Developer account
