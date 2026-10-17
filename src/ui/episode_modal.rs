use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::metadata::{TvEpisode, TvSeason};

#[allow(clippy::too_many_arguments)]
pub fn render_episode_modal(
    f: &mut Frame,
    area: Rect,
    series_title: &str,
    seasons: &[TvSeason],
    current_season_idx: usize,
    episodes: &[TvEpisode],
    selected_ep_idx: usize,
    watched_episodes: &std::collections::HashSet<i32>,
    auto_play: bool,
    audio_mode: &str,
    episode_order: &str,
) {
    // Center popup dialog
    let popup_area = centered_rect(area, 75, 75);
    f.render_widget(Clear, popup_area);

    let season_name = seasons
        .get(current_season_idx)
        .map(|s| s.name.as_str())
        .unwrap_or("Season");

    let is_sub = audio_mode.to_lowercase() == "sub";
    let mode_str = if is_sub { "[SUB]" } else { "[DUB]" };
    let is_streaming = episode_order.to_lowercase() == "streaming";
    let order_str = if is_streaming {
        "[STREAMING]"
    } else {
        "[BROADCAST]"
    };
    let auto_play_badge = if auto_play {
        Span::styled(
            format!(" {} {} [AUTO-PLAY: ON] ", mode_str, order_str),
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {} {} [AUTO-PLAY: OFF] ", mode_str, order_str),
            Style::default().fg(Theme::TEXT_MUTED),
        )
    };

    let title_line = format!(" {} • {} ", series_title, season_name);
    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            title_line,
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ))
        .title(auto_play_badge.into_right_aligned_line());

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Season tab bar
            Constraint::Min(6),    // Episode list
            Constraint::Length(4), // Selected episode synopsis / air date
            Constraint::Length(2), // Hotkey guide
        ])
        .margin(1)
        .split(popup_area);

    f.render_widget(modal_block, popup_area);

    // 1. Season Switcher Bar
    let mut season_spans = vec![Span::styled(
        "Seasons: ",
        Style::default().fg(Theme::TEXT_MUTED),
    )];
    for (i, s) in seasons.iter().enumerate() {
        let ep_count_str = if s.episode_count > 0 {
            format!(" ({})", s.episode_count)
        } else {
            String::new()
        };
        if i == current_season_idx {
            season_spans.push(Span::styled(
                format!(" [ {}{} ] ", s.name, ep_count_str),
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .bg(Theme::SELECTION_BG)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            season_spans.push(Span::styled(
                format!("  {}{}  ", s.name, ep_count_str),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }
    }
    let season_p = Paragraph::new(Line::from(season_spans));
    f.render_widget(season_p, inner_chunks[0]);

    // 2. Episode Items List
    let visible_height = inner_chunks[1].height as usize;
    let mut start_idx = 0;
    if selected_ep_idx >= visible_height {
        start_idx = selected_ep_idx - visible_height + 1;
    }
    let end_idx = (start_idx + visible_height).min(episodes.len());

    let mut ep_lines = Vec::new();
    if episodes.is_empty() {
        ep_lines.push(Line::from(Span::styled(
            "Loading episodes...",
            Style::default().fg(Theme::TEXT_MUTED),
        )));
    } else {
        for (i, ep) in episodes[start_idx..end_idx].iter().enumerate() {
            let actual_idx = start_idx + i;
            let is_sel = actual_idx == selected_ep_idx;
            let is_watched = watched_episodes.contains(&ep.episode_number);

            let ep_code = format!("E{:02}", ep.episode_number);
            let ep_title = &ep.name;
            let ep_dur = &ep.duration;
            let row_bg = if is_sel { Theme::SELECTION_BG } else { Theme::SURFACE };

            let watch_span = if is_watched {
                Span::styled(
                    " [✓]",
                    Style::default()
                        .fg(Theme::ACCENT_GREEN)
                        .bg(row_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("    ", Style::default().bg(row_bg))
            };

            let line = if is_sel {
                Line::from(vec![
                    Span::styled(
                        " ◆ ",
                        Style::default()
                            .fg(Theme::ACCENT_PINK)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        ep_code,
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    watch_span,
                    Span::styled("  ", Style::default().bg(row_bg)),
                    Span::styled(
                        ep_title,
                        Style::default()
                            .fg(Theme::TEXT_TITLE)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  ({})", ep_dur),
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(row_bg),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("   ", Style::default().bg(row_bg)),
                    Span::styled(ep_code, Style::default().fg(Theme::TEXT_MUTED).bg(row_bg)),
                    watch_span,
                    Span::styled("  ", Style::default().bg(row_bg)),
                    Span::styled(ep_title, Style::default().fg(Theme::TEXT_BODY).bg(row_bg)),
                    Span::styled(
                        format!("  ({})", ep_dur),
                        Style::default().fg(Theme::BORDER_MUTED).bg(row_bg),
                    ),
                ])
            };
            ep_lines.push(line);
        }
    }

    let ep_p = Paragraph::new(ep_lines);
    f.render_widget(ep_p, inner_chunks[1]);

    // 3. Selected Episode Details (synopsis, air date & watch status)
    if let Some(ep) = episodes.get(selected_ep_idx) {
        let is_watched = watched_episodes.contains(&ep.episode_number);
        let mut meta_spans = vec![
            Span::styled("Aired: ", Style::default().fg(Theme::ACCENT_CYAN)),
            Span::styled(&ep.air_date, Style::default().fg(Theme::TEXT_BODY)),
            Span::styled("  •  Duration: ", Style::default().fg(Theme::ACCENT_CYAN)),
            Span::styled(&ep.duration, Style::default().fg(Theme::ACCENT_GREEN)),
        ];
        if is_watched {
            meta_spans.push(Span::styled(
                "  •  [✓ Watched]",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        let ep_info = vec![
            Line::from(meta_spans),
            Line::from(Span::styled(
                if ep.overview.is_empty() {
                    "No overview available."
                } else {
                    &ep.overview
                },
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ];
        let ep_info_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Theme::BORDER_MUTED))
            .style(Style::default().bg(Theme::SURFACE_ALT));
        let ep_info_p = Paragraph::new(ep_info)
            .block(ep_info_block)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(ep_info_p, inner_chunks[2]);
    }

    // 4. Hotkey Hints
    let hints = Line::from(vec![
        Span::styled(" [Enter] Play   [s] Providers   [t] Subtitles   [d] Sub/Dub   [← / →] Season   [a] Auto-Play   [?] Help   [Esc] Close ", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        inner_chunks[3],
    );
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
