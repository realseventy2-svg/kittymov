use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::providers::StreamCandidate;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PlaybackStatus {
    Playing { pos: f64, duration: f64 },
    Paused,
    EofReached,
    UserQuit,
    Error(String),
}

pub struct MpvController {
    ipc_path: PathBuf,
    child: Option<Child>,
}

impl MpvController {
    pub fn new() -> Self {
        let pid = std::process::id();
        let ipc_path = std::env::temp_dir().join(format!("kittymov-mpv-{}.sock", pid));
        Self {
            ipc_path,
            child: None,
        }
    }

    pub fn start_player(
        &mut self,
        stream: &StreamCandidate,
        title: &str,
        cfg: &Config,
        idle: bool,
        start_sec: Option<i64>,
    ) -> Result<(), String> {
        // Clean up socket file if left over
        if self.ipc_path.exists() {
            let _ = std::fs::remove_file(&self.ipc_path);
        }

        // Validate stream URL scheme (must be http or https)
        if !is_safe_stream_url(&stream.stream_url) {
            return Err(format!(
                "Unsafe or invalid stream URL scheme: {}",
                stream.stream_url
            ));
        }

        let mut cmd = Command::new(&cfg.player);
        cmd.arg(format!("--input-ipc-server={}", self.ipc_path.display()));
        if idle {
            cmd.arg("--idle=yes");
            cmd.arg("--force-window=yes");
        }
        if let Some(sec) = start_sec {
            if sec > 0 {
                cmd.arg(format!("--start={}", sec));
            }
        }
        let quality_tag = stream.quality.as_deref().unwrap_or("Auto");
        let title_with_provider = format!(
            "KittyMov: {} [{}] [{}] [{}]",
            title,
            stream.provider_name,
            quality_tag,
            cfg.audio_mode.to_uppercase()
        );
        cmd.arg(format!("--title={}", title_with_provider));

        let q_pref = stream
            .quality
            .as_deref()
            .or(Some(cfg.preferred_quality.as_str()))
            .unwrap_or("auto")
            .to_lowercase();

        if q_pref.contains("480") {
            cmd.arg("--ytdl-format=bestvideo[height<=?480]+bestaudio/best[height<=?480]/best");
            cmd.arg("--hls-bitrate=1500000");
        } else if q_pref.contains("720") {
            cmd.arg("--ytdl-format=bestvideo[height<=?720]+bestaudio/best[height<=?720]/best");
            cmd.arg("--hls-bitrate=3500000");
        } else if q_pref.contains("1080") {
            cmd.arg("--ytdl-format=bestvideo[height<=?1080]+bestaudio/best[height<=?1080]/best");
            cmd.arg("--hls-bitrate=max");
        } else if q_pref.contains("360") {
            cmd.arg("--ytdl-format=bestvideo[height<=?360]+bestaudio/best[height<=?360]/best");
            cmd.arg("--hls-bitrate=800000");
        } else {
            cmd.arg("--hls-bitrate=max");
        }

        let is_sub = cfg.audio_mode.to_lowercase() == "sub";
        if is_sub {
            cmd.arg(format!(
                "--alang=jpn,ja,Japanese,original,{},en,eng",
                cfg.audio_language
            ));
            cmd.arg(format!(
                "--slang={},en,enUS,English,ita,ger,fre,spa",
                cfg.sub_language
            ));
            cmd.arg("--sub-visibility=yes");
            cmd.arg("--subs-with-matching-audio=yes");
            cmd.arg("--sid=auto");
            cmd.arg("--sub-auto=fuzzy");
            cmd.arg("--sub-fix-timing=yes");
            cmd.arg("--demuxer-mkv-subtitle-preroll=yes");
        } else {
            cmd.arg(format!(
                "--alang={},en,enUS,English,jpn",
                cfg.audio_language
            ));
            cmd.arg(format!(
                "--slang={},en,enUS,English,ita,ger,fre,spa",
                cfg.sub_language
            ));
            cmd.arg("--sub-visibility=yes");
            cmd.arg("--subs-with-matching-audio=yes");
            cmd.arg("--sid=auto");
            cmd.arg("--sub-auto=fuzzy");
            cmd.arg("--sub-fix-timing=yes");
            cmd.arg("--demuxer-mkv-subtitle-preroll=yes");
        }
        if cfg.sub_delay.abs() > 0.001 {
            cmd.arg(format!("--sub-delay={:.1}", cfg.sub_delay));
        }
        cmd.arg(format!("--hwdec={}", cfg.hwdec));

        // Explicit referrer and user-agent for CDN and HLS segments
        if let Some(ref embed) = stream.embed_url {
            if is_safe_stream_url(embed) {
                cmd.arg(format!("--referrer={}", embed));
            }
        } else if let Some(r) = stream.headers.get("Referer") {
            if is_safe_stream_url(r) {
                cmd.arg(format!("--referrer={}", r));
            }
        }

        if let Some(ua) = stream.headers.get("User-Agent") {
            let clean_ua: String = ua.chars().filter(|c| *c != '\r' && *c != '\n').collect();
            cmd.arg(format!("--user-agent={}", clean_ua));
        } else {
            cmd.arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36");
        }

        // Disable unauthenticated ytdl scraper fallback on direct CDN streams to prevent secondary 429 errors
        cmd.arg("--ytdl=no");

        // Enable demuxer auto-reconnect on streamed network error (matching Kunai Movie TUI resilience)
        cmd.arg("--demuxer-lavf-o=reconnect=1,reconnect_streamed=1,reconnect_on_network_error=1,reconnect_delay_max=3,reconnect_max_retries=5");

        let custom_headers: Vec<String> = stream
            .headers
            .iter()
            .filter(|(k, v)| {
                !k.eq_ignore_ascii_case("user-agent")
                    && !k.eq_ignore_ascii_case("referer")
                    && !v.trim().is_empty()
            })
            .map(|(k, v)| {
                let clean_k: String = k
                    .chars()
                    .filter(|c| *c != '\r' && *c != '\n' && *c != ':')
                    .collect();
                let clean_v: String = v.chars().filter(|c| *c != '\r' && *c != '\n').collect();
                format!("{}: {}", clean_k, clean_v)
            })
            .collect();

        if !custom_headers.is_empty() {
            cmd.arg(format!("--http-header-fields={}", custom_headers.join(",")));
        }

        for sub in &stream.subtitles {
            // Validate subtitle: must be safe http/https URL or an existing local file
            if is_safe_stream_url(sub)
                || (std::path::Path::new(sub).is_file()
                    && !sub.contains('\n')
                    && !sub.contains('\r'))
            {
                cmd.arg(format!("--sub-file={}", sub));
            }
        }

        for arg in &cfg.mpv_extra_args {
            cmd.arg(arg);
        }

        cmd.arg(&stream.stream_url);

        // Inherit stdin/stdout/stderr so keyboard controls in MPV window function natively
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn player '{}': {}. Ensure mpv is installed.",
                cfg.player, e
            )
        })?;
        self.child = Some(child);

        Ok(())
    }

    /// Wait for MPV socket to be available
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<UnixStream, String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if self.ipc_path.exists() {
                if let Ok(stream) = UnixStream::connect(&self.ipc_path).await {
                    return Ok(stream);
                }
            }
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        Err("Timed out waiting for MPV IPC socket".to_string())
    }

    /// Spawns a background task monitoring time-pos, duration, and playback events via JSON-IPC
    pub fn monitor_ipc(&self, stream: UnixStream, status_tx: mpsc::Sender<PlaybackStatus>) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Task 1: Periodic polling for time-pos and duration
        let poll_tx = status_tx.clone();
        tokio::spawn(async move {
            let mut line = String::new();
            let mut current_dur = 0.0;

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        let _ = poll_tx.send(PlaybackStatus::UserQuit).await;
                        break; // Socket closed
                    }
                    Ok(_) => {
                        if let Ok(val) = serde_json::from_str::<Value>(&line) {
                            // Check for events
                            if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                                match event {
                                    "start-file" => {
                                        current_dur = 0.0;
                                    }
                                    "end-file" => {
                                        let reason =
                                            val.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                                        if reason == "eof" {
                                            let _ = poll_tx.send(PlaybackStatus::EofReached).await;
                                        } else if reason == "quit" {
                                            let _ = poll_tx.send(PlaybackStatus::UserQuit).await;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // Check response data from property polls
                            if let Some(req_id) = val.get("request_id").and_then(|r| r.as_u64()) {
                                match req_id {
                                    1 => {
                                        if let Some(pos) = val.get("data").and_then(|d| d.as_f64()) {
                                            let _ = poll_tx
                                                .send(PlaybackStatus::Playing {
                                                    pos,
                                                    duration: current_dur,
                                                })
                                                .await;
                                        }
                                    }
                                    2 => {
                                        if let Some(dur) = val.get("data").and_then(|d| d.as_f64()) {
                                            current_dur = dur;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Err(_) => {
                        let _ = poll_tx.send(PlaybackStatus::UserQuit).await;
                        break;
                    }
                }
            }
        });

        // Task 2: Polling loop sending get_property time-pos and duration
        tokio::spawn(async move {
            let pos_cmd = json!({ "command": ["get_property", "time-pos"], "request_id": 1 }).to_string() + "\n";
            let dur_cmd = json!({ "command": ["get_property", "duration"], "request_id": 2 }).to_string() + "\n";
            loop {
                if writer.write_all(pos_cmd.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(dur_cmd.as_bytes()).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    /// Send a command to MPV over Unix socket (e.g. loadfile next_stream)
    pub async fn send_command(&self, cmd: &[&str]) -> Result<(), String> {
        let mut stream = UnixStream::connect(&self.ipc_path)
            .await
            .map_err(|e| format!("Could not connect to MPV socket: {}", e))?;

        let json_cmd = json!({ "command": cmd }).to_string() + "\n";
        stream
            .write_all(json_cmd.as_bytes())
            .await
            .map_err(|e| format!("Failed to send IPC command: {}", e))?;

        Ok(())
    }

    pub async fn wait_async(&mut self) -> Result<i32, String> {
        if let Some(mut child) = self.child.take() {
            let res = tokio::task::spawn_blocking(move || {
                let status = child.wait().map_err(|e| format!("MPV wait error: {}", e))?;
                Ok::<i32, String>(status.code().unwrap_or(0))
            })
            .await
            .map_err(|e| format!("Join error: {}", e))??;

            if self.ipc_path.exists() {
                let _ = std::fs::remove_file(&self.ipc_path);
            }
            Ok(res)
        } else {
            Ok(0)
        }
    }
}

pub fn is_safe_stream_url(url_str: &str) -> bool {
    if let Ok(parsed) = url::Url::parse(url_str) {
        let scheme = parsed.scheme();
        (scheme == "http" || scheme == "https")
            && !url_str.contains('\r')
            && !url_str.contains('\n')
    } else {
        false
    }
}

impl Drop for MpvController {
    fn drop(&mut self) {
        if self.ipc_path.exists() {
            let _ = std::fs::remove_file(&self.ipc_path);
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
