use crate::types::AppConfig;
use std::fs;
use std::path::PathBuf;

/// Where profiles, caches and logs live. `KEYTOMUSIC_DATA_DIR` overrides it
/// (tests, separate setups); a `portable` file next to the executable keeps
/// everything in a `data` folder beside it (KeyToMusic on a USB stick).
pub fn get_app_data_dir() -> PathBuf {
    custom_data_dir()
        .cloned()
        .unwrap_or_else(system_app_data_dir)
}

/// The data folder when it is not the system default (see [`get_app_data_dir`]):
/// hosts keep their own settings there too.
pub fn custom_data_dir() -> Option<&'static PathBuf> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("KEYTOMUSIC_DATA_DIR").filter(|dir| !dir.is_empty()) {
            return Some(PathBuf::from(dir));
        }
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
            .filter(|dir| dir.join("portable").is_file())
            .map(|dir| dir.join("data"))
    })
    .as_ref()
}

fn system_app_data_dir() -> PathBuf {
    let base = dirs::data_dir()
        .expect("Could not determine system data directory — the app cannot function without it");

    #[cfg(target_os = "windows")]
    {
        base.join("KeyToMusic")
    }

    #[cfg(target_os = "macos")]
    {
        base.join("KeyToMusic")
    }

    #[cfg(target_os = "linux")]
    {
        base.join("keytomusic")
    }
}

pub fn init_app_directories() -> Result<(), String> {
    let app_dir = get_app_data_dir();

    fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app dir: {}", e))?;
    fs::create_dir_all(app_dir.join("profiles"))
        .map_err(|e| format!("Failed to create profiles dir: {}", e))?;
    fs::create_dir_all(app_dir.join("cache"))
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    fs::create_dir_all(app_dir.join("logs"))
        .map_err(|e| format!("Failed to create logs dir: {}", e))?;
    fs::create_dir_all(app_dir.join("discovery"))
        .map_err(|e| format!("Failed to create discovery dir: {}", e))?;
    fs::create_dir_all(app_dir.join("models"))
        .map_err(|e| format!("Failed to create models dir: {}", e))?;

    Ok(())
}

pub fn load_config() -> Result<AppConfig, String> {
    let config_path = get_app_data_dir().join("config.json");

    if !config_path.exists() {
        let default_config = AppConfig::default();
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let contents =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_path = get_app_data_dir().join("config.json");
    let json = serde_json::to_vec_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    super::atomic_write(&config_path, &json)
}
