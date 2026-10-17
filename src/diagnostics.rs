use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::config::{get_cache_dir, get_config_dir, get_state_dir, Config};
use crate::storage::Database;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub kittymov_version: String,
    pub timestamp: String,
    pub os: String,
    pub arch: String,
    pub distro_name: String,
    pub terminal: String,
    pub config_dir: String,
    pub cache_dir: String,
    pub state_dir: String,
    pub database_path: String,
    pub mpv_installed: bool,
    pub mpv_path: Option<String>,
    pub mpv_version: Option<String>,
    pub chafa_installed: bool,
    pub chafa_path: Option<String>,
    pub chafa_version: Option<String>,
    pub tmdb_reachable: bool,
    pub tmdb_latency_ms: Option<u64>,
    pub wyzie_subs_reachable: bool,
    pub wyzie_subs_latency_ms: Option<u64>,
    pub history_entries_count: usize,
    pub favorites_count: usize,
    pub redacted_config: serde_json::Value,
    pub install_command_hint: String,
}

pub fn redact_secret(s: &str) -> String {
    if s.len() <= 6 {
        "******".to_string()
    } else {
        format!("{}...****", &s[..4])
    }
}

pub fn detect_distro() -> (String, String) {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let mut name = String::new();
        let mut id = String::new();
        let mut id_like = String::new();

        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("NAME=") {
                name = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("ID=") {
                id = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("ID_LIKE=") {
                id_like = val.trim_matches('"').to_string();
            }
        }

        let distro_label = if !name.is_empty() { name } else { id.clone() };
        let hint = if id == "arch" || id == "cachyos" || id_like.contains("arch") {
            "sudo pacman -S mpv chafa"
        } else if id == "ubuntu"
            || id == "debian"
            || id == "linuxmint"
            || id_like.contains("debian")
        {
            "sudo apt update && sudo apt install mpv chafa"
        } else if id == "fedora" || id == "rhel" || id_like.contains("fedora") {
            "sudo dnf install mpv chafa"
        } else if id == "opensuse" || id == "suse" || id_like.contains("suse") {
            "sudo zypper install mpv chafa"
        } else if id == "void" {
            "sudo xbps-install -S mpv chafa"
        } else if id == "nixos" {
            "nix-env -iA nixpkgs.mpv nixpkgs.chafa"
        } else {
            "Install 'mpv' and 'chafa' using your Linux package manager"
        };

        (distro_label, hint.to_string())
    } else {
        (
            "Linux (Generic)".to_string(),
            "Install 'mpv' and 'chafa'".to_string(),
        )
    }
}

pub fn find_executable(name: &str) -> Option<(PathBuf, String)> {
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let path = PathBuf::from(path_str);
            if path.is_file() {
                let ver_output = Command::new(&path).arg("--version").output().ok();
                let ver = ver_output
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| s.lines().next().map(|l| l.to_string()))
                    .unwrap_or_else(|| "Unknown version".to_string());
                return Some((path, ver));
            }
        }
    }

    // Direct check in PATH candidates
    for dir in ["/usr/bin", "/usr/local/bin", "/home/xands/.local/bin"] {
        let candidate = Path::new(dir).join(name);
        if candidate.is_file() {
            let ver_output = Command::new(&candidate).arg("--version").output().ok();
            let ver = ver_output
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.lines().next().map(|l| l.to_string()))
                .unwrap_or_else(|| "Unknown version".to_string());
            return Some((candidate, ver));
        }
    }

    None
}

pub async fn check_endpoint(url: &str) -> (bool, Option<u64>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .unwrap_or_default();

    let start = Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (
                resp.status().is_success() || resp.status().is_redirection(),
                Some(elapsed),
            )
        }
        Err(_) => (false, None),
    }
}

pub async fn run_doctor(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m=====================================================\x1b[0m");
    println!("\x1b[1;33m       🐱 KittyMov System & Environment Doctor       \x1b[0m");
    println!("\x1b[1;36m=====================================================\x1b[0m\n");

    let (distro, install_hint) = detect_distro();
    println!(
        " \x1b[1mOS / Distribution:\x1b[0m   {} ({})",
        distro,
        std::env::consts::ARCH
    );
    println!(
        " \x1b[1mTerminal Emulator:\x1b[0m   {}",
        std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string())
    );
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        println!(" \x1b[1mColor Support:\x1b[0m       {}", colorterm);
    }
    println!();

    // 1. Check Player Engine (mpv)
    let player_target = &cfg.player;
    print!(" \x1b[1m[1/5] Player Engine ({}):\x1b[0m ", player_target);
    let mpv_info = find_executable(player_target);
    let mut missing_critical = false;
    if let Some((path, ver)) = mpv_info {
        println!("\x1b[1;32mFOUND\x1b[0m");
        println!("       Path:    {}", path.display());
        println!("       Version: {}", ver);
    } else {
        println!("\x1b[1;31mMISSING\x1b[0m");
        println!(
            "       \x1b[31mError: Player engine '{}' is not installed or not in PATH.\x1b[0m",
            player_target
        );
        println!("       Install with: \x1b[1;33m{}\x1b[0m", install_hint);
        missing_critical = true;
    }
    println!();

    // 2. Check Artwork Engine (chafa)
    print!(" \x1b[1m[2/5] Poster Graphics Engine (chafa):\x1b[0m ");
    let chafa_info = find_executable("chafa");
    if let Some((path, ver)) = chafa_info {
        println!("\x1b[1;32mFOUND\x1b[0m");
        println!("       Path:    {}", path.display());
        println!("       Version: {}", ver);
    } else {
        println!("\x1b[1;33mOPTIONAL (Not Found)\x1b[0m");
        println!("       Note: 24-bit half-block poster previews require chafa.");
        println!("       Install with: \x1b[1;33m{}\x1b[0m", install_hint);
    }
    println!();

    // 3. Check XDG Paths & Write Permissions
    print!(" \x1b[1m[3/5] Filesystem Storage & XDG Paths:\x1b[0m ");
    let config_dir = get_config_dir();
    let cache_dir = get_cache_dir();
    let state_dir = get_state_dir();

    let config_ok = std::fs::create_dir_all(&config_dir).is_ok();
    let cache_ok = std::fs::create_dir_all(&cache_dir).is_ok();
    let state_ok = std::fs::create_dir_all(&state_dir).is_ok();

    if config_ok && cache_ok && state_ok {
        println!("\x1b[1;32mOK\x1b[0m");
        println!("       Config Dir: {}", config_dir.display());
        println!("       Cache Dir:  {}", cache_dir.display());
        println!("       State Dir:  {}", state_dir.display());
    } else {
        println!("\x1b[1;31mPERMISSION ERROR\x1b[0m");
        if !config_ok {
            println!("       Failed to write to {}", config_dir.display());
        }
        if !cache_ok {
            println!("       Failed to write to {}", cache_dir.display());
        }
        if !state_ok {
            println!("       Failed to write to {}", state_dir.display());
        }
    }
    println!();

    // 4. Check Metadata Connectivity (TMDB)
    print!(" \x1b[1m[4/5] TMDB Metadata API Connectivity:\x1b[0m ");
    let tmdb_test_url = format!(
        "https://api.themoviedb.org/3/configuration?api_key={}",
        cfg.tmdb_api_key
    );
    let (tmdb_ok, tmdb_ms) = check_endpoint(&tmdb_test_url).await;
    if tmdb_ok {
        println!(
            "\x1b[1;32mONLINE\x1b[0m ({} ms latency)",
            tmdb_ms.unwrap_or(0)
        );
    } else {
        println!("\x1b[1;31mOFFLINE or INVALID API KEY\x1b[0m");
        println!("       Check network connection or override key via TMDB_API_KEY environment variable.");
    }
    println!();

    // 5. Check Subtitle Engine Connectivity (Wyzie Subs API)
    print!(" \x1b[1m[5/5] Subtitle Engine (Wyzie Subs API):\x1b[0m ");
    let wyzie_test_url = "https://sub.wyzie.io/status/api";
    let (wyzie_ok, wyzie_ms) = check_endpoint(wyzie_test_url).await;
    if wyzie_ok {
        println!(
            "\x1b[1;32mONLINE\x1b[0m ({} ms latency)",
            wyzie_ms.unwrap_or(0)
        );
        if !cfg.wyzie_api_key.trim().is_empty() {
            println!("       API Key: Configured (Wyzie Pro/Free ad-free active)");
        } else {
            println!("       API Key: Unset (Free ad-free keys available at store.wyzie.io/redeem)");
        }
    } else {
        println!("\x1b[1;33mUNREACHABLE\x1b[0m");
        println!("       Wyzie Subs API did not respond; embedded captions will still work.");
    }
    println!();

    println!("\x1b[1;36m-----------------------------------------------------\x1b[0m");
    if missing_critical {
        println!("\x1b[1;31m✘ CRITICAL ISSUES DETECTED:\x1b[0m Missing video player.");
        println!("  Run: \x1b[1;32m{}\x1b[0m", install_hint);
    } else {
        println!(
            "\x1b[1;32m✔ SYSTEM READY FOR STREAMING!\x1b[0m Everything is configured properly."
        );
    }
    println!("\x1b[1;36m=====================================================\x1b[0m");

    Ok(())
}

pub async fn generate_diagnostic_report(cfg: &Config, db: &Database) -> Result<PathBuf, String> {
    let (distro, install_hint) = detect_distro();
    let mpv = find_executable(&cfg.player);
    let chafa = find_executable("chafa");

    let tmdb_test_url = format!(
        "https://api.themoviedb.org/3/configuration?api_key={}",
        cfg.tmdb_api_key
    );
    let (tmdb_ok, tmdb_ms) = check_endpoint(&tmdb_test_url).await;

    let wyzie_test_url = "https://sub.wyzie.io/status/api";
    let (wyzie_ok, wyzie_ms) = check_endpoint(wyzie_test_url).await;

    let mut redacted_cfg_val = serde_json::to_value(cfg).map_err(|e| e.to_string())?;
    if let Some(obj) = redacted_cfg_val.as_object_mut() {
        if let Some(v) = obj.get_mut("tmdb_api_key") {
            if let Some(s) = v.as_str() {
                *v = serde_json::Value::String(redact_secret(s));
            }
        }
        if let Some(v) = obj.get_mut("wyzie_api_key") {
            if let Some(s) = v.as_str() {
                *v = serde_json::Value::String(redact_secret(s));
            }
        }
    }

    let timestamp = chrono::Utc::now().to_rfc3339();
    let history_count = db.get_history_count();
    let favorites_count = db.get_favorites().map(|f| f.len()).unwrap_or(0);

    let report = DiagnosticReport {
        kittymov_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        distro_name: distro,
        terminal: std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string()),
        config_dir: get_config_dir().to_string_lossy().to_string(),
        cache_dir: get_cache_dir().to_string_lossy().to_string(),
        state_dir: get_state_dir().to_string_lossy().to_string(),
        database_path: get_config_dir()
            .join("kittymov.db")
            .to_string_lossy()
            .to_string(),
        mpv_installed: mpv.is_some(),
        mpv_path: mpv.as_ref().map(|(p, _)| p.to_string_lossy().to_string()),
        mpv_version: mpv.map(|(_, v)| v),
        chafa_installed: chafa.is_some(),
        chafa_path: chafa.as_ref().map(|(p, _)| p.to_string_lossy().to_string()),
        chafa_version: chafa.map(|(_, v)| v),
        tmdb_reachable: tmdb_ok,
        tmdb_latency_ms: tmdb_ms,
        wyzie_subs_reachable: wyzie_ok,
        wyzie_subs_latency_ms: wyzie_ms,
        history_entries_count: history_count,
        favorites_count,
        redacted_config: redacted_cfg_val,
        install_command_hint: install_hint,
    };

    let report_path = get_state_dir().join("diagnostic_report.json");
    let json_str = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&report_path, json_str).map_err(|e| e.to_string())?;

    Ok(report_path)
}
