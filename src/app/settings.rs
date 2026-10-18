use std::sync::Arc;

use crate::config::Config;
use crate::providers::ProviderManager;

use super::mod_utils::{cycle_list, lang_name};
use super::App;

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
            SettingsCategory::Playback => 8,
            SettingsCategory::Providers => 5,
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
            value_str: if cfg.resume_on_startup {
                "ON".to_string()
            } else {
                "OFF".to_string()
            },
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
                "wyzie" | "opensubtitles" => "Wyzie Subs Only (Ad-free & Aggregated)".to_string(),
                "both" | "all" => "Both (Provider Native + Wyzie Subs)".to_string(),
                _ => "Auto (Provider First, then Wyzie Subs)".to_string(),
            },
            description: "Preferred origin for subtitles. Provider Native uses pre-synced provider tracks; Wyzie Subs aggregates ad-free subtitles.",
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
            value_str: if cfg.auto_play_next {
                "ON (Seamless background prefetch)".to_string()
            } else {
                "OFF".to_string()
            },
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
                "vidlink" => "VidLink First (HD 1080p Direct + Subs)".to_string(),
                "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)".to_string(),
                "hianime" => "HiAnime First (Dedicated Anime 1080p CDN)".to_string(),
                "lookmovie" => "LookMovie First (Fallback CDN)".to_string(),
                _ => "Auto Fallback (VidLink → VixSrc → HiAnime → LookMovie)".to_string(),
            },
            description: "Priority order for streaming servers. Auto Fallback tries VidLink first, then VixSrc, HiAnime, and LookMovie.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 1,
            label: "Subtitle Source Strategy",
            value_str: match cfg.subtitle_source.to_lowercase().as_str() {
                "provider" => "Provider Native (LookMovie)".to_string(),
                "wyzie" | "opensubtitles" => "Wyzie Subs".to_string(),
                "both" | "all" => "Both (All Tracks Loaded)".to_string(),
                _ => "Auto (Best Match First)".to_string(),
            },
            description: "Controls whether subtitles are pulled from stream CDN or fetched externally from Wyzie Subs.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 2,
            label: "Subtitle Augmentation",
            value_str: if cfg.subtitle_augmentation {
                "ON (Auto-pull from LookMovie)".to_string()
            } else {
                "OFF".to_string()
            },
            description: "When enabled, queries Wyzie Subs tracks if primary stream lacks requested subtitles.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 3,
            label: "Wyzie Subs API Key",
            value_str: if cfg.wyzie_api_key.trim().is_empty() {
                "Unset (Free key at store.wyzie.io/redeem)".to_string()
            } else {
                format!(
                    "Active ({}••••)",
                    &cfg.wyzie_api_key[..cfg.wyzie_api_key.len().min(6)]
                )
            },
            description: "Personal API key for Wyzie Subs ad-free subtitle service.",
        },
        SettingMatch {
            category: SettingsCategory::Providers,
            category_idx: 2,
            item_idx: 4,
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
                "kitty" => "Kitty Protocol (Pixel-Perfect Native Graphics)".to_string(),
                "half-block" => "Half-Block Universal (▀▄ 24-bit ANSI)".to_string(),
                "high-res" => "High-Resolution TrueColor (Sextants & Quads)".to_string(),
                _ => {
                    if crate::ui::poster::is_kitty_graphics_supported() {
                        "Auto (Kitty Protocol Active)".to_string()
                    } else {
                        "Auto (High-Resolution Sextants)".to_string()
                    }
                }
            },
            description: "Render high-resolution colored poster previews using Kitty graphics protocol, Unicode sextants/quads, or half-blocks.",
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
            value_str: crate::config::get_config_dir()
                .join("config.json")
                .to_string_lossy()
                .to_string(),
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

impl App {
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
                        let sources = ["auto", "provider", "wyzie", "both"];
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
                        let providers = ["auto", "vidlink", "vixsrc", "hianime", "lookmovie"];
                        self.cfg.preferred_provider =
                            cycle_list(&providers, &self.cfg.preferred_provider, forward);
                    }
                    1 => {
                        let sources = ["auto", "provider", "wyzie", "both"];
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
                        let modes = ["auto", "kitty", "high-res", "half-block", "off"];
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
                    self.cfg.poster_render_mode = "auto".to_string();
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
}
