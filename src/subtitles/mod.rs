use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::config::get_cache_dir;
use crate::metadata::TmdbClient;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub id: String,
    pub lang: String, // Normalized 3-letter code: "eng", "spa", "fra", "jpn", etc.
    pub lang_name: String, // Friendly name: "English", "Spanish", "French", etc.
    pub url: String,  // Direct download URL
    pub provider: String, // "OpenSubtitles.org", "LookMovie"
    pub format: String, // "srt" or "vtt"
    pub release_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleLanguageGroup {
    pub lang_code: String,
    pub lang_name: String,
    pub track_count: usize,
    pub source: String,
    pub sample_track: SubtitleTrack,
}

pub fn normalize_language(code: &str) -> (String, String) {
    match code.to_lowercase().as_str() {
        "en" | "eng" => ("eng".to_string(), "English".to_string()),
        "es" | "spa" | "spl" => ("spa".to_string(), "Spanish".to_string()),
        "fr" | "fra" | "fre" => ("fra".to_string(), "French".to_string()),
        "de" | "deu" | "ger" => ("deu".to_string(), "German".to_string()),
        "it" | "ita" => ("ita".to_string(), "Italian".to_string()),
        "pt" | "por" | "pob" => ("por".to_string(), "Portuguese".to_string()),
        "ru" | "rus" => ("rus".to_string(), "Russian".to_string()),
        "ja" | "jpn" => ("jpn".to_string(), "Japanese".to_string()),
        "ko" | "kor" => ("kor".to_string(), "Korean".to_string()),
        "zh" | "zho" | "chi" => ("zho".to_string(), "Chinese".to_string()),
        "ar" | "ara" => ("ara".to_string(), "Arabic".to_string()),
        "vi" | "vie" => ("vie".to_string(), "Vietnamese".to_string()),
        "id" | "ind" => ("ind".to_string(), "Indonesian".to_string()),
        "tr" | "tur" => ("tur".to_string(), "Turkish".to_string()),
        "pl" | "pol" => ("pol".to_string(), "Polish".to_string()),
        "uk" | "ukr" => ("ukr".to_string(), "Ukrainian".to_string()),
        "nl" | "nld" | "dut" => ("nld".to_string(), "Dutch".to_string()),
        "el" | "ell" | "gre" => ("ell".to_string(), "Greek".to_string()),
        "he" | "heb" => ("heb".to_string(), "Hebrew".to_string()),
        "sv" | "swe" => ("swe".to_string(), "Swedish".to_string()),
        "da" | "dan" => ("dan".to_string(), "Danish".to_string()),
        "fi" | "fin" => ("fin".to_string(), "Finnish".to_string()),
        "no" | "nor" => ("nor".to_string(), "Norwegian".to_string()),
        "hi" | "hin" => ("hin".to_string(), "Hindi".to_string()),
        "th" | "tha" => ("tha".to_string(), "Thai".to_string()),
        "ro" | "ron" | "rum" => ("ron".to_string(), "Romanian".to_string()),
        "hu" | "hun" => ("hun".to_string(), "Hungarian".to_string()),
        "cs" | "ces" | "cze" => ("ces".to_string(), "Czech".to_string()),
        other => (other.to_string(), other.to_uppercase()),
    }
}

pub struct OpenSubtitlesProvider {
    client: Client,
}

impl OpenSubtitlesProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_tracks(
        &self,
        imdb_id: &str,
        media_type: &str,
        season: Option<i32>,
        episode: Option<i32>,
    ) -> Result<Vec<SubtitleTrack>, String> {
        let id_clean = imdb_id.trim();
        if id_clean.is_empty() {
            return Err("Empty IMDb ID".to_string());
        }

        let url = if media_type == "tv" {
            let s = season.unwrap_or(1);
            let ep = episode.unwrap_or(1);
            format!(
                "https://opensubtitles-v3.strem.io/subtitles/series/{}:{}:{}.json",
                id_clean, s, ep
            )
        } else {
            format!(
                "https://opensubtitles-v3.strem.io/subtitles/movie/{}.json",
                id_clean
            )
        };

        #[derive(Deserialize)]
        struct RawStremioSub {
            id: Option<String>,
            url: String,
            lang: Option<String>,
            #[serde(rename = "subtitleFileName")]
            subtitle_file_name: Option<String>,
            #[serde(rename = "movieReleaseName")]
            movie_release_name: Option<String>,
        }

        #[derive(Deserialize)]
        struct RawStremioResp {
            subtitles: Option<Vec<RawStremioSub>>,
        }

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("OpenSubtitles request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("OpenSubtitles HTTP status: {}", resp.status()));
        }

        let data: RawStremioResp = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenSubtitles JSON: {}", e))?;

        let mut tracks = Vec::new();
        if let Some(subs) = data.subtitles {
            for sub in subs {
                if sub.url.is_empty() {
                    continue;
                }
                let raw_lang = sub.lang.as_deref().unwrap_or("eng");
                let (norm_lang, lang_name) = normalize_language(raw_lang);

                let format = if sub.url.ends_with(".vtt")
                    || sub
                        .subtitle_file_name
                        .as_deref()
                        .unwrap_or("")
                        .ends_with(".vtt")
                {
                    "vtt".to_string()
                } else {
                    "srt".to_string()
                };

                let track_id = sub
                    .id
                    .unwrap_or_else(|| format!("{}_{}", norm_lang, tracks.len()));

                tracks.push(SubtitleTrack {
                    id: track_id,
                    lang: norm_lang,
                    lang_name,
                    url: sub.url,
                    provider: "OpenSubtitles.org".to_string(),
                    format,
                    release_name: sub.movie_release_name.or(sub.subtitle_file_name),
                });
            }
        }

        Ok(tracks)
    }
}

pub struct SubtitleManager {
    opensubtitles: OpenSubtitlesProvider,
    client: Client,
}

impl SubtitleManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .unwrap_or_default();

        Self {
            opensubtitles: OpenSubtitlesProvider::new(),
            client,
        }
    }

    /// Fetches all available subtitle tracks for a title across providers
    pub async fn fetch_all_subtitles(
        &self,
        imdb_id: Option<&str>,
        media_type: &str,
        season: Option<i32>,
        episode: Option<i32>,
    ) -> Vec<SubtitleTrack> {
        let mut tracks = Vec::new();

        if let Some(id) = imdb_id {
            if let Ok(ost_tracks) = self
                .opensubtitles
                .fetch_tracks(id, media_type, season, episode)
                .await
            {
                tracks.extend(ost_tracks);
            }
        }

        tracks
    }

    /// Groups tracks by language with counts for selector modals
    pub fn group_by_language(tracks: &[SubtitleTrack]) -> Vec<SubtitleLanguageGroup> {
        let mut map: HashMap<String, Vec<SubtitleTrack>> = HashMap::new();
        for t in tracks {
            map.entry(t.lang.clone()).or_default().push(t.clone());
        }

        let mut groups = Vec::new();
        for (lang, lang_tracks) in map {
            if let Some(first) = lang_tracks.first() {
                groups.push(SubtitleLanguageGroup {
                    lang_code: lang,
                    lang_name: first.lang_name.clone(),
                    track_count: lang_tracks.len(),
                    source: first.provider.clone(),
                    sample_track: first.clone(),
                });
            }
        }

        // Sort: English first, Spanish second, French third, then alphabetical by name
        groups.sort_by(|a, b| {
            let rank = |code: &str| match code {
                "eng" => 0,
                "spa" => 1,
                "fra" => 2,
                "deu" => 3,
                "jpn" => 4,
                "por" => 5,
                "ita" => 6,
                "rus" => 7,
                _ => 10,
            };
            rank(&a.lang_code)
                .cmp(&rank(&b.lang_code))
                .then(a.lang_name.cmp(&b.lang_name))
        });

        groups
    }

    /// Downloads and caches a subtitle file locally
    pub async fn download_and_cache_subtitle(
        &self,
        track: &SubtitleTrack,
        file_prefix: &str,
    ) -> Result<PathBuf, String> {
        let cache_dir = get_cache_dir().join("subtitles");
        std::fs::create_dir_all(&cache_dir).ok();

        // Sanitize file prefix
        let clean_prefix: String = file_prefix
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let filename = format!(
            "{}.{}.{}.{}",
            clean_prefix, track.lang, track.lang_name, track.format
        );
        let dest = cache_dir.join(&filename);

        if dest.exists() && dest.metadata().map(|m| m.len() > 10).unwrap_or(false) {
            return Ok(dest);
        }

        let resp = self
            .client
            .get(&track.url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Subtitle download failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Subtitle download HTTP status {}", resp.status()));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read subtitle stream: {}", e))?;

        if bytes.is_empty() {
            return Err("Empty subtitle file".to_string());
        }

        std::fs::write(&dest, &bytes)
            .map_err(|e| format!("Failed to write subtitle file to disk: {}", e))?;

        Ok(dest)
    }

    /// Prepares subtitles for MPV by querying OpenSubtitles, downloading the preferred language,
    /// plus top alternate languages, and returning local file paths.
    #[allow(clippy::too_many_arguments)]
    pub async fn prepare_subtitles(
        &self,
        tmdb: &Arc<TmdbClient>,
        tmdb_id: i64,
        title: &str,
        media_type: &str,
        season: Option<i32>,
        episode: Option<i32>,
        preferred_lang: &str,
    ) -> Vec<String> {
        let imdb_id = tmdb.get_imdb_id(tmdb_id, media_type).await.ok();
        let all_tracks = self
            .fetch_all_subtitles(imdb_id.as_deref(), media_type, season, episode)
            .await;

        if all_tracks.is_empty() {
            return Vec::new();
        }

        let file_prefix = if media_type == "tv" {
            let s = season.unwrap_or(1);
            let ep = episode.unwrap_or(1);
            format!("{}.S{:02}E{:02}", title, s, ep)
        } else {
            title.to_string()
        };

        let pref_norm = normalize_language(preferred_lang).0;

        // Collect unique best tracks:
        // 1. Preferred language tracks first
        // 2. Then other popular languages
        let mut selected_tracks: Vec<SubtitleTrack> = Vec::new();
        let mut added_langs = HashMap::new();

        // 1. First add preferred language
        for t in &all_tracks {
            if t.lang == pref_norm && !added_langs.contains_key(&t.lang) {
                selected_tracks.push(t.clone());
                added_langs.insert(t.lang.clone(), true);
                break;
            }
        }

        // 2. Add other languages up to 6 total tracks
        let priority_langs = [
            "eng", "spa", "fra", "deu", "jpn", "por", "ita", "rus", "kor", "zho",
        ];
        for lang in &priority_langs {
            if selected_tracks.len() >= 6 {
                break;
            }
            if !added_langs.contains_key(*lang) {
                if let Some(t) = all_tracks.iter().find(|t| t.lang == *lang) {
                    selected_tracks.push(t.clone());
                    added_langs.insert((*lang).to_string(), true);
                }
            }
        }

        // 3. If still fewer than 4, add any remaining distinct languages
        for t in &all_tracks {
            if selected_tracks.len() >= 6 {
                break;
            }
            if !added_langs.contains_key(&t.lang) {
                selected_tracks.push(t.clone());
                added_langs.insert(t.lang.clone(), true);
            }
        }

        // Download them in parallel with individual timeouts
        let mut result_files = Vec::new();
        for track in selected_tracks {
            match self.download_and_cache_subtitle(&track, &file_prefix).await {
                Ok(path) => {
                    result_files.push(path.to_string_lossy().to_string());
                }
                Err(_) => {
                    // Fallback directly to remote URL if download fails
                    result_files.push(track.url);
                }
            }
        }

        result_files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_language() {
        assert_eq!(
            normalize_language("en"),
            ("eng".to_string(), "English".to_string())
        );
        assert_eq!(
            normalize_language("eng"),
            ("eng".to_string(), "English".to_string())
        );
        assert_eq!(
            normalize_language("spa"),
            ("spa".to_string(), "Spanish".to_string())
        );
        assert_eq!(
            normalize_language("fre"),
            ("fra".to_string(), "French".to_string())
        );
        assert_eq!(
            normalize_language("jpn"),
            ("jpn".to_string(), "Japanese".to_string())
        );
        assert_eq!(
            normalize_language("ger"),
            ("deu".to_string(), "German".to_string())
        );
    }

    #[test]
    fn test_group_by_language() {
        let tracks = vec![
            SubtitleTrack {
                id: "1".to_string(),
                lang: "eng".to_string(),
                lang_name: "English".to_string(),
                url: "http://example.com/en1.srt".to_string(),
                provider: "OpenSubtitles.org".to_string(),
                format: "srt".to_string(),
                release_name: None,
            },
            SubtitleTrack {
                id: "2".to_string(),
                lang: "eng".to_string(),
                lang_name: "English".to_string(),
                url: "http://example.com/en2.srt".to_string(),
                provider: "OpenSubtitles.org".to_string(),
                format: "srt".to_string(),
                release_name: None,
            },
            SubtitleTrack {
                id: "3".to_string(),
                lang: "spa".to_string(),
                lang_name: "Spanish".to_string(),
                url: "http://example.com/es.srt".to_string(),
                provider: "OpenSubtitles.org".to_string(),
                format: "srt".to_string(),
                release_name: None,
            },
        ];

        let groups = SubtitleManager::group_by_language(&tracks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].lang_code, "eng");
        assert_eq!(groups[0].track_count, 2);
        assert_eq!(groups[1].lang_code, "spa");
        assert_eq!(groups[1].track_count, 1);
    }

    #[tokio::test]
    async fn test_opensubtitles_fetch_live_movie() {
        let provider = OpenSubtitlesProvider::new();
        let tracks = provider
            .fetch_tracks("tt0111161", "movie", None, None)
            .await
            .expect("Failed to fetch movie subs");
        assert!(
            !tracks.is_empty(),
            "OpenSubtitles should return tracks for Shawshank Redemption"
        );
        assert!(
            tracks.iter().any(|t| t.lang == "eng"),
            "Should contain English subtitle track"
        );
    }

    #[tokio::test]
    async fn test_opensubtitles_fetch_live_series() {
        let provider = OpenSubtitlesProvider::new();
        let tracks = provider
            .fetch_tracks("tt3061046", "tv", Some(1), Some(1))
            .await
            .expect("Failed to fetch SU subs");
        assert!(
            !tracks.is_empty(),
            "OpenSubtitles should return tracks for Steven Universe S01E01"
        );
        assert!(
            tracks.iter().any(|t| t.lang == "eng"),
            "Should contain English subtitle track for SU S01E01"
        );
    }
}
