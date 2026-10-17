mod app;
mod config;
mod diagnostics;
mod metadata;
mod player;
mod providers;
mod storage;
mod subtitles;
mod ui;

use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use crate::app::{ActiveTab, App};
use crate::config::{get_state_dir, load_config};
use crate::storage::Database;

#[derive(Parser, Debug)]
#[command(
    name = "kittymov",
    version = "2.0.0",
    about = "Modern Cyber-Minimalist Terminal Movie & TV Streaming Suite"
)]
struct Cli {
    #[arg(help = "Optional movie or series query to search immediately")]
    query: Option<String>,

    #[arg(short, long, help = "Search for a movie or TV show")]
    search: Option<String>,

    #[arg(short, long, help = "Instantly resume the most recently watched title")]
    resume: bool,

    #[arg(long, help = "Run environment diagnostics and dependency health check")]
    doctor: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging to ~/.local/state/kittymov/kittymov.log
    let log_dir = get_state_dir();
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = log_dir.join("kittymov.log");
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .with_writer(file)
            .try_init();
    }

    let cli = Cli::parse();
    let cfg = load_config();

    // Fast-path: --doctor flag
    if cli.doctor {
        return crate::diagnostics::run_doctor(&cfg).await;
    }

    let db = Database::new()?;

    // Fast-path: --resume flag
    if cli.resume {
        if let Ok(Some(latest)) = db.get_latest_watch() {
            println!("Resuming: {}...", latest.title);
            let mut app = App::new(cfg, db);
            if latest.media_type == "tv" {
                app.play_tv_series(
                    latest.tmdb_id,
                    &latest.title,
                    &latest.year,
                    &latest.poster_path,
                    latest.season.unwrap_or(1),
                    latest.episode.unwrap_or(1),
                )
                .await;
            } else {
                app.play_movie(crate::metadata::MediaItem {
                    tmdb_id: latest.tmdb_id,
                    media_type: latest.media_type,
                    title: latest.title,
                    year: latest.year,
                    rating: 0.0,
                    poster_path: latest.poster_path,
                    overview: String::new(),
                    genres: String::new(),
                    duration_str: String::new(),
                    status: String::new(),
                    episodes_count: None,
                })
                .await;
            }
            return Ok(());
        } else {
            println!("No recent watch history found.");
            return Ok(());
        }
    }

    // Set up crash-safe panic hook
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(stdout(), LeaveAlternateScreen);
        default_panic(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cfg, db);
    app.init().await;

    let initial_query = cli.search.or(cli.query);
    if let Some(q) = initial_query {
        app.search_query = q;
        app.perform_search().await;
    }

    let mut reader = crossterm::event::EventStream::new();
    let mut tick_interval = tokio::time::interval(Duration::from_millis(33)); // ~30 FPS

    while !app.should_quit {
        // Poll playback loading progress
        if app.playback_loading.is_some() {
            if let Some(event) = app.poll_playback_loading() {
                match event {
                    app::LoadingProgress::Ready {
                        stream,
                        display_title,
                    } => {
                        app.execute_player(&stream, &display_title).await;
                    }
                    app::LoadingProgress::Failed(err) => {
                        app.playback_loading = None;
                        app.status_message = format!("Playback error: {}", err);
                    }
                    _ => {}
                }
            }
        }

        terminal.draw(|f| ui::render_ui(f, &app))?;

        tokio::select! {
            _ = tick_interval.tick() => {}
            Some(Ok(event)) = reader.next() => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(&mut app, key.code, key.modifiers).await;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                app.should_quit = true;
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn handle_key_event(
    app: &mut App,
    code: KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
) {
    // -1. If Playback Loading / Bootstrap screen is active
    if app.playback_loading.is_some() {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.cancel_playback_loading();
            }
            KeyCode::Char('F') | KeyCode::Char('f') => {
                app.force_playback_fallback().await;
            }
            _ => {}
        }
        return;
    }

    // 0. If Help modal is active
    if app.show_help {
        match code {
            KeyCode::Esc
            | KeyCode::Char('?')
            | KeyCode::Char('q')
            | KeyCode::Enter
            | KeyCode::F(1) => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // 1. If Settings Modal is open
    if let Some(ref s) = app.settings_modal.clone() {
        // A. If confirmation dialog is active
        if s.confirmation.is_some() {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    app.settings_execute_confirmation().await;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.settings_cancel_confirmation();
                }
                _ => {}
            }
            return;
        }

        // B. If in search input mode
        if s.is_searching {
            match code {
                KeyCode::Char(c) => {
                    if let Some(ref mut sm) = app.settings_modal {
                        sm.search_query.push(c);
                        sm.item_idx = 0;
                    }
                }
                KeyCode::Backspace => {
                    if let Some(ref mut sm) = app.settings_modal {
                        sm.search_query.pop();
                        sm.item_idx = 0;
                    }
                }
                KeyCode::Up => {
                    app.settings_prev();
                }
                KeyCode::Down => {
                    app.settings_next();
                }
                KeyCode::Enter => {
                    app.settings_change_value(true);
                }
                KeyCode::Esc => {
                    if let Some(ref mut sm) = app.settings_modal {
                        if !sm.search_query.is_empty() {
                            sm.search_query.clear();
                        } else {
                            sm.is_searching = false;
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // C. Normal Settings Navigation
        match code {
            KeyCode::Esc => {
                if let Some(ref mut sm) = app.settings_modal {
                    if !sm.search_query.is_empty() {
                        sm.search_query.clear();
                    } else if sm.active_pane == crate::app::SettingsPane::Content {
                        sm.active_pane = crate::app::SettingsPane::Categories;
                    } else {
                        app.toggle_settings();
                    }
                }
            }
            KeyCode::Char('S') => {
                app.toggle_settings();
            }
            KeyCode::Tab | KeyCode::BackTab => {
                app.settings_switch_pane();
            }
            KeyCode::Char('/') => {
                if let Some(ref mut sm) = app.settings_modal {
                    sm.is_searching = true;
                }
            }
            KeyCode::Char('r') => {
                app.settings_reset_current_category();
            }
            KeyCode::Char('?') => {
                if let Some(ref mut sm) = app.settings_modal {
                    sm.feedback_message = Some("Keys: Tab switch pane, ↑/↓ navigate, ←/→/Space change, r reset, / search, Esc close".to_string());
                }
            }
            KeyCode::F(1) => {
                app.toggle_help();
            }
            KeyCode::Home => {
                if let Some(ref mut sm) = app.settings_modal {
                    sm.item_idx = 0;
                }
            }
            KeyCode::End => {
                if let Some(ref mut sm) = app.settings_modal {
                    match sm.active_pane {
                        crate::app::SettingsPane::Categories => {
                            sm.category_idx =
                                crate::app::SettingsCategory::ALL.len().saturating_sub(1);
                        }
                        crate::app::SettingsPane::Content => {
                            let cat = sm.current_category();
                            sm.item_idx = cat.item_count().saturating_sub(1);
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.settings_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.settings_next();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(ref sm) = app.settings_modal {
                    if sm.active_pane == crate::app::SettingsPane::Content {
                        app.settings_change_value(false);
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(ref mut sm) = app.settings_modal {
                    if sm.active_pane == crate::app::SettingsPane::Categories {
                        sm.active_pane = crate::app::SettingsPane::Content;
                    } else {
                        app.settings_change_value(true);
                    }
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                app.settings_change_value(true);
            }
            _ => {}
        }
        return;
    }

    // 2. If Subtitle Language Selector modal is open
    if app.subtitle_modal.is_some() {
        match code {
            KeyCode::Esc => {
                app.subtitle_modal = None;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                app.toggle_help();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.subtitle_modal_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.subtitle_modal_next();
            }
            KeyCode::Enter => {
                app.subtitle_modal_confirm();
            }
            KeyCode::Char('s') => {
                app.cycle_subtitle_source();
            }
            KeyCode::Char('d') => {
                app.toggle_audio_mode();
            }
            _ => {}
        }
        return;
    }

    // 3. If Provider / Tracks selector modal is open
    if app.provider_modal.is_some() {
        match code {
            KeyCode::Esc => {
                app.provider_modal = None;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                app.toggle_help();
            }
            KeyCode::Tab => {
                app.provider_modal_switch_pane();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.provider_modal_left();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                app.provider_modal_right();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.provider_modal_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.provider_modal_next();
            }
            KeyCode::Char('1') => {
                app.provider_modal_set_category(crate::app::ProviderModalCategory::Provider);
            }
            KeyCode::Char('2') => {
                app.provider_modal_set_category(crate::app::ProviderModalCategory::Quality);
            }
            KeyCode::Char('3') => {
                app.provider_modal_set_category(crate::app::ProviderModalCategory::Subtitles);
            }
            KeyCode::Char('d') => {
                app.toggle_audio_mode();
            }
            KeyCode::Enter => {
                app.provider_modal_confirm().await;
            }
            _ => {}
        }
        return;
    }

    // 4. If in search input mode
    if app.is_searching {
        match code {
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Enter => {
                app.perform_search().await;
            }
            KeyCode::Esc => {
                app.is_searching = false;
            }
            _ => {}
        }
        return;
    }

    // 5. If Episode Modal is open
    if let Some(ref mut modal) = app.episode_modal {
        match code {
            KeyCode::Esc => {
                app.episode_modal = None;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                app.toggle_help();
            }
            KeyCode::Char('s') => {
                app.open_provider_modal_for_selected();
            }
            KeyCode::Char('o') => {
                app.toggle_episode_order().await;
            }
            KeyCode::Char('t') => {
                app.open_subtitle_modal().await;
            }
            KeyCode::Char('d') => {
                app.toggle_audio_mode();
            }
            KeyCode::Char('S') => {
                app.toggle_settings();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if modal.selected_episode_idx > 0 {
                    modal.selected_episode_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !modal.episodes.is_empty()
                    && modal.selected_episode_idx + 1 < modal.episodes.len()
                {
                    modal.selected_episode_idx += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('[') => {
                if modal.selected_season_idx > 0 {
                    modal.selected_season_idx -= 1;
                    modal.selected_episode_idx = 0;
                    if let Some(s) = modal.seasons.get(modal.selected_season_idx) {
                        let tmdb_id = modal.series.tmdb_id;
                        let s_num = s.season_number;
                        if let Some(ref map) = modal.all_season_episodes {
                            modal.episodes = map.get(&s_num).cloned().unwrap_or_default();
                        } else {
                            modal.episodes = app
                                .tmdb
                                .get_season_episodes(tmdb_id, s_num)
                                .await
                                .unwrap_or_default();
                        }
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(']') => {
                if !modal.seasons.is_empty() && modal.selected_season_idx + 1 < modal.seasons.len()
                {
                    modal.selected_season_idx += 1;
                    modal.selected_episode_idx = 0;
                    if let Some(s) = modal.seasons.get(modal.selected_season_idx) {
                        let tmdb_id = modal.series.tmdb_id;
                        let s_num = s.season_number;
                        if let Some(ref map) = modal.all_season_episodes {
                            modal.episodes = map.get(&s_num).cloned().unwrap_or_default();
                        } else {
                            modal.episodes = app
                                .tmdb
                                .get_season_episodes(tmdb_id, s_num)
                                .await
                                .unwrap_or_default();
                        }
                    }
                }
            }
            KeyCode::Enter => {
                app.on_enter().await;
            }
            KeyCode::Char('a') => {
                app.cfg.auto_play_next = !app.cfg.auto_play_next;
                let _ = app.cfg.save();
                app.status_message = format!(
                    "Auto-Play Next: {}",
                    if app.cfg.auto_play_next { "ON" } else { "OFF" }
                );
            }
            _ => {}
        }
        return;
    }

    // 6. Main view navigation
    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.toggle_help();
        }
        KeyCode::Char('/') => {
            app.is_searching = true;
            app.search_query.clear();
        }
        KeyCode::Char('1') => {
            app.load_tab(ActiveTab::Movies).await;
        }
        KeyCode::Char('2') => {
            app.load_tab(ActiveTab::Series).await;
        }
        KeyCode::Char('3') => {
            app.load_tab(ActiveTab::History).await;
        }
        KeyCode::Char('4') => {
            app.load_tab(ActiveTab::Bookmarks).await;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_prev().await;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next().await;
        }
        KeyCode::Enter => {
            app.on_enter().await;
        }
        KeyCode::Char('s') => {
            app.open_provider_modal_for_selected();
        }
        KeyCode::Char('t') => {
            app.open_subtitle_modal().await;
        }
        KeyCode::Char('d') => {
            app.toggle_audio_mode();
        }
        KeyCode::Char('S') | KeyCode::F(2) | KeyCode::Char(',') => {
            app.toggle_settings();
        }
        KeyCode::Char('f') => {
            app.toggle_favorite().await;
        }
        KeyCode::Char('a') => {
            app.cfg.auto_play_next = !app.cfg.auto_play_next;
            let _ = app.cfg.save();
            app.status_message = format!(
                "Auto-Play Next: {}",
                if app.cfg.auto_play_next { "ON" } else { "OFF" }
            );
        }
        _ => {}
    }
}
