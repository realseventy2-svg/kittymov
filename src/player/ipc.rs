use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use serde_json::{json, Value};
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
    ) -> Result<(), String> {
        // Clean up socket file if left over
        if self.ipc_path.exists() {
            let _ = std::fs::remove_file(&self.ipc_path);
        }

        let mut cmd = Command::new(&cfg.player);
        cmd.arg(format!("--input-ipc-server={}", self.ipc_path.display()));
        let title_with_provider = format!("KittyMov: {} [{}] [{}]", title, stream.provider_name, cfg.audio_mode.to_uppercase());
        cmd.arg(format!("--title={}", title_with_provider));

        let is_sub = cfg.audio_mode.to_lowercase() == "sub";
        if is_sub {
            cmd.arg(format!("--alang=jpn,ja,Japanese,original,{},en,eng", cfg.audio_language));
            cmd.arg(format!("--slang={},en,enUS,English", cfg.sub_language));
            cmd.arg("--sub-visibility=yes");
            cmd.arg("--sub-auto=fuzzy");
        } else {
            cmd.arg(format!("--alang={},en,enUS,English,jpn", cfg.audio_language));
            cmd.arg(format!("--slang={},en,enUS,English", cfg.sub_language));
            cmd.arg("--sub-visibility=auto");
            cmd.arg("--sub-auto=fuzzy");
        }
        cmd.arg(format!("--hwdec={}", cfg.hwdec));

        // Explicit referrer and user-agent for CDN and HLS segments
        if let Some(ref embed) = stream.embed_url {
            cmd.arg(format!("--referrer={}", embed));
        } else if let Some(ref r) = stream.headers.get("Referer") {
            cmd.arg(format!("--referrer={}", r));
        }

        if let Some(ref ua) = stream.headers.get("User-Agent") {
            cmd.arg(format!("--user-agent={}", ua));
        }

        if !stream.headers.is_empty() {
            let headers_str = stream.headers.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(",");
            cmd.arg(format!("--http-header-fields={}", headers_str));
        }

        for sub in &stream.subtitles {
            cmd.arg(format!("--sub-file={}", sub));
        }

        for arg in &cfg.mpv_extra_args {
            cmd.arg(arg);
        }

        cmd.arg(&stream.stream_url);

        // Inherit stdin/stdout/stderr so keyboard controls in MPV window function natively
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {}", cfg.player, e))?;
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

    /// Spawns a background task monitoring time-pos and playback events via JSON-IPC
    pub fn monitor_ipc(
        &self,
        stream: UnixStream,
        status_tx: mpsc::Sender<PlaybackStatus>,
    ) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Task 1: Periodic polling for time-pos and duration
        let poll_tx = status_tx.clone();
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // Socket closed
                    Ok(_) => {
                        if let Ok(val) = serde_json::from_str::<Value>(&line) {
                            // Check for events
                            if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                                if event == "end-file" {
                                    let reason = val.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                                    if reason == "eof" {
                                        let _ = poll_tx.send(PlaybackStatus::EofReached).await;
                                    } else if reason == "quit" {
                                        let _ = poll_tx.send(PlaybackStatus::UserQuit).await;
                                        break;
                                    }
                                }
                            }
                            // Check response data
                            if let Some(data) = val.get("data") {
                                if let Some(pos) = data.as_f64() {
                                    let _ = poll_tx.send(PlaybackStatus::Playing { pos, duration: 0.0 }).await;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Task 2: Polling loop sending get_property
        tokio::spawn(async move {
            let poll_cmd = json!({ "command": ["get_property", "time-pos"] }).to_string() + "\n";
            loop {
                if writer.write_all(poll_cmd.as_bytes()).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    /// Send a command to MPV over Unix socket (e.g. loadfile next_stream)
    pub async fn send_command(&self, cmd: &[&str]) -> Result<(), String> {
        let mut stream = UnixStream::connect(&self.ipc_path).await
            .map_err(|e| format!("Could not connect to MPV socket: {}", e))?;

        let json_cmd = json!({ "command": cmd }).to_string() + "\n";
        stream.write_all(json_cmd.as_bytes()).await
            .map_err(|e| format!("Failed to send IPC command: {}", e))?;

        Ok(())
    }

    pub fn wait_sync(&mut self) -> Result<i32, String> {
        if let Some(mut child) = self.child.take() {
            let status = child.wait().map_err(|e| format!("MPV wait error: {}", e))?;
            if self.ipc_path.exists() {
                let _ = std::fs::remove_file(&self.ipc_path);
            }
            Ok(status.code().unwrap_or(0))
        } else {
            Ok(0)
        }
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
