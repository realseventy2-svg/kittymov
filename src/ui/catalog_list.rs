use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::banner::get_banner_lines;
use super::theme::Theme;
use crate::metadata::MediaItem;

pub fn render_catalog_panel(
    f: &mut Frame,
    area: Rect,
    items: &[MediaItem],
    selected_index: usize,
    search_query: &str,
    is_searching: bool,
    status_text: &str,
    active_tab_name: &str,
    auto_play_enabled: bool,
    audio_mode: &str,
    preferred_provider: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // ASCII Banner
            Constraint::Length(3), // Search bar
            Constraint::Min(8),    // List of items
            Constraint::Length(3), // Action & status footer
        ])
        .split(area);

    // 1. ASCII Banner Block
    let banner_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::SURFACE));
    let banner = Paragraph::new(get_banner_lines())
        .block(banner_block)
        .alignment(Alignment::Center);
    f.render_widget(banner, chunks[0]);

    // 2. Search Input Box
    let search_title = format!(" Catalog: {} ", active_tab_name);
    let counter = format!("{}/{} ", (selected_index + 1).min(items.len()), items.len());
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_searching {
            Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::BORDER_MUTED)
        })
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(search_title, Style::default().fg(Theme::ACCENT_CYAN)))
        .title(Span::styled(counter, Style::default().fg(Theme::TEXT_MUTED)).into_right_aligned_line());

    let display_query = if is_searching {
        format!("Search: {}█", search_query)
    } else if search_query.is_empty() {
        "Press [/] to Search Catalog...".to_string()
    } else {
        format!("Search: {}", search_query)
    };

    let search_p = Paragraph::new(Line::from(vec![
        Span::styled(
            display_query,
            if is_searching {
                Style::default().fg(Theme::TEXT_TITLE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        ),
    ])).block(search_block);
    f.render_widget(search_p, chunks[1]);

    // 3. Interactive Media List
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE));

    let visible_height = chunks[2].height.saturating_sub(2) as usize;
    let mut start_idx = 0;
    if selected_index >= visible_height {
        start_idx = selected_index - visible_height + 1;
    }
    let end_idx = (start_idx + visible_height).min(items.len());

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

            let line = if is_selected {
                Line::from(vec![
                    Span::styled(" ◆ ", Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        &item.title,
                        Style::default().fg(Theme::TEXT_TITLE).bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({})", item.year),
                        Style::default().fg(Theme::ACCENT_CYAN).bg(Theme::SELECTION_BG),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(&item.title, Style::default().fg(Theme::TEXT_BODY)),
                    Span::styled(format!(" ({})", item.year), Style::default().fg(Theme::TEXT_MUTED)),
                ])
            };
            list_lines.push(line);
        }
    }

    let list_p = Paragraph::new(list_lines).block(list_block);
    f.render_widget(list_p, chunks[2]);

    // 4. Quick Actions / Controls Footer
    let is_sub = audio_mode.to_lowercase() == "sub";
    let mode_badge = if is_sub {
        Span::styled("[SUB] ", Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[DUB] ", Style::default().fg(Theme::ACCENT_CYAN).add_modifier(Modifier::BOLD))
    };

    let server_badge = match preferred_provider.to_lowercase().as_str() {
        "vixsrc" => Span::styled("[VixSrc] ", Style::default().fg(Theme::ACCENT_CYAN)),
        "lookmovie" => Span::styled("[LookMovie] ", Style::default().fg(Theme::ACCENT_GOLD)),
        _ => Span::styled("[Auto] ", Style::default().fg(Theme::ACCENT_GREEN)),
    };

    let auto_play_str = if auto_play_enabled {
        Span::styled("[AutoNext: ON] ", Style::default().fg(Theme::ACCENT_GREEN).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[AutoNext: OFF] ", Style::default().fg(Theme::TEXT_MUTED))
    };

    let footer_lines = vec![
        Line::from(vec![
            mode_badge,
            server_badge,
            auto_play_str,
            Span::styled("[s] Server  [d] Sub/Dub  [S] Settings  [f] Fav  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(status_text, Style::default().fg(Theme::TEXT_BODY)),
        ]),
    ];
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE));
    let footer_p = Paragraph::new(footer_lines).block(footer_block);
    f.render_widget(footer_p, chunks[3]);
}
