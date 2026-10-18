use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::banner::get_banner_lines;
use super::theme::Theme;
use crate::app::ActiveTab;
use crate::metadata::MediaItem;
use crate::storage::Database;

#[allow(clippy::too_many_arguments)]
pub fn render_catalog_panel(
    f: &mut Frame,
    area: Rect,
    items: &[MediaItem],
    selected_index: usize,
    search_query: &str,
    is_searching: bool,
    status_text: &str,
    active_tab: ActiveTab,
    db: &Database,
    auto_play_enabled: bool,
    audio_mode: &str,
    preferred_provider: &str,
) {
    let show_banner = area.height >= 28;
    let banner_h = if show_banner { 7 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_h), // Optional ASCII Banner (responsive collapse on small screens)
            Constraint::Length(3),        // Interactive Tab Bar (1..4 + / Search)
            Constraint::Length(3),        // Search bar
            Constraint::Min(6),           // Rich List of items
            Constraint::Length(4),        // Action & status footer (2 inner rows)
        ])
        .split(area);

    // 1. ASCII Banner Block (only when terminal height >= 28)
    if show_banner {
        let banner_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER))
            .style(Style::default().bg(Theme::SURFACE));
        let banner = Paragraph::new(get_banner_lines())
            .block(banner_block)
            .alignment(Alignment::Center);
        f.render_widget(banner, chunks[0]);
    }

    // 2. Interactive Navigation Tabs Bar
    let tab_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            " Navigation ",
            Style::default().fg(Theme::ACCENT_CYAN),
        ));

    let make_tab_pill = |key: &str, label: &str, count: Option<usize>, is_active: bool| -> Span<'static> {
        let count_str = count.map(|c| format!(" ({})", c)).unwrap_or_default();
        if is_active {
            Span::styled(
                format!(" ▶ [{}] {}{} ◀ ", key, label, count_str),
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .bg(Theme::SELECTION_BG)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" [{}] {}{} ", key, label, count_str),
                Style::default().fg(Theme::TEXT_MUTED),
            )
        }
    };

    let item_count = if active_tab != ActiveTab::Search {
        Some(items.len())
    } else {
        None
    };

    let tab_line = Line::from(vec![
        make_tab_pill(
            "1",
            "Movies",
            if active_tab == ActiveTab::Movies { item_count } else { None },
            active_tab == ActiveTab::Movies,
        ),
        Span::raw(" "),
        make_tab_pill(
            "2",
            "Series",
            if active_tab == ActiveTab::Series { item_count } else { None },
            active_tab == ActiveTab::Series,
        ),
        Span::raw(" "),
        make_tab_pill(
            "3",
            "History",
            if active_tab == ActiveTab::History { item_count } else { None },
            active_tab == ActiveTab::History,
        ),
        Span::raw(" "),
        make_tab_pill(
            "4",
            "Saved",
            if active_tab == ActiveTab::Bookmarks { item_count } else { None },
            active_tab == ActiveTab::Bookmarks,
        ),
        Span::raw(" "),
        make_tab_pill(
            "/",
            "Search",
            if active_tab == ActiveTab::Search { item_count } else { None },
            active_tab == ActiveTab::Search || is_searching,
        ),
    ]);

    let tab_p = Paragraph::new(tab_line)
        .block(tab_block)
        .alignment(Alignment::Center);
    f.render_widget(tab_p, chunks[1]);

    // 3. Search Input Box
    let active_tab_title = match active_tab {
        ActiveTab::Movies => "Trending Movies",
        ActiveTab::Series => "Trending Series",
        ActiveTab::Search => "Search Results",
        ActiveTab::History => "Watch History",
        ActiveTab::Bookmarks => "Saved Bookmarks",
    };
    let search_title = format!(" Catalog: {} ", active_tab_title);
    let counter = format!("{}/{} ", (selected_index + 1).min(items.len()), items.len());
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_searching {
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_MUTED)
        })
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            search_title,
            Style::default().fg(Theme::ACCENT_CYAN),
        ))
        .title(
            Span::styled(counter, Style::default().fg(Theme::TEXT_MUTED)).into_right_aligned_line(),
        );

    let display_query = if is_searching {
        format!("Search: {}█", search_query)
    } else if search_query.is_empty() {
        "Press [/] to Search Catalog...".to_string()
    } else {
        format!("Search: {}", search_query)
    };

    let search_p = Paragraph::new(Line::from(vec![Span::styled(
        display_query,
        if is_searching {
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        },
    )]))
    .block(search_block);
    f.render_widget(search_p, chunks[2]);

    // 4. Interactive Rich Media List with Column Alignment
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE));

    let visible_height = chunks[3].height.saturating_sub(2) as usize;
    let mut start_idx = 0;
    if selected_index >= visible_height {
        start_idx = selected_index - visible_height + 1;
    }
    let end_idx = (start_idx + visible_height).min(items.len());

    let list_width = chunks[3].width.saturating_sub(4) as usize;

    let mut list_lines = Vec::new();
    if items.is_empty() {
        list_lines.push(Line::from(""));
        list_lines.push(Line::from(Span::styled(
            "   No titles found. Try searching for a movie or TV show above.",
            Style::default().fg(Theme::TEXT_MUTED),
        )));
    } else {
        for (i, item) in items[start_idx..end_idx].iter().enumerate() {
            let actual_idx = start_idx + i;
            let is_selected = actual_idx == selected_index;

            let is_fav = db.is_favorite(item.tmdb_id, &item.media_type);
            let is_watched = db.is_watched(item.tmdb_id, &item.media_type);

            let is_anime = item.genres.to_lowercase().contains("animation");
            let format_badge = if is_anime {
                ("[ANIME]", Theme::ACCENT_PINK)
            } else if item.media_type == "tv" {
                ("[TV]   ", Theme::ACCENT_CYAN)
            } else {
                ("[MOV]  ", Theme::TEXT_MUTED)
            };

            let rating_str = if item.rating > 0.0 {
                format!("★ {:.1} ", item.rating)
            } else {
                "       ".to_string()
            };

            let year_str = if !item.year.is_empty() {
                format!("({})", item.year)
            } else {
                "    ".to_string()
            };

            // Fixed columns: year (7) + rating (7) + format_badge (8) + status icons (4) + spacing = 28
            let right_column_len = 28;
            let available_title = list_width.saturating_sub(right_column_len + 5);

            let title_display = if item.title.chars().count() > available_title {
                let mut s: String = item.title.chars().take(available_title.saturating_sub(1)).collect();
                s.push('…');
                s
            } else {
                let current_chars = item.title.chars().count();
                let pad = available_title.saturating_sub(current_chars);
                format!("{}{}", item.title, " ".repeat(pad))
            };

            let row_bg = if is_selected {
                Theme::SELECTION_BG
            } else {
                Theme::SURFACE
            };

            let mut spans = Vec::new();

            // Prefix pointer
            if is_selected {
                spans.push(Span::styled(
                    " ◆ ",
                    Style::default()
                        .fg(Theme::ACCENT_PINK)
                        .bg(row_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("   ", Style::default().bg(row_bg)));
            }

            // Title
            spans.push(Span::styled(
                title_display,
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT_TITLE)
                        .bg(row_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_BODY).bg(row_bg)
                },
            ));

            spans.push(Span::styled(" ", Style::default().bg(row_bg)));

            // Year
            spans.push(Span::styled(
                format!("{:7}", year_str),
                Style::default().fg(Theme::TEXT_MUTED).bg(row_bg),
            ));

            // Rating
            spans.push(Span::styled(
                format!("{:7}", rating_str),
                Style::default().fg(Theme::ACCENT_GOLD).bg(row_bg),
            ));

            // Media Format Badge
            spans.push(Span::styled(
                format!("{:8}", format_badge.0),
                Style::default()
                    .fg(format_badge.1)
                    .bg(row_bg)
                    .add_modifier(Modifier::BOLD),
            ));

            // Heart Bookmark Indicator
            if is_fav {
                spans.push(Span::styled("♥ ", Style::default().fg(Theme::ACCENT_PINK).bg(row_bg)));
            } else {
                spans.push(Span::styled("  ", Style::default().bg(row_bg)));
            }

            // Watched Checkmark Indicator
            if is_watched {
                spans.push(Span::styled(
                    "✓",
                    Style::default()
                        .fg(Theme::ACCENT_GREEN)
                        .bg(row_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(" ", Style::default().bg(row_bg)));
            }

            list_lines.push(Line::from(spans));
        }
    }

    let list_p = Paragraph::new(list_lines).block(list_block);
    f.render_widget(list_p, chunks[3]);

    // 5. Quick Actions / Controls Footer
    let is_sub = audio_mode.to_lowercase() == "sub";
    let mode_badge = if is_sub {
        Span::styled(
            "[SUB] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "[DUB] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )
    };

    let server_badge = match preferred_provider.to_lowercase().as_str() {
        "vidlink" => Span::styled(
            "[VidLink] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        "vixsrc" => Span::styled(
            "[VixSrc] ",
            Style::default()
                .fg(Theme::BORDER)
                .add_modifier(Modifier::BOLD),
        ),
        "hianime" => Span::styled(
            "[HiAnime] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        "lookmovie" => Span::styled(
            "[LookMovie] ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        _ => Span::styled(
            "[Auto] ",
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let auto_play_str = if auto_play_enabled {
        Span::styled(
            "[AutoNext: ON] ",
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("[AutoNext: OFF] ", Style::default().fg(Theme::TEXT_MUTED))
    };

    let is_error = status_text.to_lowercase().contains("error")
        || status_text.to_lowercase().contains("fail")
        || status_text.to_lowercase().contains("unavailable")
        || status_text.to_lowercase().contains("not found");

    let status_line = if is_error {
        Line::from(vec![
            Span::styled(
                " [ERR] ",
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(
                status_text,
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else if !status_text.is_empty() && status_text != "Ready" {
        Line::from(vec![
            Span::styled(
                " [INFO] ",
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(status_text, Style::default().fg(Theme::TEXT_BODY)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " [READY] ",
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Idle  •  Catalog: ", Style::default().fg(Theme::BORDER_MUTED)),
            Span::styled(active_tab_title, Style::default().fg(Theme::ACCENT_CYAN)),
        ])
    };

    let controls_line = Line::from(vec![
        mode_badge,
        server_badge,
        auto_play_str,
        Span::styled("│ ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            "[?] Help  [Enter] Watch  [s] Tracks  [t] Subs  [d] Sub/Dub  [o] Order  [f] Fav  [S] Settings",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);

    let footer_border_color = if is_error {
        Theme::ACCENT_PINK
    } else {
        Theme::BORDER_MUTED
    };

    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(footer_border_color))
        .style(Style::default().bg(Theme::SURFACE));
    let footer_p = Paragraph::new(vec![status_line, controls_line]).block(footer_block);
    f.render_widget(footer_p, chunks[4]);
}
