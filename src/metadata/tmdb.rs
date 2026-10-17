use std::path::PathBuf;
use std::time::Duration;
use reqwest::Client;
use serde::Deserialize;
use crate::config::get_cache_dir;

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub tmdb_id: i64,
    pub media_type: String, // "movie" or "tv"
    pub title: String,
    pub year: String,
    pub rating: f64,
    pub poster_path: String,
    pub overview: String,
    pub genres: String,
    pub duration_str: String,
    pub status: String,
    pub episodes_count: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct TvSeason {
    pub season_number: i32,
    pub name: String,
    pub episode_count: i32,
}

#[derive(Debug, Clone)]
pub struct TvEpisode {
    pub episode_number: i32,
    pub name: String,
    pub overview: String,
    pub air_date: String,
    pub duration: String,
}

#[derive(Deserialize)]
struct TmdbRawSearchItem {
    id: i64,
    media_type: Option<String>,
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f64>,
    poster_path: Option<String>,
    overview: Option<String>,
}

#[derive(Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbRawSearchItem>,
}

#[derive(Deserialize)]
struct Genre {
    name: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TmdbMovieDetails {
    id: i64,
    title: String,
    release_date: Option<String>,
    runtime: Option<i32>,
    vote_average: Option<f64>,
    genres: Option<Vec<Genre>>,
    status: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
}

#[derive(Deserialize)]
struct RawSeason {
    season_number: i32,
    name: Option<String>,
    episode_count: Option<i32>,
}

#[derive(Deserialize)]
struct TmdbTvDetails {
    id: i64,
    name: String,
    first_air_date: Option<String>,
    number_of_episodes: Option<i32>,
    vote_average: Option<f64>,
    genres: Option<Vec<Genre>>,
    status: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    seasons: Option<Vec<RawSeason>>,
}

#[derive(Deserialize)]
struct RawEpisode {
    episode_number: i32,
    name: Option<String>,
    overview: Option<String>,
    air_date: Option<String>,
    runtime: Option<i32>,
}

#[derive(Deserialize)]
struct TmdbSeasonResponse {
    episodes: Vec<RawEpisode>,
}

pub struct TmdbClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl TmdbClient {
    pub fn new(api_key: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) CachyFlix/2.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_key: api_key.to_string(),
            base_url: "https://api.themoviedb.org/3".to_string(),
        }
    }

    pub async fn get_trending_movies(&self) -> Result<Vec<MediaItem>, String> {
        let url = format!("{}/trending/movie/week?api_key={}", self.base_url, self.api_key);
        let resp: TmdbSearchResponse = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        Ok(resp.results.into_iter().map(|item| {
            let year = item.release_date.as_deref().unwrap_or("").chars().take(4).collect();
            MediaItem {
                tmdb_id: item.id,
                media_type: "movie".to_string(),
                title: item.title.unwrap_or_else(|| "Untitled".to_string()),
                year,
                rating: (item.vote_average.unwrap_or(0.0) * 10.0).round() / 10.0,
                poster_path: item.poster_path.unwrap_or_default(),
                overview: item.overview.unwrap_or_else(|| "No synopsis available.".to_string()),
                genres: "Feature Film".to_string(),
                duration_str: "Movie".to_string(),
                status: "Released".to_string(),
                episodes_count: None,
            }
        }).collect())
    }

    pub async fn get_trending_tv(&self) -> Result<Vec<MediaItem>, String> {
        let url = format!("{}/trending/tv/week?api_key={}", self.base_url, self.api_key);
        let resp: TmdbSearchResponse = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        Ok(resp.results.into_iter().map(|item| {
            let year = item.first_air_date.as_deref().unwrap_or("").chars().take(4).collect();
            MediaItem {
                tmdb_id: item.id,
                media_type: "tv".to_string(),
                title: item.name.unwrap_or_else(|| "Untitled".to_string()),
                year,
                rating: (item.vote_average.unwrap_or(0.0) * 10.0).round() / 10.0,
                poster_path: item.poster_path.unwrap_or_default(),
                overview: item.overview.unwrap_or_else(|| "No synopsis available.".to_string()),
                genres: "TV Series".to_string(),
                duration_str: "Series".to_string(),
                status: "Ongoing".to_string(),
                episodes_count: None,
            }
        }).collect())
    }

    pub async fn search_multi(&self, query: &str) -> Result<Vec<MediaItem>, String> {
        let url = format!(
            "{}/search/multi?api_key={}&query={}&include_adult=false",
            self.base_url,
            self.api_key,
            urlencoding::encode(query)
        );
        let resp: TmdbSearchResponse = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for item in resp.results {
            let m_type = item.media_type.as_deref().unwrap_or("");
            if m_type != "movie" && m_type != "tv" {
                continue;
            }
            let is_movie = m_type == "movie";
            let title = if is_movie { item.title } else { item.name };
            let date = if is_movie { item.release_date } else { item.first_air_date };
            let year = date.as_deref().unwrap_or("").chars().take(4).collect();

            list.push(MediaItem {
                tmdb_id: item.id,
                media_type: m_type.to_string(),
                title: title.unwrap_or_else(|| "Untitled".to_string()),
                year,
                rating: (item.vote_average.unwrap_or(0.0) * 10.0).round() / 10.0,
                poster_path: item.poster_path.unwrap_or_default(),
                overview: item.overview.unwrap_or_else(|| "No synopsis available.".to_string()),
                genres: if is_movie { "Movie".to_string() } else { "Series".to_string() },
                duration_str: if is_movie { "Feature".to_string() } else { "Multi-Episode".to_string() },
                status: "Available".to_string(),
                episodes_count: None,
            });
        }
        Ok(list)
    }

    #[allow(dead_code)]
    pub async fn get_movie_details(&self, tmdb_id: i64) -> Result<MediaItem, String> {
        let url = format!("{}/movie/{}?api_key={}", self.base_url, tmdb_id, self.api_key);
        let d: TmdbMovieDetails = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        let genres = d.genres.unwrap_or_default()
            .into_iter().map(|g| g.name).collect::<Vec<_>>().join(", ");
        let year = d.release_date.as_deref().unwrap_or("").chars().take(4).collect();
        let runtime = d.runtime.map(|r| format!("{} min", r)).unwrap_or_else(|| "N/A".to_string());

        Ok(MediaItem {
            tmdb_id: d.id,
            media_type: "movie".to_string(),
            title: d.title,
            year,
            rating: (d.vote_average.unwrap_or(0.0) * 10.0).round() / 10.0,
            poster_path: d.poster_path.unwrap_or_default(),
            overview: d.overview.unwrap_or_default(),
            genres: if genres.is_empty() { "Movie".to_string() } else { genres },
            duration_str: runtime,
            status: d.status.unwrap_or_else(|| "Released".to_string()),
            episodes_count: None,
        })
    }

    pub async fn get_tv_details(&self, tmdb_id: i64) -> Result<(MediaItem, Vec<TvSeason>), String> {
        let url = format!("{}/tv/{}?api_key={}", self.base_url, tmdb_id, self.api_key);
        let d: TmdbTvDetails = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        let genres = d.genres.unwrap_or_default()
            .into_iter().map(|g| g.name).collect::<Vec<_>>().join(", ");
        let year = d.first_air_date.as_deref().unwrap_or("").chars().take(4).collect();

        let mut seasons = Vec::new();
        if let Some(raw_seasons) = d.seasons {
            for s in raw_seasons {
                if s.season_number > 0 {
                    seasons.push(TvSeason {
                        season_number: s.season_number,
                        name: s.name.unwrap_or_else(|| format!("Season {}", s.season_number)),
                        episode_count: s.episode_count.unwrap_or(0),
                    });
                }
            }
        }

        let item = MediaItem {
            tmdb_id: d.id,
            media_type: "tv".to_string(),
            title: d.name,
            year,
            rating: (d.vote_average.unwrap_or(0.0) * 10.0).round() / 10.0,
            poster_path: d.poster_path.unwrap_or_default(),
            overview: d.overview.unwrap_or_default(),
            genres: if genres.is_empty() { "TV Series".to_string() } else { genres },
            duration_str: format!("{} Seasons", seasons.len()),
            status: d.status.unwrap_or_else(|| "Series".to_string()),
            episodes_count: d.number_of_episodes,
        };

        Ok((item, seasons))
    }

    pub async fn get_season_episodes(&self, tmdb_id: i64, season: i32) -> Result<Vec<TvEpisode>, String> {
        let url = format!("{}/tv/{}/season/{}?api_key={}", self.base_url, tmdb_id, season, self.api_key);
        let resp: TmdbSeasonResponse = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;

        Ok(resp.episodes.into_iter().map(|ep| {
            let dur = ep.runtime.map(|r| format!("{}m", r)).unwrap_or_else(|| "24m".to_string());
            TvEpisode {
                episode_number: ep.episode_number,
                name: ep.name.unwrap_or_else(|| format!("Episode {}", ep.episode_number)),
                overview: ep.overview.unwrap_or_default(),
                air_date: ep.air_date.unwrap_or_else(|| "—".to_string()),
                duration: dur,
            }
        }).collect())
    }

    pub async fn get_cached_poster(&self, poster_path: &str) -> Option<PathBuf> {
        if poster_path.is_empty() {
            return None;
        }
        let filename = poster_path.trim_start_matches('/');
        let local_path = get_cache_dir().join("posters").join(filename);
        if local_path.exists() && local_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Some(local_path);
        }

        let url = format!("https://image.tmdb.org/t/p/w342/{}", filename);
        if let Ok(resp) = self.client.get(&url).send().await {
            if let Ok(bytes) = resp.bytes().await {
                if std::fs::write(&local_path, bytes).is_ok() {
                    return Some(local_path);
                }
            }
        }
        None
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
