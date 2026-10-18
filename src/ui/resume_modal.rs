use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::ResumeModalState;

pub fn render_resume_modal(
    f: &mut Frame,
    area: Rect,
    state: &ResumeModalState,
    preferred_provider: &str,
    is_kitty_poster: bool,
) {
    f.render_widget(Clear, area);

    let bg_block = Block::default().style(Style::default().bg(Theme::BG));
    f.render_widget(bg_block, area);

    // Layout: Top Status (3), Title & Subtitle (3), Filter Box (4), Options & Selection Body (Min 10), Command Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top status bar
            Constraint::Length(3), // Title & Subtitle
            Constraint::Length(4), // Filter input area
            Constraint::Min(10),   // Options list + Current selection panel
            Constraint::Length(3), // Bottom command bar
        ])
        .margin(1)
        .split(area);

    let media_type_str = if state.item.media_type == "tv" {
        "series"
    } else {
        "movie"
    };

    // 1. Top Bar: 🐱 KittyMov • Where to start? series • Provider • 🔥 3d • 🔔 1 | ● ready
    let top_left = Line::from(vec![
        Span::styled(
            " 🐱 KittyMov ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            " Where to start? ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", media_type_str),
            Style::default().fg(Theme::TEXT_BODY),
        ),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            format!("{} ", preferred_provider),
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("• 🔔 1", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let top_right = Line::from(vec![Span::styled(
        "● ready ",
        Style::default()
            .fg(Theme::ACCENT_GREEN)
            .add_modifier(Modifier::BOLD),
    )]);

    let top_bar_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Theme::BORDER_MUTED));
    let top_p = Paragraph::new(top_left)
        .block(top_bar_block)
        .alignment(Alignment::Left);
    f.render_widget(top_p, chunks[0]);

    let top_right_p = Paragraph::new(top_right).alignment(Alignment::Right);
    f.render_widget(top_right_p, chunks[0]);

    // 2. Title & Subtitle Area
    let header_lines = vec![
        Line::from(vec![Span::styled(
            "Where to start?",
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                format!("{} • {} • ", state.item.title, preferred_provider),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                if state.item.media_type == "tv" {
                    "choose the starting season and episode"
                } else {
                    "choose starting point or options"
                },
                Style::default().fg(Theme::ACCENT_CYAN),
            ),
        ]),
    ];
    let header_p = Paragraph::new(header_lines);
    f.render_widget(header_p, chunks[1]);

    // 3. Filter Box
    let filtered = state.filtered_indices();
    let selected_filtered_idx = state.selected_filtered_index();

    let filter_border = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(if state.is_filtering {
            Theme::ACCENT_PINK
        } else {
            Theme::BORDER_MUTED
        }));

    let filter_text_line = if state.is_filtering {
        Line::from(vec![
            Span::styled(
                " ⌕ ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&state.filter_query, Style::default().fg(Theme::TEXT_TITLE)),
            Span::styled(
                "█",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ])
    } else if state.filter_query.is_empty() {
        Line::from(vec![
            Span::styled(" ⌕ ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "Type to narrow this list",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ⌕ ", Style::default().fg(Theme::ACCENT_CYAN)),
            Span::styled(&state.filter_query, Style::default().fg(Theme::TEXT_TITLE)),
        ])
    };

    let filter_sub_line = Line::from(vec![
        Span::styled(
            "Type to filter",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
        Span::styled(
            format!("    {} of {}", selected_filtered_idx + 1, filtered.len()),
            Style::default().fg(Theme::ACCENT_GOLD),
        ),
    ]);

    let filter_widget = Paragraph::new(vec![
        Line::from(Span::styled(
            "Filter",
            Style::default()
                .fg(if state.is_filtering {
                    Theme::ACCENT_PINK
                } else {
                    Theme::ACCENT_CYAN
                })
                .add_modifier(Modifier::BOLD),
        )),
        filter_text_line,
        filter_sub_line,
    ])
    .block(filter_border);
    f.render_widget(filter_widget, chunks[2]);

    // 4. Main Body: Left Options List (66%) and Right Current Selection (34%)
    let body_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(chunks[3]);

    // Left Column: Options List
    let mut list_lines = Vec::new();

    for (pos_idx, &opt_idx) in filtered.iter().enumerate() {
        let opt = &state.options[opt_idx];
        let is_selected = pos_idx == selected_filtered_idx;

        if is_selected {
            // Selected item with highlighted magenta background and pink cursor
            let line1 = Line::from(vec![
                Span::styled(
                    "▎ ",
                    Style::default()
                        .fg(Theme::ACCENT_PINK)
                        .bg(Color::Rgb(48, 22, 38))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", opt.icon),
                    Style::default()
                        .fg(Theme::ACCENT_PINK)
                        .bg(Color::Rgb(48, 22, 38))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<22} ", opt.label),
                    Style::default()
                        .fg(Theme::TEXT_TITLE)
                        .bg(Color::Rgb(48, 22, 38))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    opt.sublabel.clone(),
                    Style::default()
                        .fg(Theme::ACCENT_CYAN)
                        .bg(Color::Rgb(48, 22, 38))
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            let line2 = Line::from(vec![
                Span::styled("▎     ", Style::default().bg(Color::Rgb(48, 22, 38))),
                Span::styled(
                    opt.description.clone(),
                    Style::default()
                        .fg(Theme::TEXT_BODY)
                        .bg(Color::Rgb(48, 22, 38)),
                ),
            ]);
            list_lines.push(line1);
            list_lines.push(line2);
        } else {
            // Unselected item
            let line1 = Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{} ", opt.icon),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(
                    format!("{:<22} ", opt.label),
                    Style::default()
                        .fg(Theme::TEXT_BODY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(opt.sublabel.clone(), Style::default().fg(Theme::TEXT_MUTED)),
            ]);
            let line2 = Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(
                    opt.description.clone(),
                    Style::default().fg(Color::Rgb(110, 115, 128)),
                ),
            ]);
            list_lines.push(line1);
            list_lines.push(line2);
        }
        list_lines.push(Line::from(""));
    }

    let list_p = Paragraph::new(list_lines);
    f.render_widget(list_p, body_cols[0]);

    // Right Column: Current Selection Panel
    let (_target_w, target_h) = crate::ui::poster::calculate_poster_dimensions_for_panel(
        (area.width.saturating_sub(2) as f32 * 0.38) as u16,
        area.height.saturating_sub(2),
    );

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),        // "Current Selection" header
            Constraint::Length(target_h), // Poster artwork
            Constraint::Min(4),           // Progress & details
        ])
        .split(body_cols[1]);

    // 1. Header
    let header_p = Paragraph::new(vec![
        Line::from(Span::styled(
            "Current Selection",
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ]);
    f.render_widget(header_p, right_chunks[0]);

    // 2. Poster artwork
    if let Some(ref text) = state.poster_text {
        let poster_p = Paragraph::new(text.clone()).alignment(Alignment::Center);
        f.render_widget(poster_p, right_chunks[1]);
    } else if is_kitty_poster {
        let empty_p = Paragraph::new("");
        f.render_widget(empty_p, right_chunks[1]);
    } else {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "🎬",
                Style::default().fg(Theme::ACCENT_CYAN),
            )),
            Line::from(Span::styled(
                state.item.title.as_str(),
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                state.item.year.as_str(),
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(placeholder, right_chunks[1]);
    }

    // 3. Progress details & selected option summary
    let selected_opt = state.get_selected_option();
    let mut details_lines = Vec::new();
    details_lines.push(Line::from(""));

    if let Some(opt) = selected_opt {
        details_lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", opt.icon),
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                opt.label.clone(),
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        details_lines.push(Line::from(vec![Span::styled(
            opt.sublabel.clone(),
            Style::default().fg(Theme::ACCENT_CYAN),
        )]));
    }

    if state.duration_seconds > 0 {
        let pct = ((state.progress_seconds as f64 / state.duration_seconds as f64) * 100.0)
            .clamp(0.0, 100.0) as usize;
        let bar_len = 16;
        let filled = (pct * bar_len) / 100;
        let empty = bar_len.saturating_sub(filled);
        let bar = format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), pct);

        details_lines.push(Line::from(""));
        details_lines.push(Line::from(vec![Span::styled(
            bar,
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        )]));
        details_lines.push(Line::from(vec![Span::styled(
            format!(
                "Progress: {} / {}",
                crate::app::state::format_time(state.progress_seconds),
                crate::app::state::format_time(state.duration_seconds)
            ),
            Style::default().fg(Theme::TEXT_MUTED),
        )]));
    }

    let details_p = Paragraph::new(details_lines).alignment(Alignment::Center);
    f.render_widget(details_p, right_chunks[2]);

    // 5. Command Footer
    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Theme::BORDER_MUTED));

    let footer_text = vec![
        Line::from(vec![
            Span::styled(
                "Choose starting point • ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                state.item.title.as_str(),
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                preferred_provider,
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if state.item.media_type == "tv" {
                    " • choose the starting season and episode"
                } else {
                    " • choose starting point or options"
                },
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "[esc] ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("back   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "[enter] ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("select   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "[/] ",
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("filter   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "[k/↑] ",
                Style::default()
                    .fg(Theme::TEXT_BODY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("up   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "[j/↓] ",
                Style::default()
                    .fg(Theme::TEXT_BODY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("down", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let footer_p = Paragraph::new(footer_text)
        .block(footer_block)
        .alignment(Alignment::Left);
    f.render_widget(footer_p, chunks[4]);
}
