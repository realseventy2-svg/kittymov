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
use crossterm::event::{Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use crate::app::App;
use crate::config::{get_state_dir, load_config};
use crate::storage::Database;

#[derive(Parser, Debug)]
#[command(
    name = "kittymov",
    version = "0.2.1",
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

    #[arg(long, help = "Launch the beginner-friendly initial setup wizard")]
    setup: bool,
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
    let is_first_run = crate::config::is_fresh_install();
    let cfg = load_config();

    // Fast-path: --doctor flag
    if cli.doctor {
        return crate::diagnostics::run_doctor(&cfg).await;
    }

    let db = Database::new()?;

    // Fast-path: --resume flag (unless --setup is explicitly passed)
    if cli.resume && !cli.setup {
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
        crate::ui::poster::clear_kitty_graphics();
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
    if is_first_run || cli.setup {
        app.open_setup_wizard();
    }
    app.init().await;

    let initial_query = cli.search.or(cli.query);
    if let Some(q) = initial_query {
        if app.setup_wizard.is_none() {
            app.search_query = q;
            app.perform_search().await;
        }
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
                        let title = app
                            .playback_loading
                            .as_ref()
                            .map(|l| l.title.clone())
                            .unwrap_or_else(|| "Selected Title".to_string());
                        app.status_message = format!("Stream unavailable: {}", err);
                        app.error_dialog = Some(crate::app::ErrorDialogState {
                            title: title.clone(),
                            primary_message: format!("Could not find stream for \"{}\"", title),
                            technical_details: err.clone(),
                            suggestions: vec![
                                "Press [s] to open Provider & Server selector to test individual sources.".to_string(),
                                "This title may be a recent theatrical/regional release not yet hosted on streaming CDNs.".to_string(),
                                "Check if an alternate title or series season entry is available in search.".to_string(),
                            ],
                        });
                        app.playback_loading = None;
                    }
                    _ => {}
                }
            }
        }

        terminal.draw(|f| ui::render_ui(f, &app))?;

        let (term_w, term_h) = terminal.size().map(|s| (s.width, s.height)).unwrap_or((120, 40));
        app.render_kitty_poster_if_needed(term_w, term_h);

        tokio::select! {
            _ = tick_interval.tick() => {}
            Some(Ok(event)) = reader.next() => {
                match event {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            app.handle_key_event(key.code, key.modifiers).await;
                        }
                    }
                    Event::Resize(_, _) => {
                        crate::ui::poster::clear_kitty_graphics();
                        app.kitty_poster_dirty = true;
                        app.had_kitty_poster_drawn = false;
                        app.had_resume_kitty_poster_drawn = false;
                        app.update_poster().await;
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                app.should_quit = true;
            }
        }
    }

    // Clean up any Kitty protocol raster graphics before switching terminal screen
    crate::ui::poster::clear_kitty_graphics();

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
