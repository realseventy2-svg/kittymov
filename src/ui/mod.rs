pub mod banner;
pub mod catalog_list;
pub mod episode_modal;
pub mod metadata_panel;
pub mod poster;
pub mod provider_modal;
pub mod settings_modal;
pub mod subtitle_modal;
pub mod theme;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use self::catalog_list::render_catalog_panel;
use self::episode_modal::render_episode_modal;
use self::metadata_panel::render_metadata_panel;
use self::provider_modal::render_provider_modal;
use self::settings_modal::render_settings_modal;
use self::subtitle_modal::render_subtitle_modal;
use self::theme::Theme;
use crate::app::App;

pub fn render_ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Outer frame with glowing cyber-cyan double borders
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::BG))
        .title(" KittyMov • Terminal Cinema ")
        .title_style(Style::default().fg(Theme::ACCENT_CYAN));

    let inner_area = outer_block.inner(size);
    f.render_widget(outer_block, size);

    // Split into Left metadata column (38%) and Right catalog column (62%)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(62),
        ])
        .split(inner_area);

    let selected_item = app.current_items.get(app.selected_index);
    let is_bookmarked = if let Some(item) = selected_item {
        app.db.is_favorite(item.tmdb_id, &item.media_type)
    } else {
        false
    };

    // Render left metadata column
    render_metadata_panel(
        f,
        columns[0],
        selected_item,
        app.current_poster_text.as_ref(),
        is_bookmarked,
        app.selected_index,
        app.current_items.len(),
    );

    // Render right catalog column
    let active_tab_name = match app.active_tab {
        crate::app::ActiveTab::Movies => "Trending Movies",
        crate::app::ActiveTab::Series => "Trending Series",
        crate::app::ActiveTab::Search => "Search Results",
        crate::app::ActiveTab::History => "Watch History",
        crate::app::ActiveTab::Bookmarks => "Bookmarks",
    };

    render_catalog_panel(
        f,
        columns[1],
        &app.current_items,
        app.selected_index,
        &app.search_query,
        app.is_searching,
        &app.status_message,
        active_tab_name,
        app.cfg.auto_play_next,
        &app.cfg.audio_mode,
        &app.cfg.preferred_provider,
    );

    // If Episode modal is active, overlay it
    if let Some(ref modal_state) = app.episode_modal {
        render_episode_modal(
            f,
            size,
            &modal_state.series.title,
            &modal_state.seasons,
            modal_state.selected_season_idx,
            &modal_state.episodes,
            modal_state.selected_episode_idx,
            app.cfg.auto_play_next,
            &app.cfg.audio_mode,
        );
    }

    // If Provider / Server selector modal is active, overlay it
    if let Some(ref p_modal) = app.provider_modal {
        render_provider_modal(
            f,
            size,
            &p_modal.target_title,
            &app.cfg.audio_mode,
            &p_modal.options,
            p_modal.selected_idx,
        );
    }

    // If Subtitle selector modal is active, overlay it
    if let Some(ref sub_modal) = app.subtitle_modal {
        render_subtitle_modal(
            f,
            size,
            &sub_modal.target_title,
            &app.cfg.sub_language,
            &sub_modal.options,
            sub_modal.selected_idx,
            sub_modal.is_loading,
        );
    }

    // If Settings modal is active, overlay it
    if let Some(ref s_modal) = app.settings_modal {
        let poster_cache_size = crate::config::get_poster_cache_size();
        let history_count = app.db.get_history_count();
        render_settings_modal(
            f,
            size,
            &app.cfg,
            s_modal,
            poster_cache_size,
            history_count,
        );
    }
}
