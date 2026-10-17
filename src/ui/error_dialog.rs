use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::ErrorDialogState;

pub fn render_error_dialog(f: &mut Frame, area: Rect, dialog: &ErrorDialogState) {
    let width = 74.min(area.width.saturating_sub(4)).max(48);
    let height = 18.min(area.height.saturating_sub(2)).max(14);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_PINK))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            format!(" ✖ Playback Stream Error • {} ", dialog.title),
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = modal_block.inner(popup_area);
    f.render_widget(modal_block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Footer hint
        ])
        .split(inner_area);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            &dialog.primary_message,
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled(
            "Diagnostics:",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {}", dialog.technical_details),
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled(
            "Troubleshooting & Next Steps:",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    for suggestion in &dialog.suggestions {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  • {}", suggestion),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, chunks[0]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "[ Press Enter or Esc to Dismiss • Press s to open Provider selector ]",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[1]);
}
