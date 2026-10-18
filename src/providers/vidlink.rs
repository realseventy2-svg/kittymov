use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use crypto_secretbox::{
    aead::{Aead, KeyInit},
    Key, Nonce, XSalsa20Poly1305,
};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{MediaRequest, Provider, StreamCandidate};

pub const VIDLINK_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";
pub const VIDLINK_ORIGIN: &str = "https://vidlink.pro";
pub const VIDLINK_REFERER: &str = "https://vidlink.pro/";
const VIDLINK_KEY_HEX: &str = "c75136c5668bbfe65a7ecad431a745db68b5f381555b38d8f6c699449cf11fcd";

pub struct VidLinkProvider {
    client: Client,
    key_bytes: [u8; 32],
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VidLinkResponse {
    #[serde(rename = "sourceId")]
    pub source_id: Option<String>,
    pub stream: Option<VidLinkStream>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VidLinkStream {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub stream_type: Option<String>,
    pub playlist: Option<String>,
    pub qualities: Option<HashMap<String, VidLinkQuality>>,
    pub captions: Option<Vec<VidLinkCaption>>,
    #[serde(rename = "TTL")]
    pub ttl: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct VidLinkQuality {
    #[serde(rename = "type")]
    pub quality_type: Option<String>,
    pub url: String,
    #[serde(rename = "codecName")]
    pub codec_name: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct VidLinkCaption {
    pub id: Option<String>,
    pub url: String,
    pub language: Option<String>,
    #[serde(rename = "type")]
    pub caption_type: Option<String>,
}

impl VidLinkProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new());

        let mut key_bytes = [0u8; 32];
        for (i, byte) in (0..VIDLINK_KEY_HEX.len())
            .step_by(2)
            .enumerate()
            .take(32)
        {
            if let Ok(b) = u8::from_str_radix(&VIDLINK_KEY_HEX[byte..byte + 2], 16) {
                key_bytes[i] = b;
            }
        }

        Self { client, key_bytes }
    }

    /// Encrypts media TMDB ID using XSalsa20-Poly1305 into URL-safe base64 token.
    pub fn encrypt_token(&self, media_id: &str) -> Result<String, String> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let timestamp = now_secs + 480;

        let cipher = XSalsa20Poly1305::new(Key::from_slice(&self.key_bytes));
        let nonce = Nonce::from_slice(&[0u8; 24]);

        let mut message = media_id.as_bytes().to_vec();
        message.extend_from_slice(&timestamp.to_be_bytes());

        let ct = cipher
            .encrypt(nonce, message.as_ref())
            .map_err(|e| format!("VidLink encryption failed: {:?}", e))?;

        let mut full_payload = vec![0u8; 24];
        full_payload.extend_from_slice(&ct);

        Ok(URL_SAFE_NO_PAD.encode(&full_payload))
    }

    /// Resolves stream with optional quality preference ("1080p", "720p", "480p", "360p", or "auto")
    pub async fn resolve_with_quality(
        &self,
        req: &MediaRequest,
        quality_pref: Option<&str>,
    ) -> Result<StreamCandidate, String> {
        let is_movie = req.media_type.to_lowercase() == "movie";
        let tmdb_id_str = req.tmdb_id.to_string();
        let token = self.encrypt_token(&tmdb_id_str)?;

        let api_url = if is_movie {
            format!("https://vidlink.pro/api/b/movie/{}?multiLang=1", token)
        } else {
            let s = req.season.unwrap_or(1);
            let ep = req.episode.unwrap_or(1);
            format!("https://vidlink.pro/api/b/tv/{}/{}/{}?multiLang=1", token, s, ep)
        };


        let resp = self
            .client
            .get(&api_url)
            .header("User-Agent", VIDLINK_USER_AGENT)
            .header("Origin", VIDLINK_ORIGIN)
            .header("Referer", VIDLINK_REFERER)
            .send()
            .await
            .map_err(|e| format!("VidLink HTTP request failed: {}", e))?;

        if resp.status().as_u16() == 404 {
            return Err(format!("VidLink: Title '{}' not found (HTTP 404)", req.title));
        }

        if !resp.status().is_success() {
            return Err(format!("VidLink returned HTTP {}", resp.status()));
        }

        let data: VidLinkResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse VidLink JSON: {}", e))?;

        let stream_info = data
            .stream
            .ok_or_else(|| "VidLink response missing stream payload".to_string())?;

        // Extract stream URL based on quality preference
        let (selected_url, actual_quality) = self.select_quality(&stream_info, quality_pref)?;

        // Extract subtitles, prioritizing English and popular languages
        let mut subtitles = Vec::new();
        if let Some(captions) = stream_info.captions {
            // First pass: English subtitles
            for cap in &captions {
                if let Some(ref lang) = cap.language {
                    if lang.eq_ignore_ascii_case("english") && !subtitles.contains(&cap.url) {
                        subtitles.push(cap.url.clone());
                    }
                }
            }
            // Second pass: Other available subtitle tracks up to 15 total
            for cap in captions {
                if subtitles.len() >= 15 {
                    break;
                }
                if !subtitles.contains(&cap.url) {
                    subtitles.push(cap.url);
                }
            }
        }

        // The underlying CDN (bcdn.hakunaymatata.com) blocks requests bearing the vidlink.pro
        // Referer with HTTP 429. Supplying a standard browser User-Agent without referer
        // ensures ffmpeg connects cleanly without triggering anti-bot or anti-hotlink blocks.
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), VIDLINK_USER_AGENT.to_string());

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let expires_at = now_secs + stream_info.ttl.unwrap_or(3600);

        Ok(StreamCandidate {
            stream_url: selected_url,
            provider_name: "VidLink (HD 1080p Direct)".to_string(),
            headers,
            subtitles,
            expires_at,
            embed_url: None,
            request: req.clone(),
            actual_title: None,
            quality: Some(actual_quality),
        })
    }

    fn select_quality(
        &self,
        stream_info: &VidLinkStream,
        pref: Option<&str>,
    ) -> Result<(String, String), String> {
        // If playlist exists and qualities is empty, use playlist directly
        if let Some(ref pl) = stream_info.playlist {
            if stream_info.qualities.is_none() || stream_info.qualities.as_ref().map(|q| q.is_empty()).unwrap_or(false) {
                return Ok((pl.clone(), "Auto HLS".to_string()));
            }
        }

        let qualities = stream_info
            .qualities
            .as_ref()
            .ok_or_else(|| "VidLink: No streaming qualities or playlist found".to_string())?;

        if qualities.is_empty() {
            if let Some(ref pl) = stream_info.playlist {
                return Ok((pl.clone(), "Auto HLS".to_string()));
            }
            return Err("VidLink: Empty qualities list in stream".to_string());
        }

        let pref_clean = pref
            .unwrap_or("auto")
            .to_lowercase()
            .replace('p', "")
            .trim()
            .to_string();

        let try_order: &[&str] = match pref_clean.as_str() {
            "1080" => &["1080", "720", "480", "360"],
            "720" => &["720", "1080", "480", "360"],
            "480" => &["480", "720", "360", "1080"],
            "360" => &["360", "480", "720", "1080"],
            _ => &["1080", "720", "480", "360"], // Default to best HD
        };

        for &q in try_order {
            if let Some(q_info) = qualities.get(q) {
                if !q_info.url.is_empty() {
                    let label = format!("{}p FHD", q);
                    return Ok((q_info.url.clone(), label));
                }
            }
        }

        // Fallback to any available quality
        for (q_name, q_info) in qualities {
            if !q_info.url.is_empty() {
                let label = format!("{}p", q_name);
                return Ok((q_info.url.clone(), label));
            }
        }

        if let Some(ref pl) = stream_info.playlist {
            return Ok((pl.clone(), "Auto HLS".to_string()));
        }

        Err("VidLink: Could not resolve valid stream URL from qualities".to_string())
    }
}

#[async_trait]
impl Provider for VidLinkProvider {
    fn name(&self) -> &'static str {
        "VidLink (HD 1080p Direct)"
    }

    async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
        self.resolve_with_quality(req, None).await
    }

    async fn refresh_token(&self, candidate: &mut StreamCandidate) -> Result<(), String> {
        let refreshed = self
            .resolve_with_quality(&candidate.request, candidate.quality.as_deref())
            .await?;
        candidate.stream_url = refreshed.stream_url;
        candidate.headers = refreshed.headers;
        candidate.subtitles = refreshed.subtitles;
        candidate.expires_at = refreshed.expires_at;
        candidate.quality = refreshed.quality;
        Ok(())
    }
}
