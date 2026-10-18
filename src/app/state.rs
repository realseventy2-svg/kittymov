use std::collections::HashMap;

use crate::metadata::{MediaItem, TvEpisode, TvSeason};
use crate::providers::{MediaRequest, StreamCandidate};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderModalCategory {
    Provider,
    Source,
    Quality,
    Subtitles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderModalPane {
    Categories,
    Content,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SourceTrackOption {
    pub id: String,
    pub name: String,
    pub status_str: String,
    pub is_selected: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct QualityTrackOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct SubtitleTrackOption {
    pub label: String,
    pub lang_code: String,
    pub badge: String,
    pub is_native: bool,
    pub description: String,
    #[allow(dead_code)]
    pub file_or_url: String,
}

pub fn default_qualities_for_provider(provider_id: &str) -> Vec<QualityTrackOption> {
    match provider_id.to_lowercase().as_str() {
        "hianime" => vec![
            QualityTrackOption {
                label: "Auto (1080p FHD)".to_string(),
                description: "HiAnime native FHD master stream with soft subs".to_string(),
            },
            QualityTrackOption {
                label: "1080p FHD".to_string(),
                description: "Full High-Definition (1920x1080) stream".to_string(),
            },
            QualityTrackOption {
                label: "720p HD".to_string(),
                description: "720p HD adaptive stream".to_string(),
            },
            QualityTrackOption {
                label: "480p SD".to_string(),
                description: "480p standard definition stream".to_string(),
            },
        ],
        "lookmovie" => vec![
            QualityTrackOption {
                label: "Auto (Best Available)".to_string(),
                description: "Auto-negotiate highest free stream (480p SD)".to_string(),
            },
            QualityTrackOption {
                label: "480p SD (Available)".to_string(),
                description: "Standard definition (LookMovie public tier maximum)".to_string(),
            },
            QualityTrackOption {
                label: "720p HD (Premium Gated)".to_string(),
                description: "Locked on LookMovie free CDN — falls back to 480p SD".to_string(),
            },
            QualityTrackOption {
                label: "1080p FHD (Premium Gated)".to_string(),
                description: "Locked on LookMovie free CDN — falls back to 480p SD".to_string(),
            },
        ],
        "vidlink" => vec![
            QualityTrackOption {
                label: "Auto (1080p FHD)".to_string(),
                description: "VidLink direct HD stream • Multi-bitrate CDN".to_string(),
            },
            QualityTrackOption {
                label: "1080p FHD".to_string(),
                description: "1080p Full HD direct MP4/HLS stream".to_string(),
            },
            QualityTrackOption {
                label: "720p HD".to_string(),
                description: "720p High Definition stream".to_string(),
            },
            QualityTrackOption {
                label: "480p SD".to_string(),
                description: "480p Standard Definition stream".to_string(),
            },
            QualityTrackOption {
                label: "360p Mobile".to_string(),
                description: "360p lightweight mobile stream".to_string(),
            },
        ],
        "vixsrc" => vec![
            QualityTrackOption {
                label: "Auto (1080p)".to_string(),
                description: "VixSrc Ultra-Fast adaptive HLS stream".to_string(),
            },
            QualityTrackOption {
                label: "1080p FHD".to_string(),
                description: "1080p primary content delivery stream".to_string(),
            },
            QualityTrackOption {
                label: "720p HD".to_string(),
                description: "720p compressed stream".to_string(),
            },
            QualityTrackOption {
                label: "480p SD".to_string(),
                description: "480p standard definition stream".to_string(),
            },
        ],
        _ => vec![
            QualityTrackOption {
                label: "Auto (Best Match)".to_string(),
                description: "Auto-negotiate highest available resolution".to_string(),
            },
            QualityTrackOption {
                label: "1080p FHD".to_string(),
                description: "Preferred 1080p FHD".to_string(),
            },
            QualityTrackOption {
                label: "720p HD".to_string(),
                description: "Preferred 720p HD".to_string(),
            },
            QualityTrackOption {
                label: "480p SD".to_string(),
                description: "Preferred 480p SD".to_string(),
            },
        ],
    }
}

pub fn default_subtitles_for_provider(
    provider_id: &str,
    preferred_lang: &str,
) -> Vec<SubtitleTrackOption> {
    let preferred_name = crate::app::lang_name(preferred_lang);
    match provider_id.to_lowercase().as_str() {
        "vidlink" => vec![
            SubtitleTrackOption {
                label: "Auto Match".to_string(),
                lang_code: preferred_lang.to_string(),
                badge: "DIRECT".to_string(),
                is_native: true,
                description: format!(
                    "VidLink Built-in Subtitles (.srt • {})",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [VidLink]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "DIRECT".to_string(),
                is_native: true,
                description: "Official VidLink direct timed subtitle (.srt)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Wyzie Subs]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) • Community ad-free cloud track".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Spanish [VidLink]".to_string(),
                lang_code: "spa".to_string(),
                badge: "DIRECT".to_string(),
                is_native: true,
                description: "VidLink direct Spanish subtitle (.srt)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "French [VidLink]".to_string(),
                lang_code: "fra".to_string(),
                badge: "DIRECT".to_string(),
                is_native: true,
                description: "VidLink direct French subtitle (.srt)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Off".to_string(),
                lang_code: "off".to_string(),
                badge: "OFF".to_string(),
                is_native: false,
                description: "Disable subtitle display during playback".to_string(),
                file_or_url: String::new(),
            },
        ],
        "hianime" => vec![
            SubtitleTrackOption {
                label: "Auto Match".to_string(),
                lang_code: preferred_lang.to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: "Pre-synced anime soft subtitle (.vtt • English / Native)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "English [Native]".to_string(),
                lang_code: "eng".to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: "Official HiAnime timed subtitle track (.vtt)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Wyzie Subs]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) ad-free cloud fallback".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Off".to_string(),
                lang_code: "off".to_string(),
                badge: "OFF".to_string(),
                is_native: false,
                description: "Disable subtitle display during playback".to_string(),
                file_or_url: String::new(),
            },
        ],
        "lookmovie" => vec![
            SubtitleTrackOption {
                label: "Auto Match".to_string(),
                lang_code: preferred_lang.to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: format!(
                    "Pre-synced provider track (LookMovie .vtt • {})",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Native]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: "LookMovie Native (.vtt) • Pre-synced to video encode".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Wyzie Subs]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) • Community ad-free subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Spanish [Native]".to_string(),
                lang_code: "spa".to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: "LookMovie Native (.vtt) • Pre-synced Spanish subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "French [Native]".to_string(),
                lang_code: "fra".to_string(),
                badge: "NATIVE".to_string(),
                is_native: true,
                description: "LookMovie Native (.vtt) • Pre-synced French subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Off".to_string(),
                lang_code: "off".to_string(),
                badge: "OFF".to_string(),
                is_native: false,
                description: "Disable subtitle display during playback".to_string(),
                file_or_url: String::new(),
            },
        ],
        "vixsrc" => vec![
            SubtitleTrackOption {
                label: "Auto Match".to_string(),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: format!(
                    "Wyzie Subs (.srt) • Auto-fetch cloud subtitle [{}]",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Cloud]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) • Cloud Subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Spanish [Cloud]".to_string(),
                lang_code: "spa".to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) • Cloud Subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "French [Cloud]".to_string(),
                lang_code: "fra".to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "Wyzie Subs (.srt) • Cloud Subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Off".to_string(),
                lang_code: "off".to_string(),
                badge: "OFF".to_string(),
                is_native: false,
                description: "Disable subtitle display during playback".to_string(),
                file_or_url: String::new(),
            },
        ],
        _ => vec![
            SubtitleTrackOption {
                label: "Auto Match".to_string(),
                lang_code: preferred_lang.to_string(),
                badge: "SMART".to_string(),
                is_native: true,
                description: format!(
                    "Provider native if available, Wyzie Subs fallback [{}]",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Auto]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "AUTO".to_string(),
                is_native: true,
                description: "Best available match (Native .vtt or Wyzie Subs .srt)".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Off".to_string(),
                lang_code: "off".to_string(),
                badge: "OFF".to_string(),
                is_native: false,
                description: "Disable subtitle display during playback".to_string(),
                file_or_url: String::new(),
            },
        ],
    }
}

pub fn default_sources_for_provider(provider_id: &str) -> Vec<SourceTrackOption> {
    match provider_id.to_lowercase().as_str() {
        "vidlink" => vec![
            SourceTrackOption {
                id: "primary".to_string(),
                name: "Primary CDN (HD Master)".to_string(),
                status_str: "current • ready".to_string(),
                is_selected: true,
                description: "VidLink CDN • High-speed 1080p direct stream • host vidlink.pro"
                    .to_string(),
            },
            SourceTrackOption {
                id: "vault".to_string(),
                name: "MediaVault (Direct CDN)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VidLink CDN • MediaVault • Direct high bitrate stream".to_string(),
            },
            SourceTrackOption {
                id: "edge".to_string(),
                name: "Edge MultiLang (Global)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VidLink CDN • Global edge mirror with multi-lang subtitles".to_string(),
            },
        ],
        "vixsrc" => vec![
            SourceTrackOption {
                id: "vanguard".to_string(),
                name: "Vanguard (Primary)".to_string(),
                status_str: "current • ready".to_string(),
                is_selected: true,
                description: "VixSrc CDN • Vanguard • host vixsrc.to • 1080p FHD".to_string(),
            },
            SourceTrackOption {
                id: "mirage".to_string(),
                name: "Mirage (Fast CDN)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VixSrc CDN • Mirage • host vixsrc.to • 1080p high bitrate"
                    .to_string(),
            },
            SourceTrackOption {
                id: "velocity".to_string(),
                name: "Velocity (Adaptive)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VixSrc CDN • Velocity • host vixsrc.to • 720p HD".to_string(),
            },
            SourceTrackOption {
                id: "ignite".to_string(),
                name: "Ignite (Edge Server)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VixSrc CDN • Ignite • host vixsrc.to • low latency".to_string(),
            },
            SourceTrackOption {
                id: "tampa".to_string(),
                name: "Tampa (Mirror)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VixSrc CDN • Tampa • host vixsrc.to • US-East mirror".to_string(),
            },
            SourceTrackOption {
                id: "solstice".to_string(),
                name: "Solstice (Global)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "VixSrc CDN • Solstice • global edge network".to_string(),
            },
        ],
        "hianime" => vec![
            SourceTrackOption {
                id: "zokoanime".to_string(),
                name: "ZokoAnime (FHD Master)".to_string(),
                status_str: "current • ready".to_string(),
                is_selected: true,
                description: "HiAnime CDN • ZokoAnime • 1080p FHD HLS • Sub/Dub".to_string(),
            },
            SourceTrackOption {
                id: "hd1".to_string(),
                name: "HD-1 (Edge Mirror)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "HiAnime CDN • HD-1 • 1080p FHD mirror stream".to_string(),
            },
        ],
        "lookmovie" => vec![
            SourceTrackOption {
                id: "storage5".to_string(),
                name: "Storage 5 (Master)".to_string(),
                status_str: "current • ready".to_string(),
                is_selected: true,
                description: "LookMovie CDN • Storage 5 • host lookmovie2.to • 1080p master"
                    .to_string(),
            },
            SourceTrackOption {
                id: "storage4".to_string(),
                name: "Storage 4 (Mirror)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "LookMovie CDN • Storage 4 • host lookmovie2.to • 1080p mirror"
                    .to_string(),
            },
            SourceTrackOption {
                id: "storage1".to_string(),
                name: "Storage 1 (EU Server)".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "LookMovie CDN • Storage 1 • host lookmovie2.to • 720p HD".to_string(),
            },
        ],
        _ => vec![
            SourceTrackOption {
                id: "auto".to_string(),
                name: "Auto (Best Available)".to_string(),
                status_str: "current • ready".to_string(),
                is_selected: true,
                description: "Automatic multi-provider chain (VidLink Direct → VixSrc CDN → HiAnime CDN → LookMovie CDN)"
                    .to_string(),
            },
            SourceTrackOption {
                id: "vidlink_direct".to_string(),
                name: "VidLink Primary".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "Primary HD 1080p direct stream • host vidlink.pro".to_string(),
            },
            SourceTrackOption {
                id: "vixsrc_direct".to_string(),
                name: "VixSrc Vanguard".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "Fast 1080p HLS stream • host vixsrc.to".to_string(),
            },
            SourceTrackOption {
                id: "hianime_direct".to_string(),
                name: "HiAnime ZokoAnime".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "Dedicated 1080p anime CDN • host hianime.at".to_string(),
            },
            SourceTrackOption {
                id: "lookmovie_direct".to_string(),
                name: "LookMovie Storage 5".to_string(),
                status_str: "available".to_string(),
                is_selected: false,
                description: "Pre-synced .vtt subtitles • host lookmovie2.to".to_string(),
            },
        ],
    }
}

pub fn default_provider_options() -> Vec<ProviderOption> {
    vec![
        ProviderOption {
            id: "auto".to_string(),
            name: "Auto Fallback Engine".to_string(),
            description: "Fast multi-provider chain (VidLink Direct → VixSrc CDN → HiAnime CDN → LookMovie CDN)"
                .to_string(),
        },
        ProviderOption {
            id: "vidlink".to_string(),
            name: "VidLink Server".to_string(),
            description: "Primary Direct HD CDN • 1080p FHD • Multi-Language Subtitles"
                .to_string(),
        },
        ProviderOption {
            id: "vixsrc".to_string(),
            name: "VixSrc Server".to_string(),
            description: "Ultra-Fast Secondary CDN • 1080p FHD HLS • Multi-Audio".to_string(),
        },
        ProviderOption {
            id: "hianime".to_string(),
            name: "HiAnime Server".to_string(),
            description: "Dedicated Anime CDN • 1080p FHD HLS • Pre-Synced Soft Subtitles"
                .to_string(),
        },
        ProviderOption {
            id: "lookmovie".to_string(),
            name: "LookMovie Server".to_string(),
            description: "High-Reliability Fallback CDN • 1080p/720p/480p • Multi-Subtitles"
                .to_string(),
        },
    ]
}

pub struct ProviderSelectModalState {
    pub target_title: String,
    pub request: MediaRequest,
    pub poster_path: String,
    pub active_category: ProviderModalCategory,
    pub active_pane: ProviderModalPane,
    pub options: Vec<ProviderOption>,
    pub selected_idx: usize,
    pub sources: Vec<SourceTrackOption>,
    pub selected_source_idx: usize,
    pub qualities: Vec<QualityTrackOption>,
    pub selected_quality_idx: usize,
    pub subtitles: Vec<SubtitleTrackOption>,
    pub selected_subtitle_idx: usize,
    pub is_loading_subtitles: bool,
}

impl ProviderSelectModalState {
    pub fn new(
        target_title: String,
        request: MediaRequest,
        poster_path: String,
        preferred_provider: &str,
        preferred_lang: &str,
        preferred_quality: &str,
    ) -> Self {
        let options = default_provider_options();
        let selected_idx = options
            .iter()
            .position(|o| o.id.eq_ignore_ascii_case(preferred_provider))
            .unwrap_or(0);
        let chosen_prov = options[selected_idx].id.as_str();
        let sources = default_sources_for_provider(chosen_prov);
        let qualities = default_qualities_for_provider(chosen_prov);
        let q_target = preferred_quality.to_lowercase();
        let selected_quality_idx = qualities
            .iter()
            .position(|q| {
                if q_target.contains("1080") {
                    q.label.contains("1080")
                } else if q_target.contains("720") {
                    q.label.contains("720")
                } else if q_target.contains("480") {
                    q.label.contains("480")
                } else {
                    q.label.contains("Auto")
                }
            })
            .unwrap_or(0);
        let subtitles = default_subtitles_for_provider(chosen_prov, preferred_lang);

        Self {
            target_title,
            request,
            poster_path,
            active_category: ProviderModalCategory::Provider,
            active_pane: ProviderModalPane::Categories,
            options,
            selected_idx,
            sources,
            selected_source_idx: 0,
            qualities,
            selected_quality_idx,
            subtitles,
            selected_subtitle_idx: 0,
            is_loading_subtitles: false,
        }
    }

    pub fn update_dependent_tracks(&mut self, preferred_lang: &str) {
        let prov_id = self
            .options
            .get(self.selected_idx)
            .map(|o| o.id.as_str())
            .unwrap_or("auto");
        self.sources = default_sources_for_provider(prov_id);
        self.selected_source_idx = 0;
        let prev_quality_label = self
            .qualities
            .get(self.selected_quality_idx)
            .map(|q| q.label.clone());
        self.qualities = default_qualities_for_provider(prov_id);
        if let Some(prev) = prev_quality_label {
            let key = if prev.contains("1080") {
                "1080"
            } else if prev.contains("720") {
                "720"
            } else if prev.contains("480") {
                "480"
            } else {
                "auto"
            };
            self.selected_quality_idx = self
                .qualities
                .iter()
                .position(|q| q.label.to_lowercase().contains(key))
                .unwrap_or(0);
        } else {
            self.selected_quality_idx = 0;
        }
        self.subtitles = default_subtitles_for_provider(prov_id, preferred_lang);
        self.selected_subtitle_idx = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingStep {
    ResolvingMetadata,
    ConnectingProvider,
    FetchingSubtitles,
    LaunchingPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct StepProgressItem {
    pub step: LoadingStep,
    pub status: StepStatus,
    pub title: &'static str,
    pub detail: String,
}

pub struct PlaybackLoadingState {
    pub title: String,
    pub subtitle: String,
    pub media_type: String,
    pub provider_name: String,
    pub current_step: LoadingStep,
    pub steps: Vec<StepProgressItem>,
    pub session_autoskip: bool,
    pub session_autoplay: bool,
    pub session_audio_mode: String,
    pub poster_text: Option<ratatui::text::Text<'static>>,
    pub year: String,
    pub genre: String,
    pub score: f64,
    #[allow(dead_code)]
    pub episodes_count: Option<i32>,
    pub studio: String,
    pub synopsis: String,
    pub up_next: Option<String>,
    pub spinner_idx: usize,
    pub request: MediaRequest,
    pub poster_path: String,
    pub start_sec: Option<i64>,
}

#[derive(Debug)]
pub enum LoadingProgress {
    StepUpdate {
        step: LoadingStep,
        status: StepStatus,
        detail: String,
    },
    FallbackNotice {
        new_provider: String,
    },
    Ready {
        stream: Box<StreamCandidate>,
        display_title: String,
    },
    Failed(String),
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

#[derive(Debug, Clone)]
pub struct ErrorDialogState {
    pub title: String,
    pub primary_message: String,
    pub technical_details: String,
    pub suggestions: Vec<String>,
}

pub struct EpisodeModalState {
    pub series: MediaItem,
    pub seasons: Vec<TvSeason>,
    pub selected_season_idx: usize,
    pub episodes: Vec<TvEpisode>,
    pub selected_episode_idx: usize,
    pub all_season_episodes: Option<HashMap<i32, Vec<TvEpisode>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeOptionKind {
    Resume,
    Restart,
    NextEpisode,
    PickSeasonEpisode,
    SwitchProvider,
    PurgeEpisodeCache,
    PurgeTitleCache,
}

#[derive(Debug, Clone)]
pub struct ResumeOption {
    pub kind: ResumeOptionKind,
    pub label: String,
    pub sublabel: String,
    pub description: String,
    pub icon: &'static str,
}

#[derive(Debug, Clone)]
pub struct ResumeModalState {
    pub item: MediaItem,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub progress_seconds: i64,
    pub duration_seconds: i64,
    pub options: Vec<ResumeOption>,
    pub selected_idx: usize,
    pub filter_query: String,
    pub is_filtering: bool,
    pub poster_text: Option<ratatui::text::Text<'static>>,
}

impl ResumeModalState {
    pub fn filtered_indices(&self) -> Vec<usize> {
        if self.filter_query.trim().is_empty() {
            (0..self.options.len()).collect()
        } else {
            let q = self.filter_query.to_lowercase();
            (0..self.options.len())
                .filter(|&i| {
                    let opt = &self.options[i];
                    opt.label.to_lowercase().contains(&q)
                        || opt.sublabel.to_lowercase().contains(&q)
                        || opt.description.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn selected_filtered_index(&self) -> usize {
        let indices = self.filtered_indices();
        if indices.is_empty() {
            0
        } else if self.selected_idx >= indices.len() {
            indices.len() - 1
        } else {
            self.selected_idx
        }
    }

    pub fn get_selected_option(&self) -> Option<&ResumeOption> {
        let indices = self.filtered_indices();
        indices
            .get(self.selected_filtered_index())
            .and_then(|&idx| self.options.get(idx))
    }
}

pub fn format_time(sec: i64) -> String {
    if sec <= 0 {
        return "0:00".to_string();
    }
    let h = sec / 3600;
    let m = (sec % 3600) / 60;
    let s = sec % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
