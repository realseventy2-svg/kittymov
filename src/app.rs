use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::metadata::{MediaItem, TmdbClient, TvEpisode, TvSeason};
use crate::player::{MpvController, PlaybackStatus};
use crate::providers::vixsrc::VixSrcProvider;
use crate::providers::{MediaRequest, Provider, StreamCandidate};
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
    pub vixsrc: Arc<VixSrcProvider>,

    pub active_tab: ActiveTab,
    pub current_items: Vec<MediaItem>,
    pub selected_index: usize,

    pub search_query: String,
    pub is_searching: bool,
    pub status_message: String,

    pub current_poster_text: Option<ratatui::text::Text<'static>>,
    pub episode_modal: Option<EpisodeModalState>,
    pub should_quit: bool,
}

impl App {
    pub fn new(cfg: Config, db: Database) -> Self {
        let tmdb = Arc::new(TmdbClient::new(&cfg.tmdb_api_key));
        let vixsrc = Arc::new(VixSrcProvider::new());

        Self {
            cfg,
            db,
            tmdb,
            vixsrc,
            active_tab: ActiveTab::Movies,
            current_items: Vec::new(),
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
            status_message: "Ready".to_string(),
            current_poster_text: None,
            episode_modal: None,
            should_quit: false,
        }
    }

    pub async fn init(&mut self) {
        self.load_tab(ActiveTab::Movies).await;
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

    pub async fn play_movie(&mut self, item: MediaItem) {
        self.status_message = format!("Resolving stream for '{}'...", item.title);

        let req = MediaRequest {
            tmdb_id: item.tmdb_id,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: item.title.clone(),
        };

        match self.vixsrc.resolve(&req).await {
            Ok(stream) => {
                let _ = self.db.record_watch(
                    item.tmdb_id,
                    "movie",
                    &item.title,
                    &item.year,
                    &item.poster_path,
                    None,
                    None,
                    None,
                    0,
                    0,
                );

                self.launch_mpv_session(&stream, &item.title).await;
            }
            Err(e) => {
                self.status_message = format!("Stream error: {}", e);
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
        let display_title = format!("{} S{:02}E{:02}", title, season, episode);
        self.status_message = format!("Resolving stream for {}...", display_title);

        let req = MediaRequest {
            tmdb_id,
            media_type: "tv".to_string(),
            season: Some(season),
            episode: Some(episode),
            title: display_title.clone(),
        };

        match self.vixsrc.resolve(&req).await {
            Ok(mut stream) => {
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
                    let _ = self.vixsrc.refresh_token(&mut stream).await;
                }

                self.launch_mpv_tv_auto_next(tmdb_id, title, year, poster_path, season, episode, stream).await;
            }
            Err(e) => {
                self.status_message = format!("Stream error: {}", e);
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
        self.status_message = format!("Finished watching '{}'", title);
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
                                let vixsrc = self.vixsrc.clone();
                                let next_req = MediaRequest {
                                    tmdb_id,
                                    media_type: "tv".to_string(),
                                    season: Some(current_season),
                                    episode: Some(current_episode + 1),
                                    title: format!("{} S{:02}E{:02}", title, current_season, current_episode + 1),
                                };

                                tokio::spawn(async move {
                                    vixsrc.resolve(&next_req).await
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
                                    title: format!("{} S{:02}E{:02}", title, current_season, current_episode),
                                };

                                if let Ok(mut next_stream) = self.vixsrc.resolve(&next_req).await {
                                    // JIT token refresh to ensure no expired links
                                    if next_stream.is_expired_or_expiring_soon() {
                                        let _ = self.vixsrc.refresh_token(&mut next_stream).await;
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
                                    let _ = controller.send_command(&["set_property", "title", &format!("KittyMov: {}", next_title)]).await;
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
