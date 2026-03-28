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
    /// TUN file descriptor returned by the Android VPN service. -1 if not available.
    #[serde(default = "default_fd")]
    pub fd: i32,
}

fn default_fd() -> i32 {
    -1
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
            fd: -1,
        })
    }
    pub fn get_tun_fd(&self) -> Result<ServiceResult, String> {
        Ok(ServiceResult {
            success: true,
            message: None,
            fd: -1,
        })
    }
    pub fn stop_service(&self) -> Result<ServiceResult, String> {
        Ok(ServiceResult {
            success: true,
            message: None,
            fd: -1,
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
