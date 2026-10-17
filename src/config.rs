use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub player: String,
    pub auto_play_next: bool,
    pub audio_language: String,
    pub sub_language: String,
    #[serde(default = "default_audio_mode")]
    pub audio_mode: String,
    pub hwdec: String,
    #[serde(default = "default_preferred_provider")]
    pub preferred_provider: String,
    pub tmdb_api_key: String,
    pub mpv_extra_args: Vec<String>,
}

fn default_audio_mode() -> String {
    "dub".to_string()
}

fn default_preferred_provider() -> String {
    "auto".to_string()
}

impl Config {
    pub fn save(&self) -> Result<(), std::io::Error> {
        let cfg_path = get_config_dir().join("config.json");
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&cfg_path, data)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            player: "mpv".to_string(),
            auto_play_next: true,
            audio_language: "eng".to_string(),
            sub_language: "eng".to_string(),
            audio_mode: default_audio_mode(),
            hwdec: "auto".to_string(),
            preferred_provider: default_preferred_provider(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults_and_backward_compatibility() {
        let old_json = r#"{
            "player": "mpv",
            "auto_play_next": false,
            "audio_language": "jpn",
            "sub_language": "spa",
            "hwdec": "vaapi",
            "tmdb_api_key": "custom_key",
            "mpv_extra_args": []
        }"#;

        let cfg: Config = serde_json::from_str(old_json).expect("Should deserialize old config");
        assert_eq!(cfg.audio_mode, "dub");
        assert_eq!(cfg.preferred_provider, "auto");
        assert_eq!(cfg.audio_language, "jpn");
        assert_eq!(cfg.sub_language, "spa");
        assert!(!cfg.auto_play_next);
    }
}

