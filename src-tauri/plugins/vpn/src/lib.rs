use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionResult {
    pub granted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResult {
    pub success: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[cfg(mobile)]
mod mobile;

#[cfg(mobile)]
pub use mobile::Vpn;

/// Desktop stub — all methods return sensible defaults.
#[cfg(not(mobile))]
pub struct Vpn;

#[cfg(not(mobile))]
impl Vpn {
    pub fn check_permission(&self) -> Result<PermissionResult, String> {
        Ok(PermissionResult { granted: true })
    }
    pub fn request_permission(&self) -> Result<PermissionResult, String> {
        Ok(PermissionResult { granted: true })
    }
    pub fn start_service(&self, _handle: u64) -> Result<ServiceResult, String> {
        Ok(ServiceResult {
            success: true,
            message: None,
        })
    }
    pub fn stop_service(&self) -> Result<ServiceResult, String> {
        Ok(ServiceResult {
            success: true,
            message: None,
        })
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("vpn")
        .setup(|app, api| {
            #[cfg(mobile)]
            {
                let vpn = mobile::init(app, api)?;
                app.manage(vpn);
            }
            #[cfg(not(mobile))]
            {
                let _ = (app, api);
                app.manage(Vpn);
            }
            Ok(())
        })
        .build()
}
