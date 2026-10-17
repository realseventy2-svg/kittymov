use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::ProviderOption;

pub fn render_provider_modal(
    f: &mut Frame,
    area: Rect,
    target_title: &str,
    audio_mode: &str,
    options: &[ProviderOption],
    selected_idx: usize,
) {
    let popup_area = centered_rect(area, 70, 58);
    f.render_widget(Clear, popup_area);

    let is_sub = audio_mode.to_lowercase() == "sub";
    let mode_badge = if is_sub {
        Span::styled(" [MODE: SUB] ", Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [MODE: DUB] ", Style::default().fg(Theme::ACCENT_CYAN).add_modifier(Modifier::BOLD))
    };

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(" KittyMov • Streaming Server Selector ", Style::default().fg(Theme::ACCENT_CYAN).add_modifier(Modifier::BOLD)))
        .title(mode_badge.into_right_aligned_line());

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Target title & audio mode line
            Constraint::Min(6),    // Provider options list
            Constraint::Length(3), // Hotkey footer
        ])
        .margin(1)
        .split(popup_area);

    f.render_widget(modal_block, popup_area);

    // 1. Target Title & Mode Description
    let header_lines = vec![
        Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(target_title, Style::default().fg(Theme::TEXT_TITLE).add_modifier(Modifier::BOLD)),
            Span::styled("  •  Mode: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                if is_sub { "Sub (Original + English Subs)" } else { "Dub (English Audio Track)" },
                Style::default().fg(if is_sub { Theme::ACCENT_PINK } else { Theme::ACCENT_CYAN }).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [d]  •  Subs: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("OpenSubtitles [t]", Style::default().fg(Theme::ACCENT_GOLD).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let header_p = Paragraph::new(header_lines)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Theme::BORDER_MUTED)));
    f.render_widget(header_p, inner_chunks[0]);

    // 2. Provider Options List
    let mut list_lines = Vec::new();
    for (i, opt) in options.iter().enumerate() {
        let is_sel = i == selected_idx;
        let num_badge = format!("[{}] ", i + 1);

        if is_sel {
            list_lines.push(Line::from(vec![
                Span::styled(" ◆ ", Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD)),
                Span::styled(num_badge, Style::default().fg(Theme::ACCENT_CYAN).bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD)),
                Span::styled(&opt.name, Style::default().fg(Theme::TEXT_TITLE).bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD)),
            ]));
            list_lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(&opt.description, Style::default().fg(Theme::ACCENT_GREEN).bg(Theme::SELECTION_BG)),
            ]));
        } else {
            list_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(num_badge, Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(&opt.name, Style::default().fg(Theme::TEXT_BODY)),
            ]));
            list_lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(&opt.description, Style::default().fg(Theme::TEXT_MUTED)),
            ]));
        }
        list_lines.push(Line::from(""));
    }

    let list_p = Paragraph::new(list_lines);
    f.render_widget(list_p, inner_chunks[1]);

    // 3. Hotkey Footer
    let hints = Line::from(vec![
        Span::styled(" [Enter / 1-3] Launch Server   [↑/↓ / k/j] Select   [d] Sub/Dub   [t] Subtitles   [Esc] Cancel ", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    let footer_p = Paragraph::new(hints)
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Theme::BORDER_MUTED)))
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
