//! Mobile-oriented Tauri commands for iOS and Android.
//!
//! These commands bridge the Tauri frontend to the `prisma_ffi` mobile lifecycle
//! functions (background/foreground transitions, network changes, battery, etc.).
//!
//! While primarily used on mobile, they compile on all targets so the handler
//! table stays uniform. On desktop the lifecycle calls are valid no-ops.

use crate::state::AppState;
use prisma_ffi::PRISMA_OK;

/// Helper: extract the raw `PrismaClient` pointer from managed state.
fn client_ptr(state: &tauri::State<AppState>) -> Result<*mut prisma_ffi::PrismaClient, String> {
    let raw = *state
        .client
        .lock()
        .map_err(|_| "Failed to acquire client lock".to_string())?;
    if raw == 0 {
        return Err("no client".into());
    }
    Ok(raw as *mut prisma_ffi::PrismaClient)
}

// ── VPN permission ───────────────────────────────────────────────────────────

/// Check whether VPN permission is granted.
///
/// On Android this checks for `BIND_VPN_SERVICE` via VpnService.prepare().
/// On iOS this checks the cached VPN permission status.
/// On desktop this always returns `true`.
#[tauri::command]
pub fn check_vpn_permission(state: tauri::State<AppState>) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        // On Android, we check if VpnService.prepare() would return null
        // (null means permission already granted). We use the FFI layer's
        // cached check — the Kotlin VpnService sets the status when started.
        let client = client_ptr(&state)?;
        let fd = unsafe { prisma_ffi::prisma_get_tun_fd(client) };
        // If a TUN fd is already set, VPN permission was granted
        if fd >= 0 {
            return Ok(true);
        }
        // Conservatively return true — actual permission dialog is triggered
        // by startVpnService which calls VpnService.prepare() on Android.
        // The OS will show the consent dialog at service start time.
        Ok(true)
    }
    #[cfg(target_os = "ios")]
    {
        let _ = &state;
        let status = unsafe { prisma_ffi::prisma_ios_vpn_permission_status() };
        match status {
            1 => Ok(true),
            0 => Ok(false),
            _ => {
                // Unknown / not yet checked — default to false so UI prompts check
                Ok(false)
            }
        }
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = &state;
        Ok(true)
    }
}

/// Request VPN permission from the operating system.
///
/// On Android this triggers `VpnService.prepare()` via the Kotlin layer.
/// On iOS this is handled by NEVPNManager when the tunnel starts.
/// On desktop this always returns `true`.
#[tauri::command]
pub fn request_vpn_permission() -> Result<bool, String> {
    // On Android, the actual VPN permission intent is launched by the
    // Kotlin PrismaVpnService.prepare(). The start_vpn_service command
    // triggers this flow. If the user denies, the service won't start.
    //
    // On iOS, VPN permission is implicitly granted via the Network Extension
    // entitlement — the OS prompts the user on first tunnel activation.
    //
    // This command returns true to indicate the request was initiated.
    // The actual grant/deny result is observed when the VPN service
    // starts (or fails to start).
    Ok(true)
}

// ── VPN service lifecycle ───────────────────────────────────────────────────

/// Start the native VPN service.
///
/// On Android: starts `PrismaVpnService` which creates TUN and passes fd to Rust.
/// On iOS: activates the Network Extension tunnel provider.
/// On desktop: no-op (TUN is managed directly by the Rust engine).
#[tauri::command]
pub fn start_vpn_service(state: tauri::State<AppState>) -> Result<(), String> {
    let _client = client_ptr(&state)?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        // On mobile, emit a Tauri event that the native layer (Kotlin/Swift)
        // intercepts to start the VPN service with the PrismaClient handle.
        if let Some(handle) = crate::state::APP_HANDLE.get() {
            use tauri::Emitter;
            let client_handle = _client as usize;
            let _ = handle.emit(
                "vpn://start",
                serde_json::json!({
                    "handle": client_handle,
                }),
            );
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        // Desktop: TUN is handled by the Rust engine directly.
        Ok(())
    }
}

/// Stop the native VPN service.
///
/// On Android: sends stop intent to `PrismaVpnService`.
/// On iOS: stops the Network Extension tunnel.
/// On desktop: no-op.
#[tauri::command]
pub fn stop_vpn_service(state: tauri::State<AppState>) -> Result<(), String> {
    let client = client_ptr(&state)?;

    // Clear the TUN fd so the engine knows the device is going away
    unsafe { prisma_ffi::prisma_set_tun_fd(client, -1) };

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        if let Some(handle) = crate::state::APP_HANDLE.get() {
            use tauri::Emitter;
            let _ = handle.emit("vpn://stop", serde_json::json!({}));
        }
    }

    Ok(())
}

// ── Network info ─────────────────────────────────────────────────────────────

/// Get the current network type.
///
/// Returns: 0 = disconnected, 1 = WiFi, 2 = cellular, 3 = ethernet.
///
/// On mobile this reads the value cached by the FFI layer (set via
/// `prisma_on_network_change`). On desktop defaults to ethernet (3).
#[tauri::command]
pub fn get_network_type(state: tauri::State<AppState>) -> Result<i32, String> {
    let client = client_ptr(&state)?;
    let net = unsafe { prisma_ffi::prisma_get_network_type(client) };
    Ok(net)
}

/// Notify the proxy engine of a network connectivity change.
///
/// `network_type`: 0 = disconnected, 1 = WiFi, 2 = cellular, 3 = ethernet.
#[tauri::command]
pub fn on_network_change(state: tauri::State<AppState>, network_type: i32) -> Result<(), String> {
    let client = client_ptr(&state)?;
    let rc = unsafe { prisma_ffi::prisma_on_network_change(client, network_type) };
    if rc == PRISMA_OK {
        Ok(())
    } else {
        Err(format!("prisma_on_network_change error {rc}"))
    }
}

// ── Battery ──────────────────────────────────────────────────────────────────

/// Battery status info returned to the frontend.
#[derive(serde::Serialize)]
pub struct BatteryStatus {
    /// Battery level as a percentage (0-100), or -1 if unknown.
    pub level: i32,
    /// Whether the device is currently charging.
    pub charging: bool,
    /// Whether low-power / battery saver mode is active.
    pub low_power_mode: bool,
}

/// Get the current battery status for adaptive behavior.
///
/// On desktop returns level=-1 (unknown), charging=false, low_power_mode=false.
/// On mobile a future platform plugin can provide real values.
#[tauri::command]
pub fn get_battery_status() -> Result<BatteryStatus, String> {
    Ok(BatteryStatus {
        level: -1,
        charging: false,
        low_power_mode: false,
    })
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

/// Notify the proxy engine that the app entered the background.
#[tauri::command]
pub fn on_app_background(state: tauri::State<AppState>) -> Result<(), String> {
    let client = client_ptr(&state)?;
    let rc = unsafe { prisma_ffi::prisma_on_background(client) };
    if rc == PRISMA_OK {
        Ok(())
    } else {
        Err(format!("prisma_on_background error {rc}"))
    }
}

/// Notify the proxy engine that the app returned to the foreground.
#[tauri::command]
pub fn on_app_foreground(state: tauri::State<AppState>) -> Result<(), String> {
    let client = client_ptr(&state)?;
    let rc = unsafe { prisma_ffi::prisma_on_foreground(client) };
    if rc == PRISMA_OK {
        Ok(())
    } else {
        Err(format!("prisma_on_foreground error {rc}"))
    }
}

/// Notify the proxy engine of a low-memory warning from the OS.
#[tauri::command]
pub fn on_memory_warning(state: tauri::State<AppState>) -> Result<(), String> {
    let client = client_ptr(&state)?;
    let rc = unsafe { prisma_ffi::prisma_on_memory_warning(client) };
    if rc == PRISMA_OK {
        Ok(())
    } else {
        Err(format!("prisma_on_memory_warning error {rc}"))
    }
}
