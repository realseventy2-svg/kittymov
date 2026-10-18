use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::metadata::{MediaItem, TmdbClient};
use crate::player::{MpvController, PlaybackStatus};
use crate::providers::{MediaRequest, ProviderManager, StreamCandidate};

use super::state::{
    LoadingProgress, LoadingStep, PlaybackLoadingState, StepProgressItem, StepStatus,
    SubtitleTrackOption,
};
use super::App;

/// Maps canonical episode numbers to streaming episode numbers or vice versa when primary
/// provider episode order differs from target provider episode order.
pub async fn resolve_target_episode(
    tmdb: &Arc<TmdbClient>,
    req: &MediaRequest,
    preferred_provider: &str,
    provider_choice: &str,
) -> MediaRequest {
    let mut mapped_req = req.clone();
    if mapped_req.media_type != "tv" {
        return mapped_req;
    }

    if let (Some(s), Some(e)) = (mapped_req.season, mapped_req.episode) {
        let primary_order = App::provider_episode_order(preferred_provider);
        let target_order = App::provider_episode_order(provider_choice);

        if primary_order != target_order {
            if target_order == "streaming" {
                if let Ok((_info, _seasons, Some(stream_map))) = tmdb
                    .get_tv_details_with_order(mapped_req.tmdb_id, "streaming")
                    .await
                {
                    if let Ok(canon_eps) = tmdb.get_season_episodes(mapped_req.tmdb_id, s).await {
                        if let Some(target_canon_ep) =
                            canon_eps.iter().find(|x| x.episode_number == e)
                        {
                            let target_name = target_canon_ep.name.trim().to_lowercase();
                            let mut found = None;
                            for (stream_s, eps) in &stream_map {
                                for stream_ep in eps {
                                    if stream_ep.name.trim().to_lowercase() == target_name {
                                        found = Some((*stream_s, stream_ep.episode_number));
                                        break;
                                    }
                                }
                                if found.is_some() {
                                    break;
                                }
                            }
                            if let Some((s_m, e_m)) = found {
                                mapped_req.season = Some(s_m);
                                mapped_req.episode = Some(e_m);
                            }
                        }
                    }
                }
            } else if let Ok((_info, _seasons, Some(stream_map))) = tmdb
                .get_tv_details_with_order(mapped_req.tmdb_id, "streaming")
                .await
            {
                if let Some(stream_eps) = stream_map.get(&s) {
                    if let Some(target_stream_ep) =
                        stream_eps.iter().find(|x| x.episode_number == e)
                    {
                        let target_name = target_stream_ep.name.trim().to_lowercase();
                        if let Ok((_canon_info, canon_seasons, _)) = tmdb
                            .get_tv_details_with_order(mapped_req.tmdb_id, "broadcast")
                            .await
                        {
                            let mut found = None;
                            for cs in &canon_seasons {
                                if let Ok(canon_eps) = tmdb
                                    .get_season_episodes(mapped_req.tmdb_id, cs.season_number)
                                    .await
                                {
                                    for cep in canon_eps {
                                        if cep.name.trim().to_lowercase() == target_name {
                                            found = Some((cs.season_number, cep.episode_number));
                                            break;
                                        }
                                    }
                                    if found.is_some() {
                                        break;
                                    }
                                }
                            }
                            if let Some((s_m, e_m)) = found {
                                mapped_req.season = Some(s_m);
                                mapped_req.episode = Some(e_m);
                            }
                        }
                    }
                }
            }
        }
    }
    mapped_req
}

/// Resolves video stream candidates across configured primary provider and fallback CDN ladder.
pub async fn resolve_stream_candidate(
    providers: &Arc<ProviderManager>,
    mapped_req: &MediaRequest,
    req_clone: &MediaRequest,
    active_provider: &str,
    effective_quality: Option<&str>,
    effective_audio: Option<&str>,
    tx: &mpsc::UnboundedSender<LoadingProgress>,
) -> Result<StreamCandidate, String> {
    let prov_label = match active_provider {
        "vidlink" => "VidLink Server",
        "vixsrc" => "VixSrc Server",
        "hianime" => "HiAnime Server",
        "lookmovie" => "LookMovie Server",
        _ => {
            if mapped_req.is_anime() {
                "HiAnime (Auto Anime Routing)"
            } else {
                "Auto Fallback Engine (VidLink Primary)"
            }
        }
    };
    let _ = tx.send(LoadingProgress::StepUpdate {
        step: LoadingStep::ConnectingProvider,
        status: StepStatus::Active,
        detail: format!("Connecting to {} CDN...", prov_label),
    });

    let resolve_result = providers
        .resolve_with_choice_and_quality(
            active_provider,
            mapped_req,
            effective_quality,
            effective_audio,
        )
        .await;

    match resolve_result {
        Ok(s) => Ok(s),
        Err(err_primary) => {
            if active_provider == "auto" {
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::ConnectingProvider,
                    status: StepStatus::Failed,
                    detail: format!("All providers failed: {}", err_primary),
                });
                Err(format!("All providers failed: {}", err_primary))
            } else if active_provider == "vidlink" {
                let _ = tx.send(LoadingProgress::FallbackNotice {
                    new_provider: "VixSrc CDN".to_string(),
                });
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::ConnectingProvider,
                    status: StepStatus::Active,
                    detail: "VidLink failed; failing over to VixSrc...".to_string(),
                });
                match providers
                    .resolve_with_choice_and_quality(
                        "vixsrc",
                        req_clone,
                        effective_quality,
                        effective_audio,
                    )
                    .await
                {
                    Ok(s) => Ok(s),
                    Err(err_vix) => {
                        let _ = tx.send(LoadingProgress::FallbackNotice {
                            new_provider: "LookMovie (Fallback CDN)".to_string(),
                        });
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::ConnectingProvider,
                            status: StepStatus::Active,
                            detail: "VixSrc failed; failing over to LookMovie...".to_string(),
                        });
                        match providers
                            .resolve_with_choice_and_quality(
                                "lookmovie",
                                req_clone,
                                effective_quality,
                                effective_audio,
                            )
                            .await
                        {
                            Ok(s) => Ok(s),
                            Err(err_look) => {
                                let _ = tx.send(LoadingProgress::StepUpdate {
                                    step: LoadingStep::ConnectingProvider,
                                    status: StepStatus::Failed,
                                    detail: format!("All providers failed: {}", err_look),
                                });
                                Err(format!(
                                    "Providers failed: VidLink ({}), VixSrc ({}), LookMovie ({})",
                                    err_primary, err_vix, err_look
                                ))
                            }
                        }
                    }
                }
            } else if active_provider == "vixsrc" {
                let _ = tx.send(LoadingProgress::FallbackNotice {
                    new_provider: "HiAnime CDN".to_string(),
                });
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::ConnectingProvider,
                    status: StepStatus::Active,
                    detail: "VixSrc failed; failing over to HiAnime...".to_string(),
                });
                match providers
                    .resolve_with_choice_and_quality(
                        "hianime",
                        req_clone,
                        effective_quality,
                        effective_audio,
                    )
                    .await
                {
                    Ok(s) => Ok(s),
                    Err(err_hianime) => {
                        let _ = tx.send(LoadingProgress::FallbackNotice {
                            new_provider: "LookMovie (Fallback CDN)".to_string(),
                        });
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::ConnectingProvider,
                            status: StepStatus::Active,
                            detail: "HiAnime failed; failing over to LookMovie...".to_string(),
                        });
                        match providers
                            .resolve_with_choice_and_quality(
                                "lookmovie",
                                req_clone,
                                effective_quality,
                                effective_audio,
                            )
                            .await
                        {
                            Ok(s) => Ok(s),
                            Err(err_look) => {
                                let _ = tx.send(LoadingProgress::StepUpdate {
                                    step: LoadingStep::ConnectingProvider,
                                    status: StepStatus::Failed,
                                    detail: format!("All providers failed: {}", err_look),
                                });
                                Err(format!(
                                    "Providers failed: VixSrc ({}), HiAnime ({}), LookMovie ({})",
                                    err_primary, err_hianime, err_look
                                ))
                            }
                        }
                    }
                }
            } else if active_provider == "hianime" {
                let _ = tx.send(LoadingProgress::FallbackNotice {
                    new_provider: "LookMovie (Fallback CDN)".to_string(),
                });
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::ConnectingProvider,
                    status: StepStatus::Active,
                    detail: "HiAnime failed; failing over to LookMovie...".to_string(),
                });
                match providers
                    .resolve_with_choice_and_quality(
                        "lookmovie",
                        req_clone,
                        effective_quality,
                        effective_audio,
                    )
                    .await
                {
                    Ok(s) => Ok(s),
                    Err(err_fallback) => {
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::ConnectingProvider,
                            status: StepStatus::Failed,
                            detail: format!("LookMovie fallback failed: {}", err_fallback),
                        });
                        Err(format!(
                            "Both providers failed: HiAnime ({}), LookMovie ({})",
                            err_primary, err_fallback
                        ))
                    }
                }
            } else {
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::ConnectingProvider,
                    status: StepStatus::Failed,
                    detail: format!("Provider connection failed: {}", err_primary),
                });
                Err(format!("Provider failed: {}", err_primary))
            }
        }
    }
}

/// Resolves and integrates subtitles according to user preference and provider native track availability.
#[allow(clippy::too_many_arguments)]
pub async fn prepare_playback_subtitles(
    subtitles_mgr: &Arc<crate::subtitles::SubtitleManager>,
    tmdb: &Arc<TmdbClient>,
    req_clone: &MediaRequest,
    stream: &mut StreamCandidate,
    subtitle_choice: Option<&SubtitleTrackOption>,
    pref_sub_lang: &str,
    wyzie_api_key: &str,
    tx: &mpsc::UnboundedSender<LoadingProgress>,
) {
    let _ = tx.send(LoadingProgress::StepUpdate {
        step: LoadingStep::FetchingSubtitles,
        status: StepStatus::Active,
        detail: "Analyzing subtitle inventory...".to_string(),
    });

    if let Some(sub_opt) = subtitle_choice {
        if sub_opt.lang_code == "off" {
            stream.subtitles.clear();
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Completed,
                detail: "Subtitles disabled".to_string(),
            });
        } else if sub_opt.is_native && !stream.subtitles.is_empty() {
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Completed,
                detail: format!(
                    "Selected {} pre-synced provider native tracks (.vtt)",
                    stream.subtitles.len()
                ),
            });
        } else {
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Active,
                detail: format!("Fetching Wyzie Subs cloud track [{}]...", sub_opt.lang_code),
            });
            let ext = subtitles_mgr
                .prepare_subtitles(
                    tmdb,
                    req_clone.tmdb_id,
                    &req_clone.title,
                    &req_clone.media_type,
                    req_clone.season,
                    req_clone.episode,
                    &sub_opt.lang_code,
                    wyzie_api_key,
                )
                .await;
            if !ext.is_empty() {
                stream.subtitles = ext;
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::FetchingSubtitles,
                    status: StepStatus::Completed,
                    detail: format!("Loaded Wyzie Subs cloud tracks [{}]", sub_opt.lang_code),
                });
            } else if !stream.subtitles.is_empty() {
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::FetchingSubtitles,
                    status: StepStatus::Completed,
                    detail: "Cloud track not found; using provider native track".to_string(),
                });
            } else {
                let _ = tx.send(LoadingProgress::StepUpdate {
                    step: LoadingStep::FetchingSubtitles,
                    status: StepStatus::Completed,
                    detail: "No subtitles found for requested language".to_string(),
                });
            }
        }
    } else if !stream.subtitles.is_empty() {
        let _ = tx.send(LoadingProgress::StepUpdate {
            step: LoadingStep::FetchingSubtitles,
            status: StepStatus::Completed,
            detail: format!(
                "Auto-selected {} pre-synced native tracks (.vtt)",
                stream.subtitles.len()
            ),
        });
    } else {
        let _ = tx.send(LoadingProgress::StepUpdate {
            step: LoadingStep::FetchingSubtitles,
            status: StepStatus::Active,
            detail: format!(
                "Stream has no native subtitles; auto-fetching Wyzie Subs [{}]...",
                pref_sub_lang
            ),
        });
        let ext = subtitles_mgr
            .prepare_subtitles(
                tmdb,
                req_clone.tmdb_id,
                &req_clone.title,
                &req_clone.media_type,
                req_clone.season,
                req_clone.episode,
                pref_sub_lang,
                wyzie_api_key,
            )
            .await;
        if !ext.is_empty() {
            stream.subtitles = ext;
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Completed,
                detail: format!("Auto-loaded Wyzie Subs cloud tracks [{}]", pref_sub_lang),
            });
        } else {
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Completed,
                detail: "No subtitles available for this stream".to_string(),
            });
        }
    }
}

impl App {
    pub async fn start_playback_loading(
        &mut self,
        mut req: MediaRequest,
        poster_path: String,
        provider_choice: String,
        quality_choice: Option<String>,
        subtitle_choice: Option<SubtitleTrackOption>,
        catalog_item: Option<MediaItem>,
    ) {
        if req.genres.is_none() {
            if let Some(ref item) = catalog_item {
                req.genres = Some(item.genres.clone());
                req.is_anime = item.genres.to_lowercase().contains("animation");
            }
        }
        if req.audio_mode.is_none() {
            req.audio_mode = Some(self.cfg.audio_mode.clone());
        }

        if let Some(h) = self.loading_task.take() {
            h.abort();
        }

        let is_tv = req.media_type == "tv";
        let display_title = if is_tv {
            format!(
                "{} S{:02}E{:02}",
                req.title,
                req.season.unwrap_or(1),
                req.episode.unwrap_or(1)
            )
        } else {
            req.title.clone()
        };

        let subtitle_str = if is_tv {
            format!(
                "Season {}, Episode {}",
                req.season.unwrap_or(1),
                req.episode.unwrap_or(1)
            )
        } else {
            req.year.clone().unwrap_or_else(|| "Movie".to_string())
        };

        let provider_display_name = match provider_choice.to_lowercase().as_str() {
            "vidlink" => "VidLink Server",
            "vixsrc" => "VixSrc Server",
            "hianime" => "HiAnime Server",
            "lookmovie" => "LookMovie Server",
            _ => "Auto Fallback Engine",
        }
        .to_string();

        let initial_steps = vec![
            StepProgressItem {
                step: LoadingStep::ResolvingMetadata,
                status: StepStatus::Active,
                title: "Resolving",
                detail: "Canonical streams & episode maps".to_string(),
            },
            StepProgressItem {
                step: LoadingStep::ConnectingProvider,
                status: StepStatus::Pending,
                title: "Providers",
                detail: format!("Connecting to media network ({})", provider_display_name),
            },
            StepProgressItem {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Pending,
                title: "Subtitles",
                detail: "Analyzing audio & subtitle inventory".to_string(),
            },
            StepProgressItem {
                step: LoadingStep::LaunchingPlayer,
                status: StepStatus::Pending,
                title: "Player",
                detail: "Buffering stream in mpv hardware decoder".to_string(),
            },
        ];

        let (year, genre, score, episodes_count, synopsis) = if let Some(ref item) = catalog_item {
            (
                item.year.clone(),
                item.genres.clone(),
                item.rating,
                item.episodes_count,
                item.overview.clone(),
            )
        } else {
            (
                req.year.clone().unwrap_or_else(|| "2024".to_string()),
                "Animation, Sci-Fi, Drama".to_string(),
                8.2,
                None,
                String::new(),
            )
        };

        let up_next = if is_tv {
            Some(format!(
                "S{:02}E{:02} • Up Next",
                req.season.unwrap_or(1),
                req.episode.unwrap_or(1) + 1
            ))
        } else {
            None
        };

        self.playback_loading = Some(PlaybackLoadingState {
            title: display_title.clone(),
            subtitle: subtitle_str,
            media_type: if is_tv { "tv" } else { "movie" }.to_string(),
            provider_name: provider_display_name,
            current_step: LoadingStep::ResolvingMetadata,
            steps: initial_steps,
            session_autoskip: true,
            session_autoplay: self.cfg.auto_play_next,
            session_audio_mode: self.cfg.audio_mode.clone(),
            poster_text: self.current_poster_text.clone(),
            year,
            genre,
            score,
            episodes_count,
            studio: "Studio / Network".to_string(),
            synopsis,
            up_next,
            spinner_idx: 0,
            request: req.clone(),
            poster_path: poster_path.clone(),
        });

        let (tx, rx) = mpsc::unbounded_channel();
        self.loading_rx = Some(rx);

        let tmdb = self.tmdb.clone();
        let providers = self.providers.clone();
        let subtitles_mgr = self.subtitles.clone();
        let cfg = self.cfg.clone();
        let db = self.db.clone();
        let disp_title_clone = display_title.clone();
        let req_clone = req.clone();
        let poster_clone = poster_path.clone();
        let quality_choice_clone = quality_choice.clone();

        let handle = tokio::spawn(async move {
            // STAGE 1: Metadata Resolution & Episode mapping
            let mapped_req = resolve_target_episode(
                &tmdb,
                &req_clone,
                &cfg.preferred_provider,
                &provider_choice,
            )
            .await;

            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::ResolvingMetadata,
                status: StepStatus::Completed,
                detail: format!("Found canonical title index for '{}'", disp_title_clone),
            });

            // STAGE 2: Provider Connection & Multi-CDN Fallback
            let effective_quality = quality_choice_clone
                .as_deref()
                .or(Some(cfg.preferred_quality.as_str()));
            let effective_audio = Some(cfg.audio_mode.as_str());

            let mut stream = match resolve_stream_candidate(
                &providers,
                &mapped_req,
                &req_clone,
                &provider_choice,
                effective_quality,
                effective_audio,
                &tx,
            )
            .await
            {
                Ok(s) => s,
                Err(err_msg) => {
                    let _ = tx.send(LoadingProgress::Failed(err_msg));
                    return;
                }
            };

            let quality_label = stream.quality.as_deref().unwrap_or("Auto");
            let detail_msg = if stream.provider_name.to_lowercase().contains("lookmovie") {
                if let Some(req_q) = effective_quality {
                    if (req_q.contains("1080") || req_q.contains("720"))
                        && quality_label.contains("480")
                    {
                        "LookMovie free CDN is limited to 480p (720p/1080p premium-gated); selected 480p SD".to_string()
                    } else {
                        format!("CDN Stream live: LookMovie [{}]", quality_label)
                    }
                } else {
                    format!("CDN Stream live: LookMovie [{}]", quality_label)
                }
            } else if stream.provider_name.to_lowercase().contains("hianime") {
                format!("CDN Stream live: HiAnime [{}]", quality_label)
            } else {
                format!(
                    "CDN Stream live: {} [{}]",
                    stream.provider_name, quality_label
                )
            };

            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::ConnectingProvider,
                status: StepStatus::Completed,
                detail: detail_msg,
            });

            // STAGE 3: Subtitles resolution
            prepare_playback_subtitles(
                &subtitles_mgr,
                &tmdb,
                &req_clone,
                &mut stream,
                subtitle_choice.as_ref(),
                &cfg.sub_language,
                &cfg.wyzie_api_key,
                &tx,
            )
            .await;

            // STAGE 4: Player preparation & watch recording
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::LaunchingPlayer,
                status: StepStatus::Active,
                detail: "Starting MPV hardware-accelerated pipeline...".to_string(),
            });

            let _ = db.record_watch(
                req_clone.tmdb_id,
                &req_clone.media_type,
                &req_clone.title,
                req_clone.year.as_deref().unwrap_or(""),
                &poster_clone,
                req_clone.season,
                req_clone.episode,
                None,
                0,
                0,
            );

            let _ = tx.send(LoadingProgress::Ready {
                stream: Box::new(stream),
                display_title: disp_title_clone,
            });
        });

        self.loading_task = Some(handle);
    }

    pub fn poll_playback_loading(&mut self) -> Option<LoadingProgress> {
        if let Some(ref mut st) = self.playback_loading {
            st.spinner_idx = st.spinner_idx.wrapping_add(1);
        }

        if let Some(ref mut rx) = self.loading_rx {
            let mut ready_or_failed = None;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    LoadingProgress::StepUpdate {
                        step,
                        status,
                        ref detail,
                    } => {
                        if let Some(ref mut st) = self.playback_loading {
                            st.current_step = step;
                            for s in &mut st.steps {
                                if s.step == step {
                                    s.status = status;
                                    s.detail = detail.clone();
                                }
                            }
                        }
                    }
                    LoadingProgress::FallbackNotice { ref new_provider } => {
                        if let Some(ref mut st) = self.playback_loading {
                            st.provider_name = new_provider.clone();
                        }
                    }
                    LoadingProgress::Ready { .. } | LoadingProgress::Failed(_) => {
                        ready_or_failed = Some(msg);
                        break;
                    }
                }
            }
            if ready_or_failed.is_some() {
                self.loading_rx = None;
                return ready_or_failed;
            }
        }
        None
    }

    pub fn cancel_playback_loading(&mut self) {
        if let Some(h) = self.loading_task.take() {
            h.abort();
        }
        self.loading_rx = None;
        self.playback_loading = None;
        self.status_message = "Playback cancelled".to_string();
    }

    pub async fn force_playback_fallback(&mut self) {
        if let Some(ref loading) = self.playback_loading {
            let req = loading.request.clone();
            let poster = loading.poster_path.clone();
            let current_prov = loading.provider_name.to_lowercase();
            let next_prov = if current_prov.contains("vixsrc") {
                "lookmovie".to_string()
            } else if current_prov.contains("lookmovie") {
                "hianime".to_string()
            } else {
                "vixsrc".to_string()
            };
            self.cancel_playback_loading();
            self.start_playback_loading(req, poster, next_prov, None, None, None)
                .await;
        }
    }

    pub async fn execute_player(&mut self, stream: &StreamCandidate, title: &str) {
        self.playback_loading = None;
        self.launch_mpv_session(stream, title).await;
    }

    pub async fn play_movie(&mut self, item: MediaItem) {
        let choice = self.cfg.preferred_provider.clone();
        self.play_movie_with_provider(
            item.tmdb_id,
            &item.title,
            &item.year,
            &item.poster_path,
            &choice,
        )
        .await;
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
            "vidlink" => "VidLink Server",
            "vixsrc" => "VixSrc Server",
            "hianime" => "HiAnime Server",
            "lookmovie" => "LookMovie Server",
            _ => "Auto Fallback",
        };
        self.status_message = format!(
            "Resolving '{}' on {} [{}]...",
            title,
            display_name,
            self.cfg.audio_mode.to_uppercase()
        );

        let req = MediaRequest {
            tmdb_id,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: title.to_string(),
            year: Some(year.to_string()),
            genres: None,
            is_anime: false,
            audio_mode: Some(self.cfg.audio_mode.clone()),
        };

        match self
            .providers
            .resolve_with_choice_and_quality(
                provider_choice,
                &req,
                Some(&self.cfg.preferred_quality),
                Some(&self.cfg.audio_mode),
            )
            .await
        {
            Ok(mut stream) => {
                let sub_src = self.cfg.subtitle_source.to_lowercase();
                let should_fetch_ext = match sub_src.as_str() {
                    "provider" => stream.subtitles.is_empty() && self.cfg.subtitle_augmentation,
                    "wyzie" | "opensubtitles" | "both" | "all" => true,
                    _ => stream.subtitles.is_empty() || self.cfg.subtitle_augmentation,
                };

                if sub_src == "wyzie" || sub_src == "opensubtitles" {
                    stream.subtitles.clear();
                }

                if should_fetch_ext {
                    self.status_message = format!(
                        "Fetching subtitles for '{}' [{}]...",
                        title, self.cfg.sub_language
                    );
                    let external_subs = self
                        .subtitles
                        .prepare_subtitles(
                            &self.tmdb,
                            tmdb_id,
                            title,
                            "movie",
                            None,
                            None,
                            &self.cfg.sub_language,
                            &self.cfg.wyzie_api_key,
                        )
                        .await;
                    if !external_subs.is_empty() {
                        if stream.subtitles.is_empty() {
                            stream.subtitles = external_subs;
                        } else {
                            for ext in external_subs {
                                if !stream.subtitles.contains(&ext) {
                                    stream.subtitles.push(ext);
                                }
                            }
                        }
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
        self.play_tv_series_with_provider(
            tmdb_id,
            title,
            year,
            poster_path,
            season,
            episode,
            &choice,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
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
            "vidlink" => "VidLink Server",
            "vixsrc" => "VixSrc Server",
            "hianime" => "HiAnime Server",
            "lookmovie" => "LookMovie Server",
            _ => "Auto Fallback",
        };
        self.status_message = format!(
            "Resolving {} on {} [{}]...",
            display_title,
            display_name,
            self.cfg.audio_mode.to_uppercase()
        );

        let req = MediaRequest {
            tmdb_id,
            media_type: "tv".to_string(),
            season: Some(season),
            episode: Some(episode),
            title: title.to_string(),
            year: Some(year.to_string()),
            genres: None,
            is_anime: false,
            audio_mode: Some(self.cfg.audio_mode.clone()),
        };

        match self
            .providers
            .resolve_with_choice_and_quality(
                provider_choice,
                &req,
                Some(&self.cfg.preferred_quality),
                Some(&self.cfg.audio_mode),
            )
            .await
        {
            Ok(mut stream) => {
                let sub_src = self.cfg.subtitle_source.to_lowercase();
                let should_fetch_ext = match sub_src.as_str() {
                    "provider" => stream.subtitles.is_empty() && self.cfg.subtitle_augmentation,
                    "wyzie" | "opensubtitles" | "both" | "all" => true,
                    _ => stream.subtitles.is_empty() || self.cfg.subtitle_augmentation,
                };

                if sub_src == "wyzie" || sub_src == "opensubtitles" {
                    stream.subtitles.clear();
                }

                if should_fetch_ext {
                    self.status_message = format!(
                        "Fetching subtitles for {} [{}]...",
                        display_title, self.cfg.sub_language
                    );
                    let external_subs = self
                        .subtitles
                        .prepare_subtitles(
                            &self.tmdb,
                            tmdb_id,
                            title,
                            "tv",
                            Some(season),
                            Some(episode),
                            &self.cfg.sub_language,
                            &self.cfg.wyzie_api_key,
                        )
                        .await;
                    if !external_subs.is_empty() {
                        if stream.subtitles.is_empty() {
                            stream.subtitles = external_subs;
                        } else {
                            for ext in external_subs {
                                if !stream.subtitles.contains(&ext) {
                                    stream.subtitles.push(ext);
                                }
                            }
                        }
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

                if stream.is_expired_or_expiring_soon() {
                    let _ = self.providers.refresh_candidate(&mut stream).await;
                }

                self.launch_mpv_tv_auto_next(
                    tmdb_id,
                    title,
                    year,
                    poster_path,
                    season,
                    episode,
                    stream,
                )
                .await;
            }
            Err(e) => {
                self.status_message = format!("Stream error ({}): {}", display_name, e);
            }
        }
    }

    async fn launch_mpv_session(&mut self, stream: &StreamCandidate, title: &str) {
        crate::ui::poster::clear_kitty_graphics();
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();

        let mut controller = MpvController::new();
        let result = match controller.start_player(stream, title, &self.cfg) {
            Ok(()) => {
                let _ = controller.wait_async().await;
                Ok(())
            }
            Err(e) => Err(e),
        };

        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();
        self.kitty_poster_dirty = true;

        match result {
            Ok(()) => {
                self.status_message =
                    format!("Finished watching '{}' [{}]", title, stream.provider_name);
            }
            Err(e) => {
                self.status_message = format!("Playback launch error: {}", e);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        crate::ui::poster::clear_kitty_graphics();
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();

        let mut controller = MpvController::new();
        let display_title = if let Some(ref act) = initial_stream.actual_title {
            format!("{} — {}", title, act)
        } else {
            format!("{} S{:02}E{:02}", title, current_season, current_episode)
        };

        let result = match controller.start_player(&initial_stream, &display_title, &self.cfg) {
            Ok(()) => {
                let (status_tx, mut status_rx) = mpsc::channel(32);

                if let Ok(socket) = controller.wait_for_socket(Duration::from_secs(4)).await {
                    controller.monitor_ipc(socket, status_tx);

                    let mut prefetch_started = false;

                    while let Some(status) = status_rx.recv().await {
                        match status {
                            PlaybackStatus::Playing { pos, duration } => {
                                if self.cfg.auto_play_next
                                    && !prefetch_started
                                    && (duration > 0.0 && pos / duration >= 0.8 || pos >= 1200.0)
                                {
                                    prefetch_started = true;
                                    let providers = self.providers.clone();
                                    let next_req = MediaRequest {
                                        tmdb_id,
                                        media_type: "tv".to_string(),
                                        season: Some(current_season),
                                        episode: Some(current_episode + 1),
                                        title: title.to_string(),
                                        year: Some(year.to_string()),
                                        genres: initial_stream.request.genres.clone(),
                                        is_anime: initial_stream.request.is_anime,
                                        audio_mode: initial_stream.request.audio_mode.clone(),
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
                                        genres: initial_stream.request.genres.clone(),
                                        is_anime: initial_stream.request.is_anime,
                                        audio_mode: initial_stream.request.audio_mode.clone(),
                                    };

                                    if let Ok(mut next_stream) =
                                        self.providers.resolve_with_fallback(&next_req).await
                                    {
                                        if next_stream.is_expired_or_expiring_soon() {
                                            let _ = self
                                                .providers
                                                .refresh_candidate(&mut next_stream)
                                                .await;
                                        }
                                        if next_stream.subtitles.is_empty()
                                            || self.cfg.subtitle_augmentation
                                        {
                                            let external_subs = self
                                                .subtitles
                                                .prepare_subtitles(
                                                    &self.tmdb,
                                                    tmdb_id,
                                                    title,
                                                    "tv",
                                                    Some(current_season),
                                                    Some(current_episode),
                                                    &self.cfg.sub_language,
                                                    &self.cfg.wyzie_api_key,
                                                )
                                                .await;
                                            if !external_subs.is_empty() {
                                                next_stream.subtitles = external_subs;
                                            }
                                        }

                                        if let Some(ref embed) = next_stream.embed_url {
                                            let _ = controller
                                                .send_command(&["set_property", "referrer", embed])
                                                .await;
                                        } else {
                                            let _ = controller
                                                .send_command(&["set_property", "referrer", ""])
                                                .await;
                                        }
                                        let next_title =
                                            if let Some(ref act) = next_stream.actual_title {
                                                format!("{} — {}", title, act)
                                            } else {
                                                format!(
                                                    "{} S{:02}E{:02}",
                                                    title, current_season, current_episode
                                                )
                                            };
                                        let _ = controller
                                            .send_command(&[
                                                "set_property",
                                                "title",
                                                &format!(
                                                    "KittyMov: {} [{}]",
                                                    next_title, next_stream.provider_name
                                                ),
                                            ])
                                            .await;
                                        for sub in &next_stream.subtitles {
                                            let _ = controller
                                                .send_command(&["sub-add", sub, "auto"])
                                                .await;
                                        }
                                        let _ = controller
                                            .send_command(&[
                                                "loadfile",
                                                &next_stream.stream_url,
                                                "replace",
                                            ])
                                            .await;

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
                let _ = controller.wait_async().await;
                Ok(())
            }
            Err(e) => Err(e),
        };

        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();
        self.kitty_poster_dirty = true;

        match result {
            Ok(()) => {
                self.status_message = format!(
                    "Returned from watching {} S{:02}E{:02}",
                    title, current_season, current_episode
                );
            }
            Err(e) => {
                self.status_message = format!("Playback launch error: {}", e);
            }
        }
    }
}
