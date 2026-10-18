pub mod events;
pub mod modals;
pub mod mod_utils;
pub mod navigation;
pub mod playback;
pub mod settings;
pub mod state;
pub mod wizard;

pub use mod_utils::*;
#[allow(unused_imports)]
pub use playback::*;
pub use settings::*;
pub use state::*;
pub use wizard::*;

use std::sync::Arc;

use crate::config::Config;
use crate::metadata::{MediaItem, TmdbClient};
use crate::providers::ProviderManager;
use crate::storage::Database;

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
    pub current_poster_kitty: Option<Vec<u8>>,
    pub kitty_poster_dirty: bool,
    pub had_kitty_poster_drawn: bool,
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
            current_poster_kitty: None,
            kitty_poster_dirty: false,
            had_kitty_poster_drawn: false,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::providers::MediaRequest;
    use crate::storage::Database;

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
        assert_eq!(SettingsCategory::Playback.item_count(), 8);
        assert_eq!(SettingsCategory::Providers.item_count(), 5);
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
            genres: None,
            is_anime: false,
            audio_mode: None,
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

        // Switch to vidlink
        let vid_idx = modal
            .options
            .iter()
            .position(|o| o.id == "vidlink")
            .expect("vidlink should be an option");
        modal.selected_idx = vid_idx;
        modal.update_dependent_tracks("eng");

        // Verify VidLink sources, qualities, and subtitles
        assert!(modal.sources.iter().any(|s| s.name.contains("Primary CDN")));
        assert!(modal.qualities.iter().any(|q| q.label.contains("1080p FHD")));
        assert!(modal
            .subtitles
            .iter()
            .any(|s| s.is_native && s.badge == "DIRECT"));
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
            genres: None,
            is_anime: false,
            audio_mode: None,
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
    async fn test_primary_provider_updates_episode_list_order() {
        let cfg = Config::default();
        let db = Database::in_memory().unwrap();
        let mut app = App::new(cfg, db);

        // Verify provider episode order mappings
        assert_eq!(App::provider_episode_order("auto"), "broadcast");
        assert_eq!(App::provider_episode_order("vidlink"), "broadcast");
        assert_eq!(App::provider_episode_order("vixsrc"), "streaming");
        assert_eq!(App::provider_episode_order("lookmovie"), "broadcast");
        assert_eq!(App::provider_episode_order("hianime"), "broadcast");

        assert_eq!(app.cfg.preferred_provider, "auto");

        // Cycle to vidlink
        app.cycle_preferred_provider(true).await;
        assert_eq!(app.cfg.preferred_provider, "vidlink");
        assert_eq!(
            App::provider_episode_order(&app.cfg.preferred_provider),
            "broadcast"
        );
        assert!(app.status_message.contains("VidLink"));

        // Cycle to vixsrc
        app.cycle_preferred_provider(true).await;
        assert_eq!(app.cfg.preferred_provider, "vixsrc");
        assert_eq!(
            App::provider_episode_order(&app.cfg.preferred_provider),
            "streaming"
        );
        assert!(app.status_message.contains("VixSrc"));
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
