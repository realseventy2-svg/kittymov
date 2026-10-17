use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::subtitles::SubtitleLanguageGroup;

#[allow(clippy::too_many_arguments)]
pub fn render_subtitle_modal(
    f: &mut Frame,
    area: Rect,
    target_title: &str,
    current_lang: &str,
    subtitle_source: &str,
    options: &[SubtitleLanguageGroup],
    selected_idx: usize,
    is_loading: bool,
) {
    let width = 64.min(area.width.saturating_sub(4)).max(45);
    let height = (options.len() as u16 + 8)
        .min(area.height.saturating_sub(4))
        .max(12);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_CYAN))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            " KittyMov • Subtitle & Language Selector ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title & provider source
            Constraint::Min(5),    // Language list
            Constraint::Length(3), // Hotkey guide footer
        ])
        .margin(1)
        .split(popup_area);

    f.render_widget(modal_block, popup_area);

    // 1. Header subtitle
    let source_label = match subtitle_source.to_lowercase().as_str() {
        "provider" => "Provider Native Only",
        "opensubtitles" => "OpenSubtitles Only",
        "both" | "all" => "Both (All Tracks)",
        _ => "Auto (Provider + OpenSubtitles)",
    };
    let desc = Line::from(vec![
        Span::styled(
            format!("Media: {}  ", target_title),
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("• Mode: {}", source_label),
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(desc), inner_chunks[0]);

    // 2. Language options
    let mut lines = Vec::new();

    if is_loading {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "   Fetching subtitle tracks from OpenSubtitles...",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::ITALIC),
        )]));
    } else if options.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "   No external subtitle tracks found for this title.",
            Style::default().fg(Theme::TEXT_MUTED),
        )]));
        lines.push(Line::from(
            "   Embedded video captions will be used if available.",
        ));
    } else {
        for (i, opt) in options.iter().enumerate() {
            let is_sel = i == selected_idx;
            let is_active = opt.lang_code == current_lang;

            let badge = if is_active { " [ACTIVE] " } else { "" };
            let count_str = format!("({} tracks)", opt.track_count);

            if is_sel {
                lines.push(Line::from(vec![
                    Span::styled(
                        " ◆ ",
                        Style::default()
                            .fg(Theme::ACCENT_PINK)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("[{}] ", opt.lang_code),
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<18}", opt.lang_name),
                        Style::default()
                            .fg(Theme::TEXT_TITLE)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<14}", count_str),
                        Style::default()
                            .fg(Theme::TEXT_MUTED)
                            .bg(Theme::SELECTION_BG),
                    ),
                    Span::styled(
                        badge,
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(
                        format!("[{}] ", opt.lang_code),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                    Span::styled(
                        format!("{:<18}", opt.lang_name),
                        Style::default().fg(Theme::TEXT_BODY),
                    ),
                    Span::styled(
                        format!("{:<14}", count_str),
                        Style::default().fg(Theme::BORDER_MUTED),
                    ),
                    Span::styled(badge, Style::default().fg(Theme::ACCENT_GREEN)),
                ]));
            }
        }
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner_chunks[1]);

    // 3. Hotkey footer
    let hints = Line::from(vec![Span::styled(
        " [Enter] Select Language   [s] Cycle Source   [↑/↓] Navigate   [Esc] Close ",
        Style::default().fg(Theme::TEXT_MUTED),
    )]);
    let footer_p = Paragraph::new(hints)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER_MUTED)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer_p, inner_chunks[2]);
}
