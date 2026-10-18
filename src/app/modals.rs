use std::sync::Arc;

use crate::metadata::MediaItem;
use crate::providers::{MediaRequest, ProviderManager};

use super::mod_utils::cycle_list;
use super::state::{
    EpisodeModalState, ProviderModalCategory, ProviderModalPane, ProviderSelectModalState,
    SubtitleSelectModalState,
};
use super::App;

impl App {
    pub fn open_provider_modal_for_selected(&mut self) {
        if let Some(ref modal) = self.episode_modal {
            if let Some(ep) = modal.episodes.get(modal.selected_episode_idx) {
                let season_num = modal
                    .seasons
                    .get(modal.selected_season_idx)
                    .map(|s| s.season_number)
                    .unwrap_or(1);
                let ep_num = ep.episode_number;
                let title = modal.series.title.clone();
                let year = modal.series.year.clone();
                let poster = modal.series.poster_path.clone();
                let tmdb_id = modal.series.tmdb_id;

                let target_title =
                    format!("{} S{:02}E{:02} — {}", title, season_num, ep_num, ep.name);
                let is_anime = modal.series.genres.to_lowercase().contains("animation");
                let request = MediaRequest {
                    tmdb_id,
                    media_type: "tv".to_string(),
                    season: Some(season_num),
                    episode: Some(ep_num),
                    title,
                    year: Some(year),
                    genres: Some(modal.series.genres.clone()),
                    is_anime,
                    audio_mode: Some(self.cfg.audio_mode.clone()),
                };

                let p_modal = ProviderSelectModalState::new(
                    target_title,
                    request,
                    poster,
                    &self.cfg.preferred_provider,
                    &self.cfg.sub_language,
                    &self.cfg.preferred_quality,
                );
                self.provider_modal = Some(p_modal);
                return;
            }
        }

        if let Some(item) = self.current_items.get(self.selected_index).cloned() {
            let is_anime = item.genres.to_lowercase().contains("animation");
            if item.media_type == "tv" {
                let target_title = format!("{} S01E01", item.title);
                let request = MediaRequest {
                    tmdb_id: item.tmdb_id,
                    media_type: "tv".to_string(),
                    season: Some(1),
                    episode: Some(1),
                    title: item.title.clone(),
                    year: Some(item.year.clone()),
                    genres: Some(item.genres.clone()),
                    is_anime,
                    audio_mode: Some(self.cfg.audio_mode.clone()),
                };
                let p_modal = ProviderSelectModalState::new(
                    target_title,
                    request,
                    item.poster_path,
                    &self.cfg.preferred_provider,
                    &self.cfg.sub_language,
                    &self.cfg.preferred_quality,
                );
                self.provider_modal = Some(p_modal);
            } else {
                let target_title = format!("{} ({})", item.title, item.year);
                let request = MediaRequest {
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
                let p_modal = ProviderSelectModalState::new(
                    target_title,
                    request,
                    item.poster_path,
                    &self.cfg.preferred_provider,
                    &self.cfg.sub_language,
                    &self.cfg.preferred_quality,
                );
                self.provider_modal = Some(p_modal);
            }
        }
    }

    pub fn provider_modal_next(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            match m.active_pane {
                ProviderModalPane::Categories => {
                    m.active_category = match m.active_category {
                        ProviderModalCategory::Provider => ProviderModalCategory::Source,
                        ProviderModalCategory::Source => ProviderModalCategory::Quality,
                        ProviderModalCategory::Quality => ProviderModalCategory::Subtitles,
                        ProviderModalCategory::Subtitles => ProviderModalCategory::Provider,
                    };
                }
                ProviderModalPane::Content => match m.active_category {
                    ProviderModalCategory::Provider => {
                        if !m.options.is_empty() {
                            m.selected_idx = (m.selected_idx + 1) % m.options.len();
                            m.update_dependent_tracks(&self.cfg.sub_language);
                        }
                    }
                    ProviderModalCategory::Source => {
                        if !m.sources.is_empty() {
                            m.selected_source_idx = (m.selected_source_idx + 1) % m.sources.len();
                        }
                    }
                    ProviderModalCategory::Quality => {
                        if !m.qualities.is_empty() {
                            m.selected_quality_idx =
                                (m.selected_quality_idx + 1) % m.qualities.len();
                        }
                    }
                    ProviderModalCategory::Subtitles => {
                        if !m.subtitles.is_empty() {
                            m.selected_subtitle_idx =
                                (m.selected_subtitle_idx + 1) % m.subtitles.len();
                        }
                    }
                },
            }
        }
    }

    pub fn provider_modal_prev(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            match m.active_pane {
                ProviderModalPane::Categories => {
                    m.active_category = match m.active_category {
                        ProviderModalCategory::Provider => ProviderModalCategory::Subtitles,
                        ProviderModalCategory::Source => ProviderModalCategory::Provider,
                        ProviderModalCategory::Quality => ProviderModalCategory::Source,
                        ProviderModalCategory::Subtitles => ProviderModalCategory::Quality,
                    };
                }
                ProviderModalPane::Content => match m.active_category {
                    ProviderModalCategory::Provider => {
                        if !m.options.is_empty() {
                            m.selected_idx =
                                (m.selected_idx + m.options.len() - 1) % m.options.len();
                            m.update_dependent_tracks(&self.cfg.sub_language);
                        }
                    }
                    ProviderModalCategory::Source => {
                        if !m.sources.is_empty() {
                            m.selected_source_idx =
                                (m.selected_source_idx + m.sources.len() - 1) % m.sources.len();
                        }
                    }
                    ProviderModalCategory::Quality => {
                        if !m.qualities.is_empty() {
                            m.selected_quality_idx = (m.selected_quality_idx + m.qualities.len()
                                - 1)
                                % m.qualities.len();
                        }
                    }
                    ProviderModalCategory::Subtitles => {
                        if !m.subtitles.is_empty() {
                            m.selected_subtitle_idx = (m.selected_subtitle_idx + m.subtitles.len()
                                - 1)
                                % m.subtitles.len();
                        }
                    }
                },
            }
        }
    }

    pub fn provider_modal_switch_pane(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            m.active_pane = match m.active_pane {
                ProviderModalPane::Categories => ProviderModalPane::Content,
                ProviderModalPane::Content => ProviderModalPane::Categories,
            };
        }
    }

    pub fn provider_modal_left(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            m.active_pane = ProviderModalPane::Categories;
        }
    }

    pub fn provider_modal_right(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            m.active_pane = ProviderModalPane::Content;
        }
    }

    pub fn provider_modal_set_category(&mut self, cat: ProviderModalCategory) {
        if let Some(ref mut m) = self.provider_modal {
            m.active_category = cat;
            m.active_pane = ProviderModalPane::Content;
        }
    }

    pub async fn provider_modal_confirm(&mut self) {
        if let Some(ref mut m) = self.provider_modal {
            if m.active_pane == ProviderModalPane::Categories {
                m.active_pane = ProviderModalPane::Content;
                return;
            }
        }

        if let Some(modal) = self.provider_modal.take() {
            let choice = modal
                .options
                .get(modal.selected_idx)
                .map(|o| o.id.clone())
                .unwrap_or_else(|| "auto".to_string());
            let quality_choice = modal
                .qualities
                .get(modal.selected_quality_idx)
                .map(|q| q.label.clone());
            let sub_choice = modal.subtitles.get(modal.selected_subtitle_idx).cloned();
            let req = modal.request;
            let poster = modal.poster_path;

            self.start_playback_loading(req, poster, choice, quality_choice, sub_choice, None)
                .await;
        }
    }

    pub async fn open_subtitle_modal(&mut self) {
        let (tmdb_id, media_type, season, episode, target_title) =
            if let Some(ref modal) = self.episode_modal {
                if let Some(ep) = modal.episodes.get(modal.selected_episode_idx) {
                    let season_num = modal
                        .seasons
                        .get(modal.selected_season_idx)
                        .map(|s| s.season_number)
                        .unwrap_or(1);
                    let ep_num = ep.episode_number;
                    let title = modal.series.title.clone();
                    (
                        modal.series.tmdb_id,
                        "tv".to_string(),
                        Some(season_num),
                        Some(ep_num),
                        format!("{} S{:02}E{:02}", title, season_num, ep_num),
                    )
                } else {
                    return;
                }
            } else if let Some(ref modal) = self.provider_modal {
                let req = &modal.request;
                (
                    req.tmdb_id,
                    req.media_type.clone(),
                    req.season,
                    req.episode,
                    modal.target_title.clone(),
                )
            } else if let Some(item) = self.current_items.get(self.selected_index).cloned() {
                if item.media_type == "tv" {
                    (
                        item.tmdb_id,
                        "tv".to_string(),
                        Some(1),
                        Some(1),
                        format!("{} S01E01", item.title),
                    )
                } else {
                    (
                        item.tmdb_id,
                        "movie".to_string(),
                        None,
                        None,
                        format!("{} ({})", item.title, item.year),
                    )
                }
            } else {
                return;
            };

        self.status_message = format!("Searching Wyzie Subs for {}...", target_title);
        let imdb_id = self.tmdb.get_imdb_id(tmdb_id, &media_type).await.ok();
        let tracks = self
            .subtitles
            .fetch_all_subtitles(
                imdb_id.as_deref(),
                Some(tmdb_id),
                &media_type,
                season,
                episode,
                &self.cfg.wyzie_api_key,
            )
            .await;
        let groups = crate::subtitles::SubtitleManager::group_by_language(&tracks);

        let selected_idx = groups
            .iter()
            .position(|g| g.lang_code == self.cfg.sub_language)
            .unwrap_or(0);

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
                self.status_message = format!(
                    "Subtitle Language set to: {} [{}]",
                    opt.lang_name, opt.lang_code
                );
            }
        }
    }

    pub fn provider_episode_order(preferred_provider: &str) -> &'static str {
        match preferred_provider.to_lowercase().as_str() {
            "vixsrc" | "vixsrc_only" => "streaming",
            _ => "broadcast",
        }
    }

    pub async fn open_episode_modal(&mut self, item: MediaItem) {
        self.status_message = format!("Loading seasons for {}...", item.title);
        let order = Self::provider_episode_order(&self.cfg.preferred_provider);
        if let Ok((_details, seasons, stream_episodes_map)) = self
            .tmdb
            .get_tv_details_with_order(item.tmdb_id, order)
            .await
        {
            let first_season = seasons.first().map(|s| s.season_number).unwrap_or(1);
            let episodes = if let Some(ref map) = stream_episodes_map {
                map.get(&first_season).cloned().unwrap_or_default()
            } else {
                self.tmdb
                    .get_season_episodes(item.tmdb_id, first_season)
                    .await
                    .unwrap_or_default()
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

    pub async fn refresh_episode_modal(&mut self) {
        if let Some(ref modal) = self.episode_modal {
            let item = modal.series.clone();
            let curr_s_idx = modal.selected_season_idx;
            let curr_ep_idx = modal.selected_episode_idx;
            let order = Self::provider_episode_order(&self.cfg.preferred_provider);

            if let Ok((_details, seasons, stream_episodes_map)) = self
                .tmdb
                .get_tv_details_with_order(item.tmdb_id, order)
                .await
            {
                let s_idx = if curr_s_idx < seasons.len() {
                    curr_s_idx
                } else {
                    0
                };
                let season_num = seasons.get(s_idx).map(|s| s.season_number).unwrap_or(1);
                let episodes = if let Some(ref map) = stream_episodes_map {
                    map.get(&season_num).cloned().unwrap_or_default()
                } else {
                    self.tmdb
                        .get_season_episodes(item.tmdb_id, season_num)
                        .await
                        .unwrap_or_default()
                };
                let ep_idx = if curr_ep_idx < episodes.len() {
                    curr_ep_idx
                } else {
                    0
                };

                self.episode_modal = Some(EpisodeModalState {
                    series: item,
                    seasons,
                    selected_season_idx: s_idx,
                    episodes,
                    selected_episode_idx: ep_idx,
                    all_season_episodes: stream_episodes_map,
                });
            }
        }
    }

    pub fn cycle_subtitle_source(&mut self) {
        let sources = ["auto", "provider", "wyzie", "both"];
        self.cfg.subtitle_source = cycle_list(&sources, &self.cfg.subtitle_source, true);
        let _ = self.cfg.save();
        let label = match self.cfg.subtitle_source.as_str() {
            "provider" => "Provider Native Only",
            "wyzie" | "opensubtitles" => "Wyzie Subs Only",
            "both" | "all" => "Both (All Tracks)",
            _ => "Auto (Best Match)",
        };
        self.status_message = format!("Subtitle Source: {}", label);
    }

    pub async fn cycle_preferred_provider(&mut self, forward: bool) {
        let providers = ["auto", "vidlink", "vixsrc", "hianime", "lookmovie"];
        self.cfg.preferred_provider =
            cycle_list(&providers, &self.cfg.preferred_provider, forward);
        let _ = self.cfg.save();
        self.providers = Arc::new(ProviderManager::with_preference(
            &self.cfg.preferred_provider,
        ));
        let label = match self.cfg.preferred_provider.to_lowercase().as_str() {
            "vidlink" => "VidLink First (HD 1080p Direct)",
            "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)",
            "hianime" => "HiAnime First (Dedicated Anime 1080p CDN)",
            "lookmovie" => "LookMovie First (Backup CDN)",
            _ => "Auto Fallback (VidLink → VixSrc → HiAnime → LookMovie)",
        };
        self.status_message = format!("Primary Provider: {} (Updated episode guide)", label);

        if self.episode_modal.is_some() {
            self.refresh_episode_modal().await;
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
}
