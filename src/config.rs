use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub player: String,
    pub auto_play_next: bool,
    pub audio_language: String,
    pub sub_language: String,
    pub hwdec: String,
    pub tmdb_api_key: String,
    pub mpv_extra_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            player: "mpv".to_string(),
            auto_play_next: true,
            audio_language: "eng".to_string(),
            sub_language: "eng".to_string(),
            hwdec: "auto".to_string(),
            tmdb_api_key: "653bb8af90162bd98fc7ee32bcbbfb3d".to_string(),
            mpv_extra_args: vec![
                "--save-position-on-quit".to_string(),
                "--force-window=immediate".to_string(),
                "--autofit=85%x85%".to_string(),
            ],
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    dir.push("kittymov");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn get_cache_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("~/.cache"));
    dir.push("kittymov");
    let posters = dir.join("posters");
    std::fs::create_dir_all(&posters).ok();
    dir
}

pub fn load_config() -> Config {
    let cfg_path = get_config_dir().join("config.json");
    if cfg_path.exists() {
        if let Ok(data) = std::fs::read_to_string(&cfg_path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
    }
    let default_cfg = Config::default();
    if let Ok(data) = serde_json::to_string_pretty(&default_cfg) {
        std::fs::write(&cfg_path, data).ok();
    }
    default_cfg
}

// Minimal dirs helper fallback if dirs crate is not present
mod dirs {
    use std::path::PathBuf;
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }
    pub fn cache_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache"))
    }
}
