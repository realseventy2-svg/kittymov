use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;

#[allow(clippy::vec_init_then_push)]
pub fn render_help_modal(f: &mut Frame, area: Rect) {
    let width = 76.min(area.width.saturating_sub(4)).max(50);
    let height = 24.min(area.height.saturating_sub(2)).max(16);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_GOLD))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            " ⌨ KittyMov Keyboard Reference & Help ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = modal_block.inner(popup_area);
    f.render_widget(modal_block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Content
            Constraint::Length(2), // Footer hint
        ])
        .split(inner_area);

    let mut lines = Vec::new();

    // Section 1: Main Catalog
    lines.push(Line::from(vec![Span::styled(
        "◆ MAIN CATALOG & NAVIGATION",
        Style::default()
            .fg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled("  [1 - 4]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Switch Tabs (Movies, Series, History, Bookmarks)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  [↑ / ↓ / j / k] ",
            Style::default().fg(Theme::ACCENT_GOLD),
        ),
        Span::styled(
            "Navigate catalog items list",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [Enter]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Play movie or open Episode Guide for series",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [/]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Live Search (type query, Enter to search, Esc to exit)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [s]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Server Selector modal (Auto Fallback, VixSrc, LookMovie)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [t]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Subtitle Language Selector modal (OpenSubtitles CDN)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [d]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Toggle Sub / Dub audio preference mode",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [f]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Toggle Favorite / Bookmark on highlighted title",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [a]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Toggle Auto-Play Next Episode (seamless marathon)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [S] or [,]    ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Open Settings & Preferences split-pane menu",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [q]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Quit KittyMov cleanly",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(""));

    // Section 2: Episode Guide
    lines.push(Line::from(vec![Span::styled(
        "◆ EPISODE GUIDE & MODALS",
        Style::default()
            .fg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled(
            "  [← / → / h / l] ",
            Style::default().fg(Theme::ACCENT_GOLD),
        ),
        Span::styled(
            "Cycle seasons forward / backward",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  [↑ / ↓ / j / k] ",
            Style::default().fg(Theme::ACCENT_GOLD),
        ),
        Span::styled(
            "Select episode in current season",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [1 - 3]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Direct numeric server choice in Server Selector",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [Esc]         ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Close modal and return to previous view",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(""));

    // Section 3: In-Player MPV Controls
    lines.push(Line::from(vec![Span::styled(
        "◆ MPV PLAYBACK CONTROLS",
        Style::default()
            .fg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled("  [Space]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("Play / Pause video", Style::default().fg(Theme::TEXT_BODY)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [← / →]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Seek 5 seconds backward / forward",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [j / Shift+j] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Cycle subtitle tracks forward / backward on screen",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [#]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Cycle audio language tracks",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [9 / 0]       ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Volume down / up (m to mute)",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [f]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Toggle fullscreen window",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [q]           ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            "Quit player and return to KittyMov",
            Style::default().fg(Theme::TEXT_BODY),
        ),
    ]));

    let p = Paragraph::new(lines);
    f.render_widget(p, chunks[0]);

    let footer_text = Line::from(vec![Span::styled(
        " Press [Esc] or [?] to close this help screen ",
        Style::default()
            .fg(Theme::TEXT_MUTED)
            .add_modifier(Modifier::ITALIC),
    )]);
    let footer_p = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER_MUTED)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[1]);
}
