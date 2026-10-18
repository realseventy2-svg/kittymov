use crate::metadata::MediaItem;
use crate::providers::MediaRequest;
use crate::ui::poster::{calculate_poster_dimensions_for_panel, render_poster_to_text};

use super::state::ActiveTab;
use super::App;

impl App {
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
                    self.current_items = hist
                        .into_iter()
                        .map(|h| {
                            let is_tv = h.media_type == "tv";
                            let title = if is_tv {
                                format!(
                                    "{} S{:02}E{:02}",
                                    h.title,
                                    h.season.unwrap_or(1),
                                    h.episode.unwrap_or(1)
                                )
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
                                genres: if is_tv {
                                    "Series".to_string()
                                } else {
                                    "Movie".to_string()
                                },
                                duration_str: format!("{}m", h.progress_seconds / 60),
                                status: "History".to_string(),
                                episodes_count: None,
                            }
                        })
                        .collect();
                    self.status_message =
                        format!("Loaded {} history items", self.current_items.len());
                }
            }
            ActiveTab::Bookmarks => {
                self.status_message = "Loading bookmarks...".to_string();
                if let Ok(favs) = self.db.get_favorites() {
                    self.current_items = favs
                        .into_iter()
                        .map(|f| MediaItem {
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
                        })
                        .collect();
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
        if self.cfg.poster_render_mode == "off" {
            self.current_poster_text = None;
            self.current_poster_kitty = None;
            self.kitty_poster_dirty = true;
            return;
        }

        let use_kitty = match self.cfg.poster_render_mode.as_str() {
            "kitty" => true,
            "auto" => crate::ui::poster::is_kitty_graphics_supported(),
            _ => false,
        };

        if let Some(item) = self.current_items.get(self.selected_index) {
            if !item.poster_path.is_empty() {
                if let Some(path) = self.tmdb.get_cached_poster(&item.poster_path).await {
                    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((120, 40));
                    let inner_w = term_w.saturating_sub(2);
                    let inner_h = term_h.saturating_sub(2);
                    let panel_w = (inner_w as f32 * 0.38) as u16;
                    let (w, h) = calculate_poster_dimensions_for_panel(panel_w, inner_h);

                    if use_kitty {
                        if let Some(raw) = crate::ui::poster::render_poster_to_kitty(&path, w, h) {
                            self.current_poster_kitty = Some(raw);
                            self.current_poster_text = None;
                            self.kitty_poster_dirty = true;
                            return;
                        }
                    }

                    let text_mode = if self.cfg.poster_render_mode == "half-block" {
                        "half-block"
                    } else {
                        "high-res"
                    };
                    self.current_poster_text = render_poster_to_text(&path, w, h, text_mode);
                    self.current_poster_kitty = None;
                    self.kitty_poster_dirty = true;
                    return;
                }
            }
        }
        self.current_poster_text = None;
        self.current_poster_kitty = None;
        self.kitty_poster_dirty = true;
    }

    pub fn is_catalog_visible_without_modals(&self) -> bool {
        self.setup_wizard.is_none()
            && self.playback_loading.is_none()
            && self.provider_modal.is_none()
            && self.episode_modal.is_none()
            && self.settings_modal.is_none()
            && self.subtitle_modal.is_none()
            && self.error_dialog.is_none()
            && !self.show_help
            && (self.active_tab == ActiveTab::Movies
                || self.active_tab == ActiveTab::Series
                || self.active_tab == ActiveTab::History
                || self.active_tab == ActiveTab::Bookmarks
                || (self.active_tab == ActiveTab::Search && !self.current_items.is_empty()))
    }

    pub fn render_kitty_poster_if_needed(&mut self, term_w: u16, term_h: u16) {
        if self.is_catalog_visible_without_modals() {
            if let Some(ref kp) = self.current_poster_kitty {
                if self.kitty_poster_dirty || !self.had_kitty_poster_drawn {
                    let rect = crate::ui::poster::calculate_poster_screen_rect(term_w, term_h);
                    crate::ui::poster::draw_kitty_graphics(kp, rect.x, rect.y);
                    self.had_kitty_poster_drawn = true;
                    self.kitty_poster_dirty = false;
                }
                return;
            }
        }

        if self.had_kitty_poster_drawn {
            crate::ui::poster::clear_kitty_graphics();
            self.had_kitty_poster_drawn = false;
        }
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

        if self.episode_modal.is_some() {
            self.open_provider_modal_for_selected();
            return;
        }

        if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            if item.media_type == "tv" {
                self.open_episode_modal(item).await;
            } else {
                self.open_provider_modal_for_selected();
            }
        }
    }

    pub async fn quick_play_selected(&mut self) {
        let ep_play_info = if let Some(ref modal) = self.episode_modal {
            modal.episodes.get(modal.selected_episode_idx).map(|ep| {
                let season_num = modal
                    .seasons
                    .get(modal.selected_season_idx)
                    .map(|s| s.season_number)
                    .unwrap_or(1);
                (
                    modal.series.tmdb_id,
                    modal.series.title.clone(),
                    modal.series.year.clone(),
                    modal.series.poster_path.clone(),
                    season_num,
                    ep.episode_number,
                    modal.series.clone(),
                )
            })
        } else {
            None
        };

        if let Some((tmdb_id, title, year, poster, season_num, ep_num, series)) = ep_play_info {
            let is_anime = series.genres.to_lowercase().contains("animation");
            let req = MediaRequest {
                tmdb_id,
                media_type: "tv".to_string(),
                season: Some(season_num),
                episode: Some(ep_num),
                title,
                year: Some(year),
                genres: Some(series.genres.clone()),
                is_anime,
                audio_mode: Some(self.cfg.audio_mode.clone()),
            };

            let choice = self.cfg.preferred_provider.clone();
            self.start_playback_loading(req, poster, choice, None, None, Some(series))
                .await;
            return;
        }

        if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            if item.media_type == "tv" {
                self.open_episode_modal(item).await;
            } else {
                let is_anime = item.genres.to_lowercase().contains("animation");
                let req = MediaRequest {
                    tmdb_id: item.tmdb_id,
                    media_type: "movie".to_string(),
                    season: None,
                    episode: None,
                    title: item.title.clone(),
                    year: Some(item.year.clone()),
                    genres: Some(item.genres.clone()),
                    is_anime,
                    audio_mode: Some(self.cfg.audio_mode.clone()),
                };
                let choice = self.cfg.preferred_provider.clone();
                self.start_playback_loading(
                    req,
                    item.poster_path.clone(),
                    choice,
                    None,
                    None,
                    Some(item),
                )
                .await;
            }
        }
    }
}
