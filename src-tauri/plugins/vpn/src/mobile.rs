use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::{PermissionResult, ServiceResult};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.prisma.vpn";

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_vpn);

pub struct Vpn<R: Runtime>(PluginHandle<R>);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<Vpn<R>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "VpnPlugin")?;

    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_vpn)?;

    Ok(Vpn(handle))
}

impl<R: Runtime> Vpn<R> {
    /// Check if VPN permission is granted.
    /// On Android: calls VpnService.prepare() — null means granted.
    pub fn check_permission(&self) -> Result<PermissionResult, String> {
        self.0
            .run_mobile_plugin::<PermissionResult>("checkPermission", ())
            .map_err(|e| e.to_string())
    }

    /// Request VPN permission from the OS.
    /// On Android: launches the VPN consent dialog via startActivityForResult.
    pub fn request_permission(&self) -> Result<PermissionResult, String> {
        self.0
            .run_mobile_plugin::<PermissionResult>("requestPermission", ())
            .map_err(|e| e.to_string())
    }

    /// Start the native VPN service.
    /// On Android: starts PrismaVpnService with the client handle.
    pub fn start_service(&self, handle: u64) -> Result<ServiceResult, String> {
        self.0
            .run_mobile_plugin::<ServiceResult>(
                "startService",
                serde_json::json!({ "handle": handle }),
            )
            .map_err(|e| e.to_string())
    }

    /// Stop the native VPN service.
    pub fn stop_service(&self) -> Result<ServiceResult, String> {
        self.0
            .run_mobile_plugin::<ServiceResult>("stopService", ())
            .map_err(|e| e.to_string())
    }
}
