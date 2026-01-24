use crate::types::AppConfig;
use std::fs;
use std::path::PathBuf;

pub fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // C:\Users\{user}\AppData\Roaming\KeyToMusic\
        dirs::data_dir().unwrap().join("KeyToMusic")
    }

    #[cfg(target_os = "macos")]
    {
        // /Users/{user}/Library/Application Support/KeyToMusic/
        dirs::data_dir().unwrap().join("KeyToMusic")
    }

    #[cfg(target_os = "linux")]
    {
        // /home/{user}/.local/share/keytomusic/
        dirs::data_dir().unwrap().join("keytomusic")
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

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
