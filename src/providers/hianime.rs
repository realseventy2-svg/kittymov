use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use super::{MediaRequest, Provider, StreamCandidate};

pub const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
pub const BASE_URL: &str = "https://hianime.at";
const XOR_KEY: &[u8] = b"otaku-embed-v1";

pub struct HiAnimeProvider {
    client: Client,
    search_re: Regex,
    episodes_re: Regex,
    servers_re: Regex,
    window_p_re: Regex,
}

#[derive(Deserialize)]
struct HiAnimeJsonResponse {
    html: Option<String>,
}

#[derive(Deserialize)]
struct ZokoStreamData {
    src: Option<String>,
    subtitles: Option<Vec<ZokoSubtitle>>,
}

#[derive(Deserialize)]
struct ZokoSubtitle {
    src: Option<String>,
    #[allow(dead_code)]
    lang: Option<String>,
    #[allow(dead_code)]
    label: Option<String>,
}

fn normalize_title(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn is_stop_word(w: &str) -> bool {
    matches!(
        w,
        "the" | "a" | "an" | "and" | "of" | "in" | "to" | "for" | "on" | "at" | "by" | "with"
    )
}

impl Default for HiAnimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl HiAnimeProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| Client::new());

        let search_re =
            Regex::new(r#"<h3 class="film-name">\s*<a href="[^"]*/([^"]*)"\s*title="([^"]*)""#)
                .unwrap();
        let episodes_re = Regex::new(r#"data-number="([^"]*)"[\s\S]*?data-id="([0-9]+)""#).unwrap();
        let servers_re = Regex::new(
            r#"<div class="[^"]*server-item[^"]*"[\s\S]*?data-type="([^"]*)"[\s\S]*?data-server-name="([^"]*)"[\s\S]*?data-hash="([^"]*)""#,
        )
        .unwrap();
        let window_p_re = Regex::new(r#"window\.__P\s*=\s*["']([^"']+)["']"#).unwrap();

        Self {
            client,
            search_re,
            episodes_re,
            servers_re,
            window_p_re,
        }
    }

    fn select_best_slug(
        &self,
        results: &[(String, String)],
        query_title: &str,
        season: Option<i32>,
    ) -> Option<String> {
        let q_norm = normalize_title(query_title);
        let q_words: Vec<String> = query_title
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty() && w.len() > 1 && !is_stop_word(&w.to_lowercase()))
            .map(|w| w.to_lowercase())
            .collect();
        let season_num = season.unwrap_or(1);

        // 1. Season-aware matching
        if season_num > 1 {
            let s_tag = format!("season-{}", season_num);
            let s_tag2 = format!("season {}", season_num);
            let p_tag = format!("part-{}", season_num);
            let p_tag2 = format!("part {}", season_num);
            for (slug, title) in results {
                let sl_lower = slug.to_lowercase();
                let ti_lower = title.to_lowercase();
                if sl_lower.contains(&s_tag)
                    || sl_lower.contains(&p_tag)
                    || ti_lower.contains(&s_tag2)
                    || ti_lower.contains(&p_tag2)
                {
                    return Some(slug.clone());
                }
            }
        } else {
            // For season 1, prefer non-sequel results matching exact title
            for (slug, title) in results {
                let sl = slug.to_lowercase();
                let ti = title.to_lowercase();
                if sl.contains("season-2")
                    || sl.contains("season-3")
                    || sl.contains("part-2")
                    || sl.contains("part-3")
                    || ti.contains("season 2")
                    || ti.contains("season 3")
                    || ti.contains("part 2")
                    || ti.contains("part 3")
                {
                    continue;
                }
                if normalize_title(title) == q_norm {
                    return Some(slug.clone());
                }
            }
        }

        // 2. Exact normalized title match
        for (slug, title) in results {
            if normalize_title(title) == q_norm {
                return Some(slug.clone());
            }
        }

        // 3. Normalized prefix / contains match
        for (slug, title) in results {
            let t_norm = normalize_title(title);
            if t_norm == q_norm || t_norm.starts_with(&q_norm) || q_norm.starts_with(&t_norm) {
                return Some(slug.clone());
            }
        }

        // 4. Word overlap: all significant words must appear in candidate title
        if !q_words.is_empty() {
            for (slug, title) in results {
                let ti_lower = title.to_lowercase();
                let matching = q_words.iter().filter(|w| ti_lower.contains(w.as_str())).count();
                if matching == q_words.len() {
                    return Some(slug.clone());
                }
            }
        }

        // 5. If query only has 1 word, must match a standalone word
        if q_words.len() == 1 {
            let single = &q_words[0];
            for (slug, title) in results {
                let ti_lower = title.to_lowercase();
                if ti_lower.split(|c: char| !c.is_alphanumeric()).any(|w| w == single) {
                    return Some(slug.clone());
                }
            }
        }

        None
    }

    async fn search_titles(&self, query: &str) -> Result<Vec<(String, String)>, String> {
        let clean_q: String = query
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        let encoded_title =
            url::form_urlencoded::byte_serialize(clean_q.trim().as_bytes()).collect::<String>();
        let search_url = format!("{}/search?keyword={}", BASE_URL, encoded_title);

        let search_res = self
            .client
            .get(&search_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", BASE_URL)
            .send()
            .await
            .map_err(|e| format!("HiAnime search network error: {}", e))?;

        if !search_res.status().is_success() {
            return Err(format!("HiAnime search returned HTTP {}", search_res.status()));
        }

        let search_html = search_res
            .text()
            .await
            .map_err(|e| format!("Failed to read HiAnime search HTML: {}", e))?;

        let mut results = Vec::new();
        for caps in self.search_re.captures_iter(&search_html) {
            if let (Some(s), Some(t)) = (caps.get(1), caps.get(2)) {
                results.push((s.as_str().to_string(), t.as_str().to_string()));
            }
        }
        Ok(results)
    }

    fn select_episode_id(&self, episodes: &[(String, String)], target_ep: i32) -> Option<String> {
        let ep_str = target_ep.to_string();
        // 1. Direct number match
        for (num, id) in episodes {
            if num.trim() == ep_str {
                return Some(id.clone());
            }
        }
        // 2. Index match (0-indexed)
        if target_ep > 0 && (target_ep as usize) <= episodes.len() {
            return Some(episodes[(target_ep - 1) as usize].1.clone());
        }
        // 3. Fallback to first
        episodes.first().map(|(_, id)| id.clone())
    }

    fn decode_blob(&self, blob_b64: &str) -> Result<ZokoStreamData, String> {
        let clean_blob = blob_b64.trim();
        let blob_bytes = BASE64_STANDARD
            .decode(clean_blob.as_bytes())
            .map_err(|e| format!("HiAnime: Base64 decode error on blob: {}", e))?;

        let deobf: Vec<u8> = blob_bytes
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()])
            .collect();

        serde_json::from_slice(&deobf)
            .map_err(|e| format!("HiAnime: JSON parse error on stream payload: {}", e))
    }

    pub async fn resolve_with_quality(
        &self,
        req: &MediaRequest,
        quality: Option<&str>,
        audio_mode: Option<&str>,
    ) -> Result<StreamCandidate, String> {
        let is_dub = audio_mode
            .map(|m| m.eq_ignore_ascii_case("dub"))
            .unwrap_or(false);

        // Step 1: Search for title
        let results = self.search_titles(&req.title).await.unwrap_or_default();
        let mut matched_slug = self.select_best_slug(&results, &req.title, req.season);

        // If no match found and title contains ':' or '-', try searching the subtitle portion
        if matched_slug.is_none() && (req.title.contains(':') || req.title.contains('-')) {
            let subpart = if let Some((_, sub)) = req.title.split_once(':') {
                sub.trim()
            } else if let Some((_, sub)) = req.title.split_once('-') {
                sub.trim()
            } else {
                ""
            };
            if !subpart.is_empty() {
                if let Ok(sub_results) = self.search_titles(subpart).await {
                    matched_slug = self
                        .select_best_slug(&sub_results, &req.title, req.season)
                        .or_else(|| self.select_best_slug(&sub_results, subpart, req.season));
                }
            }
        }

        let slug = matched_slug
            .ok_or_else(|| format!("HiAnime: No matching anime found for '{}'", req.title))?;

        let anime_id = slug
            .rsplit('-')
            .next()
            .ok_or_else(|| format!("HiAnime: Invalid anime slug format '{}'", slug))?;

        // Step 2: Fetch episodes
        let episodes_url = format!("{}/api/theme/episode/list/{}", BASE_URL, anime_id);
        let ep_resp = self
            .client
            .get(&episodes_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{}/watch/{}", BASE_URL, slug))
            .send()
            .await
            .map_err(|e| format!("HiAnime episodes list request failed: {}", e))?;

        let ep_json: HiAnimeJsonResponse = ep_resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from HiAnime episodes list: {}", e))?;

        let ep_html = ep_json
            .html
            .ok_or_else(|| "HiAnime: No HTML returned in episode list".to_string())?;

        let mut episodes = Vec::new();
        for caps in self.episodes_re.captures_iter(&ep_html) {
            if let (Some(num), Some(id)) = (caps.get(1), caps.get(2)) {
                episodes.push((num.as_str().to_string(), id.as_str().to_string()));
            }
        }

        if episodes.is_empty() {
            return Err(format!("HiAnime: No episodes found for '{}'", req.title));
        }

        let target_ep = req.episode.unwrap_or(1);
        let episode_id = self
            .select_episode_id(&episodes, target_ep)
            .ok_or_else(|| format!("HiAnime: Episode {} not found", target_ep))?;

        // Step 3: Fetch servers for episode
        let servers_url = format!(
            "{}/api/theme/episode/servers?episodeId={}",
            BASE_URL, episode_id
        );
        let servers_resp = self
            .client
            .get(&servers_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{}/watch/{}", BASE_URL, slug))
            .send()
            .await
            .map_err(|e| format!("HiAnime servers request failed: {}", e))?;

        let servers_json: HiAnimeJsonResponse = servers_resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from HiAnime servers: {}", e))?;

        let s_html = servers_json
            .html
            .ok_or_else(|| "HiAnime: No HTML returned in servers endpoint".to_string())?;

        let mut available_servers = Vec::new();
        for caps in self.servers_re.captures_iter(&s_html) {
            if let (Some(dtype), Some(sname), Some(hash)) =
                (caps.get(1), caps.get(2), caps.get(3))
            {
                available_servers.push((
                    dtype.as_str().to_lowercase(),
                    sname.as_str().to_string(),
                    hash.as_str().to_string(),
                ));
            }
        }

        if available_servers.is_empty() {
            return Err("HiAnime: No streaming servers available for this episode".to_string());
        }

        // Select server matching preferred audio mode (sub vs dub) and player
        let target_mode = if is_dub { "dub" } else { "sub" };
        let chosen_server = available_servers
            .iter()
            .find(|(m, name, _)| m == target_mode && name.contains("ZokoAnime"))
            .or_else(|| available_servers.iter().find(|(m, _, _)| m == target_mode))
            .or_else(|| available_servers.iter().find(|(_, name, _)| name.contains("ZokoAnime")))
            .or_else(|| available_servers.first())
            .ok_or_else(|| "HiAnime: Could not select a valid server".to_string())?;

        let embed_url_bytes = BASE64_STANDARD
            .decode(chosen_server.2.trim().as_bytes())
            .map_err(|e| format!("HiAnime: Base64 decode server hash error: {}", e))?;
        let embed_url = String::from_utf8(embed_url_bytes)
            .map_err(|e| format!("HiAnime: Server embed URL utf-8 error: {}", e))?;

        // Step 4: Fetch embed and deobfuscate payload
        let embed_resp = self
            .client
            .get(&embed_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", format!("{}/", BASE_URL))
            .send()
            .await
            .map_err(|e| format!("HiAnime embed request failed: {}", e))?;

        let embed_html = embed_resp
            .text()
            .await
            .map_err(|e| format!("Failed to read HiAnime embed HTML: {}", e))?;

        let blob = self
            .window_p_re
            .captures(&embed_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "HiAnime: window.__P payload not found in embed page".to_string())?;

        let stream_data = self.decode_blob(blob)?;
        let master_url = stream_data
            .src
            .ok_or_else(|| "HiAnime: Stream source URL not found in payload".to_string())?;

        let mut subtitles = Vec::new();
        if let Some(subs) = stream_data.subtitles {
            for sub in subs {
                if let Some(s) = sub.src {
                    if (s.ends_with(".vtt") || s.contains(".vtt")) && !subtitles.contains(&s) {
                        subtitles.push(s);
                    }
                }
            }
        }

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
        let referer = if embed_url.starts_with("http") {
            let parts: Vec<&str> = embed_url.split('/').collect();
            if parts.len() >= 3 {
                format!("{}//{}/", parts[0], parts[2])
            } else {
                "https://zokoanime.video/".to_string()
            }
        } else {
            "https://zokoanime.video/".to_string()
        };
        headers.insert("Referer".to_string(), referer);

        let chosen_quality_label = match quality.unwrap_or("1080p").to_lowercase().as_str() {
            q if q.contains("720") => "720p HD",
            q if q.contains("480") => "480p SD",
            _ => "1080p FHD",
        };

        Ok(StreamCandidate {
            stream_url: master_url,
            provider_name: self.name().to_string(),
            headers,
            subtitles,
            expires_at: 0,
            embed_url: Some(embed_url),
            request: req.clone(),
            actual_title: Some(req.title.clone()),
            quality: Some(chosen_quality_label.to_string()),
        })
    }
}

#[async_trait]
impl Provider for HiAnimeProvider {
    fn name(&self) -> &'static str {
        "HiAnime (1080p Anime CDN)"
    }

    async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
        self.resolve_with_quality(req, None, None).await
    }

    async fn refresh_token(&self, candidate: &mut StreamCandidate) -> Result<(), String> {
        let fresh = self.resolve(&candidate.request).await?;
        *candidate = fresh;
        Ok(())
    }
}
