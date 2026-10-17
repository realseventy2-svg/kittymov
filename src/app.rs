use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::metadata::{MediaItem, TmdbClient, TvEpisode, TvSeason};
use crate::player::{MpvController, PlaybackStatus};
use crate::providers::{MediaRequest, ProviderManager, StreamCandidate};
use crate::storage::Database;
use crate::ui::poster::render_poster_to_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Movies,
    Series,
    Search,
    History,
    Bookmarks,
}

#[derive(Debug, Clone)]
pub struct ProviderOption {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct ProviderSelectModalState {
    pub target_title: String,
    pub request: MediaRequest,
    pub poster_path: String,
    pub options: Vec<ProviderOption>,
    pub selected_idx: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SubtitleSelectModalState {
    pub target_title: String,
    pub tmdb_id: i64,
    pub media_type: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub options: Vec<crate::subtitles::SubtitleLanguageGroup>,
    pub selected_idx: usize,
    pub is_loading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPane {
    Categories,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    General,
    Playback,
    Providers,
    Appearance,
    Storage,
}

impl SettingsCategory {
    pub const ALL: [SettingsCategory; 5] = [
        SettingsCategory::General,
        SettingsCategory::Playback,
        SettingsCategory::Providers,
        SettingsCategory::Appearance,
        SettingsCategory::Storage,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Playback => "Playback",
            SettingsCategory::Providers => "Providers",
            SettingsCategory::Appearance => "Appearance & UI",
            SettingsCategory::Storage => "Storage & Advanced",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SettingsCategory::General => "⚙",
            SettingsCategory::Playback => "▶",
            SettingsCategory::Providers => "🌐",
            SettingsCategory::Appearance => "🎨",
            SettingsCategory::Storage => "💾",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SettingsCategory::General => "Startup behavior, default tab, and player engine",
            SettingsCategory::Playback => "Audio mode, preferred languages, and video decoding",
            SettingsCategory::Providers => "Provider priority order and subtitle augmentation",
            SettingsCategory::Appearance => "Visual theme palette and poster art render mode",
            SettingsCategory::Storage => "Cache size, watch history database, and factory reset",
        }
    }

    pub fn item_count(&self) -> usize {
        match self {
            SettingsCategory::General => 4,
            SettingsCategory::Playback => 5,
            SettingsCategory::Providers => 3,
            SettingsCategory::Appearance => 3,
            SettingsCategory::Storage => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsConfirmation {
    ClearPosterCache,
    ClearWatchHistory,
    ResetDefaults,
}

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub active_pane: SettingsPane,
    pub category_idx: usize,
    pub item_idx: usize,
    pub search_query: String,
    pub is_searching: bool,
    pub confirmation: Option<SettingsConfirmation>,
    pub feedback_message: Option<String>,
}

impl SettingsModalState {
    pub fn current_category(&self) -> SettingsCategory {
        SettingsCategory::ALL[self.category_idx.min(SettingsCategory::ALL.len() - 1)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingMatch {
    pub category: SettingsCategory,
    pub category_idx: usize,
    pub item_idx: usize,
    pub label: &'static str,
    pub value_str: String,
    pub description: &'static str,
}

pub fn lang_name(code: &str) -> &'static str {
    match code.to_lowercase().as_str() {
        "eng" => "English",
        "jpn" => "Japanese",
        "spa" => "Spanish",
        "fra" => "French",
        "deu" => "German",
        "ita" => "Italian",
        "por" => "Portuguese",
        "rus" => "Russian",
        "kor" => "Korean",
        "zho" => "Chinese",
        _ => "Custom",
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn get_all_settings_metadata(cfg: &Config) -> Vec<SettingMatch> {
    vec![
        // General (0..4)
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 0,
            label: "Default Startup Tab",
            value_str: match cfg.default_tab.to_lowercase().as_str() {
                "series" | "tv" => "Series".to_string(),
                "history" => "History".to_string(),
                "bookmarks" => "Bookmarks".to_string(),
                _ => "Movies".to_string(),
            },
            description: "Default landing view presented on kitty-mov launch.",
        },
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 1,
            label: "Resume on Startup",
            value_str: if cfg.resume_on_startup { "ON".to_string() } else { "OFF".to_string() },
            description: "Automatically resume playback of your most recently watched title on launch.",
        },
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 2,
            label: "Player Engine",
            value_str: "mpv (External High-Performance Player)".to_string(),
            description: "Hardware-accelerated player engine powering video playback and subtitle sync.",
        },
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 3,
            label: "TMDB Metadata API",
            value_str: "Connected (v3 API Active)".to_string(),
            description: "Global metadata provider supplying artwork, cast details, episode titles, and summaries.",
        },

        // Playback (0..5)
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 0,
            label: "Audio Preference Mode",
            value_str: match cfg.audio_mode.to_lowercase().as_str() {
                "sub" => "Sub (Original Audio + Subtitles)".to_string(),
                _ => "Dub (English / Native Audio Track)".to_string(),
            },
            description: "Dub selects localized English audio; Sub selects original foreign audio with subtitles.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 1,
            label: "Preferred Audio Language",
            value_str: format!("{} ({})", cfg.audio_language, lang_name(&cfg.audio_language)),
            description: "Default voice language requested from streaming providers and selected in mpv.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 2,
            label: "Preferred Subtitle Language",
            value_str: format!("{} ({})", cfg.sub_language, lang_name(&cfg.sub_language)),
            description: "Default subtitle language automatically loaded and rendered on screen.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 3,
            label: "Auto-Play Next Episode",
            value_str: if cfg.auto_play_next { "ON (Seamless background prefetch)".to_string() } else { "OFF".to_string() },
            description: "Seamlessly start the next episode upon reaching the conclusion of current episode.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 4,
            label: "Hardware Decoding (hwdec)",
            value_str: match cfg.hwdec.as_str() {
                "auto" => "auto (MPV auto detection)".to_string(),
                "auto-safe" => "auto-safe (Safe hardware decoder)".to_string(),
                "vaapi" => "vaapi (Intel / AMD Linux GPU)".to_string(),
                "nvdec" => "nvdec (NVIDIA GPU)".to_string(),
                "videotoolbox" => "videotoolbox (Apple Silicon / macOS)".to_string(),
                "no" => "no (Software CPU decoding)".to_string(),
                other => other.to_string(),
            },
            description: "GPU video acceleration profile passed to mpv for efficient, stutter-free playback.",
        },

        // Providers (0..3)
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 0,
            label: "Preferred Provider",
            value_str: match cfg.preferred_provider.to_lowercase().as_str() {
                "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)".to_string(),
                "lookmovie" => "LookMovie First (Fallback CDN)".to_string(),
                _ => "Auto Fallback (VixSrc → LookMovie)".to_string(),
            },
            description: "Priority order for streaming servers. Auto Fallback tries VixSrc first, then LookMovie.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 1,
            label: "Subtitle Augmentation",
            value_str: if cfg.subtitle_augmentation { "ON (Auto-pull from LookMovie)".to_string() } else { "OFF".to_string() },
            description: "When enabled, queries LookMovie subtitle tracks if primary stream lacks requested subtitles.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 2,
            label: "Network Timeout",
            value_str: "10 seconds (Auto Failover)".to_string(),
            description: "Maximum duration kitty-mov waits for a provider response before failing over.",
        },

        // Appearance (0..3)
        SettingMatch {
            category: SettingsCategory::Appearance,
            category_idx: 3,
            item_idx: 0,
            label: "Color Theme Palette",
            value_str: match cfg.theme_name.as_str() {
                "amber" => "Warm Amber (Cinematic)".to_string(),
                "rose" => "Neon Rose (Cyberpunk)".to_string(),
                "emerald" => "Emerald Matrix (Green)".to_string(),
                _ => "Cyber Cyan (Default Glow)".to_string(),
            },
            description: "Color palette accents for UI borders, highlights, badges, and focus indicators.",
        },
        SettingMatch {
            category: SettingsCategory::Appearance,
            category_idx: 3,
            item_idx: 1,
            label: "Poster Art Render Mode",
            value_str: match cfg.poster_render_mode.as_str() {
                "off" => "Off (Text-Only Mode)".to_string(),
                _ => "Half-Block 24-bit ANSI (High Quality)".to_string(),
            },
            description: "Render high-resolution colored poster previews using Unicode half-blocks or text-only fallback.",
        },
        SettingMatch {
            category: SettingsCategory::Appearance,
            category_idx: 3,
            item_idx: 2,
            label: "Interface Layout",
            value_str: "Spacious Split-Pane (Terminal Adaptive)".to_string(),
            description: "Dual-pane layout with persistent navigation and contextual inspector.",
        },

        // Storage (0..4)
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 0,
            label: "Configuration File",
            value_str: crate::config::get_config_dir().join("config.json").to_string_lossy().to_string(),
            description: "Persistent JSON configuration location on your system.",
        },
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 1,
            label: "Poster Artwork Cache",
            value_str: "[ Clear Poster Cache ]".to_string(),
            description: "Delete locally cached movie and TV show poster thumbnails to reclaim disk space.",
        },
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 2,
            label: "Watch History Database",
            value_str: "[ Clear Watch History ]".to_string(),
            description: "Clear all playback timestamps, progress records, and watch history from SQLite.",
        },
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 3,
            label: "Factory Reset",
            value_str: "[ Reset All to Factory Defaults ]".to_string(),
            description: "Restore all configuration options to their original defaults without deleting history.",
        },
    ]
}

pub fn search_settings_matches(cfg: &Config, query: &str) -> Vec<SettingMatch> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let all = get_all_settings_metadata(cfg);
    all.into_iter()
        .filter(|m| {
            m.label.to_lowercase().contains(&q)
                || m.category.title().to_lowercase().contains(&q)
                || m.description.to_lowercase().contains(&q)
                || m.value_str.to_lowercase().contains(&q)
        })
        .collect()
}

pub struct EpisodeModalState {
    pub series: MediaItem,
    pub seasons: Vec<TvSeason>,
    pub selected_season_idx: usize,
    pub episodes: Vec<TvEpisode>,
    pub selected_episode_idx: usize,
    pub all_season_episodes: Option<HashMap<i32, Vec<TvEpisode>>>,
}

pub struct App {
    pub cfg: Config,
    pub db: Database,
    pub tmdb: Arc<TmdbClient>,
    pub providers: Arc<ProviderManager>,

    pub active_tab: ActiveTab,
    pub current_items: Vec<MediaItem>,
    pub selected_index: usize,

    pub search_query: String,
    pub is_searching: bool,
    pub status_message: String,

    pub current_poster_text: Option<ratatui::text::Text<'static>>,
    pub episode_modal: Option<EpisodeModalState>,
    pub provider_modal: Option<ProviderSelectModalState>,
    pub subtitle_modal: Option<SubtitleSelectModalState>,
    pub settings_modal: Option<SettingsModalState>,
    pub subtitles: Arc<crate::subtitles::SubtitleManager>,
    pub should_quit: bool,
}

impl App {
    pub fn new(cfg: Config, db: Database) -> Self {
        let tmdb = Arc::new(TmdbClient::new(&cfg.tmdb_api_key));
        let providers = Arc::new(ProviderManager::with_preference(&cfg.preferred_provider));
        let subtitles = Arc::new(crate::subtitles::SubtitleManager::new());

        Self {
            cfg,
            db,
            tmdb,
            providers,
            subtitles,
            active_tab: ActiveTab::Movies,
            current_items: Vec::new(),
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
            status_message: "Ready".to_string(),
            current_poster_text: None,
            episode_modal: None,
            provider_modal: None,
            subtitle_modal: None,
            settings_modal: None,
            should_quit: false,
        }
    }

    pub async fn init(&mut self) {
        if self.cfg.resume_on_startup {
            if let Ok(Some(latest)) = self.db.get_latest_watch() {
                self.status_message = format!("Resuming {}...", latest.title);
                if latest.media_type == "tv" {
                    self.play_tv_series(
                        latest.tmdb_id,
                        &latest.title,
                        &latest.year,
                        &latest.poster_path,
                        latest.season.unwrap_or(1),
                        latest.episode.unwrap_or(1),
                    ).await;
                    return;
                } else {
                    self.play_movie(crate::metadata::MediaItem {
                        tmdb_id: latest.tmdb_id,
                        media_type: latest.media_type,
                        title: latest.title,
                        year: latest.year,
                        rating: 0.0,
                        poster_path: latest.poster_path,
                        overview: format!("Last watched: {}", latest.last_watched),
                        genres: "Movie".to_string(),
                        duration_str: format!("{}m", latest.progress_seconds / 60),
                        status: "History".to_string(),
                        episodes_count: None,
                    }).await;
                    return;
                }
            }
        }

        let tab = match self.cfg.default_tab.to_lowercase().as_str() {
            "series" | "tv" => ActiveTab::Series,
            "history" => ActiveTab::History,
            "bookmarks" => ActiveTab::Bookmarks,
            _ => ActiveTab::Movies,
        };
        self.load_tab(tab).await;
    }

    pub async fn load_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        self.selected_index = 0;
        self.current_poster_text = None;

        match tab {
            ActiveTab::Movies => {
                self.status_message = "Fetching trending movies...".to_string();
                if let Ok(items) = self.tmdb.get_trending_movies().await {
                    self.current_items = items;
                    self.status_message = "Loaded trending movies".to_string();
                } else {
                    self.status_message = "Failed to load movies".to_string();
                }
            }
            ActiveTab::Series => {
                self.status_message = "Fetching trending TV series...".to_string();
                if let Ok(items) = self.tmdb.get_trending_tv().await {
                    self.current_items = items;
                    self.status_message = "Loaded trending series".to_string();
                } else {
                    self.status_message = "Failed to load series".to_string();
                }
            }
            ActiveTab::Search => {
                self.is_searching = true;
                self.status_message = "Type title and press Enter to search".to_string();
            }
            ActiveTab::History => {
                self.status_message = "Loading watch history...".to_string();
                if let Ok(hist) = self.db.get_history(50) {
                    self.current_items = hist.into_iter().map(|h| {
                        let is_tv = h.media_type == "tv";
                        let title = if is_tv {
                            format!("{} S{:02}E{:02}", h.title, h.season.unwrap_or(1), h.episode.unwrap_or(1))
                        } else {
                            h.title
                        };
                        MediaItem {
                            tmdb_id: h.tmdb_id,
                            media_type: h.media_type,
                            title,
                            year: h.year,
                            rating: 0.0,
                            poster_path: h.poster_path,
                            overview: format!("Last watched: {}", h.last_watched),
                            genres: if is_tv { "Series".to_string() } else { "Movie".to_string() },
                            duration_str: format!("{}m", h.progress_seconds / 60),
                            status: "History".to_string(),
                            episodes_count: None,
                        }
                    }).collect();
                    self.status_message = format!("Loaded {} history items", self.current_items.len());
                }
            }
            ActiveTab::Bookmarks => {
                self.status_message = "Loading bookmarks...".to_string();
                if let Ok(favs) = self.db.get_favorites() {
                    self.current_items = favs.into_iter().map(|f| MediaItem {
                        tmdb_id: f.tmdb_id,
                        media_type: f.media_type,
                        title: f.title,
                        year: f.year,
                        rating: f.rating,
                        poster_path: f.poster_path,
                        overview: f.overview,
                        genres: "Saved".to_string(),
                        duration_str: "Bookmark".to_string(),
                        status: "Saved".to_string(),
                        episodes_count: None,
                    }).collect();
                    self.status_message = format!("Loaded {} bookmarks", self.current_items.len());
                }
            }
        }
        self.update_poster().await;
    }

    pub async fn perform_search(&mut self) {
        if self.search_query.trim().is_empty() {
            return;
        }
        self.active_tab = ActiveTab::Search;
        self.is_searching = false;
        self.status_message = format!("Searching for '{}'...", self.search_query);

        if let Ok(items) = self.tmdb.search_multi(&self.search_query).await {
            self.current_items = items;
            self.selected_index = 0;
            self.status_message = format!("Found {} results", self.current_items.len());
            self.update_poster().await;
        } else {
            self.status_message = "Search failed".to_string();
        }
    }

    pub async fn update_poster(&mut self) {
        if let Some(item) = self.current_items.get(self.selected_index) {
            if !item.poster_path.is_empty() {
                if let Some(path) = self.tmdb.get_cached_poster(&item.poster_path).await {
                    self.current_poster_text = render_poster_to_text(&path, 24, 12);
                    return;
                }
            }
        }
        self.current_poster_text = None;
    }

    pub async fn select_next(&mut self) {
        if !self.current_items.is_empty() && self.selected_index + 1 < self.current_items.len() {
            self.selected_index += 1;
            self.update_poster().await;
        }
    }

    pub async fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_poster().await;
        }
    }

    pub async fn toggle_favorite(&mut self) {
        if let Some(item) = self.current_items.get(self.selected_index) {
            if let Ok(added) = self.db.toggle_favorite(
                item.tmdb_id,
                &item.media_type,
                &item.title,
                &item.year,
                &item.poster_path,
                item.rating,
                &item.overview,
            ) {
                self.status_message = if added {
                    format!("Saved '{}' to bookmarks", item.title)
                } else {
                    format!("Removed '{}' from bookmarks", item.title)
                };
            }
        }
    }

    pub async fn on_enter(&mut self) {
        if self.is_searching {
            self.perform_search().await;
            return;
        }

        if let Some(ref mut modal) = self.episode_modal {
            // Watch selected episode from modal
            if let Some(ep) = modal.episodes.get(modal.selected_episode_idx) {
                let season_num = modal.seasons.get(modal.selected_season_idx)
                    .map(|s| s.season_number)
                    .unwrap_or(1);
                let ep_num = ep.episode_number;
                let title = modal.series.title.clone();
                let year = modal.series.year.clone();
                let poster = modal.series.poster_path.clone();
                let tmdb_id = modal.series.tmdb_id;

                self.play_tv_series(tmdb_id, &title, &year, &poster, season_num, ep_num).await;
            }
            return;
        }

        if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            if item.media_type == "tv" {
                self.open_episode_modal(item).await;
            } else {
                self.play_movie(item).await;
            }
        }
    }

    pub async fn open_episode_modal(&mut self, item: MediaItem) {
        self.status_message = format!("Loading seasons for {}...", item.title);
        if let Ok((_details, seasons, stream_episodes_map)) = self.tmdb.get_tv_details(item.tmdb_id).await {
            let first_season = seasons.first().map(|s| s.season_number).unwrap_or(1);
            let episodes = if let Some(ref map) = stream_episodes_map {
                map.get(&first_season).cloned().unwrap_or_default()
            } else {
                self.tmdb.get_season_episodes(item.tmdb_id, first_season).await.unwrap_or_default()
            };

            self.episode_modal = Some(EpisodeModalState {
                series: item,
                seasons,
                selected_season_idx: 0,
                episodes,
                selected_episode_idx: 0,
                all_season_episodes: stream_episodes_map,
            });
            self.status_message = "Opened Episode Guide".to_string();
        } else {
            self.status_message = "Failed to load TV seasons".to_string();
        }
    }

    pub fn toggle_audio_mode(&mut self) {
        if self.cfg.audio_mode.to_lowercase() == "sub" {
            self.cfg.audio_mode = "dub".to_string();
            self.status_message = "Audio Mode: DUB (English Audio Track)".to_string();
        } else {
            self.cfg.audio_mode = "sub".to_string();
            self.status_message = "Audio Mode: SUB (Original Audio + Subtitles)".to_string();
        }
        let _ = self.cfg.save();
    }

    pub fn toggle_settings(&mut self) {
        if self.settings_modal.is_some() {
            self.save_settings();
            self.settings_modal = None;
            self.status_message = "Settings saved".to_string();
        } else {
            self.settings_modal = Some(SettingsModalState {
                active_pane: SettingsPane::Categories,
                category_idx: 0,
                item_idx: 0,
                search_query: String::new(),
                is_searching: false,
                confirmation: None,
                feedback_message: None,
            });
            self.status_message = "Settings Menu Opened".to_string();
        }
    }

    pub fn save_settings(&mut self) {
        let _ = self.cfg.save();
        self.providers = Arc::new(ProviderManager::with_preference(&self.cfg.preferred_provider));
    }

    pub fn settings_switch_pane(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            s.feedback_message = None;
            s.active_pane = match s.active_pane {
                SettingsPane::Categories => SettingsPane::Content,
                SettingsPane::Content => SettingsPane::Categories,
            };
        }
    }

    pub fn settings_next(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            s.feedback_message = None;
            if s.confirmation.is_some() {
                return;
            }
            if !s.search_query.trim().is_empty() {
                let matches = search_settings_matches(&self.cfg, &s.search_query);
                if !matches.is_empty() {
                    s.item_idx = (s.item_idx + 1) % matches.len();
                }
                return;
            }

            match s.active_pane {
                SettingsPane::Categories => {
                    s.category_idx = (s.category_idx + 1) % SettingsCategory::ALL.len();
                    s.item_idx = 0;
                }
                SettingsPane::Content => {
                    let cat = SettingsCategory::ALL[s.category_idx];
                    let count = cat.item_count();
                    if count > 0 {
                        s.item_idx = (s.item_idx + 1) % count;
                    }
                }
            }
        }
    }

    pub fn settings_prev(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            s.feedback_message = None;
            if s.confirmation.is_some() {
                return;
            }
            if !s.search_query.trim().is_empty() {
                let matches = search_settings_matches(&self.cfg, &s.search_query);
                if !matches.is_empty() {
                    s.item_idx = (s.item_idx + matches.len() - 1) % matches.len();
                }
                return;
            }

            match s.active_pane {
                SettingsPane::Categories => {
                    s.category_idx = (s.category_idx + SettingsCategory::ALL.len() - 1) % SettingsCategory::ALL.len();
                    s.item_idx = 0;
                }
                SettingsPane::Content => {
                    let cat = SettingsCategory::ALL[s.category_idx];
                    let count = cat.item_count();
                    if count > 0 {
                        s.item_idx = (s.item_idx + count - 1) % count;
                    }
                }
            }
        }
    }

    pub fn settings_change_value(&mut self, forward: bool) {
        if let Some(ref mut s) = self.settings_modal {
            s.feedback_message = None;
            if s.confirmation.is_some() {
                return;
            }
            if s.active_pane == SettingsPane::Categories {
                s.active_pane = SettingsPane::Content;
                return;
            }

            if !s.search_query.trim().is_empty() {
                let matches = search_settings_matches(&self.cfg, &s.search_query);
                if let Some(m) = matches.get(s.item_idx) {
                    s.category_idx = m.category_idx;
                    s.item_idx = m.item_idx;
                    s.search_query.clear();
                    s.is_searching = false;
                    s.active_pane = SettingsPane::Content;
                }
                return;
            }

            let cat = SettingsCategory::ALL[s.category_idx];
            match cat {
                SettingsCategory::General => match s.item_idx {
                    0 => {
                        let tabs = ["movies", "series", "history", "bookmarks"];
                        self.cfg.default_tab = cycle_list(&tabs, &self.cfg.default_tab, forward);
                    }
                    1 => {
                        self.cfg.resume_on_startup = !self.cfg.resume_on_startup;
                    }
                    _ => {}
                },
                SettingsCategory::Playback => match s.item_idx {
                    0 => {
                        self.cfg.audio_mode = if self.cfg.audio_mode.to_lowercase() == "sub" {
                            "dub".to_string()
                        } else {
                            "sub".to_string()
                        };
                    }
                    1 => {
                        let langs = ["eng", "jpn", "spa", "fra", "deu", "ita", "por", "rus", "kor", "zho"];
                        self.cfg.audio_language = cycle_list(&langs, &self.cfg.audio_language, forward);
                    }
                    2 => {
                        let langs = ["eng", "spa", "fra", "deu", "ita", "jpn", "por", "rus", "kor", "zho"];
                        self.cfg.sub_language = cycle_list(&langs, &self.cfg.sub_language, forward);
                    }
                    3 => {
                        self.cfg.auto_play_next = !self.cfg.auto_play_next;
                    }
                    4 => {
                        let hwdecs = ["auto", "auto-safe", "vaapi", "nvdec", "videotoolbox", "no"];
                        self.cfg.hwdec = cycle_list(&hwdecs, &self.cfg.hwdec, forward);
                    }
                    _ => {}
                },
                SettingsCategory::Providers => match s.item_idx {
                    0 => {
                        let providers = ["auto", "vixsrc", "lookmovie"];
                        self.cfg.preferred_provider = cycle_list(&providers, &self.cfg.preferred_provider, forward);
                    }
                    1 => {
                        self.cfg.subtitle_augmentation = !self.cfg.subtitle_augmentation;
                    }
                    _ => {}
                },
                SettingsCategory::Appearance => match s.item_idx {
                    0 => {
                        let themes = ["cyber", "amber", "rose", "emerald"];
                        self.cfg.theme_name = cycle_list(&themes, &self.cfg.theme_name, forward);
                    }
                    1 => {
                        let modes = ["half-block", "off"];
                        self.cfg.poster_render_mode = cycle_list(&modes, &self.cfg.poster_render_mode, forward);
                    }
                    _ => {}
                },
                SettingsCategory::Storage => match s.item_idx {
                    1 => {
                        s.confirmation = Some(SettingsConfirmation::ClearPosterCache);
                    }
                    2 => {
                        s.confirmation = Some(SettingsConfirmation::ClearWatchHistory);
                    }
                    3 => {
                        s.confirmation = Some(SettingsConfirmation::ResetDefaults);
                    }
                    _ => {}
                },
            }
            let _ = self.cfg.save();
            self.providers = Arc::new(ProviderManager::with_preference(&self.cfg.preferred_provider));
        }
    }

    pub fn settings_execute_confirmation(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            if let Some(ref conf) = s.confirmation.clone() {
                match conf {
                    SettingsConfirmation::ClearPosterCache => {
                        match crate::config::clear_poster_cache() {
                            Ok(count) => {
                                s.feedback_message = Some(format!("Poster cache cleared ({} files removed)", count));
                            }
                            Err(e) => {
                                s.feedback_message = Some(format!("Failed to clear cache: {}", e));
                            }
                        }
                    }
                    SettingsConfirmation::ClearWatchHistory => {
                        match self.db.clear_history() {
                            Ok(count) => {
                                s.feedback_message = Some(format!("Watch history cleared ({} records removed)", count));
                            }
                            Err(e) => {
                                s.feedback_message = Some(format!("Failed to clear history: {}", e));
                            }
                        }
                    }
                    SettingsConfirmation::ResetDefaults => {
                        self.cfg.reset_to_defaults();
                        let _ = self.cfg.save();
                        self.providers = Arc::new(ProviderManager::with_preference(&self.cfg.preferred_provider));
                        s.feedback_message = Some("All settings restored to factory defaults".to_string());
                    }
                }
                s.confirmation = None;
            }
        }
    }

    pub fn settings_cancel_confirmation(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            s.confirmation = None;
            s.feedback_message = Some("Action cancelled".to_string());
        }
    }

    pub fn settings_reset_current_category(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            let cat = SettingsCategory::ALL[s.category_idx];
            match cat {
                SettingsCategory::General => {
                    self.cfg.default_tab = "movies".to_string();
                    self.cfg.resume_on_startup = false;
                }
                SettingsCategory::Playback => {
                    self.cfg.audio_mode = "sub".to_string();
                    self.cfg.audio_language = "eng".to_string();
                    self.cfg.sub_language = "eng".to_string();
                    self.cfg.auto_play_next = true;
                    self.cfg.hwdec = "auto".to_string();
                }
                SettingsCategory::Providers => {
                    self.cfg.preferred_provider = "auto".to_string();
                    self.cfg.subtitle_augmentation = true;
                }
                SettingsCategory::Appearance => {
                    self.cfg.theme_name = "cyber".to_string();
                    self.cfg.poster_render_mode = "half-block".to_string();
                }
                SettingsCategory::Storage => {
                    s.confirmation = Some(SettingsConfirmation::ResetDefaults);
                    return;
                }
            }
            let _ = self.cfg.save();
            self.providers = Arc::new(ProviderManager::with_preference(&self.cfg.preferred_provider));
            s.feedback_message = Some(format!("Reset {} settings to defaults", cat.title()));
        }
    }

    pub fn open_provider_modal_for_selected(&mut self) {
        if let Some(ref modal) = self.episode_modal {
            if let Some(ep) = modal.episodes.get(modal.selected_episode_idx) {
                let season_num = modal.seasons.get(modal.selected_season_idx)
                    .map(|s| s.season_number)
                    .unwrap_or(1);
                let ep_num = ep.episode_number;
                let title = modal.series.title.clone();
                let year = modal.series.year.clone();
                let poster = modal.series.poster_path.clone();
                let tmdb_id = modal.series.tmdb_id;

                let target_title = format!("{} S{:02}E{:02} — {}", title, season_num, ep_num, ep.name);
                let request = MediaRequest {
                    tmdb_id,
                    media_type: "tv".to_string(),
                    season: Some(season_num),
                    episode: Some(ep_num),
                    title,
                    year: Some(year),
                };

                self.provider_modal = Some(ProviderSelectModalState {
                    target_title,
                    request,
                    poster_path: poster,
                    options: default_provider_options(),
                    selected_idx: 0,
                });
                return;
            }
        }

        if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            if item.media_type == "tv" {
                let target_title = format!("{} S01E01", item.title);
                let request = MediaRequest {
                    tmdb_id: item.tmdb_id,
                    media_type: "tv".to_string(),
                    season: Some(1),
                    episode: Some(1),
                    title: item.title.clone(),
                    year: Some(item.year.clone()),
                };
                self.provider_modal = Some(ProviderSelectModalState {
                    target_title,
                    request,
                    poster_path: item.poster_path,
                    options: default_provider_options(),
                    selected_idx: 0,
                });
            } else {
                let target_title = format!("{} ({})", item.title, item.year);
                let request = MediaRequest {
                    tmdb_id: item.tmdb_id,
                    media_type: "movie".to_string(),
                    season: None,
                    episode: None,
                    title: item.title.clone(),
                    year: Some(item.year.clone()),
                };
                self.provider_modal = Some(ProviderSelectModalState {
                    target_title,
                    request,
                    poster_path: item.poster_path,
                    options: default_provider_options(),
                    selected_idx: 0,
                });
            }
        }
    }

    pub fn provider_modal_next(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            if m.selected_idx + 1 < m.options.len() {
                m.selected_idx += 1;
            } else {
                m.selected_idx = 0;
            }
        }
    }

    pub fn provider_modal_prev(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            if m.selected_idx > 0 {
                m.selected_idx -= 1;
            } else {
                m.selected_idx = m.options.len().saturating_sub(1);
            }
        }
    }

    pub async fn provider_modal_confirm(&mut self) {
        if let Some(modal) = self.provider_modal.take() {
            let choice = modal.options.get(modal.selected_idx)
                .map(|o| o.id.clone())
                .unwrap_or_else(|| "auto".to_string());
            let req = modal.request;
            let poster = modal.poster_path;

            if req.media_type == "tv" {
                let season = req.season.unwrap_or(1);
                let episode = req.episode.unwrap_or(1);
                self.play_tv_series_with_provider(
                    req.tmdb_id,
                    &req.title,
                    req.year.as_deref().unwrap_or(""),
                    &poster,
                    season,
                    episode,
                    &choice,
                ).await;
            } else {
                self.play_movie_with_provider(
                    req.tmdb_id,
                    &req.title,
                    req.year.as_deref().unwrap_or(""),
                    &poster,
                    &choice,
                ).await;
            }
        }
    }

    pub async fn open_subtitle_modal(&mut self) {
        let (tmdb_id, media_type, season, episode, target_title) = if let Some(ref modal) = self.episode_modal {
            if let Some(ep) = modal.episodes.get(modal.selected_episode_idx) {
                let season_num = modal.seasons.get(modal.selected_season_idx)
                    .map(|s| s.season_number)
                    .unwrap_or(1);
                let ep_num = ep.episode_number;
                let title = modal.series.title.clone();
                (modal.series.tmdb_id, "tv".to_string(), Some(season_num), Some(ep_num), format!("{} S{:02}E{:02}", title, season_num, ep_num))
            } else {
                return;
            }
        } else if let Some(ref modal) = self.provider_modal {
            let req = &modal.request;
            (req.tmdb_id, req.media_type.clone(), req.season, req.episode, modal.target_title.clone())
        } else if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            if item.media_type == "tv" {
                (item.tmdb_id, "tv".to_string(), Some(1), Some(1), format!("{} S01E01", item.title))
            } else {
                (item.tmdb_id, "movie".to_string(), None, None, format!("{} ({})", item.title, item.year))
            }
        } else {
            return;
        };

        self.status_message = format!("Searching OpenSubtitles for {}...", target_title);
        let imdb_id = self.tmdb.get_imdb_id(tmdb_id, &media_type).await.ok();
        let tracks = self.subtitles.fetch_all_subtitles(imdb_id.as_deref(), &media_type, season, episode).await;
        let groups = crate::subtitles::SubtitleManager::group_by_language(&tracks);

        let selected_idx = groups.iter().position(|g| g.lang_code == self.cfg.sub_language).unwrap_or(0);

        self.subtitle_modal = Some(SubtitleSelectModalState {
            target_title,
            tmdb_id,
            media_type,
            season,
            episode,
            options: groups,
            selected_idx,
            is_loading: false,
        });
    }

    pub fn subtitle_modal_next(&mut self) {
        if let Some(ref mut m) = self.subtitle_modal {
            if !m.options.is_empty() {
                m.selected_idx = (m.selected_idx + 1) % m.options.len();
            }
        }
    }

    pub fn subtitle_modal_prev(&mut self) {
        if let Some(ref mut m) = self.subtitle_modal {
            if !m.options.is_empty() {
                m.selected_idx = (m.selected_idx + m.options.len() - 1) % m.options.len();
            }
        }
    }

    pub fn subtitle_modal_confirm(&mut self) {
        if let Some(modal) = self.subtitle_modal.take() {
            if let Some(opt) = modal.options.get(modal.selected_idx) {
                self.cfg.sub_language = opt.lang_code.clone();
                let _ = self.cfg.save();
                self.status_message = format!("Subtitle Language set to: {} [{}]", opt.lang_name, opt.lang_code);
            }
        }
    }

    pub async fn play_movie(&mut self, item: MediaItem) {
        let choice = self.cfg.preferred_provider.clone();
        self.play_movie_with_provider(item.tmdb_id, &item.title, &item.year, &item.poster_path, &choice).await;
    }

    pub async fn play_movie_with_provider(
        &mut self,
        tmdb_id: i64,
        title: &str,
        year: &str,
        poster_path: &str,
        provider_choice: &str,
    ) {
        let display_name = match provider_choice {
            "vixsrc" => "VixSrc Server",
            "lookmovie" => "LookMovie Server",
            _ => "Auto Fallback",
        };
        self.status_message = format!("Resolving '{}' on {} [{}]...", title, display_name, self.cfg.audio_mode.to_uppercase());

        let req = MediaRequest {
            tmdb_id,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: title.to_string(),
            year: Some(year.to_string()),
        };

        match self.providers.resolve_with_choice(provider_choice, &req).await {
            Ok(mut stream) => {
                // Augment with dedicated Subtitle Provider (OpenSubtitles)
                if stream.subtitles.is_empty() || self.cfg.subtitle_augmentation {
                    self.status_message = format!("Fetching subtitles for '{}' [{}]...", title, self.cfg.sub_language);
                    let external_subs = self.subtitles.prepare_subtitles(
                        &self.tmdb,
                        tmdb_id,
                        title,
                        "movie",
                        None,
                        None,
                        &self.cfg.sub_language,
                    ).await;
                    if !external_subs.is_empty() {
                        stream.subtitles = external_subs;
                    }
                }

                let _ = self.db.record_watch(
                    tmdb_id,
                    "movie",
                    title,
                    year,
                    poster_path,
                    None,
                    None,
                    None,
                    0,
                    0,
                );

                self.launch_mpv_session(&stream, title).await;
            }
            Err(e) => {
                self.status_message = format!("Stream error ({}): {}", display_name, e);
            }
        }
    }

    pub async fn play_tv_series(
        &mut self,
        tmdb_id: i64,
        title: &str,
        year: &str,
        poster_path: &str,
        season: i32,
        episode: i32,
    ) {
        let choice = self.cfg.preferred_provider.clone();
        self.play_tv_series_with_provider(tmdb_id, title, year, poster_path, season, episode, &choice).await;
    }

    pub async fn play_tv_series_with_provider(
        &mut self,
        tmdb_id: i64,
        title: &str,
        year: &str,
        poster_path: &str,
        season: i32,
        episode: i32,
        provider_choice: &str,
    ) {
        let display_title = format!("{} S{:02}E{:02}", title, season, episode);
        let display_name = match provider_choice {
            "vixsrc" => "VixSrc Server",
            "lookmovie" => "LookMovie Server",
            _ => "Auto Fallback",
        };
        self.status_message = format!("Resolving {} on {} [{}]...", display_title, display_name, self.cfg.audio_mode.to_uppercase());

        let req = MediaRequest {
            tmdb_id,
            media_type: "tv".to_string(),
            season: Some(season),
            episode: Some(episode),
            title: title.to_string(),
            year: Some(year.to_string()),
        };

        match self.providers.resolve_with_choice(provider_choice, &req).await {
            Ok(mut stream) => {
                // Augment with dedicated Subtitle Provider (OpenSubtitles)
                if stream.subtitles.is_empty() || self.cfg.subtitle_augmentation {
                    self.status_message = format!("Fetching subtitles for {} [{}]...", display_title, self.cfg.sub_language);
                    let external_subs = self.subtitles.prepare_subtitles(
                        &self.tmdb,
                        tmdb_id,
                        title,
                        "tv",
                        Some(season),
                        Some(episode),
                        &self.cfg.sub_language,
                    ).await;
                    if !external_subs.is_empty() {
                        stream.subtitles = external_subs;
                    }
                }

                let _ = self.db.record_watch(
                    tmdb_id,
                    "tv",
                    title,
                    year,
                    poster_path,
                    Some(season),
                    Some(episode),
                    None,
                    0,
                    0,
                );

                // Check anti-expiration
                if stream.is_expired_or_expiring_soon() {
                    let _ = self.providers.refresh_candidate(&mut stream).await;
                }

                self.launch_mpv_tv_auto_next(tmdb_id, title, year, poster_path, season, episode, stream).await;
            }
            Err(e) => {
                self.status_message = format!("Stream error ({}): {}", display_name, e);
            }
        }
    }

    async fn launch_mpv_session(&mut self, stream: &StreamCandidate, title: &str) {
        // Temporarily release terminal for MPV
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();

        let mut controller = MpvController::new();
        if let Ok(()) = controller.start_player(stream, title, &self.cfg) {
            let _ = controller.wait_sync();
        }

        // Restore terminal for Ratatui
        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();
        self.status_message = format!("Finished watching '{}' [{}]", title, stream.provider_name);
    }

    /// Full Auto-Play Next Episode Engine with background prefetching and JIT token refreshing
    async fn launch_mpv_tv_auto_next(
        &mut self,
        tmdb_id: i64,
        title: &str,
        year: &str,
        poster_path: &str,
        current_season: i32,
        mut current_episode: i32,
        initial_stream: StreamCandidate,
    ) {
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();

        let mut controller = MpvController::new();
        let display_title = if let Some(ref act) = initial_stream.actual_title {
            format!("{} — {}", title, act)
        } else {
            format!("{} S{:02}E{:02}", title, current_season, current_episode)
        };

        if let Ok(()) = controller.start_player(&initial_stream, &display_title, &self.cfg) {
            let (status_tx, mut status_rx) = mpsc::channel(32);

            if let Ok(socket) = controller.wait_for_socket(Duration::from_secs(4)).await {
                controller.monitor_ipc(socket, status_tx);

                let mut prefetch_started = false;

                while let Some(status) = status_rx.recv().await {
                    match status {
                        PlaybackStatus::Playing { pos, duration } => {
                            // When progress reaches 80% (or last 3 minutes), start pre-resolving next episode
                            if self.cfg.auto_play_next && !prefetch_started && (duration > 0.0 && pos / duration >= 0.8 || pos >= 1200.0) {
                                prefetch_started = true;
                                let providers = self.providers.clone();
                                let next_req = MediaRequest {
                                    tmdb_id,
                                    media_type: "tv".to_string(),
                                    season: Some(current_season),
                                    episode: Some(current_episode + 1),
                                    title: title.to_string(),
                                    year: Some(year.to_string()),
                                };

                                tokio::spawn(async move {
                                    providers.resolve_with_fallback(&next_req).await
                                });
                            }
                        }
                        PlaybackStatus::EofReached => {
                            if self.cfg.auto_play_next {
                                current_episode += 1;
                                let next_req = MediaRequest {
                                    tmdb_id,
                                    media_type: "tv".to_string(),
                                    season: Some(current_season),
                                    episode: Some(current_episode),
                                    title: title.to_string(),
                                    year: Some(year.to_string()),
                                };

                                if let Ok(mut next_stream) = self.providers.resolve_with_fallback(&next_req).await {
                                    // JIT token refresh to ensure no expired links
                                    if next_stream.is_expired_or_expiring_soon() {
                                        let _ = self.providers.refresh_candidate(&mut next_stream).await;
                                    }
                                    if next_stream.subtitles.is_empty() || self.cfg.subtitle_augmentation {
                                        let external_subs = self.subtitles.prepare_subtitles(
                                            &self.tmdb,
                                            tmdb_id,
                                            title,
                                            "tv",
                                            Some(current_season),
                                            Some(current_episode),
                                            &self.cfg.sub_language,
                                        ).await;
                                        if !external_subs.is_empty() {
                                            next_stream.subtitles = external_subs;
                                        }
                                    }

                                    // Seamlessly inject next episode into running MPV window!
                                    if let Some(ref embed) = next_stream.embed_url {
                                        let _ = controller.send_command(&["set_property", "referrer", embed]).await;
                                    }
                                    let next_title = if let Some(ref act) = next_stream.actual_title {
                                        format!("{} — {}", title, act)
                                    } else {
                                        format!("{} S{:02}E{:02}", title, current_season, current_episode)
                                    };
                                    let _ = controller.send_command(&["set_property", "title", &format!("KittyMov: {} [{}]", next_title, next_stream.provider_name)]).await;
                                    for sub in &next_stream.subtitles {
                                        let _ = controller.send_command(&["sub-add", sub, "auto"]).await;
                                    }
                                    let _ = controller.send_command(&["loadfile", &next_stream.stream_url, "replace"]).await;

                                    let _ = self.db.record_watch(
                                        tmdb_id,
                                        "tv",
                                        title,
                                        year,
                                        poster_path,
                                        Some(current_season),
                                        Some(current_episode),
                                        None,
                                        0,
                                        0,
                                    );
                                    prefetch_started = false;
                                    continue;
                                }
                            }
                            break;
                        }
                        PlaybackStatus::UserQuit => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
            let _ = controller.wait_sync();
        }

        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();
        self.status_message = format!("Returned from watching {} S{:02}E{:02}", title, current_season, current_episode);
    }
}

fn default_provider_options() -> Vec<ProviderOption> {
    vec![
        ProviderOption {
            id: "auto".to_string(),
            name: "Auto Fallback Engine".to_string(),
            description: "Fast multi-provider chain (VixSrc CDN → LookMovie CDN)".to_string(),
        },
        ProviderOption {
            id: "vixsrc".to_string(),
            name: "VixSrc Server".to_string(),
            description: "Primary Ultra-Fast CDN • 1080p FHD HLS • Multi-Audio".to_string(),
        },
        ProviderOption {
            id: "lookmovie".to_string(),
            name: "LookMovie Server".to_string(),
            description: "High-Reliability Fallback CDN • 1080p/720p/480p • Multi-Subtitles".to_string(),
        },
    ]
}

fn cycle_list(list: &[&str], current: &str, forward: bool) -> String {
    let current_lower = current.to_lowercase();
    let idx = list.iter().position(|&x| x == current_lower).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % list.len()
    } else {
        (idx + list.len() - 1) % list.len()
    };
    list[next_idx].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_list_forward_and_backward() {
        let langs = ["eng", "jpn", "spa"];
        assert_eq!(cycle_list(&langs, "eng", true), "jpn");
        assert_eq!(cycle_list(&langs, "jpn", true), "spa");
        assert_eq!(cycle_list(&langs, "spa", true), "eng");

        assert_eq!(cycle_list(&langs, "eng", false), "spa");
        assert_eq!(cycle_list(&langs, "spa", false), "jpn");
    }

    #[test]
    fn test_settings_categories_and_counts() {
        assert_eq!(SettingsCategory::ALL.len(), 5);
        assert_eq!(SettingsCategory::General.item_count(), 4);
        assert_eq!(SettingsCategory::Playback.item_count(), 5);
        assert_eq!(SettingsCategory::Providers.item_count(), 3);
        assert_eq!(SettingsCategory::Appearance.item_count(), 3);
        assert_eq!(SettingsCategory::Storage.item_count(), 4);
    }

    #[test]
    fn test_settings_pane_switching_and_navigation() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);

        assert!(app.settings_modal.is_none());
        app.toggle_settings();
        assert!(app.settings_modal.is_some());

        // Initially on Categories pane
        let sm = app.settings_modal.as_ref().unwrap();
        assert_eq!(sm.active_pane, SettingsPane::Categories);
        assert_eq!(sm.category_idx, 0);
        assert_eq!(sm.item_idx, 0);

        // Switch pane to Content
        app.settings_switch_pane();
        assert_eq!(app.settings_modal.as_ref().unwrap().active_pane, SettingsPane::Content);

        // Switch back to Categories
        app.settings_switch_pane();
        assert_eq!(app.settings_modal.as_ref().unwrap().active_pane, SettingsPane::Categories);

        // Category navigation wraps around
        app.settings_next();
        assert_eq!(app.settings_modal.as_ref().unwrap().category_idx, 1);
        app.settings_prev();
        assert_eq!(app.settings_modal.as_ref().unwrap().category_idx, 0);
        app.settings_prev();
        assert_eq!(app.settings_modal.as_ref().unwrap().category_idx, 4);

        // Switch to Content pane and navigate items
        app.settings_switch_pane();
        assert_eq!(app.settings_modal.as_ref().unwrap().item_idx, 0);
        app.settings_next();
        assert_eq!(app.settings_modal.as_ref().unwrap().item_idx, 1);
        app.settings_prev();
        assert_eq!(app.settings_modal.as_ref().unwrap().item_idx, 0);
        app.settings_prev();
        assert_eq!(app.settings_modal.as_ref().unwrap().item_idx, 3); // Storage has 4 items (0..3)
    }

    #[test]
    fn test_settings_value_changes() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);
        app.toggle_settings();

        // General Category (category_idx = 0)
        app.settings_modal.as_mut().unwrap().active_pane = SettingsPane::Content;
        app.settings_modal.as_mut().unwrap().item_idx = 0; // Default Tab
        let initial_tab = app.cfg.default_tab.clone();
        app.settings_change_value(true);
        assert_ne!(app.cfg.default_tab, initial_tab);

        app.settings_modal.as_mut().unwrap().item_idx = 1; // Resume on startup
        assert_eq!(app.cfg.resume_on_startup, false);
        app.settings_change_value(true);
        assert_eq!(app.cfg.resume_on_startup, true);

        // Playback Category (category_idx = 1)
        app.settings_modal.as_mut().unwrap().category_idx = 1;
        app.settings_modal.as_mut().unwrap().item_idx = 0; // Audio mode
        let initial_mode = app.cfg.audio_mode.clone();
        app.settings_change_value(true);
        assert_ne!(app.cfg.audio_mode, initial_mode);

        app.settings_modal.as_mut().unwrap().item_idx = 3; // Auto-play next
        let initial_auto = app.cfg.auto_play_next;
        app.settings_change_value(true);
        assert_eq!(app.cfg.auto_play_next, !initial_auto);
    }

    #[test]
    fn test_settings_category_reset() {
        let mut cfg = Config::default();
        cfg.default_tab = "history".to_string();
        cfg.resume_on_startup = true;

        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);
        app.toggle_settings();

        // Reset General Category
        app.settings_modal.as_mut().unwrap().category_idx = 0;
        app.settings_reset_current_category();

        assert_eq!(app.cfg.default_tab, "movies");
        assert_eq!(app.cfg.resume_on_startup, false);
        assert!(app.settings_modal.as_ref().unwrap().feedback_message.is_some());
    }

    #[test]
    fn test_settings_search_filtering() {
        let cfg = Config::default();

        let sub_matches = search_settings_matches(&cfg, "sub");
        assert!(!sub_matches.is_empty());
        assert!(sub_matches.iter().any(|m| m.label.to_lowercase().contains("subtitle")));

        let theme_matches = search_settings_matches(&cfg, "theme");
        assert!(!theme_matches.is_empty());
        assert_eq!(theme_matches[0].category, SettingsCategory::Appearance);

        let empty_matches = search_settings_matches(&cfg, "nonexistentquery12345");
        assert!(empty_matches.is_empty());
    }

    #[test]
    fn test_settings_confirmation_workflow() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);
        app.toggle_settings();

        // Go to Storage -> Reset Defaults
        app.settings_modal.as_mut().unwrap().category_idx = 4;
        app.settings_modal.as_mut().unwrap().active_pane = SettingsPane::Content;
        app.settings_modal.as_mut().unwrap().item_idx = 3; // Reset Defaults action

        app.settings_change_value(true);
        assert_eq!(
            app.settings_modal.as_ref().unwrap().confirmation,
            Some(SettingsConfirmation::ResetDefaults)
        );

        // Cancel confirmation
        app.settings_cancel_confirmation();
        assert_eq!(app.settings_modal.as_ref().unwrap().confirmation, None);

        // Trigger again and execute
        app.settings_change_value(true);
        assert!(app.settings_modal.as_ref().unwrap().confirmation.is_some());
        app.settings_execute_confirmation();
        assert_eq!(app.settings_modal.as_ref().unwrap().confirmation, None);
        assert!(app.settings_modal.as_ref().unwrap().feedback_message.is_some());
    }
}

