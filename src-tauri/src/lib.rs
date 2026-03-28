mod commands;
mod mobile;
mod state;

#[cfg(desktop)]
mod tray;

use std::ffi::CStr;
use tauri::{Emitter, Manager};

unsafe extern "C" fn on_ffi_event(json: *const std::ffi::c_char, _userdata: *mut std::ffi::c_void) {
    if json.is_null() {
        return;
    }
    // Wrap entire callback in catch_unwind to prevent panics from crashing the Tauri process
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let s = CStr::from_ptr(json).to_string_lossy().to_string();
        if let Some(handle) = state::APP_HANDLE.get() {
            #[cfg(desktop)]
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
                match parsed["type"].as_str() {
                    Some("status_changed") => {
                        let code = parsed["code"].as_i64().unwrap_or(0) as i32;
                        let h = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            tray::update_status(&h, code);
                        });
                    }
                    Some("stats") => {
                        let h = handle.clone();
                        let up_bps = parsed["speed_up_bps"].as_f64().unwrap_or(0.0);
                        let down_bps = parsed["speed_down_bps"].as_f64().unwrap_or(0.0);
                        let bytes_up = parsed["bytes_up"].as_f64().unwrap_or(0.0);
                        let bytes_down = parsed["bytes_down"].as_f64().unwrap_or(0.0);
                        let uptime_secs = parsed["uptime_secs"].as_u64().unwrap_or(0);
                        let _ = handle.run_on_main_thread(move || {
                            tray::update_tooltip(
                                &h,
                                &tray::TrayStatsUpdate {
                                    up_bps,
                                    down_bps,
                                    bytes_up,
                                    bytes_down,
                                    connections: 0,
                                    profile_name: "",
                                    uptime_secs,
                                },
                            );
                        });
                    }
                    _ => {}
                }
            }
            let _ = handle.emit("prisma://event", s);
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = prisma_ffi::prisma_create();
    if !client.is_null() {
        unsafe {
            prisma_ffi::prisma_set_callback(client, Some(on_ffi_event), std::ptr::null_mut());
        }
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ));

    builder
        .manage(state::AppState {
            client: std::sync::Mutex::new(client as usize),
        })
        .setup(|app| {
            state::APP_HANDLE.set(app.handle().clone()).ok();

            // Copy bundled wintun.dll next to the exe so wintun::load() finds it.
            // Always overwrite to ensure the correct version is present.
            #[cfg(windows)]
            {
                let copied = (|| -> Option<()> {
                    let resource_dir = app.path().resource_dir().ok()?;
                    let dll_src = resource_dir.join("resources").join("wintun.dll");
                    if !dll_src.exists() {
                        tracing::warn!("Bundled wintun.dll not found at {}", dll_src.display());
                        return None;
                    }
                    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
                    let dll_dst = exe_dir.join("wintun.dll");
                    if let Err(e) = std::fs::copy(&dll_src, &dll_dst) {
                        tracing::error!("Failed to copy wintun.dll to {}: {e}", dll_dst.display());
                        return None;
                    }
                    tracing::info!("wintun.dll ready at {}", dll_dst.display());
                    Some(())
                })();
                if copied.is_none() {
                    tracing::warn!("wintun.dll setup incomplete — TUN mode may not work");
                }
            }

            #[cfg(desktop)]
            tray::setup(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── Shared commands ──────────────────────────────────────
            commands::connect,
            commands::disconnect,
            commands::get_status,
            commands::get_stats,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::profile_to_qr,
            commands::profile_from_qr,
            commands::decode_qr_image,
            commands::profile_to_uri,
            commands::profile_config_to_toml,
            commands::check_update,
            commands::apply_update,
            commands::ping_server,
            commands::speed_test,
            commands::get_pac_url,
            commands::set_system_proxy,
            commands::clear_system_proxy,
            commands::refresh_tray_profiles,
            commands::set_active_profile_id,
            commands::set_tray_port,
            commands::quit_app,
            commands::set_tray_proxy_mode,
            commands::get_active_profile_id,
            commands::get_proxy_mode,
            commands::update_tray_stats,
            commands::update_tray_recent,
            commands::sync_tray_toggles,
            commands::check_elevation,
            commands::set_per_app_filter,
            commands::clear_per_app_filter,
            commands::get_running_apps,
            commands::get_per_app_filter,
            commands::import_subscription,
            commands::refresh_subscriptions,
            commands::get_profiles_dir,
            commands::open_folder,
            commands::download_file,
            // ── Rule provider commands ────────────────────────────────
            commands::update_rule_provider,
            commands::list_rule_providers,
            // ── Mobile commands (compile on all targets) ──────────────
            mobile::check_vpn_permission,
            mobile::request_vpn_permission,
            mobile::start_vpn_service,
            mobile::stop_vpn_service,
            mobile::get_network_type,
            mobile::on_network_change,
            mobile::get_battery_status,
            mobile::on_app_background,
            mobile::on_app_foreground,
            mobile::on_memory_warning,
        ])
        .build(tauri::generate_context!())
        .expect("tauri build failed")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Clean up the FFI PrismaClient on exit to avoid resource leaks
                if let Some(handle) = state::APP_HANDLE.get() {
                    if let Some(app_state) = handle.try_state::<state::AppState>() {
                        if let Ok(raw) = app_state.client.lock() {
                            let ptr = *raw as *mut prisma_ffi::PrismaClient;
                            if !ptr.is_null() && *raw != 0 {
                                unsafe {
                                    prisma_ffi::prisma_destroy(ptr);
                                }
                            }
                        }
                    }
                }
            }
        });
}
