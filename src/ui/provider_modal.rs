use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::{ProviderModalCategory, ProviderModalPane, ProviderSelectModalState};

pub fn render_provider_modal(f: &mut Frame, area: Rect, state: &ProviderSelectModalState) {
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::BG));

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Layout: Top Bar (3), Header (3), Body (Min 12), Footer (4)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top status bar
            Constraint::Length(4), // Tracks summary header
            Constraint::Min(12),   // Two-column split
            Constraint::Length(4), // Bottom command footer
        ])
        .margin(1)
        .split(inner_area);

    let selected_provider = state
        .options
        .get(state.selected_idx)
        .map(|p| p.name.as_str())
        .unwrap_or("Auto Fallback");

    let selected_source = state
        .sources
        .get(state.selected_source_idx)
        .map(|s| s.name.as_str())
        .unwrap_or("Vanguard");

    let selected_quality = state
        .qualities
        .get(state.selected_quality_idx)
        .map(|q| q.label.as_str())
        .unwrap_or("Auto");

    let selected_subtitle = state
        .subtitles
        .get(state.selected_subtitle_idx)
        .map(|s| s.label.as_str())
        .unwrap_or("Auto Match");

    // 1. Top Bar (Kunai: 🦊 Kunai • [Tracks] series • Videasy->Rivestream • 🔔 1 | ● ready)
    let top_left = Line::from(vec![
        Span::styled(
            " 🐱 KittyMov ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            " [Tracks] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", state.request.media_type),
            Style::default().fg(Theme::TEXT_BODY),
        ),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            format!("{} ", selected_provider),
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

    // 2. Summary Header Area
    let header_lines = vec![
        Line::from(vec![
            Span::styled("Provider ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                state
                    .options
                    .get(state.selected_idx)
                    .map(|p| p.id.as_str())
                    .unwrap_or("auto"),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "🐱 KittyMov ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Tracks",
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!(
                "{} providers • {} sources • {} qualities • {} subtitles • {}",
                state.options.len(),
                state.sources.len(),
                state.qualities.len(),
                state.subtitles.len(),
                selected_source
            ),
            Style::default().fg(Theme::TEXT_MUTED),
        )]),
    ];
    let header_p = Paragraph::new(header_lines);
    f.render_widget(header_p, chunks[1]);

    // 3. Two-Column Split (Left Categories 26 chars, Right Items List)
    let body_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(40)])
        .split(chunks[2]);

    // --- LEFT COLUMN: Dimensions (Provider, Source, Quality, Subtitles) ---
    let left_is_active = state.active_pane == ProviderModalPane::Categories;
    let mut left_lines = Vec::new();
    left_lines.push(Line::from(""));

    let categories = [
        (
            ProviderModalCategory::Provider,
            "Provider",
            selected_provider,
        ),
        (ProviderModalCategory::Source, "Source", selected_source),
        (ProviderModalCategory::Quality, "Quality", selected_quality),
        (
            ProviderModalCategory::Subtitles,
            "Subtitles",
            selected_subtitle,
        ),
    ];

    for (cat, name, cur_val) in categories {
        let is_focused = state.active_category == cat;
        let prefix = if is_focused { "▸ " } else { "  " };

        let short_val = if cur_val.len() > 12 {
            format!("{}...", &cur_val[..9])
        } else {
            cur_val.to_string()
        };

        if is_focused && left_is_active {
            left_lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(Theme::ACCENT_PINK)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<10} ", name),
                    Style::default()
                        .fg(Theme::TEXT_TITLE)
                        .bg(Theme::SELECTION_BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    short_val,
                    Style::default()
                        .fg(Theme::ACCENT_CYAN)
                        .bg(Theme::SELECTION_BG),
                ),
            ]));
        } else if is_focused {
            left_lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(Theme::ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<10} ", name),
                    Style::default()
                        .fg(Theme::TEXT_TITLE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(short_val, Style::default().fg(Theme::ACCENT_CYAN)),
            ]));
        } else {
            left_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(
                    format!("{:<10} ", name),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(short_val, Style::default().fg(Theme::BORDER_MUTED)),
            ]));
        }
        left_lines.push(Line::from(""));
    }

    let left_border = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Theme::BORDER_MUTED));
    let left_p = Paragraph::new(left_lines).block(left_border);
    f.render_widget(left_p, body_cols[0]);

    // --- RIGHT COLUMN: Active Dimension Item Inspector ---
    let right_is_active = state.active_pane == ProviderModalPane::Content;
    let mut right_lines = Vec::new();

    match state.active_category {
        ProviderModalCategory::Provider => {
            right_lines.push(Line::from(vec![Span::styled(
                format!("PROVIDER • {}", state.options.len()),
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )]));
            right_lines.push(Line::from(""));

            for (i, opt) in state.options.iter().enumerate() {
                let is_sel = i == state.selected_idx;
                let cursor = if is_sel { "› ✓ " } else { "    " };

                let (name_style, desc_style) = if is_sel && right_is_active {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(Theme::SELECTION_BG),
                    )
                } else if is_sel {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Theme::TEXT_BODY),
                    )
                } else {
                    (
                        Style::default().fg(Theme::TEXT_BODY),
                        Style::default().fg(Theme::TEXT_MUTED),
                    )
                };

                right_lines.push(Line::from(vec![
                    Span::styled(
                        cursor,
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{:<20}", opt.name), name_style),
                    Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
                    Span::styled(&opt.description, desc_style),
                ]));
                right_lines.push(Line::from(""));
            }
        }
        ProviderModalCategory::Source => {
            right_lines.push(Line::from(vec![Span::styled(
                format!("SOURCE • {}", state.sources.len()),
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )]));
            right_lines.push(Line::from(""));

            for (i, src) in state.sources.iter().enumerate() {
                let is_sel = i == state.selected_source_idx;
                let cursor = if is_sel { "› ✓ " } else { "    " };

                let (name_style, desc_style) = if is_sel && right_is_active {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(Theme::SELECTION_BG),
                    )
                } else if is_sel {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Theme::TEXT_BODY),
                    )
                } else {
                    (
                        Style::default().fg(Theme::TEXT_BODY),
                        Style::default().fg(Theme::TEXT_MUTED),
                    )
                };

                right_lines.push(Line::from(vec![
                    Span::styled(
                        cursor,
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{:<20}", src.name), name_style),
                    Span::styled(
                        format!(" {:<16}", src.status_str),
                        Style::default().fg(if is_sel {
                            Theme::ACCENT_GREEN
                        } else {
                            Theme::TEXT_MUTED
                        }),
                    ),
                    Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
                    Span::styled(&src.description, desc_style),
                ]));
                right_lines.push(Line::from(""));
            }
        }
        ProviderModalCategory::Quality => {
            right_lines.push(Line::from(vec![Span::styled(
                format!("QUALITY • {}", state.qualities.len()),
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )]));
            right_lines.push(Line::from(""));

            for (i, q) in state.qualities.iter().enumerate() {
                let is_sel = i == state.selected_quality_idx;
                let cursor = if is_sel { "› ✓ " } else { "    " };

                let (label_style, desc_style) = if is_sel && right_is_active {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(Theme::SELECTION_BG),
                    )
                } else if is_sel {
                    (
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Theme::TEXT_BODY),
                    )
                } else {
                    (
                        Style::default().fg(Theme::TEXT_BODY),
                        Style::default().fg(Theme::TEXT_MUTED),
                    )
                };

                right_lines.push(Line::from(vec![
                    Span::styled(
                        cursor,
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{:<18}", q.label), label_style),
                    Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
                    Span::styled(&q.description, desc_style),
                ]));
                right_lines.push(Line::from(""));
            }
        }
        ProviderModalCategory::Subtitles => {
            right_lines.push(Line::from(vec![Span::styled(
                format!("SUBTITLES • {}", state.subtitles.len()),
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )]));
            right_lines.push(Line::from(""));

            if state.is_loading_subtitles {
                right_lines.push(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(
                        "Loading subtitle inventory...",
                        Style::default()
                            .fg(Theme::ACCENT_GOLD)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            } else {
                for (i, sub) in state.subtitles.iter().enumerate() {
                    let is_sel = i == state.selected_subtitle_idx;
                    let cursor = if is_sel { "› ✓ " } else { "    " };

                    let badge_color = if sub.is_native {
                        Theme::ACCENT_GREEN
                    } else if sub.lang_code == "off" {
                        Theme::TEXT_MUTED
                    } else {
                        Theme::ACCENT_CYAN
                    };

                    let (label_style, desc_style) = if is_sel && right_is_active {
                        (
                            Style::default()
                                .fg(Theme::TEXT_TITLE)
                                .bg(Theme::SELECTION_BG)
                                .add_modifier(Modifier::BOLD),
                            Style::default()
                                .fg(Theme::TEXT_BODY)
                                .bg(Theme::SELECTION_BG),
                        )
                    } else if is_sel {
                        (
                            Style::default()
                                .fg(Theme::TEXT_TITLE)
                                .add_modifier(Modifier::BOLD),
                            Style::default().fg(Theme::TEXT_BODY),
                        )
                    } else {
                        (
                            Style::default().fg(Theme::TEXT_BODY),
                            Style::default().fg(Theme::TEXT_MUTED),
                        )
                    };

                    right_lines.push(Line::from(vec![
                        Span::styled(
                            cursor,
                            Style::default()
                                .fg(Theme::ACCENT_GREEN)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("{:<14}", sub.label), label_style),
                        Span::styled(
                            format!(" [{}] ", sub.badge),
                            Style::default()
                                .fg(badge_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
                        Span::styled(&sub.description, desc_style),
                    ]));
                    right_lines.push(Line::from(""));
                }
            }
        }
    }

    let right_p = Paragraph::new(right_lines);
    f.render_widget(right_p, body_cols[1]);

    // 4. Hotkey Guide Footer (matching screenshot layout)
    let hints = Line::from(vec![
        Span::styled(
            "[↑↓] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("choose  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[→] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("enter  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[enter] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("switch / play  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[f] ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("favorite  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[d] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("sub/dub  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[q / Esc] ",
            Style::default()
                .fg(Theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("back", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let footer_lines = vec![
        Line::from(Span::styled(
            "Tracks",
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        hints,
    ];

    let footer_p = Paragraph::new(footer_lines);
    f.render_widget(footer_p, chunks[3]);
}
