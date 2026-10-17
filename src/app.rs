use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::metadata::{MediaItem, TmdbClient, TvEpisode, TvSeason};
use crate::player::{MpvController, PlaybackStatus};
use crate::providers::{MediaRequest, ProviderManager, StreamCandidate};
use crate::storage::Database;
use crate::ui::poster::{calculate_poster_dimensions_for_panel, render_poster_to_text};

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
    let preferred_name = lang_name(preferred_lang);
    match provider_id.to_lowercase().as_str() {
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
                label: format!("{} [OpenSubtitles]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "OpenSubtitles.org (.srt) cloud fallback".to_string(),
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
                label: format!("{} [OpenSubtitles]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "OpenSubtitles.org (.srt) • Community external subtitle".to_string(),
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
                    "OpenSubtitles.org (.srt) • Auto-fetch cloud subtitle [{}]",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Cloud]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "OpenSubtitles.org (.srt) • Cloud Subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "Spanish [Cloud]".to_string(),
                lang_code: "spa".to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "OpenSubtitles.org (.srt) • Cloud Subtitle".to_string(),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: "French [Cloud]".to_string(),
                lang_code: "fra".to_string(),
                badge: "CLOUD".to_string(),
                is_native: false,
                description: "OpenSubtitles.org (.srt) • Cloud Subtitle".to_string(),
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
                    "Provider native if available, OpenSubtitles fallback [{}]",
                    preferred_name
                ),
                file_or_url: String::new(),
            },
            SubtitleTrackOption {
                label: format!("{} [Auto]", preferred_name),
                lang_code: preferred_lang.to_string(),
                badge: "AUTO".to_string(),
                is_native: true,
                description: "Best available match (Native .vtt or OpenSubtitles .srt)".to_string(),
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
                description: "Automatic multi-provider chain (VixSrc CDN → HiAnime CDN → LookMovie CDN)"
                    .to_string(),
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
            SettingsCategory::General => 5,
            SettingsCategory::Playback => 8,
            SettingsCategory::Providers => 4,
            SettingsCategory::Appearance => 3,
            SettingsCategory::Storage => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsConfirmation {
    ClearPosterCache,
    ClearWatchHistory,
    ExportDiagnostics,
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
        // General (0..5)
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
            label: "TV Episode Order Mode",
            value_str: match cfg.episode_order.to_lowercase().as_str() {
                "streaming" => "Alternate / Production Order (Digital Groups)".to_string(),
                _ => "Broadcast Order (Canonical • Recommended)".to_string(),
            },
            description: "TV episode listing scheme. Broadcast matches canonical TV airings (default across providers). Alternate switches to DVD/digital production order.",
        },
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 3,
            label: "Player Engine",
            value_str: "mpv (External High-Performance Player)".to_string(),
            description: "Hardware-accelerated player engine powering video playback and subtitle sync.",
        },
        SettingMatch {
            category: SettingsCategory::General,
            category_idx: 0,
            item_idx: 4,
            label: "TMDB Metadata API",
            value_str: "Connected (v3 API Active)".to_string(),
            description: "Global metadata provider supplying artwork, cast details, episode titles, and summaries.",
        },

        // Playback (0..7)
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
            label: "Preferred Subtitle Source",
            value_str: match cfg.subtitle_source.to_lowercase().as_str() {
                "provider" => "Provider Native Only (LookMovie .vtt)".to_string(),
                "opensubtitles" => "OpenSubtitles.org Only".to_string(),
                "both" | "all" => "Both (Provider Native + OpenSubtitles)".to_string(),
                _ => "Auto (Provider First, then OpenSubtitles)".to_string(),
            },
            description: "Preferred origin for subtitles. Provider Native uses pre-synced provider tracks; OpenSubtitles offers broader languages.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 4,
            label: "Subtitle Timing Offset",
            value_str: format!("{:+.1}s (Use z/Z in MPV)", cfg.sub_delay),
            description: "Default timing offset applied to subtitles. Use [z] and [Z] inside MPV for on-the-fly +/-100ms sync.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 5,
            label: "Auto-Play Next Episode",
            value_str: if cfg.auto_play_next { "ON (Seamless background prefetch)".to_string() } else { "OFF".to_string() },
            description: "Seamlessly start the next episode upon reaching the conclusion of current episode.",
        },
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 6,
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
        SettingMatch {
            category: SettingsCategory::Playback,
            category_idx: 1,
            item_idx: 7,
            label: "Preferred Stream Quality",
            value_str: match cfg.preferred_quality.to_lowercase().as_str() {
                "480p" => "480p SD (Data Saver)".to_string(),
                "720p" => "720p HD (Balanced)".to_string(),
                "1080p" => "1080p FHD (High Quality)".to_string(),
                _ => "Auto (Best Available)".to_string(),
            },
            description: "Default stream resolution requested from providers and configured in MPV bitrate profiles.",
        },

        // Providers (0..4)
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 0,
            label: "Preferred Provider",
            value_str: match cfg.preferred_provider.to_lowercase().as_str() {
                "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)".to_string(),
                "hianime" => "HiAnime First (Dedicated Anime 1080p CDN)".to_string(),
                "lookmovie" => "LookMovie First (Fallback CDN)".to_string(),
                _ => "Auto Fallback (VixSrc → HiAnime → LookMovie)".to_string(),
            },
            description: "Priority order for streaming servers. Auto Fallback tries VixSrc first, then HiAnime (anime) and LookMovie.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 1,
            label: "Subtitle Source Strategy",
            value_str: match cfg.subtitle_source.to_lowercase().as_str() {
                "provider" => "Provider Native (LookMovie)".to_string(),
                "opensubtitles" => "OpenSubtitles.org".to_string(),
                "both" | "all" => "Both (All Tracks Loaded)".to_string(),
                _ => "Auto (Best Match First)".to_string(),
            },
            description: "Controls whether subtitles are pulled from LookMovie's CDN or fetched externally from OpenSubtitles.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 2,
            label: "Subtitle Augmentation",
            value_str: if cfg.subtitle_augmentation { "ON (Auto-pull from LookMovie)".to_string() } else { "OFF".to_string() },
            description: "When enabled, queries OpenSubtitles tracks if primary stream lacks requested subtitles.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 3,
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
                "half-block" => "Half-Block Universal (▀▄ 24-bit ANSI)".to_string(),
                _ => "High-Resolution TrueColor (Sextant & Quad Subpixels)".to_string(),
            },
            description: "Render high-resolution colored poster previews using Unicode sextants/quads, half-blocks, or text-only fallback.",
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
            label: "Export Diagnostic Report",
            value_str: "[ Export Sanitized Report ]".to_string(),
            description: "Generate a privacy-redacted diagnostic summary for troubleshooting and bug reporting.",
        },
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 4,
            label: "Factory Reset",
            value_str: "[ Reset All to Factory Defaults ]".to_string(),
            description: "Restore all configuration options to their original defaults without deleting history.",
        },
        SettingMatch {
            category: SettingsCategory::Storage,
            category_idx: 4,
            item_idx: 5,
            label: "Initial Setup Wizard",
            value_str: "[ Launch Initial Setup Wizard ]".to_string(),
            description: "Re-run the beginner-friendly setup wizard to reconfigure preferences.",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    Welcome = 0,
    Playback = 1,
    Subtitles = 2,
    Providers = 3,
    Appearance = 4,
    Complete = 5,
}

impl SetupStep {
    pub const ALL: [SetupStep; 6] = [
        SetupStep::Welcome,
        SetupStep::Playback,
        SetupStep::Subtitles,
        SetupStep::Providers,
        SetupStep::Appearance,
        SetupStep::Complete,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            SetupStep::Welcome => "Welcome",
            SetupStep::Playback => "Playback",
            SetupStep::Subtitles => "Audio & Subs",
            SetupStep::Providers => "Providers",
            SetupStep::Appearance => "Appearance",
            SetupStep::Complete => "Ready",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SetupStep::Welcome => "Welcome to KittyMov & Quick Setup",
            SetupStep::Playback => "Video player engine & hardware GPU acceleration",
            SetupStep::Subtitles => "Voice audio tracks, dubbing, and subtitle languages",
            SetupStep::Providers => "Streaming server prioritization and TV episode order",
            SetupStep::Appearance => "Color themes and terminal poster cover art",
            SetupStep::Complete => "Setup overview and essential keyboard shortcuts",
        }
    }

    pub fn item_count(&self) -> usize {
        match self {
            SetupStep::Welcome => 2,
            SetupStep::Playback => 4,
            SetupStep::Subtitles => 5,
            SetupStep::Providers => 4,
            SetupStep::Appearance => 4,
            SetupStep::Complete => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetupWizardState {
    pub current_step: SetupStep,
    pub selected_item_idx: usize,
}

impl Default for SetupWizardState {
    fn default() -> Self {
        Self {
            current_step: SetupStep::Welcome,
            selected_item_idx: 0,
        }
    }
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
    pub playback_loading: Option<PlaybackLoadingState>,
    pub loading_rx: Option<tokio::sync::mpsc::UnboundedReceiver<LoadingProgress>>,
    pub loading_task: Option<tokio::task::JoinHandle<()>>,
    pub subtitles: Arc<crate::subtitles::SubtitleManager>,
    pub error_dialog: Option<ErrorDialogState>,
    pub setup_wizard: Option<SetupWizardState>,
    pub show_help: bool,
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
            playback_loading: None,
            loading_rx: None,
            loading_task: None,
            error_dialog: None,
            setup_wizard: None,
            show_help: false,
            should_quit: false,
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub async fn init(&mut self) {
        if self.setup_wizard.is_some() {
            let tab = match self.cfg.default_tab.to_lowercase().as_str() {
                "series" | "tv" => ActiveTab::Series,
                "history" => ActiveTab::History,
                "bookmarks" => ActiveTab::Bookmarks,
                _ => ActiveTab::Movies,
            };
            self.load_tab(tab).await;
            return;
        }

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
                    )
                    .await;
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
                    })
                    .await;
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
            return;
        }
        if let Some(item) = self.current_items.get(self.selected_index) {
            if !item.poster_path.is_empty() {
                if let Some(path) = self.tmdb.get_cached_poster(&item.poster_path).await {
                    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((120, 40));
                    let inner_w = term_w.saturating_sub(2);
                    let inner_h = term_h.saturating_sub(2);
                    let panel_w = (inner_w as f32 * 0.38) as u16;
                    let (w, h) = calculate_poster_dimensions_for_panel(panel_w, inner_h);
                    self.current_poster_text =
                        render_poster_to_text(&path, w, h, &self.cfg.poster_render_mode);
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
            let req = MediaRequest {
                tmdb_id,
                media_type: "tv".to_string(),
                season: Some(season_num),
                episode: Some(ep_num),
                title,
                year: Some(year),
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
                let req = MediaRequest {
                    tmdb_id: item.tmdb_id,
                    media_type: "movie".to_string(),
                    season: None,
                    episode: None,
                    title: item.title.clone(),
                    year: Some(item.year.clone()),
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

    pub async fn open_episode_modal(&mut self, item: MediaItem) {
        self.status_message = format!("Loading seasons for {}...", item.title);
        if let Ok((_details, seasons, stream_episodes_map)) = self
            .tmdb
            .get_tv_details_with_order(item.tmdb_id, &self.cfg.episode_order)
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

    pub fn cycle_subtitle_source(&mut self) {
        let sources = ["auto", "provider", "opensubtitles", "both"];
        self.cfg.subtitle_source = cycle_list(&sources, &self.cfg.subtitle_source, true);
        let _ = self.cfg.save();
        let label = match self.cfg.subtitle_source.as_str() {
            "provider" => "Provider Native Only",
            "opensubtitles" => "OpenSubtitles Only",
            "both" | "all" => "Both (All Tracks)",
            _ => "Auto (Best Match)",
        };
        self.status_message = format!("Subtitle Source: {}", label);
    }

    pub async fn toggle_episode_order(&mut self) {
        if self.cfg.episode_order.to_lowercase() == "streaming" {
            self.cfg.episode_order = "broadcast".to_string();
            self.status_message = "Episode Order: Broadcast (Canonical • Recommended)".to_string();
        } else {
            self.cfg.episode_order = "streaming".to_string();
            self.status_message =
                "Episode Order: Alternate / Production (Digital Groups)".to_string();
        }
        let _ = self.cfg.save();

        if let Some(ref modal) = self.episode_modal {
            let item = modal.series.clone();
            let curr_s_idx = modal.selected_season_idx;
            self.open_episode_modal(item).await;
            if let Some(ref mut new_modal) = self.episode_modal {
                if curr_s_idx < new_modal.seasons.len() {
                    new_modal.selected_season_idx = curr_s_idx;
                    let season_num = new_modal.seasons[curr_s_idx].season_number;
                    if let Some(ref map) = new_modal.all_season_episodes {
                        new_modal.episodes = map.get(&season_num).cloned().unwrap_or_default();
                    } else if let Ok(eps) = self
                        .tmdb
                        .get_season_episodes(new_modal.series.tmdb_id, season_num)
                        .await
                    {
                        new_modal.episodes = eps;
                    }
                }
            }
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
        self.providers = Arc::new(ProviderManager::with_preference(
            &self.cfg.preferred_provider,
        ));
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
                    s.category_idx = (s.category_idx + SettingsCategory::ALL.len() - 1)
                        % SettingsCategory::ALL.len();
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
                    2 => {
                        let orders = ["broadcast", "streaming"];
                        self.cfg.episode_order =
                            cycle_list(&orders, &self.cfg.episode_order, forward);
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
                        let langs = [
                            "eng", "jpn", "spa", "fra", "deu", "ita", "por", "rus", "kor", "zho",
                        ];
                        self.cfg.audio_language =
                            cycle_list(&langs, &self.cfg.audio_language, forward);
                    }
                    2 => {
                        let langs = [
                            "eng", "spa", "fra", "deu", "ita", "jpn", "por", "rus", "kor", "zho",
                        ];
                        self.cfg.sub_language = cycle_list(&langs, &self.cfg.sub_language, forward);
                    }
                    3 => {
                        let sources = ["auto", "provider", "opensubtitles", "both"];
                        self.cfg.subtitle_source =
                            cycle_list(&sources, &self.cfg.subtitle_source, forward);
                    }
                    4 => {
                        let delta = if forward { 0.1 } else { -0.1 };
                        let next = (self.cfg.sub_delay + delta).clamp(-10.0, 10.0);
                        self.cfg.sub_delay = (next * 10.0).round() / 10.0;
                    }
                    5 => {
                        self.cfg.auto_play_next = !self.cfg.auto_play_next;
                    }
                    6 => {
                        let hwdecs = ["auto", "auto-safe", "vaapi", "nvdec", "videotoolbox", "no"];
                        self.cfg.hwdec = cycle_list(&hwdecs, &self.cfg.hwdec, forward);
                    }
                    7 => {
                        let qualities = ["auto", "1080p", "720p", "480p"];
                        self.cfg.preferred_quality =
                            cycle_list(&qualities, &self.cfg.preferred_quality, forward);
                    }
                    _ => {}
                },
                SettingsCategory::Providers => match s.item_idx {
                    0 => {
                        let providers = ["auto", "vixsrc", "hianime", "lookmovie"];
                        self.cfg.preferred_provider =
                            cycle_list(&providers, &self.cfg.preferred_provider, forward);
                    }
                    1 => {
                        let sources = ["auto", "provider", "opensubtitles", "both"];
                        self.cfg.subtitle_source =
                            cycle_list(&sources, &self.cfg.subtitle_source, forward);
                    }
                    2 => {
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
                        let modes = ["high-res", "half-block", "off"];
                        self.cfg.poster_render_mode =
                            cycle_list(&modes, &self.cfg.poster_render_mode, forward);
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
                        s.confirmation = Some(SettingsConfirmation::ExportDiagnostics);
                    }
                    4 => {
                        s.confirmation = Some(SettingsConfirmation::ResetDefaults);
                    }
                    5 => {
                        self.settings_modal = None;
                        self.open_setup_wizard();
                        return;
                    }
                    _ => {}
                },
            }
            let _ = self.cfg.save();
            self.providers = Arc::new(ProviderManager::with_preference(
                &self.cfg.preferred_provider,
            ));
        }
    }

    pub async fn settings_execute_confirmation(&mut self) {
        if let Some(ref mut s) = self.settings_modal {
            if let Some(ref conf) = s.confirmation.clone() {
                match conf {
                    SettingsConfirmation::ClearPosterCache => {
                        match crate::config::clear_poster_cache() {
                            Ok(count) => {
                                s.feedback_message =
                                    Some(format!("Poster cache cleared ({} files removed)", count));
                            }
                            Err(e) => {
                                s.feedback_message = Some(format!("Failed to clear cache: {}", e));
                            }
                        }
                    }
                    SettingsConfirmation::ClearWatchHistory => match self.db.clear_history() {
                        Ok(count) => {
                            s.feedback_message =
                                Some(format!("Watch history cleared ({} records removed)", count));
                        }
                        Err(e) => {
                            s.feedback_message = Some(format!("Failed to clear history: {}", e));
                        }
                    },
                    SettingsConfirmation::ExportDiagnostics => {
                        match crate::diagnostics::generate_diagnostic_report(&self.cfg, &self.db)
                            .await
                        {
                            Ok(path) => {
                                s.feedback_message =
                                    Some(format!("Exported diagnostic report: {}", path.display()));
                            }
                            Err(e) => {
                                s.feedback_message =
                                    Some(format!("Failed to export report: {}", e));
                            }
                        }
                    }
                    SettingsConfirmation::ResetDefaults => {
                        self.cfg.reset_to_defaults();
                        let _ = self.cfg.save();
                        self.providers = Arc::new(ProviderManager::with_preference(
                            &self.cfg.preferred_provider,
                        ));
                        s.feedback_message =
                            Some("All settings restored to factory defaults".to_string());
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
                    self.cfg.episode_order = "broadcast".to_string();
                }
                SettingsCategory::Playback => {
                    self.cfg.audio_mode = "sub".to_string();
                    self.cfg.audio_language = "eng".to_string();
                    self.cfg.sub_language = "eng".to_string();
                    self.cfg.subtitle_source = "auto".to_string();
                    self.cfg.sub_delay = 0.0;
                    self.cfg.auto_play_next = true;
                    self.cfg.hwdec = "auto".to_string();
                    self.cfg.preferred_quality = "1080p".to_string();
                }
                SettingsCategory::Providers => {
                    self.cfg.preferred_provider = "auto".to_string();
                    self.cfg.subtitle_source = "auto".to_string();
                    self.cfg.subtitle_augmentation = true;
                }
                SettingsCategory::Appearance => {
                    self.cfg.theme_name = "cyber".to_string();
                    self.cfg.poster_render_mode = "high-res".to_string();
                }
                SettingsCategory::Storage => {
                    s.confirmation = Some(SettingsConfirmation::ResetDefaults);
                    return;
                }
            }
            let _ = self.cfg.save();
            self.providers = Arc::new(ProviderManager::with_preference(
                &self.cfg.preferred_provider,
            ));
            s.feedback_message = Some(format!("Reset {} settings to defaults", cat.title()));
        }
    }

    pub fn open_setup_wizard(&mut self) {
        self.setup_wizard = Some(SetupWizardState::default());
        self.status_message = "Initial Setup Wizard Active".to_string();
    }

    pub fn setup_wizard_next_item(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let count = wizard.current_step.item_count();
            if count > 0 && wizard.selected_item_idx + 1 < count {
                wizard.selected_item_idx += 1;
            }
        }
    }

    pub fn setup_wizard_prev_item(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            if wizard.selected_item_idx > 0 {
                wizard.selected_item_idx -= 1;
            }
        }
    }

    pub fn setup_wizard_change_value(&mut self, forward: bool) {
        let (step, item_idx) = match self.setup_wizard {
            Some(ref w) => (w.current_step, w.selected_item_idx),
            None => return,
        };

        match step {
            SetupStep::Welcome => {
                if forward {
                    self.setup_wizard_next_step();
                }
            }
            SetupStep::Playback => match item_idx {
                0 => {
                    let modes = ["auto", "vaapi", "nvdec", "vdpau", "no"];
                    self.cfg.hwdec = cycle_list(&modes, &self.cfg.hwdec, forward);
                }
                1 => {
                    self.cfg.resume_on_startup = !self.cfg.resume_on_startup;
                }
                2 => {
                    self.cfg.auto_play_next = !self.cfg.auto_play_next;
                }
                3 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Subtitles => match item_idx {
                0 => {
                    let langs = [
                        "eng", "jpn", "spa", "fra", "deu", "ita", "por", "kor", "zho",
                    ];
                    self.cfg.audio_language = cycle_list(&langs, &self.cfg.audio_language, forward);
                }
                1 => {
                    let modes = ["sub", "dub"];
                    self.cfg.audio_mode = cycle_list(&modes, &self.cfg.audio_mode, forward);
                }
                2 => {
                    let langs = [
                        "eng", "spa", "fra", "deu", "ita", "por", "ara", "ind", "jpn",
                    ];
                    self.cfg.sub_language = cycle_list(&langs, &self.cfg.sub_language, forward);
                }
                3 => {
                    self.cfg.subtitle_augmentation = !self.cfg.subtitle_augmentation;
                }
                4 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Providers => match item_idx {
                0 => {
                    let provs = ["auto", "vixsrc", "hianime", "lookmovie"];
                    self.cfg.preferred_provider =
                        cycle_list(&provs, &self.cfg.preferred_provider, forward);
                }
                1 => {
                    let orders = ["broadcast", "streaming"];
                    self.cfg.episode_order = cycle_list(&orders, &self.cfg.episode_order, forward);
                }
                2 => {
                    let sources = ["auto", "provider", "opensubtitles"];
                    self.cfg.subtitle_source =
                        cycle_list(&sources, &self.cfg.subtitle_source, forward);
                }
                3 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Appearance => match item_idx {
                0 => {
                    let themes = ["cyber", "matrix", "synthwave", "dracula", "nord"];
                    self.cfg.theme_name = cycle_list(&themes, &self.cfg.theme_name, forward);
                }
                1 => {
                    let modes = ["high-res", "half-block", "off"];
                    self.cfg.poster_render_mode =
                        cycle_list(&modes, &self.cfg.poster_render_mode, forward);
                }
                2 => {
                    let tabs = ["movies", "series", "bookmarks", "history"];
                    self.cfg.default_tab = cycle_list(&tabs, &self.cfg.default_tab, forward);
                }
                3 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Complete => {
                if item_idx == 0 {
                    self.setup_wizard_finish();
                } else {
                    self.setup_wizard_set_step(SetupStep::Welcome);
                }
            }
        }
    }

    pub fn setup_wizard_confirm(&mut self) {
        let (step, item_idx) = match self.setup_wizard {
            Some(ref w) => (w.current_step, w.selected_item_idx),
            None => return,
        };

        match step {
            SetupStep::Welcome => {
                if item_idx == 0 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_finish();
                }
            }
            SetupStep::Playback => {
                if item_idx == 3 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Subtitles => {
                if item_idx == 4 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Providers => {
                if item_idx == 3 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Appearance => {
                if item_idx == 3 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Complete => {
                if item_idx == 0 {
                    self.setup_wizard_finish();
                } else {
                    self.setup_wizard_set_step(SetupStep::Welcome);
                }
            }
        }
    }

    pub fn setup_wizard_next_step(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let cur = wizard.current_step as usize;
            if cur + 1 < SetupStep::ALL.len() {
                wizard.current_step = SetupStep::ALL[cur + 1];
                wizard.selected_item_idx = 0;
            } else {
                self.setup_wizard_finish();
            }
        }
    }

    pub fn setup_wizard_prev_step(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let cur = wizard.current_step as usize;
            if cur > 0 {
                wizard.current_step = SetupStep::ALL[cur - 1];
                wizard.selected_item_idx = 0;
            }
        }
    }

    pub fn setup_wizard_set_step(&mut self, step: SetupStep) {
        if let Some(ref mut wizard) = self.setup_wizard {
            wizard.current_step = step;
            wizard.selected_item_idx = 0;
        }
    }

    pub fn setup_wizard_finish(&mut self) {
        let _ = self.cfg.save();
        self.providers = Arc::new(ProviderManager::with_preference(
            &self.cfg.preferred_provider,
        ));
        self.setup_wizard = None;
        self.status_message = "Welcome to KittyMov! Preferences saved.".to_string();
    }

    pub fn setup_wizard_cancel(&mut self) {
        let _ = self.cfg.save();
        self.setup_wizard = None;
        self.status_message = "Setup skipped. Default configuration active.".to_string();
    }

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
                let request = MediaRequest {
                    tmdb_id,
                    media_type: "tv".to_string(),
                    season: Some(season_num),
                    episode: Some(ep_num),
                    title,
                    year: Some(year),
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

    pub async fn start_playback_loading(
        &mut self,
        req: MediaRequest,
        poster_path: String,
        provider_choice: String,
        quality_choice: Option<String>,
        subtitle_choice: Option<SubtitleTrackOption>,
        catalog_item: Option<MediaItem>,
    ) {
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
            // STEP 1: Metadata Resolution & Episode mapping
            let mut mapped_req = req_clone.clone();
            if mapped_req.media_type == "tv" {
                if let (Some(s), Some(e)) = (mapped_req.season, mapped_req.episode) {
                    if (provider_choice == "vixsrc" || provider_choice == "auto")
                        && cfg.episode_order.to_lowercase() == "broadcast"
                    {
                        if let Ok((_info, _seasons, Some(stream_map))) = tmdb
                            .get_tv_details_with_order(mapped_req.tmdb_id, "streaming")
                            .await
                        {
                            if let Ok(canon_eps) =
                                tmdb.get_season_episodes(mapped_req.tmdb_id, s).await
                            {
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
                    }
                }
            }

            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::ResolvingMetadata,
                status: StepStatus::Completed,
                detail: format!("Found canonical title index for '{}'", disp_title_clone),
            });

            // STEP 2: Connecting Provider
            let active_provider = provider_choice.clone();
            let prov_label = match active_provider.as_str() {
                "vixsrc" => "VixSrc Server",
                "hianime" => "HiAnime Server",
                "lookmovie" => "LookMovie Server",
                _ => "Auto Fallback Engine",
            };
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::ConnectingProvider,
                status: StepStatus::Active,
                detail: format!("Connecting to {} CDN...", prov_label),
            });

            let effective_quality = quality_choice_clone
                .as_deref()
                .or(Some(cfg.preferred_quality.as_str()));

            let resolve_result = providers
                .resolve_with_choice_and_quality(&active_provider, &mapped_req, effective_quality)
                .await;
            let mut stream = match resolve_result {
                Ok(s) => s,
                Err(err_primary) => {
                    if active_provider == "auto" {
                        // In auto mode, ProviderManager's resolve_with_fallback already chained vixsrc -> hianime -> lookmovie
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::ConnectingProvider,
                            status: StepStatus::Failed,
                            detail: format!("All providers failed: {}", err_primary),
                        });
                        let _ = tx.send(LoadingProgress::Failed(format!(
                            "All providers failed: {}",
                            err_primary
                        )));
                        return;
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
                            .resolve_with_choice_and_quality("hianime", &req_clone, effective_quality)
                            .await
                        {
                            Ok(s) => s,
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
                                    .resolve_with_choice_and_quality("lookmovie", &req_clone, effective_quality)
                                    .await
                                {
                                    Ok(s) => s,
                                    Err(err_look) => {
                                        let _ = tx.send(LoadingProgress::StepUpdate {
                                            step: LoadingStep::ConnectingProvider,
                                            status: StepStatus::Failed,
                                            detail: format!("All providers failed: {}", err_look),
                                        });
                                        let _ = tx.send(LoadingProgress::Failed(format!(
                                            "Providers failed: VixSrc ({}), HiAnime ({}), LookMovie ({})",
                                            err_primary, err_hianime, err_look
                                        )));
                                        return;
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
                            .resolve_with_choice_and_quality("lookmovie", &req_clone, effective_quality)
                            .await
                        {
                            Ok(s) => s,
                            Err(err_fallback) => {
                                let _ = tx.send(LoadingProgress::StepUpdate {
                                    step: LoadingStep::ConnectingProvider,
                                    status: StepStatus::Failed,
                                    detail: format!("LookMovie fallback failed: {}", err_fallback),
                                });
                                let _ = tx.send(LoadingProgress::Failed(format!(
                                    "Both providers failed: HiAnime ({}), LookMovie ({})",
                                    err_primary, err_fallback
                                )));
                                return;
                            }
                        }
                    } else {
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::ConnectingProvider,
                            status: StepStatus::Failed,
                            detail: format!("Provider connection failed: {}", err_primary),
                        });
                        let _ = tx.send(LoadingProgress::Failed(format!(
                            "Provider failed: {}",
                            err_primary
                        )));
                        return;
                    }
                }
            };

            let quality_label = stream.quality.as_deref().unwrap_or("Auto");
            let detail_msg = if stream.provider_name.to_lowercase().contains("lookmovie") {
                if let Some(req_q) = effective_quality {
                    if (req_q.contains("1080") || req_q.contains("720")) && quality_label.contains("480") {
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
                format!("CDN Stream live: {} [{}]", stream.provider_name, quality_label)
            };

            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::ConnectingProvider,
                status: StepStatus::Completed,
                detail: detail_msg,
            });

            // STEP 3: Subtitles resolution
            let _ = tx.send(LoadingProgress::StepUpdate {
                step: LoadingStep::FetchingSubtitles,
                status: StepStatus::Active,
                detail: "Analyzing subtitle inventory...".to_string(),
            });

            let pref_sub_lang = cfg.sub_language.clone();
            if let Some(ref sub_opt) = subtitle_choice {
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
                        detail: format!(
                            "Fetching OpenSubtitles cloud track [{}]...",
                            sub_opt.lang_code
                        ),
                    });
                    let ext = subtitles_mgr
                        .prepare_subtitles(
                            &tmdb,
                            req_clone.tmdb_id,
                            &req_clone.title,
                            &req_clone.media_type,
                            req_clone.season,
                            req_clone.episode,
                            &sub_opt.lang_code,
                        )
                        .await;
                    if !ext.is_empty() {
                        stream.subtitles = ext;
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::FetchingSubtitles,
                            status: StepStatus::Completed,
                            detail: format!(
                                "Loaded OpenSubtitles.org cloud tracks [{}]",
                                sub_opt.lang_code
                            ),
                        });
                    } else if !stream.subtitles.is_empty() {
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::FetchingSubtitles,
                            status: StepStatus::Completed,
                            detail: "Cloud track not found; using provider native track"
                                .to_string(),
                        });
                    } else {
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::FetchingSubtitles,
                            status: StepStatus::Completed,
                            detail: "No subtitles found for requested language".to_string(),
                        });
                    }
                }
            } else {
                // Automatic: if stream has native subtitles (LookMovie .vtt), use them;
                // if it doesn't (VixSrc), automatically query OpenSubtitles!
                if !stream.subtitles.is_empty() {
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
                            "Stream has no native subtitles; auto-fetching OpenSubtitles [{}]...",
                            pref_sub_lang
                        ),
                    });
                    let ext = subtitles_mgr
                        .prepare_subtitles(
                            &tmdb,
                            req_clone.tmdb_id,
                            &req_clone.title,
                            &req_clone.media_type,
                            req_clone.season,
                            req_clone.episode,
                            &pref_sub_lang,
                        )
                        .await;
                    if !ext.is_empty() {
                        stream.subtitles = ext;
                        let _ = tx.send(LoadingProgress::StepUpdate {
                            step: LoadingStep::FetchingSubtitles,
                            status: StepStatus::Completed,
                            detail: format!(
                                "Auto-loaded OpenSubtitles.org cloud tracks [{}]",
                                pref_sub_lang
                            ),
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

            // STEP 4: Launching player
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
        if let Some(st) = self.playback_loading.take() {
            if let Some(h) = self.loading_task.take() {
                h.abort();
            }
            self.loading_rx = None;
            self.start_playback_loading(
                st.request,
                st.poster_path,
                "lookmovie".to_string(),
                None,
                None,
                None,
            )
            .await;
        }
    }

    pub async fn execute_player(&mut self, stream: &StreamCandidate, title: &str) {
        self.playback_loading = None;
        self.launch_mpv_session(stream, title).await;
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

        self.status_message = format!("Searching OpenSubtitles for {}...", target_title);
        let imdb_id = self.tmdb.get_imdb_id(tmdb_id, &media_type).await.ok();
        let tracks = self
            .subtitles
            .fetch_all_subtitles(imdb_id.as_deref(), &media_type, season, episode)
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
        };

        match self
            .providers
            .resolve_with_choice_and_quality(
                provider_choice,
                &req,
                Some(&self.cfg.preferred_quality),
            )
            .await
        {
            Ok(mut stream) => {
                // Subtitle source selection (provider native vs OpenSubtitles vs both vs auto)
                let sub_src = self.cfg.subtitle_source.to_lowercase();
                let should_fetch_ext = match sub_src.as_str() {
                    "provider" => stream.subtitles.is_empty() && self.cfg.subtitle_augmentation,
                    "opensubtitles" | "both" | "all" => true,
                    _ => stream.subtitles.is_empty() || self.cfg.subtitle_augmentation,
                };

                if sub_src == "opensubtitles" {
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
                        )
                        .await;
                    if !external_subs.is_empty() {
                        if stream.subtitles.is_empty() {
                            stream.subtitles = external_subs;
                        } else {
                            // Preserve provider-native subtitles (exact sync) and append external tracks
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
        };

        match self
            .providers
            .resolve_with_choice_and_quality(
                provider_choice,
                &req,
                Some(&self.cfg.preferred_quality),
            )
            .await
        {
            Ok(mut stream) => {
                // Subtitle source selection (provider native vs OpenSubtitles vs both vs auto)
                let sub_src = self.cfg.subtitle_source.to_lowercase();
                let should_fetch_ext = match sub_src.as_str() {
                    "provider" => stream.subtitles.is_empty() && self.cfg.subtitle_augmentation,
                    "opensubtitles" | "both" | "all" => true,
                    _ => stream.subtitles.is_empty() || self.cfg.subtitle_augmentation,
                };

                if sub_src == "opensubtitles" {
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
                        )
                        .await;
                    if !external_subs.is_empty() {
                        if stream.subtitles.is_empty() {
                            stream.subtitles = external_subs;
                        } else {
                            // Preserve provider-native subtitles (exact sync) and append external tracks
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

                // Check anti-expiration
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
        // Temporarily release terminal for MPV
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

        // Restore terminal for Ratatui
        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();

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

    /// Full Auto-Play Next Episode Engine with background prefetching and JIT token refreshing
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
                                // When progress reaches 80% (or last 3 minutes), start pre-resolving next episode
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

                                    if let Ok(mut next_stream) =
                                        self.providers.resolve_with_fallback(&next_req).await
                                    {
                                        // JIT token refresh to ensure no expired links
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
                                                )
                                                .await;
                                            if !external_subs.is_empty() {
                                                next_stream.subtitles = external_subs;
                                            }
                                        }

                                        // Seamlessly inject next episode into running MPV window!
                                        if let Some(ref embed) = next_stream.embed_url {
                                            let _ = controller
                                                .send_command(&["set_property", "referrer", embed])
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

fn default_provider_options() -> Vec<ProviderOption> {
    vec![
        ProviderOption {
            id: "auto".to_string(),
            name: "Auto Fallback Engine".to_string(),
            description: "Fast multi-provider chain (VixSrc CDN → HiAnime CDN → LookMovie CDN)"
                .to_string(),
        },
        ProviderOption {
            id: "vixsrc".to_string(),
            name: "VixSrc Server".to_string(),
            description: "Primary Ultra-Fast CDN • 1080p FHD HLS • Multi-Audio".to_string(),
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
        assert_eq!(SettingsCategory::General.item_count(), 5);
        assert_eq!(SettingsCategory::Playback.item_count(), 8);
        assert_eq!(SettingsCategory::Providers.item_count(), 4);
        assert_eq!(SettingsCategory::Appearance.item_count(), 3);
        assert_eq!(SettingsCategory::Storage.item_count(), 6);
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
        assert_eq!(
            app.settings_modal.as_ref().unwrap().active_pane,
            SettingsPane::Content
        );

        // Switch back to Categories
        app.settings_switch_pane();
        assert_eq!(
            app.settings_modal.as_ref().unwrap().active_pane,
            SettingsPane::Categories
        );

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
        assert_eq!(app.settings_modal.as_ref().unwrap().item_idx, 5); // Storage has 6 items (0..5)
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
        assert!(!app.cfg.resume_on_startup);
        app.settings_change_value(true);
        assert!(app.cfg.resume_on_startup);

        app.settings_modal.as_mut().unwrap().item_idx = 2; // TV Episode Order
        assert_eq!(app.cfg.episode_order, "broadcast");
        app.settings_change_value(true);
        assert_eq!(app.cfg.episode_order, "streaming");

        // Playback Category (category_idx = 1)
        app.settings_modal.as_mut().unwrap().category_idx = 1;
        app.settings_modal.as_mut().unwrap().item_idx = 0; // Audio mode
        let initial_mode = app.cfg.audio_mode.clone();
        app.settings_change_value(true);
        assert_ne!(app.cfg.audio_mode, initial_mode);

        app.settings_modal.as_mut().unwrap().item_idx = 3; // Subtitle Source
        assert_eq!(app.cfg.subtitle_source, "auto");
        app.settings_change_value(true);
        assert_eq!(app.cfg.subtitle_source, "provider");

        app.settings_modal.as_mut().unwrap().item_idx = 4; // Subtitle Delay
        assert_eq!(app.cfg.sub_delay, 0.0);
        app.settings_change_value(true);
        assert!((app.cfg.sub_delay - 0.1).abs() < 0.001);

        app.settings_modal.as_mut().unwrap().item_idx = 5; // Auto-play next
        let initial_auto = app.cfg.auto_play_next;
        app.settings_change_value(true);
        assert_eq!(app.cfg.auto_play_next, !initial_auto);
    }

    #[test]
    fn test_settings_category_reset() {
        let cfg = Config {
            default_tab: "history".to_string(),
            resume_on_startup: true,
            ..Config::default()
        };

        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);
        app.toggle_settings();

        // Reset General Category
        app.settings_modal.as_mut().unwrap().category_idx = 0;
        app.settings_reset_current_category();

        assert_eq!(app.cfg.default_tab, "movies");
        assert!(!app.cfg.resume_on_startup);
        assert!(app
            .settings_modal
            .as_ref()
            .unwrap()
            .feedback_message
            .is_some());
    }

    #[test]
    fn test_settings_search_filtering() {
        let cfg = Config::default();

        let sub_matches = search_settings_matches(&cfg, "sub");
        assert!(!sub_matches.is_empty());
        assert!(sub_matches
            .iter()
            .any(|m| m.label.to_lowercase().contains("subtitle")));

        let theme_matches = search_settings_matches(&cfg, "theme");
        assert!(!theme_matches.is_empty());
        assert_eq!(theme_matches[0].category, SettingsCategory::Appearance);

        let empty_matches = search_settings_matches(&cfg, "nonexistentquery12345");
        assert!(empty_matches.is_empty());
    }

    #[tokio::test]
    async fn test_settings_confirmation_workflow() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);
        app.toggle_settings();

        // Go to Storage -> Reset Defaults (item 4)
        app.settings_modal.as_mut().unwrap().category_idx = 4;
        app.settings_modal.as_mut().unwrap().active_pane = SettingsPane::Content;
        app.settings_modal.as_mut().unwrap().item_idx = 4; // Reset Defaults action

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
        app.settings_execute_confirmation().await;
        assert_eq!(app.settings_modal.as_ref().unwrap().confirmation, None);
        assert!(app
            .settings_modal
            .as_ref()
            .unwrap()
            .feedback_message
            .is_some());
    }

    #[test]
    fn test_provider_modal_state_navigation_and_dependent_tracks() {
        let req = MediaRequest {
            tmdb_id: 12345,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Test Movie".to_string(),
            year: Some("2024".to_string()),
        };

        let mut modal = ProviderSelectModalState::new(
            "Test Movie (2024)".to_string(),
            req,
            "/test.jpg".to_string(),
            "auto",
            "eng",
            "1080p",
        );

        assert_eq!(modal.active_category, ProviderModalCategory::Provider);
        assert_eq!(modal.active_pane, ProviderModalPane::Categories);

        // Switch to lookmovie
        let look_idx = modal
            .options
            .iter()
            .position(|o| o.id == "lookmovie")
            .expect("lookmovie should be an option");
        modal.selected_idx = look_idx;
        modal.update_dependent_tracks("eng");

        // Verify sources and qualities for LookMovie
        assert!(modal.sources.iter().any(|s| s.name.contains("Storage")));
        assert!(modal.qualities.iter().any(|q| q.label.contains("1080p")));
        // Verify native subtitles are present for LookMovie
        assert!(modal
            .subtitles
            .iter()
            .any(|s| s.is_native && s.badge == "NATIVE"));

        // Switch to vixsrc
        let vix_idx = modal
            .options
            .iter()
            .position(|o| o.id == "vixsrc")
            .expect("vixsrc should be an option");
        modal.selected_idx = vix_idx;
        modal.update_dependent_tracks("eng");

        // Verify VixSrc sources and subtitles
        assert!(modal.sources.iter().any(|s| s.name.contains("Vanguard")));
        assert!(modal
            .subtitles
            .iter()
            .any(|s| !s.is_native && s.badge == "CLOUD"));

        // Switch to hianime
        let hi_idx = modal
            .options
            .iter()
            .position(|o| o.id == "hianime")
            .expect("hianime should be an option");
        modal.selected_idx = hi_idx;
        modal.update_dependent_tracks("eng");

        // Verify HiAnime sources and qualities
        assert!(modal.sources.iter().any(|s| s.name.contains("ZokoAnime")));
        assert!(modal.qualities.iter().any(|q| q.label.contains("1080p FHD")));
        assert!(modal
            .subtitles
            .iter()
            .any(|s| s.is_native && s.badge == "NATIVE"));
    }

    #[test]
    fn test_playback_loading_state_stepper_and_poll() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);

        let req = MediaRequest {
            tmdb_id: 9999,
            media_type: "movie".to_string(),
            season: None,
            episode: None,
            title: "Cinematic Demo".to_string(),
            year: Some("2024".to_string()),
        };

        let initial_steps = vec![
            StepProgressItem {
                step: LoadingStep::ResolvingMetadata,
                status: StepStatus::Active,
                title: "Resolving",
                detail: "Canonical streams".to_string(),
            },
            StepProgressItem {
                step: LoadingStep::ConnectingProvider,
                status: StepStatus::Pending,
                title: "Providers",
                detail: "Connecting".to_string(),
            },
        ];

        app.playback_loading = Some(PlaybackLoadingState {
            title: "Cinematic Demo".to_string(),
            subtitle: "2024".to_string(),
            media_type: "movie".to_string(),
            provider_name: "Auto Fallback".to_string(),
            current_step: LoadingStep::ResolvingMetadata,
            steps: initial_steps,
            session_autoskip: true,
            session_autoplay: true,
            session_audio_mode: "dub".to_string(),
            poster_text: None,
            year: "2024".to_string(),
            genre: "Sci-Fi".to_string(),
            score: 9.0,
            episodes_count: None,
            studio: "Studio".to_string(),
            synopsis: "Demo".to_string(),
            up_next: None,
            spinner_idx: 0,
            request: req,
            poster_path: String::new(),
        });

        assert_eq!(app.playback_loading.as_ref().unwrap().spinner_idx, 0);
        let _ = app.poll_playback_loading();
        assert_eq!(app.playback_loading.as_ref().unwrap().spinner_idx, 1);

        app.cancel_playback_loading();
        assert!(app.playback_loading.is_none());
        assert_eq!(app.status_message, "Playback cancelled");
    }

    #[tokio::test]
    async fn test_broadcast_order_default_and_toggle() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);

        assert_eq!(app.cfg.episode_order, "broadcast");

        app.toggle_episode_order().await;
        assert_eq!(app.cfg.episode_order, "streaming");
        assert!(app.status_message.contains("Alternate / Production"));

        app.toggle_episode_order().await;
        assert_eq!(app.cfg.episode_order, "broadcast");
        assert!(app.status_message.contains("Broadcast"));
    }

    #[tokio::test]
    async fn test_setup_wizard_flow_and_navigation() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);

        assert!(app.setup_wizard.is_none());
        app.open_setup_wizard();
        assert!(app.setup_wizard.is_some());
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Welcome
        );

        // Advance to Playback
        app.setup_wizard_next_step();
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Playback
        );

        // Change hwdec value
        let initial_hwdec = app.cfg.hwdec.clone();
        app.setup_wizard_change_value(true);
        assert_ne!(app.cfg.hwdec, initial_hwdec);

        // Advance to Subtitles
        app.setup_wizard_next_step();
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Subtitles
        );

        // Advance to Providers
        app.setup_wizard_next_step();
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Providers
        );

        // Advance to Appearance
        app.setup_wizard_next_step();
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Appearance
        );

        // Advance to Complete
        app.setup_wizard_next_step();
        assert_eq!(
            app.setup_wizard.as_ref().unwrap().current_step,
            SetupStep::Complete
        );

        // Finish
        app.setup_wizard_finish();
        assert!(app.setup_wizard.is_none());
        assert!(app.status_message.contains("Welcome to KittyMov"));
    }

    #[tokio::test]
    async fn test_setup_wizard_prevents_auto_resume() {
        let cfg = Config {
            resume_on_startup: true,
            ..Config::default()
        };
        let db = Database::in_memory().unwrap();
        // Record watch history
        let _ = db.record_watch(
            101,
            "movie",
            "Sample Watch",
            "2024",
            "",
            None,
            None,
            None,
            120,
            3600,
        );

        let mut app = App::new(cfg, db);
        app.open_setup_wizard();
        assert!(app.setup_wizard.is_some());

        // Call init() while setup wizard is open
        app.init().await;

        // Auto-resume should NOT have launched playback
        assert!(app.playback_loading.is_none());
        assert!(app.setup_wizard.is_some());
    }
}
