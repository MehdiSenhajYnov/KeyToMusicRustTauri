use crate::types::AppConfig;
use std::fs;
use std::path::PathBuf;

pub fn get_app_data_dir() -> PathBuf {
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
    let tmp_path = config_path.with_extension("json.tmp");

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&tmp_path, json).map_err(|e| format!("Failed to write config temp file: {}", e))?;
    fs::rename(&tmp_path, &config_path)
        .map_err(|e| format!("Failed to rename config file: {}", e))?;

    Ok(())
}
