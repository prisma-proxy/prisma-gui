//! Mobile-oriented Tauri commands for iOS and Android.
//!
//! VPN permission and service commands use the `tauri-plugin-vpn` Kotlin plugin
//! on Android, which calls `VpnService.prepare()` and starts `PrismaVpnService`.
//! Other lifecycle commands (background, foreground, network, battery) call
//! directly into `prisma_ffi`.

use crate::state::AppState;
use prisma_ffi::PRISMA_OK;
#[cfg(mobile)]
use tauri::Manager;

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
/// Android: calls VpnPlugin.checkPermission() → VpnService.prepare().
/// iOS: calls VpnPlugin.checkPermission() → NETunnelProviderManager.loadAllFromPreferences.
/// Desktop: always true.
#[tauri::command]
pub fn check_vpn_permission(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<bool, String> {
    #[cfg(mobile)]
    {
        let _ = &state;
        let vpn = app.state::<tauri_plugin_vpn::Vpn<tauri::Wry>>();
        vpn.check_permission().map(|r| r.granted)
    }
    #[cfg(not(mobile))]
    {
        let _ = (&app, &state);
        Ok(true)
    }
}

/// Request VPN permission from the operating system.
///
/// Android: launches the system VPN consent dialog.
/// iOS: saves a VPN config profile which triggers the system "Allow VPN" alert.
/// Desktop: always true.
#[tauri::command]
pub fn request_vpn_permission(app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(mobile)]
    {
        let vpn = app.state::<tauri_plugin_vpn::Vpn<tauri::Wry>>();
        vpn.request_permission().map(|r| r.granted)
    }
    #[cfg(not(mobile))]
    {
        let _ = &app;
        Ok(true)
    }
}

// ── VPN service lifecycle ───────────────────────────────────────────────────

/// Start the native VPN service.
///
/// Android: starts PrismaVpnService via VpnPlugin.
/// iOS: starts NETunnelProviderManager tunnel via VpnPlugin.
/// Desktop: no-op.
#[tauri::command]
pub fn start_vpn_service(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let _client = client_ptr(&state)?;

    #[cfg(mobile)]
    {
        // Disconnect any previous session to release bound ports
        let rc = unsafe { prisma_ffi::prisma_disconnect(_client) };
        if rc != PRISMA_OK {
            tracing::warn!(rc, "prisma_disconnect before VPN start returned non-zero");
        }

        let vpn = app.state::<tauri_plugin_vpn::Vpn<tauri::Wry>>();
        // Stop any existing VPN service first
        let _ = vpn.stop_service();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let result = vpn.start_service(_client as u64)?;
        if !result.success {
            return Err(result
                .message
                .unwrap_or_else(|| "Failed to start VPN".into()));
        }
        // Poll for the TUN fd in a background thread.
        // Wait up to 30 seconds (150 × 200ms) for the VPN service to create the TUN.
        // Convert raw pointer to usize (which is Send) to cross the thread boundary.
        let client_addr = _client as usize;
        let handle = app.clone();
        std::thread::spawn(move || {
            let vpn = handle.state::<tauri_plugin_vpn::Vpn<tauri::Wry>>();
            for i in 0..150 {
                std::thread::sleep(std::time::Duration::from_millis(200));
                match vpn.get_tun_fd() {
                    Ok(r) if r.fd >= 0 => {
                        tracing::info!(fd = r.fd, poll_count = i, "Got TUN fd from VPN service");
                        let client = client_addr as *mut prisma_ffi::PrismaClient;
                        unsafe { prisma_ffi::prisma_set_tun_fd(client, r.fd) };
                        return;
                    }
                    Ok(_) => {} // fd == -1, service not ready yet
                    Err(e) => {
                        tracing::warn!(error = %e, poll_count = i, "Error polling TUN fd");
                    }
                }
            }
            tracing::error!("Timed out waiting for TUN fd from VPN service (30s)");
        });
        Ok(())
    }
    #[cfg(not(mobile))]
    {
        let _ = &app;
        Ok(())
    }
}

/// Stop the native VPN service.
///
/// Android: stops PrismaVpnService via VpnPlugin.stopService().
/// iOS: emits stop event.
/// Desktop: no-op.
#[tauri::command]
pub fn stop_vpn_service(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let _client = client_ptr(&state)?;
    unsafe { prisma_ffi::prisma_set_tun_fd(_client, -1) };

    #[cfg(mobile)]
    {
        let vpn = app.state::<tauri_plugin_vpn::Vpn<tauri::Wry>>();
        let _ = vpn.stop_service();
    }
    #[cfg(not(mobile))]
    {
        let _ = &app;
    }

    Ok(())
}

// ── Network info ─────────────────────────────────────────────────────────────

/// Get the current network type.
/// Returns: 0 = disconnected, 1 = WiFi, 2 = cellular, 3 = ethernet.
#[tauri::command]
pub fn get_network_type(state: tauri::State<AppState>) -> Result<i32, String> {
    let client = client_ptr(&state)?;
    let net = unsafe { prisma_ffi::prisma_get_network_type(client) };
    Ok(net)
}

/// Notify the proxy engine of a network connectivity change.
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

#[derive(serde::Serialize)]
pub struct BatteryStatus {
    pub level: i32,
    pub charging: bool,
    pub low_power_mode: bool,
}

#[tauri::command]
pub fn get_battery_status() -> Result<BatteryStatus, String> {
    Ok(BatteryStatus {
        level: -1,
        charging: false,
        low_power_mode: false,
    })
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

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
