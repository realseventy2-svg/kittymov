use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use super::{MediaRequest, Provider, StreamCandidate};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct VixSrcProvider {
    client: Client,
    playlist_re: Regex,
    token_re: Regex,
    expires_re: Regex,
}

#[derive(Deserialize)]
struct VixSrcApiResponse {
    src: Option<String>,
}

impl VixSrcProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new());

        let playlist_re =
            Regex::new(r"url:\s*'(https://vixsrc\.to/playlist/[0-9]+(?:\?[^']*)?)'").unwrap();
        let token_re = Regex::new(r"'token':\s*'([^']+)'").unwrap();
        let expires_re = Regex::new(r"'expires':\s*'([^']+)'").unwrap();

        Self {
            client,
            playlist_re,
            token_re,
            expires_re,
        }
    }

    fn extract_from_html(&self, html: &str) -> Result<(String, String, u64), String> {
        let p_match = self
            .playlist_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Could not find playlist URL in embed HTML".to_string())?;

        let token_match = self
            .token_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Could not find token in embed HTML".to_string())?;

        let expires_str = self
            .expires_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Could not find expires in embed HTML".to_string())?;

        let expires_at = expires_str.parse::<u64>().unwrap_or(0);

        Ok((p_match.to_string(), token_match.to_string(), expires_at))
    }
}

#[async_trait]
impl Provider for VixSrcProvider {
    fn name(&self) -> &'static str {
        "VixSrc (Fast CDN)"
    }

    async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
        let base_url = "https://vixsrc.to";
        let api_url = if req.media_type == "movie" {
            format!("{}/api/movie/{}", base_url, req.tmdb_id)
        } else {
            let s = req.season.unwrap_or(1);
            let e = req.episode.unwrap_or(1);
            format!("{}/api/tv/{}/{}/{}", base_url, req.tmdb_id, s, e)
        };

        let resp = self
            .client
            .get(&api_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .send()
            .await
            .map_err(|e| format!("VixSrc network error: {}", e))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("Title not found on VixSrc (HTTP 404)".to_string());
        }

        if !resp.status().is_success() {
            return Err(format!("VixSrc returned HTTP {}", resp.status()));
        }

        let resp_json: VixSrcApiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from VixSrc API: {}", e))?;

        let embed_path = resp_json
            .src
            .ok_or_else(|| "No embed src returned by VixSrc API".to_string())?;
        let embed_url = format!("{}{}", base_url, embed_path);

        let embed_resp = self
            .client
            .get(&embed_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", base_url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach VixSrc embed host: {}", e))?;

        if !embed_resp.status().is_success() {
            return Err(format!("VixSrc embed host returned HTTP {}", embed_resp.status()));
        }

        let html = embed_resp
            .text()
            .await
            .map_err(|e| format!("Failed to read embed HTML: {}", e))?;

        let (base_playlist, token, expires_at) = self.extract_from_html(&html)?;

        let sep = if base_playlist.contains('?') {
            "&"
        } else {
            "?"
        };
        let master_url = format!(
            "{}{}token={}&expires={}&asn=&h=1",
            base_playlist, sep, token, expires_at
        );

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
        headers.insert("Referer".to_string(), embed_url.clone());

        let actual_title = extract_title_from_embed_path(&embed_path);

        Ok(StreamCandidate {
            stream_url: master_url,
            provider_name: self.name().to_string(),
            headers,
            subtitles: Vec::new(),
            expires_at,
            embed_url: Some(embed_url),
            request: req.clone(),
            actual_title,
        })
    }

    async fn refresh_token(&self, candidate: &mut StreamCandidate) -> Result<(), String> {
        let fresh = self.resolve(&candidate.request).await?;
        *candidate = fresh;
        Ok(())
    }
}

fn extract_title_from_embed_path(embed_path: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;

    let pos = embed_path.find("&d=")?;
    let raw = &embed_path[pos + 3..];
    let token = raw.split('&').next()?;
    let decoded_url: String = url::form_urlencoded::parse(token.as_bytes())
        .map(|(k, _)| k.to_string())
        .collect();
    let to_decode = if decoded_url.is_empty() {
        token
    } else {
        &decoded_url
    };
    let bytes = BASE64_STANDARD.decode(to_decode.as_bytes()).ok()?;
    String::from_utf8(bytes).ok()
}
