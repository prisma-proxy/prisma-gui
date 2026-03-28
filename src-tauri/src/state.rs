use std::sync::Mutex;
use std::sync::OnceLock;
#[cfg(desktop)]
use std::sync::RwLock;

pub struct AppState {
    /// PrismaClient* stored as usize for Send-safety across threads.
    pub client: Mutex<usize>,
}

/// Global Tauri app handle — set once in setup, read in FFI callback.
pub static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

/// SOCKS5 port for "Copy Proxy Address" tray menu item.
pub static SOCKS5_PORT: Mutex<u16> = Mutex::new(0);

// ── Consolidated tray state ─────────────────────────────────────────────────

/// All tray-related mutable state in a single struct, protected by one RwLock.
/// This eliminates the lock contention between the FFI stats callback (every 3s)
/// and the UI-thread tray menu click handler that previously competed for 10+
/// individual mutexes.
#[cfg(desktop)]
pub struct TrayState {
    pub connect_label: String,
    pub speed_up: String,
    pub speed_down: String,
    pub total_up: String,
    pub total_down: String,
    pub connections: String,
    pub auto_connect: bool,
    pub allow_lan: bool,
    pub tun_enabled: bool,
    pub proxy_mode: u32,
    pub active_profile_id: Option<String>,
    pub recent_connections: Vec<String>,
    pub profiles_json: String,
}

#[cfg(desktop)]
impl Default for TrayState {
    fn default() -> Self {
        Self {
            connect_label: "Connect".to_string(),
            speed_up: "\u{2191} 0 B/s".to_string(),
            speed_down: "\u{2193} 0 B/s".to_string(),
            total_up: "0 B".to_string(),
            total_down: "0 B".to_string(),
            connections: "0 connections".to_string(),
            auto_connect: false,
            allow_lan: false,
            tun_enabled: false,
            proxy_mode: 0x02,
            active_profile_id: None,
            recent_connections: Vec::new(),
            profiles_json: "[]".to_string(),
        }
    }
}

#[cfg(desktop)]
pub static TRAY_STATE: RwLock<TrayState> = RwLock::new(TrayState {
    connect_label: String::new(),
    speed_up: String::new(),
    speed_down: String::new(),
    total_up: String::new(),
    total_down: String::new(),
    connections: String::new(),
    auto_connect: false,
    allow_lan: false,
    tun_enabled: false,
    proxy_mode: 0x02,
    active_profile_id: None,
    recent_connections: Vec::new(),
    profiles_json: String::new(),
});

/// Initialize TRAY_STATE with proper default strings.
/// Must be called once at startup before the tray is built.
#[cfg(desktop)]
pub fn init_tray_state() {
    if let Ok(mut s) = TRAY_STATE.write() {
        if s.connect_label.is_empty() {
            *s = TrayState::default();
        }
    }
}
