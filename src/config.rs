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
    #[serde(default = "default_default_tab")]
    pub default_tab: String,
    #[serde(default)]
    pub resume_on_startup: bool,
    #[serde(default = "default_subtitle_augmentation")]
    pub subtitle_augmentation: bool,
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default = "default_poster_render_mode")]
    pub poster_render_mode: String,
    pub tmdb_api_key: String,
    pub mpv_extra_args: Vec<String>,
}

fn default_audio_mode() -> String {
    "dub".to_string()
}

fn default_preferred_provider() -> String {
    "auto".to_string()
}

fn default_default_tab() -> String {
    "movies".to_string()
}

fn default_subtitle_augmentation() -> bool {
    true
}

fn default_theme_name() -> String {
    "cyber".to_string()
}

fn default_poster_render_mode() -> String {
    "half-block".to_string()
}

impl Config {
    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = get_config_dir();
        let cfg_path = dir.join("config.json");
        let tmp_path = dir.join("config.json.tmp");
        let data = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp_path, data)?;
        std::fs::rename(&tmp_path, &cfg_path)
    }

    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
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
            default_tab: default_default_tab(),
            resume_on_startup: false,
            subtitle_augmentation: default_subtitle_augmentation(),
            theme_name: default_theme_name(),
            poster_render_mode: default_poster_render_mode(),
            tmdb_api_key: "653bb8af90162bd98fc7ee32bcbbfb3d".to_string(),
            mpv_extra_args: vec![
                "--save-position-on-quit".to_string(),
                "--force-window=immediate".to_string(),
                "--autofit=85%x85%".to_string(),
            ],
        }
    }
}

pub fn get_poster_cache_size() -> u64 {
    let dir = get_cache_dir().join("posters");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        0
    }
}

pub fn clear_poster_cache() -> Result<usize, std::io::Error> {
    let dir = get_cache_dir().join("posters");
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() && std::fs::remove_file(entry.path()).is_ok() {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

pub fn xdg_config_home() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".")
}

pub fn xdg_cache_home() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache");
    }
    std::env::temp_dir()
}

pub fn xdg_state_home() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/state");
    }
    std::env::temp_dir()
}

pub fn get_config_dir() -> PathBuf {
    let dir = xdg_config_home().join("kittymov");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn get_cache_dir() -> PathBuf {
    let dir = xdg_cache_home().join("kittymov");
    let posters = dir.join("posters");
    std::fs::create_dir_all(&posters).ok();
    let subtitles = dir.join("subtitles");
    std::fs::create_dir_all(&subtitles).ok();
    dir
}

pub fn get_state_dir() -> PathBuf {
    let dir = xdg_state_home().join("kittymov");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn load_config() -> Config {
    let cfg_path = get_config_dir().join("config.json");
    if cfg_path.exists() {
        match std::fs::read_to_string(&cfg_path) {
            Ok(data) => match serde_json::from_str::<Config>(&data) {
                Ok(mut cfg) => {
                    if let Ok(key) = std::env::var("TMDB_API_KEY") {
                        if !key.trim().is_empty() {
                            cfg.tmdb_api_key = key;
                        }
                    }
                    return cfg;
                }
                Err(err) => {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let backup_path =
                        get_config_dir().join(format!("config.json.corrupted.{}.bak", timestamp));
                    let _ = std::fs::write(&backup_path, &data);
                    eprintln!(
                        "Warning: Malformed configuration file ({:?}).\nBacked up to: {:?}\nError: {}\nRestoring safe defaults.",
                        cfg_path, backup_path, err
                    );
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read config file {:?}: {}", cfg_path, e);
            }
        }
    }

    let default_cfg = Config::default();
    let tmp_path = get_config_dir().join("config.json.tmp");
    if let Ok(data) = serde_json::to_string_pretty(&default_cfg) {
        if std::fs::write(&tmp_path, data).is_ok() {
            let _ = std::fs::rename(&tmp_path, &cfg_path);
        }
    }
    default_cfg
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
        assert_eq!(cfg.default_tab, "movies");
        assert!(!cfg.resume_on_startup);
        assert!(cfg.subtitle_augmentation);
        assert_eq!(cfg.theme_name, "cyber");
        assert_eq!(cfg.poster_render_mode, "half-block");
    }

    #[test]
    fn test_config_reset_to_defaults() {
        let mut cfg = Config {
            default_tab: "history".to_string(),
            resume_on_startup: true,
            theme_name: "amber".to_string(),
            ..Config::default()
        };

        cfg.reset_to_defaults();

        assert_eq!(cfg.default_tab, "movies");
        assert!(!cfg.resume_on_startup);
        assert_eq!(cfg.theme_name, "cyber");
    }
}
