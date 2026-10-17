use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;
use crate::metadata::MediaItem;

#[allow(clippy::too_many_arguments)]
pub fn render_metadata_panel(
    f: &mut Frame,
    area: Rect,
    item: Option<&MediaItem>,
    poster_text: Option<&ratatui::text::Text>,
    is_kitty_poster: bool,
    is_bookmarked: bool,
    current_index: usize,
    total_items: usize,
) {
    let (_, poster_h) =
        crate::ui::poster::calculate_poster_dimensions_for_panel(area.width, area.height);
    let poster_frame_height = poster_h + 2;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(poster_frame_height), // Poster frame
            Constraint::Length(3),                  // Title
            Constraint::Min(8),                     // Metadata table
        ])
        .split(area);

    // 1. Poster Frame
    let poster_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            format!(" {}/{} ", current_index + 1, total_items.max(1)),
            Style::default().fg(Theme::TEXT_MUTED),
        ))
        .title_alignment(Alignment::Right);

    if let Some(text) = poster_text {
        let p = Paragraph::new(text.clone())
            .block(poster_block)
            .alignment(Alignment::Center);
        f.render_widget(p, chunks[0]);
    } else if is_kitty_poster {
        let p = Paragraph::new("").block(poster_block);
        f.render_widget(p, chunks[0]);
    } else {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[ No Artwork ]",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ])
        .block(poster_block)
        .alignment(Alignment::Center);
        f.render_widget(p, chunks[0]);
    }

    if let Some(media) = item {
        // 2. Title Header with horizontal dividers
        let title_lines = vec![
            Line::from(Span::styled(
                &media.title,
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "────────────────────────────────────────────────",
                Style::default().fg(Theme::BORDER_MUTED),
            )),
        ];
        let title_p = Paragraph::new(title_lines).alignment(Alignment::Center);
        f.render_widget(title_p, chunks[1]);

        // 3. Metadata Table Rows
        let stars = format_stars(media.rating);
        let status = if !media.status.is_empty() {
            media.status.as_str()
        } else if media.media_type == "movie" {
            "RELEASED"
        } else {
            "SERIES"
        };
        let is_anime = media.genres.to_lowercase().contains("animation");
        let format_badge_str = if is_anime {
            if media.media_type == "movie" {
                "ANIME MOVIE"
            } else {
                "ANIME SERIES"
            }
        } else if media.media_type == "movie" {
            "FEATURE MOVIE"
        } else {
            "TV SERIES"
        };

        let mut genre_spans = vec![Span::styled("Genres      ", Style::default().fg(Theme::ACCENT_CYAN))];
        let genres_list: Vec<&str> = media
            .genres
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if genres_list.is_empty() {
            genre_spans.push(Span::styled("—", Style::default().fg(Theme::TEXT_MUTED)));
        } else {
            for g in genres_list {
                genre_spans.push(Span::styled(
                    format!("[{}] ", g),
                    Style::default().fg(Theme::TEXT_BODY).bg(Theme::SURFACE_ALT),
                ));
            }
        }

        let rows = vec![
            Line::from(vec![
                Span::styled("Score       ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(
                    format!("{} {:.1}/10", stars, media.rating),
                    Style::default()
                        .fg(Theme::ACCENT_GOLD)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Saved       ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(
                    if is_bookmarked {
                        "♥ Bookmarked in Saved"
                    } else {
                        "—"
                    },
                    Style::default().fg(if is_bookmarked {
                        Theme::ACCENT_PINK
                    } else {
                        Theme::TEXT_MUTED
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Status      ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(
                    status.to_uppercase(),
                    Style::default().fg(Theme::ACCENT_GREEN),
                ),
            ]),
            Line::from(Span::styled(
                "────────────────────────────────────────────────",
                Style::default().fg(Theme::BORDER_MUTED),
            )),
            Line::from(vec![
                Span::styled("Episodes    ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(
                    media
                        .episodes_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "1".to_string()),
                    Style::default().fg(Theme::TEXT_BODY),
                ),
            ]),
            Line::from(vec![
                Span::styled("Duration    ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(&media.duration_str, Style::default().fg(Theme::TEXT_BODY)),
            ]),
            Line::from(Span::styled(
                "────────────────────────────────────────────────",
                Style::default().fg(Theme::BORDER_MUTED),
            )),
            Line::from(genre_spans),
            Line::from(vec![
                Span::styled("Format      ", Style::default().fg(Theme::ACCENT_CYAN)),
                Span::styled(
                    format_badge_str,
                    Style::default()
                        .fg(Theme::ACCENT_PINK)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "────────────────────────────────────────────────",
                Style::default().fg(Theme::BORDER_MUTED),
            )),
            Line::from(vec![Span::styled(
                "Overview    ",
                Style::default().fg(Theme::ACCENT_CYAN),
            )]),
            Line::from(Span::styled(
                &media.overview,
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ];

        let table_p = Paragraph::new(rows).wrap(Wrap { trim: true });
        f.render_widget(table_p, chunks[2]);
    } else {
        let p = Paragraph::new(vec![Line::from(Span::styled(
            "Select an item to view metadata",
            Style::default().fg(Theme::TEXT_MUTED),
        ))]);
        f.render_widget(p, chunks[1]);
    }
}

fn format_stars(rating: f64) -> &'static str {
    if rating >= 8.5 {
        "★★★★★"
    } else if rating >= 7.0 {
        "★★★★☆"
    } else if rating >= 5.0 {
        "★★★☆☆"
    } else if rating >= 3.0 {
        "★★☆☆☆"
    } else {
        "★☆☆☆☆"
    }
}
