use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::{PlaybackLoadingState, StepStatus};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render_loading_screen(f: &mut Frame, area: Rect, state: &PlaybackLoadingState) {
    // Clear whole area
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::BG));

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Layout: Top bar (3 rows), Center body (Min 10), Bottom bar (3 rows)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top bar
            Constraint::Min(12),   // Split body
            Constraint::Length(3), // Bottom hints
        ])
        .split(inner_area);

    // 1. Top Bar
    let spinner_char = SPINNER_FRAMES[state.spinner_idx % SPINNER_FRAMES.len()];
    let top_left = Line::from(vec![
        Span::styled(
            " 🐱 KittyMov ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            "Now Playing ",
            Style::default()
                .fg(Theme::TEXT_BODY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", state.media_type),
            Style::default().fg(Theme::ACCENT_PINK),
        ),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            format!("{} ", state.provider_name),
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("• 🔔 1", Style::default().fg(Theme::TEXT_MUTED)),
    ]);

    let top_right = Line::from(vec![
        Span::styled(
            format!("{} ", spinner_char),
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Loading... ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let top_bar_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Theme::BORDER_MUTED));
    let top_p = Paragraph::new(top_left)
        .block(top_bar_block)
        .alignment(Alignment::Left);
    f.render_widget(top_p, rows[0]);

    let top_right_p = Paragraph::new(top_right).alignment(Alignment::Right);
    f.render_widget(top_right_p, rows[0]);

    // 2. Center Body (Split into Left Stepper Pane 58% and Right Metadata/Poster Pane 42%)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(rows[1]);

    // --- LEFT PANE: Playback Bootstrap Stepper ---
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Resolving via header
            Constraint::Length(8), // Stepper rows
            Constraint::Min(2),    // Session info & buffer
        ])
        .margin(1)
        .split(cols[0]);

    // Resolving header with flower symbol: ✿ ✿ ✿ Resolving via ...
    let resolving_title = Line::from(vec![
        Span::styled(
            "✿ ✿ ✿  ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Resolving via {}", state.provider_name),
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(resolving_title), left_layout[0]);

    // Stepper rows
    let mut step_lines = Vec::new();
    for item in &state.steps {
        let (icon, icon_style, title_style) = match item.status {
            StepStatus::Completed => (
                "✓ ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Theme::TEXT_BODY),
            ),
            StepStatus::Active => (
                "● ",
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            StepStatus::Pending => (
                "• ",
                Style::default().fg(Theme::TEXT_MUTED),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            StepStatus::Failed => (
                "✕ ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        step_lines.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(icon, icon_style),
            Span::styled(item.title, title_style),
            Span::styled("  ", Style::default()),
            Span::styled(
                &item.detail,
                Style::default()
                    .fg(if item.status == StepStatus::Active {
                        Theme::ACCENT_GOLD
                    } else {
                        Theme::TEXT_MUTED
                    })
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(step_lines), left_layout[1]);

    // Session status line: session autoskip on • autoplay on • sub
    let autoskip_str = if state.session_autoskip { "on" } else { "off" };
    let autoplay_str = if state.session_autoplay { "on" } else { "off" };
    let mode_str = state.session_audio_mode.to_lowercase();

    let session_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   session  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("autoskip {} ", autoskip_str),
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
            Span::styled(
                format!("autoplay {} ", autoplay_str),
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
            Span::styled(
                format!("audio: {} ", mode_str.to_uppercase()),
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(session_lines), left_layout[2]);

    // --- RIGHT PANE: Artwork, Details & Up Next ---
    let right_border = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE));
    let right_inner = right_border.inner(cols[1]);
    f.render_widget(right_border, cols[1]);

    let poster_len = if let Some(ref text) = state.poster_text {
        (text.lines.len() as u16)
            .min(right_inner.height.saturating_sub(16))
            .max(10)
    } else {
        12
    };
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(poster_len), // Responsive poster frame
            Constraint::Length(3),          // Title + episode info
            Constraint::Length(5),          // — details —
            Constraint::Min(4),             // — synopsis —
            Constraint::Length(4),          // — up next —
        ])
        .margin(1)
        .split(right_inner);

    // 1. Poster Art
    if let Some(ref text) = state.poster_text {
        let p = Paragraph::new(text.clone()).alignment(Alignment::Center);
        f.render_widget(p, right_layout[0]);
    } else {
        let empty_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[ Artwork Loading... ]",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(empty_p, right_layout[0]);
    }

    // 2. Title & Type Badge
    let title_line = vec![
        Line::from(vec![Span::styled(
            format!("* {} ", state.media_type),
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            &state.title,
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            &state.subtitle,
            Style::default().fg(Theme::TEXT_MUTED),
        )]),
    ];
    f.render_widget(Paragraph::new(title_line), right_layout[1]);

    // 3. — details —
    let details_lines = vec![
        Line::from(Span::styled(
            "— details —",
            Style::default().fg(Theme::BORDER_MUTED),
        )),
        Line::from(vec![
            Span::styled("year      ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.year, Style::default().fg(Theme::TEXT_BODY)),
        ]),
        Line::from(vec![
            Span::styled("genre     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.genre, Style::default().fg(Theme::TEXT_BODY)),
        ]),
        Line::from(vec![
            Span::styled("score     ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("★ {:.1}", state.score),
                Style::default().fg(Theme::ACCENT_GOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("studio    ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.studio, Style::default().fg(Theme::TEXT_BODY)),
        ]),
    ];
    f.render_widget(Paragraph::new(details_lines), right_layout[2]);

    // 4. — synopsis —
    let mut syn_lines = vec![Line::from(Span::styled(
        "— synopsis —",
        Style::default().fg(Theme::BORDER_MUTED),
    ))];
    if state.synopsis.is_empty() {
        syn_lines.push(Line::from(Span::styled(
            "No synopsis available.",
            Style::default().fg(Theme::TEXT_MUTED),
        )));
    } else {
        syn_lines.push(Line::from(Span::styled(
            &state.synopsis,
            Style::default().fg(Theme::TEXT_MUTED),
        )));
    }
    let syn_p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
    f.render_widget(syn_p, right_layout[3]);

    // 5. — up next —
    if let Some(ref up) = state.up_next {
        let up_lines = vec![
            Line::from(Span::styled(
                "— up next —",
                Style::default().fg(Theme::BORDER_MUTED),
            )),
            Line::from(vec![
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Theme::ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    up,
                    Style::default()
                        .fg(Theme::TEXT_BODY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(up_lines), right_layout[4]);
    }

    // 3. Bottom Bar / Hints
    let footer_hints = Line::from(vec![
        Span::styled(
            "Playback bootstrap ",
            Style::default()
                .fg(Theme::TEXT_BODY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            "[⇧F] ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("fallback provider ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("• ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(
            "[q / Esc] ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("cancel", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    let footer_p = Paragraph::new(footer_hints)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER_MUTED)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer_p, rows[2]);
}
