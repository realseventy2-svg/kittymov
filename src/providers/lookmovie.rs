use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use super::{MediaRequest, Provider, StreamCandidate};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct LookMovieProvider {
    client: Client,
    movie_play_re: Regex,
    show_play_re: Regex,
    hash_re: Regex,
    expires_re: Regex,
    movie_id_re: Regex,
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    result: Vec<SearchResultItem>,
}

#[derive(Deserialize)]
struct SearchResultItem {
    slug: Option<String>,
    title: Option<String>,
    year: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AccessResponse {
    #[serde(default)]
    success: bool,
    streams: Option<HashMap<String, Option<String>>>,
    subtitles: Option<Vec<SubtitleItem>>,
}

#[derive(Deserialize)]
struct SubtitleItem {
    language: Option<String>,
    file: Option<serde_json::Value>,
}

impl LookMovieProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        let movie_play_re = Regex::new(r#"href="(/movies/play/[^"]+)""#).unwrap();
        let show_play_re = Regex::new(r#"href="(/shows/play/[^"]+)""#).unwrap();
        let hash_re = Regex::new(r#"hash:\s*["']([^"']+)["']"#).unwrap();
        let expires_re = Regex::new(r#"expires:\s*(\d+)"#).unwrap();
        let movie_id_re = Regex::new(r#"id_movie:\s*(\d+)"#).unwrap();

        Self {
            client,
            movie_play_re,
            show_play_re,
            hash_re,
            expires_re,
            movie_id_re,
        }
    }

    fn select_best_slug(
        &self,
        items: &[SearchResultItem],
        title: &str,
        year_opt: Option<&str>,
    ) -> Option<String> {
        let target_title = title.trim().to_lowercase();

        // 1. Check title + year
        if let Some(target_year) = year_opt {
            for item in items {
                if let (Some(t), Some(slug)) = (&item.title, &item.slug) {
                    if t.trim().to_lowercase() == target_title {
                        if let Some(ref y_val) = item.year {
                            let y_str = match y_val {
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::String(s) => s.clone(),
                                _ => String::new(),
                            };
                            if y_str.starts_with(target_year) {
                                return Some(slug.clone());
                            }
                        }
                    }
                }
            }
        }

        // 2. Check title only
        for item in items {
            if let (Some(t), Some(slug)) = (&item.title, &item.slug) {
                if t.trim().to_lowercase() == target_title {
                    return Some(slug.clone());
                }
            }
        }

        // 3. Fallback to first available slug
        items.iter().find_map(|item| item.slug.clone())
    }
}

#[async_trait]
impl Provider for LookMovieProvider {
    fn name(&self) -> &'static str {
        "LookMovie (Fallback CDN)"
    }

    async fn resolve(&self, req: &MediaRequest) -> Result<StreamCandidate, String> {
        let base_url = "https://www.lookmovie2.to";
        let is_movie = req.media_type == "movie";

        // Step 1: Search for title
        let search_endpoint = if is_movie { "movies" } else { "shows" };
        let encoded_query =
            url::form_urlencoded::byte_serialize(req.title.as_bytes()).collect::<String>();
        let search_url = format!(
            "{}/api/v1/{}/do-search/?q={}",
            base_url, search_endpoint, encoded_query
        );

        let search_res = self
            .client
            .get(&search_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("LookMovie network error: {}", e))?;

        if search_res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("LookMovie: Title '{}' not found (HTTP 404)", req.title));
        }

        if !search_res.status().is_success() {
            return Err(format!("LookMovie search returned HTTP {}", search_res.status()));
        }

        let search_resp: SearchResponse = search_res
            .json()
            .await
            .map_err(|e| format!("Invalid search JSON from LookMovie: {}", e))?;

        if search_resp.result.is_empty() {
            return Err(format!(
                "LookMovie: No stream entries found for '{}'",
                req.title
            ));
        }

        let slug = self
            .select_best_slug(&search_resp.result, &req.title, req.year.as_deref())
            .ok_or_else(|| format!("LookMovie: No matching slug for '{}'", req.title))?;

        // Step 2: Fetch view page to extract play link
        let view_path = if is_movie {
            format!("movies/view/{}", slug)
        } else {
            format!("shows/view/{}", slug)
        };
        let view_url = format!("{}/{}", base_url, view_path);

        let view_html = self
            .client
            .get(&view_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", base_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch LookMovie view page: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read LookMovie view page: {}", e))?;

        let play_re = if is_movie {
            &self.movie_play_re
        } else {
            &self.show_play_re
        };
        let play_path = play_re
            .captures(&view_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "Could not find play link in LookMovie view page".to_string())?;

        let play_url = format!("{}{}", base_url, play_path);

        // Step 3: Fetch play page to extract storage credentials
        let play_html = self
            .client
            .get(&play_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", &view_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch LookMovie play page: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read LookMovie play page: {}", e))?;

        let hash = self
            .hash_re
            .captures(&play_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "Could not find hash in LookMovie play page".to_string())?;

        let expires_str = self
            .expires_re
            .captures(&play_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Could not find expires in LookMovie play page".to_string())?;
        let expires_at = expires_str.parse::<u64>().unwrap_or(0);

        // Step 4: Call security access endpoint
        let access_url = if is_movie {
            let id_movie = self
                .movie_id_re
                .captures(&play_html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .ok_or_else(|| "Could not find id_movie in LookMovie play page".to_string())?;

            format!(
                "{}/api/v1/security/movie-access?id_movie={}&hash={}&expires={}",
                base_url, id_movie, hash, expires_str
            )
        } else {
            let unescaped = extract_window_seasons(&play_html).ok_or_else(|| {
                "Could not find window.seasons in LookMovie show play page".to_string()
            })?;

            let seasons_json: serde_json::Value = serde_json::from_str(&unescaped)
                .map_err(|e| format!("Failed to parse window.seasons JSON: {}", e))?;

            let season_str = req.season.unwrap_or(1).to_string();
            let episode_str = req.episode.unwrap_or(1).to_string();

            let id_episode =
                find_episode_id(&seasons_json, &season_str, &episode_str).ok_or_else(|| {
                    format!(
                        "LookMovie: Season {} Episode {} not found",
                        season_str, episode_str
                    )
                })?;

            format!(
                "{}/api/v1/security/episode-access?id_episode={}&hash={}&expires={}",
                base_url, id_episode, hash, expires_str
            )
        };

        let access_resp: AccessResponse = self
            .client
            .get(&access_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", &play_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("LookMovie access request failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid access JSON from LookMovie: {}", e))?;

        if !access_resp.success {
            return Err("LookMovie security access returned success: false".to_string());
        }

        let streams = access_resp
            .streams
            .ok_or_else(|| "No streams returned by LookMovie access endpoint".to_string())?;

        // Pick highest quality available
        let quality_candidates = ["1080p", "1080", "720p", "720", "480p", "480", "360p", "360"];
        let mut chosen_stream = None;
        for q in quality_candidates {
            if let Some(Some(u)) = streams.get(q) {
                if !u.is_empty() {
                    chosen_stream = Some(u.clone());
                    break;
                }
            }
        }

        let stream_url = chosen_stream.ok_or_else(|| {
            "LookMovie: No playable stream URL found in access response".to_string()
        })?;

        // Extract subtitles (prioritizing English and popular languages)
        let mut subtitles = Vec::new();
        if let Some(subs) = access_resp.subtitles {
            // Priority 1: English
            for sub in &subs {
                if let Some(ref lang) = sub.language {
                    if lang.eq_ignore_ascii_case("english") {
                        if let Some(ref file_val) = sub.file {
                            if let Some(s) = file_val.as_str() {
                                let sub_url = if s.starts_with('/') {
                                    format!("{}{}", base_url, s)
                                } else {
                                    s.to_string()
                                };
                                if !subtitles.contains(&sub_url) {
                                    subtitles.push(sub_url);
                                }
                            }
                        }
                    }
                }
            }
            // Priority 2: Other available languages (up to 15 total)
            for sub in subs {
                if subtitles.len() >= 15 {
                    break;
                }
                if let Some(ref file_val) = sub.file {
                    if let Some(s) = file_val.as_str() {
                        let sub_url = if s.starts_with('/') {
                            format!("{}{}", base_url, s)
                        } else {
                            s.to_string()
                        };
                        if !subtitles.contains(&sub_url) {
                            subtitles.push(sub_url);
                        }
                    }
                }
            }
        }

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
        headers.insert(
            "Referer".to_string(),
            "https://www.lookmovie2.to/".to_string(),
        );

        Ok(StreamCandidate {
            stream_url,
            provider_name: self.name().to_string(),
            headers,
            subtitles,
            expires_at,
            embed_url: Some(play_url),
            request: req.clone(),
            actual_title: Some(req.title.clone()),
        })
    }

    async fn refresh_token(&self, candidate: &mut StreamCandidate) -> Result<(), String> {
        let fresh = self.resolve(&candidate.request).await?;
        *candidate = fresh;
        Ok(())
    }
}

fn extract_window_seasons(html: &str) -> Option<String> {
    for line in html.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("window.seasons") {
            let after = &trimmed[idx..];
            if let Some(first_quote) = after.find('\'') {
                let rest = &after[first_quote + 1..];
                if let Some(last_quote) = rest.rfind('\'') {
                    let raw = &rest[..last_quote];
                    return Some(unescape_js_string(raw));
                }
            }
        }
    }
    None
}

fn unescape_js_string(s: &str) -> String {
    s.replace(r#"\""#, "\"")
        .replace(r#"\/"#, "/")
        .replace(r#"\'"#, "'")
        .replace(r#"\\"#, r#"\"#)
}

fn find_episode_id(val: &serde_json::Value, season_str: &str, episode_str: &str) -> Option<String> {
    let get_id = |ep: &serde_json::Value| -> Option<String> {
        if let Some(id) = ep.get("id_episode") {
            if let Some(s) = id.as_str() {
                return Some(s.to_string());
            } else if let Some(n) = id.as_i64() {
                return Some(n.to_string());
            }
        }
        None
    };

    let ep_number_matches = |ep: &serde_json::Value, target_str: &str| -> bool {
        let check = |v: &serde_json::Value| -> bool {
            if let Some(s) = v.as_str() {
                s == target_str || s.trim_start_matches('0') == target_str.trim_start_matches('0')
            } else if let Some(n) = v.as_i64() {
                n.to_string() == target_str
            } else {
                false
            }
        };

        if let Some(v) = ep.get("episode_number") {
            if check(v) {
                return true;
            }
        }
        if let Some(v) = ep.get("episode") {
            if check(v) {
                return true;
            }
        }
        if let Some(v) = ep.get("number") {
            if check(v) {
                return true;
            }
        }
        false
    };

    // Find the season object in val
    let season_obj = if let Some(seasons_map) = val.as_object() {
        seasons_map
            .get(season_str)
            .or_else(|| {
                let norm = season_str.trim_start_matches('0');
                seasons_map
                    .iter()
                    .find(|(k, _)| k.trim_start_matches('0') == norm)
                    .map(|(_, v)| v)
            })
            .cloned()
    } else if let Some(seasons_list) = val.as_array() {
        seasons_list
            .iter()
            .find(|s| {
                let check = |v: &serde_json::Value| {
                    if let Some(str_v) = v.as_str() {
                        str_v == season_str
                            || str_v.trim_start_matches('0') == season_str.trim_start_matches('0')
                    } else if let Some(n) = v.as_i64() {
                        n.to_string() == season_str
                    } else {
                        false
                    }
                };
                s.get("season_number").is_some_and(check) || s.get("season").is_some_and(check)
            })
            .cloned()
    } else {
        None
    }?;

    let eps_val = season_obj.get("episodes")?;

    // 1. If episodes is a JSON array
    if let Some(episodes_list) = eps_val.as_array() {
        // Pass 1: Strict match on episode number metadata
        for ep in episodes_list {
            if ep_number_matches(ep, episode_str) {
                if let Some(id) = get_id(ep) {
                    return Some(id);
                }
            }
        }
        // Pass 2: Fallback to 1-based index if metadata is missing
        if let Ok(num) = episode_str.parse::<usize>() {
            if num > 0 && num <= episodes_list.len() {
                if let Some(ep) = episodes_list.get(num - 1) {
                    if let Some(id) = get_id(ep) {
                        return Some(id);
                    }
                }
            }
        }
    } else if let Some(episodes_map) = eps_val.as_object() {
        // 2. If episodes is a JSON object / map
        // Pass 1: Strict match on episode_number across all items (prevents 0-based key offset bugs)
        for (_k, ep) in episodes_map {
            if ep_number_matches(ep, episode_str) {
                if let Some(id) = get_id(ep) {
                    return Some(id);
                }
            }
        }

        // Pass 2: Only fallback to dictionary key if no episode_number metadata matched
        let norm_ep = episode_str.trim_start_matches('0');
        for (k, ep) in episodes_map {
            if k == episode_str || k.trim_start_matches('0') == norm_ep {
                if let Some(id) = get_id(ep) {
                    return Some(id);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_episode_id_zero_based_dict_no_offset() {
        // Simulates LookMovie dictionary with 0-based string keys
        let raw = serde_json::json!({
            "1": {
                "episodes": {
                    "0": { "id_episode": 100, "episode_number": 1, "title": "Pilot" },
                    "1": { "id_episode": 101, "episode_number": 2, "title": "Second" },
                    "2": { "id_episode": 102, "episode_number": 3, "title": "Third" }
                }
            }
        });

        // Episode 1 must resolve to id 100, NOT id 101!
        assert_eq!(find_episode_id(&raw, "1", "1"), Some("100".to_string()));
        assert_eq!(find_episode_id(&raw, "1", "2"), Some("101".to_string()));
        assert_eq!(find_episode_id(&raw, "1", "3"), Some("102".to_string()));
    }

    #[test]
    fn test_find_episode_id_array_with_pilot_zero() {
        // Simulates LookMovie array with Pilot (episode 0) at index 0
        let raw = serde_json::json!({
            "3": {
                "episodes": [
                    { "id_episode": "pilot_id", "episode_number": 0, "title": "Pilot" },
                    { "id_episode": "ep1_id", "episode_number": 1, "title": "Super Watermelon Island" },
                    { "id_episode": "ep2_id", "episode_number": 2, "title": "Gem Drill" }
                ]
            }
        });

        assert_eq!(find_episode_id(&raw, "3", "1"), Some("ep1_id".to_string()));
        assert_eq!(find_episode_id(&raw, "3", "2"), Some("ep2_id".to_string()));
    }
}
