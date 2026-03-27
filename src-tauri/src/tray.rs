use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Emitter, Manager};

const TRAY_SIZE: u32 = 22;

fn icon_off() -> Image<'static> {
    Image::new(
        include_bytes!("../icons/tray-off.rgba"),
        TRAY_SIZE,
        TRAY_SIZE,
    )
}
fn icon_on() -> Image<'static> {
    Image::new(
        include_bytes!("../icons/tray-on.rgba"),
        TRAY_SIZE,
        TRAY_SIZE,
    )
}
fn icon_connecting() -> Image<'static> {
    Image::new(
        include_bytes!("../icons/tray-connecting.rgba"),
        TRAY_SIZE,
        TRAY_SIZE,
    )
}

// ── Formatting helpers ──────────────────────────────────────────────────────

fn fmt_speed(bps: f64) -> String {
    if bps < 1_024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1_048_576.0 {
        format!("{:.1} KB/s", bps / 1_024.0)
    } else {
        format!("{:.1} MB/s", bps / 1_048_576.0)
    }
}

fn fmt_bytes(bytes: f64) -> String {
    if bytes < 1_024.0 {
        format!("{:.0} B", bytes)
    } else if bytes < 1_048_576.0 {
        format!("{:.1} KB", bytes / 1_024.0)
    } else if bytes < 1_073_741_824.0 {
        format!("{:.1} MB", bytes / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes / 1_073_741_824.0)
    }
}

fn fmt_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3_600, (secs % 3_600) / 60)
    }
}

// ── Submenu builders ────────────────────────────────────────────────────────
// All submenu builders read from TRAY_STATE (already locked by the caller via
// a read-guard snapshot) so they never acquire individual mutexes.

fn build_proxy_mode_submenu(
    mgr: &AppHandle,
    current_mode: u32,
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let check = |flag: u32| if current_mode == flag { "\u{2713} " } else { "  " };

    SubmenuBuilder::new(mgr, "Proxy Mode")
        .item(&MenuItem::with_id(
            mgr,
            "mode:system",
            format!("{}System Proxy", check(0x02)),
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            mgr,
            "mode:direct",
            format!("{}Direct (SOCKS5 only)", check(0x01)),
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            mgr,
            "mode:pac",
            format!("{}PAC", check(0x10)),
            true,
            None::<&str>,
        )?)
        .build()
}

fn build_speed_stats_submenu(
    mgr: &AppHandle,
    speed_up: &str,
    speed_down: &str,
    total_up: &str,
    total_down: &str,
    connections: &str,
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let upload = MenuItem::with_id(
        mgr,
        "stat:upload",
        format!("\u{2191} Upload: {speed_up}"),
        false,
        None::<&str>,
    )?;
    let download = MenuItem::with_id(
        mgr,
        "stat:download",
        format!("\u{2193} Download: {speed_down}"),
        false,
        None::<&str>,
    )?;
    let total_up_item =
        MenuItem::with_id(mgr, "stat:total-up", format!("Total Up: {total_up}"), false, None::<&str>)?;
    let total_down_item = MenuItem::with_id(
        mgr,
        "stat:total-down",
        format!("Total Down: {total_down}"),
        false,
        None::<&str>,
    )?;
    let connections_item = MenuItem::with_id(
        mgr,
        "stat:connections",
        format!("Connections: {connections}"),
        false,
        None::<&str>,
    )?;

    SubmenuBuilder::new(mgr, "Speed Stats")
        .item(&upload)
        .item(&download)
        .separator()
        .item(&total_up_item)
        .item(&total_down_item)
        .item(&connections_item)
        .build()
}

fn build_quick_toggles_submenu(
    mgr: &AppHandle,
    auto_connect: bool,
    allow_lan: bool,
    tun_enabled: bool,
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let check = |v: bool| if v { "\u{2713} " } else { "  " };

    SubmenuBuilder::new(mgr, "Quick Toggles")
        .item(&MenuItem::with_id(
            mgr,
            "toggle:autoConnect",
            format!("{}Auto-connect on startup", check(auto_connect)),
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            mgr,
            "toggle:allowLan",
            format!("{}Allow LAN connections", check(allow_lan)),
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            mgr,
            "toggle:tunEnabled",
            format!("{}TUN mode", check(tun_enabled)),
            true,
            None::<&str>,
        )?)
        .build()
}

fn build_recent_connections_submenu(
    mgr: &AppHandle,
    recent: &[String],
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let mut sub = SubmenuBuilder::new(mgr, "Recent Connections");
    if recent.is_empty() {
        sub = sub.item(&MenuItem::with_id(
            mgr,
            "recent:empty",
            "(empty)",
            false,
            None::<&str>,
        )?);
    } else {
        for (i, dest) in recent.iter().enumerate() {
            sub = sub.item(&MenuItem::with_id(
                mgr,
                format!("recent:{i}"),
                dest.as_str(),
                false,
                None::<&str>,
            )?);
        }
    }
    sub.build()
}

fn build_profiles_submenu(
    app: &AppHandle,
    profiles_json: &str,
    active_id: Option<&str>,
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let profiles: Vec<serde_json::Value> =
        serde_json::from_str(profiles_json).unwrap_or_default();

    let mut sub = SubmenuBuilder::new(app, "Profiles");
    if profiles.is_empty() {
        sub = sub.item(&MenuItem::with_id(
            app,
            "profile:none",
            "(no profiles)",
            false,
            None::<&str>,
        )?);
    } else {
        for p in &profiles {
            if let (Some(id), Some(name)) = (p["id"].as_str(), p["name"].as_str()) {
                let is_active = active_id == Some(id);
                let label = if is_active {
                    format!("\u{25CF} {name}")
                } else {
                    format!("  {name}")
                };
                sub = sub.item(&MenuItem::with_id(
                    app,
                    format!("profile:{id}"),
                    label,
                    true,
                    None::<&str>,
                )?);
            }
        }
    }
    sub.build()
}

// ── Build the full menu ─────────────────────────────────────────────────────
// Takes a single TRAY_STATE.read() snapshot and builds every submenu from it.
// No individual mutex locks are acquired.

fn build_full_menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    // Take a read snapshot of all tray state at once.
    // If the RwLock is poisoned, recover the guard so the menu still renders.
    let state = crate::state::TRAY_STATE.read().unwrap_or_else(|poisoned| {
        tracing::warn!("TRAY_STATE RwLock poisoned, recovering");
        poisoned.into_inner()
    });

    let connect = MenuItem::with_id(
        app,
        "tray-connect",
        &state.connect_label,
        true,
        None::<&str>,
    )?;

    let show = MenuItem::with_id(app, "tray-show", "Show Window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit Prisma", true, None::<&str>)?;
    let copy_addr = MenuItem::with_id(
        app,
        "tray-copy-addr",
        "Copy Proxy Address",
        true,
        None::<&str>,
    )?;
    let copy_terminal = MenuItem::with_id(
        app,
        "copy-terminal-proxy",
        "Copy Terminal Proxy",
        true,
        None::<&str>,
    )?;
    let check_updates = MenuItem::with_id(
        app,
        "tray-check-update",
        "Check for Updates",
        true,
        None::<&str>,
    )?;
    let update_providers = MenuItem::with_id(
        app,
        "tray-update-providers",
        "Update Rule Providers",
        true,
        None::<&str>,
    )?;

    let mode_sub = build_proxy_mode_submenu(app, state.proxy_mode)?;
    let profiles_sub = build_profiles_submenu(
        app,
        &state.profiles_json,
        state.active_profile_id.as_deref(),
    )?;
    let stats_sub = build_speed_stats_submenu(
        app,
        &state.speed_up,
        &state.speed_down,
        &state.total_up,
        &state.total_down,
        &state.connections,
    )?;
    let recent_sub = build_recent_connections_submenu(app, &state.recent_connections)?;
    let toggles_sub = build_quick_toggles_submenu(
        app,
        state.auto_connect,
        state.allow_lan,
        state.tun_enabled,
    )?;

    // Drop the read guard before building the menu (no longer needed)
    drop(state);

    MenuBuilder::new(app)
        .item(&connect)
        .separator()
        .item(&mode_sub)
        .item(&profiles_sub)
        .separator()
        .item(&stats_sub)
        .item(&recent_sub)
        .separator()
        .item(&toggles_sub)
        .separator()
        .item(&update_providers)
        .item(&check_updates)
        .separator()
        .item(&copy_addr)
        .item(&copy_terminal)
        .separator()
        .item(&show)
        .item(&quit)
        .build()
}

/// Helper: rebuild the tray menu from current TRAY_STATE. Used after state mutations.
fn rebuild_menu(app: &AppHandle) {
    if let Ok(menu) = build_full_menu(app) {
        if let Some(tray) = app.tray_by_id("prisma-tray") {
            if let Err(e) = tray.set_menu(Some(menu)) {
                tracing::warn!("Failed to set tray menu: {}", e);
            }
        }
    }
}

// ── Handle menu events ──────────────────────────────────────────────────────

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "tray-quit" => {
            let _ = prisma_ffi::prisma_clear_system_proxy();
            app.exit(0);
        }
        "tray-show" => {
            if let Some(w) = app.get_webview_window("main") {
                if let Err(e) = w.show() {
                    tracing::warn!("Failed to show window: {}", e);
                }
                if let Err(e) = w.set_focus() {
                    tracing::warn!("Failed to set window focus: {}", e);
                }
            }
        }
        "tray-connect" => {
            if let Err(e) = app.emit("tray://connect-toggle", ()) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "tray-copy-addr" => {
            if let Err(e) = app.emit("tray://copy-proxy-address", ()) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "copy-terminal-proxy" => {
            if let Err(e) = app.emit("tray://copy-terminal-proxy", ()) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "tray-check-update" => {
            if let Err(e) = app.emit("tray://check-update", ()) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "tray-update-providers" => {
            if let Err(e) = app.emit("tray://update-providers", ()) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "mode:system" => {
            if let Err(e) = app.emit("tray://proxy-mode-change", 0x02u32) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "mode:direct" => {
            if let Err(e) = app.emit("tray://proxy-mode-change", 0x01u32) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        "mode:pac" => {
            if let Err(e) = app.emit("tray://proxy-mode-change", 0x10u32) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        id if id.starts_with("profile:") => {
            let profile_id = id["profile:".len()..].to_owned();
            if let Err(e) = app.emit("tray://profile-select", profile_id) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
        }
        id if id.starts_with("toggle:") => {
            let key = id["toggle:".len()..].to_owned();
            // Flip the current value under write lock, then drop before rebuilding
            let new_value = {
                let mut guard = match crate::state::TRAY_STATE.write() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                match key.as_str() {
                    "autoConnect" => {
                        guard.auto_connect = !guard.auto_connect;
                        guard.auto_connect
                    }
                    "allowLan" => {
                        guard.allow_lan = !guard.allow_lan;
                        guard.allow_lan
                    }
                    "tunEnabled" => {
                        guard.tun_enabled = !guard.tun_enabled;
                        guard.tun_enabled
                    }
                    _ => return,
                }
                // guard is dropped here at end of block
            };
            if let Err(e) = app.emit(
                "tray://toggle-setting",
                serde_json::json!({ "key": key, "value": new_value }),
            ) {
                tracing::warn!("Failed to emit tray event: {}", e);
            }
            // Rebuild menu after lock is released to update checkmarks
            rebuild_menu(app);
        }
        _ => {}
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

pub fn setup(app: &App) -> tauri::Result<()> {
    // Initialize default strings in TRAY_STATE
    crate::state::init_tray_state();

    // Populate cached profiles_json from FFI so first menu build has data
    {
        let ptr = prisma_ffi::prisma_profiles_list_json();
        if !ptr.is_null() {
            let json = unsafe {
                let s = std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string();
                prisma_ffi::prisma_free_string(ptr as *mut _);
                s
            };
            if let Ok(mut guard) = crate::state::TRAY_STATE.write() {
                guard.profiles_json = json;
            }
        }
    }

    let handle = app.handle();
    let menu = build_full_menu(handle)?;

    TrayIconBuilder::with_id("prisma-tray")
        .tooltip("Prisma")
        .icon(icon_off())
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    Ok(())
}

/// Update the tray icon and connect/disconnect label based on connection status.
/// If the status changed (connected vs disconnected), the menu is rebuilt to reflect
/// the new connect label. Uses try_write to avoid blocking the UI thread.
pub fn update_status(handle: &AppHandle, status: i32) {
    tracing::info!(status, "Tray status update");
    let icon = match status {
        2 => icon_on(),
        1 => icon_connecting(),
        _ => icon_off(),
    };

    if let Some(tray) = handle.tray_by_id("prisma-tray") {
        if let Err(e) = tray.set_icon(Some(icon)) {
            tracing::warn!("Failed to set tray icon: {}", e);
        }
    }

    // Update the connect_label in TRAY_STATE and detect if it actually changed
    let label_changed = {
        if let Ok(mut guard) = crate::state::TRAY_STATE.write() {
            let new_label = if status == 2 {
                "Disconnect".to_string()
            } else {
                "Connect".to_string()
            };
            let changed = guard.connect_label != new_label;
            guard.connect_label = new_label;
            changed
        } else {
            false
        }
    };

    // Update tooltip
    if status == 0 {
        if let Some(tray) = handle.tray_by_id("prisma-tray") {
            if let Err(e) = tray.set_tooltip(Some("Prisma \u{2014} Disconnected")) {
                tracing::warn!("Failed to set tray tooltip: {}", e);
            }
        }
    }

    // If the label changed, rebuild menu to show Connect vs Disconnect
    if label_changed {
        rebuild_menu(handle);
    }
}

/// Parameters for updating the tray tooltip and speed stats.
pub struct TrayStatsUpdate<'a> {
    pub up_bps: f64,
    pub down_bps: f64,
    pub bytes_up: f64,
    pub bytes_down: f64,
    pub connections: u32,
    pub profile_name: &'a str,
    pub uptime_secs: u64,
}

/// Update the tray tooltip with rich connection info and cache stats strings
/// in TRAY_STATE. Uses try_write so that if the lock is contended (e.g. the
/// menu is being rebuilt on the UI thread), we skip this tick -- the next stats
/// callback (3s later) will catch up.
pub fn update_tooltip(handle: &AppHandle, stats: &TrayStatsUpdate<'_>) {
    // Format the display strings
    let speed_up = fmt_speed(stats.up_bps);
    let speed_down = fmt_speed(stats.down_bps);
    let total_up = fmt_bytes(stats.bytes_up);
    let total_down = fmt_bytes(stats.bytes_down);
    let connections = format!("{}", stats.connections);

    // try_write: if contended, skip this update -- next tick catches up
    if let Ok(mut guard) = crate::state::TRAY_STATE.try_write() {
        guard.speed_up = speed_up.clone();
        guard.speed_down = speed_down.clone();
        guard.total_up = total_up.clone();
        guard.total_down = total_down.clone();
        guard.connections = connections.clone();
        // guard dropped here
    }

    // Enhanced tooltip (hover text on the tray icon)
    let tooltip = if stats.profile_name.is_empty() {
        format!(
            "Prisma \u{2014} Connected\n\u{2191} {} \u{2193} {}",
            speed_up, speed_down,
        )
    } else {
        format!(
            "Prisma \u{2014} Connected\nProfile: {}\nUptime: {}\n\u{2191} {} \u{2193} {}",
            stats.profile_name,
            fmt_uptime(stats.uptime_secs),
            speed_up,
            speed_down,
        )
    };
    if let Some(tray) = handle.tray_by_id("prisma-tray") {
        if let Err(e) = tray.set_tooltip(Some(&tooltip)) {
            tracing::warn!("Failed to set tray tooltip: {}", e);
        }
    }
}

/// Rebuild the full tray menu (used when profiles, toggles, or recent connections change).
pub fn refresh_profiles(app: &AppHandle) -> tauri::Result<()> {
    // Re-fetch profiles from FFI and cache them
    let ptr = prisma_ffi::prisma_profiles_list_json();
    if !ptr.is_null() {
        let json = unsafe {
            let s = std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string();
            prisma_ffi::prisma_free_string(ptr as *mut _);
            s
        };
        if let Ok(mut guard) = crate::state::TRAY_STATE.write() {
            guard.profiles_json = json;
        }
    }
    let menu = build_full_menu(app)?;
    if let Some(tray) = app.tray_by_id("prisma-tray") {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

/// Rebuild the Recent Connections submenu with the given destinations.
pub fn refresh_recent_connections(app: &AppHandle, destinations: Vec<String>) -> tauri::Result<()> {
    // Update TRAY_STATE with new recent connections
    if let Ok(mut guard) = crate::state::TRAY_STATE.write() {
        guard.recent_connections = destinations;
    }
    // Rebuild full menu to pick up the new recent connections
    let menu = build_full_menu(app)?;
    if let Some(tray) = app.tray_by_id("prisma-tray") {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}
