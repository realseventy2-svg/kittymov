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
    seasons_re: Regex,
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
        let seasons_re = Regex::new(r#"window\.seasons\s*=\s*'([^']+)'"#).unwrap();

        Self {
            client,
            movie_play_re,
            show_play_re,
            hash_re,
            expires_re,
            movie_id_re,
            seasons_re,
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
        let encoded_query = url::form_urlencoded::byte_serialize(req.title.as_bytes()).collect::<String>();
        let search_url = format!("{}/api/v1/{}/do-search/?q={}", base_url, search_endpoint, encoded_query);

        let search_resp: SearchResponse = self.client.get(&search_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .send().await
            .map_err(|e| format!("LookMovie search request failed: {}", e))?
            .json().await
            .map_err(|e| format!("Invalid search JSON from LookMovie: {}", e))?;

        if search_resp.result.is_empty() {
            return Err(format!("LookMovie: No search results found for '{}'", req.title));
        }

        let slug = self.select_best_slug(&search_resp.result, &req.title, req.year.as_deref())
            .ok_or_else(|| format!("LookMovie: No matching slug for '{}'", req.title))?;

        // Step 2: Fetch view page to extract play link
        let view_path = if is_movie {
            format!("movies/view/{}", slug)
        } else {
            format!("shows/view/{}", slug)
        };
        let view_url = format!("{}/{}", base_url, view_path);

        let view_html = self.client.get(&view_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", base_url)
            .send().await
            .map_err(|e| format!("Failed to fetch LookMovie view page: {}", e))?
            .text().await
            .map_err(|e| format!("Failed to read LookMovie view page: {}", e))?;

        let play_re = if is_movie { &self.movie_play_re } else { &self.show_play_re };
        let play_path = play_re.captures(&view_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "Could not find play link in LookMovie view page".to_string())?;

        let play_url = format!("{}{}", base_url, play_path);

        // Step 3: Fetch play page to extract storage credentials
        let play_html = self.client.get(&play_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", &view_url)
            .send().await
            .map_err(|e| format!("Failed to fetch LookMovie play page: {}", e))?
            .text().await
            .map_err(|e| format!("Failed to read LookMovie play page: {}", e))?;

        let hash = self.hash_re.captures(&play_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "Could not find hash in LookMovie play page".to_string())?;

        let expires_str = self.expires_re.captures(&play_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Could not find expires in LookMovie play page".to_string())?;
        let expires_at = expires_str.parse::<u64>().unwrap_or(0);

        // Step 4: Call security access endpoint
        let access_url = if is_movie {
            let id_movie = self.movie_id_re.captures(&play_html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .ok_or_else(|| "Could not find id_movie in LookMovie play page".to_string())?;

            format!(
                "{}/api/v1/security/movie-access?id_movie={}&hash={}&expires={}",
                base_url, id_movie, hash, expires_str
            )
        } else {
            let seasons_raw = self.seasons_re.captures(&play_html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .ok_or_else(|| "Could not find window.seasons in LookMovie show play page".to_string())?;

            let unescaped = unescape_js_string(seasons_raw);
            let seasons_json: serde_json::Value = serde_json::from_str(&unescaped)
                .map_err(|e| format!("Failed to parse window.seasons JSON: {}", e))?;

            let season_str = req.season.unwrap_or(1).to_string();
            let episode_str = req.episode.unwrap_or(1).to_string();

            let id_episode = find_episode_id(&seasons_json, &season_str, &episode_str)
                .ok_or_else(|| format!("LookMovie: Season {} Episode {} not found", season_str, episode_str))?;

            format!(
                "{}/api/v1/security/episode-access?id_episode={}&hash={}&expires={}",
                base_url, id_episode, hash, expires_str
            )
        };

        let access_resp: AccessResponse = self.client.get(&access_url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", &play_url)
            .header("Accept", "application/json")
            .send().await
            .map_err(|e| format!("LookMovie access request failed: {}", e))?
            .json().await
            .map_err(|e| format!("Invalid access JSON from LookMovie: {}", e))?;

        if !access_resp.success {
            return Err("LookMovie security access returned success: false".to_string());
        }

        let streams = access_resp.streams
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

        let stream_url = chosen_stream
            .ok_or_else(|| "LookMovie: No playable stream URL found in access response".to_string())?;

        // Extract English subtitles if available
        let mut subtitles = Vec::new();
        if let Some(subs) = access_resp.subtitles {
            for sub in subs {
                if let Some(ref lang) = sub.language {
                    if lang.eq_ignore_ascii_case("english") {
                        if let Some(file_val) = sub.file {
                            match file_val {
                                serde_json::Value::String(s) => {
                                    let sub_url = if s.starts_with('/') {
                                        format!("{}{}", base_url, s)
                                    } else {
                                        s
                                    };
                                    subtitles.push(sub_url);
                                }
                                serde_json::Value::Array(arr) => {
                                    for item in arr {
                                        if let Some(s) = item.as_str() {
                                            let sub_url = if s.starts_with('/') {
                                                format!("{}{}", base_url, s)
                                            } else {
                                                s.to_string()
                                            };
                                            subtitles.push(sub_url);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
        headers.insert("Referer".to_string(), "https://www.lookmovie2.to/".to_string());

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

fn unescape_js_string(s: &str) -> String {
    s.replace(r#"\""#, "\"")
        .replace(r#"\/"#, "/")
        .replace(r#"\\"#, r#"\"#)
}

fn find_episode_id(val: &serde_json::Value, season_str: &str, episode_str: &str) -> Option<String> {
    let seasons_map = val.as_object()?;
    let season_obj = seasons_map.get(season_str)?;
    let episodes_map = season_obj.get("episodes")?.as_object()?;

    // Search by key or by episode_number field
    if let Some(ep) = episodes_map.get(episode_str) {
        if let Some(id) = ep.get("id_episode") {
            if let Some(s) = id.as_str() {
                return Some(s.to_string());
            } else if let Some(n) = id.as_i64() {
                return Some(n.to_string());
            }
        }
    }

    for (_key, ep) in episodes_map {
        let ep_num_match = ep.get("episode_number")
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s == episode_str)
                } else if let Some(n) = v.as_i64() {
                    Some(n.to_string() == episode_str)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if ep_num_match {
            if let Some(id) = ep.get("id_episode") {
                if let Some(s) = id.as_str() {
                    return Some(s.to_string());
                } else if let Some(n) = id.as_i64() {
                    return Some(n.to_string());
                }
            }
        }
    }

    None
}
