use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::{ProviderModalCategory, ProviderModalPane, ProviderSelectModalState};

pub fn render_provider_modal(f: &mut Frame, area: Rect, state: &ProviderSelectModalState) {
    let popup_area = centered_rect(area, 80, 72);
    f.render_widget(Clear, popup_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_CYAN))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            format!(" 🐱 KittyMov • Tracks • {} ", state.target_title),
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ))
        .title(
            Span::styled(
                " ● ready ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            )
            .into_right_aligned_line(),
        );

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary bar
            Constraint::Min(10),   // Two-column split
            Constraint::Length(3), // Hotkey guide footer
        ])
        .margin(1)
        .split(popup_area);

    f.render_widget(modal_block, popup_area);

    // 1. Summary Bar (Kunai style: Provider <name> • KittyMov Tracks • N providers • N qualities • N subtitles)
    let selected_provider = state
        .options
        .get(state.selected_idx)
        .map(|p| p.name.as_str())
        .unwrap_or("Auto Fallback");

    let header_lines = vec![
        Line::from(vec![
            Span::styled("Provider  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                selected_provider,
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" 🐱 KittyMov ", Style::default().fg(Theme::ACCENT_PINK)),
            Span::styled("Tracks  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!(
                    "{} providers • {} qualities • {} subtitles • {}",
                    state.options.len(),
                    state.qualities.len(),
                    state.subtitles.len(),
                    selected_provider
                ),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
    ];
    let header_p = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Theme::BORDER_MUTED)),
    );
    f.render_widget(header_p, inner_chunks[0]);

    // 2. Split into Left Category Column (width 26) and Right Content Column
    let body_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(40)])
        .split(inner_chunks[1]);

    // --- LEFT COLUMN: Dimensions (Provider, Quality, Subtitles) ---
    let left_is_active = state.active_pane == ProviderModalPane::Categories;
    let mut left_lines = Vec::new();
    left_lines.push(Line::from(""));

    let categories = [
        (
            ProviderModalCategory::Provider,
            "Provider",
            selected_provider,
        ),
        (
            ProviderModalCategory::Quality,
            "Quality",
            state
                .qualities
                .get(state.selected_quality_idx)
                .map(|q| q.label.as_str())
                .unwrap_or("Auto"),
        ),
        (
            ProviderModalCategory::Subtitles,
            "Subtitles",
            state
                .subtitles
                .get(state.selected_subtitle_idx)
                .map(|s| s.label.as_str())
                .unwrap_or("Auto Match"),
        ),
    ];

    for (cat, name, cur_val) in categories {
        let is_focused = state.active_category == cat;
        let prefix = if is_focused { "▸ " } else { "  " };

        let short_val = if cur_val.len() > 13 {
            format!("{}...", &cur_val[..10])
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

    // --- RIGHT COLUMN: Active Category Items List ---
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
                    Span::styled(format!("{:<18}", opt.name), name_style),
                    Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
                    Span::styled(&opt.description, desc_style),
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
                        Span::styled(format!("{:<12}", sub.label), label_style),
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

    // 3. Hotkey Guide Footer
    let hints = Line::from(vec![
        Span::styled(
            "Tracks ",
            Style::default()
                .fg(Theme::TEXT_BODY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            "[↑↓] ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("choose  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[→ / Tab] ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("enter  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[← / Tab] ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("back  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[Enter] ",
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("switch / play  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[d] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("sub/dub  ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            "[Esc] ",
            Style::default()
                .fg(Theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("close", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    let footer_p = Paragraph::new(hints)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER_MUTED)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer_p, inner_chunks[2]);
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
