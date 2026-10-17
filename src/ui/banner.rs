use super::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn get_banner_lines<'a>() -> Vec<Line<'a>> {
    vec![
        Line::from(vec![Span::styled(
            " ██╗  ██╗██╗████████╗████████╗██╗   ██╗███╗   ███╗ ██████╗ ██╗   ██╗",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            " ██║ ██╔╝██║╚══██╔══╝╚══██╔══╝╚██╗ ██╔╝████╗ ████║██╔═══██╗██║   ██║",
            Style::default().fg(Theme::ACCENT_CYAN),
        )]),
        Line::from(vec![Span::styled(
            " █████╔╝ ██║   ██║      ██║    ╚████╔╝ ██╔████╔██║██║   ██║██║   ██║",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            " ██╔═██╗ ██║   ██║      ██║     ╚██╔╝  ██║╚██╔╝██║██║   ██║╚██╗ ██╔╝",
            Style::default().fg(Theme::ACCENT_CYAN),
        )]),
        Line::from(vec![Span::styled(
            " ██║  ██╗██║   ██║      ██║      ██║   ██║ ╚═╝ ██║╚██████╔╝ ╚████╔╝ ",
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            " ╚═╝  ╚═╝╚═╝   ╚═╝      ╚═╝      ╚═╝   ╚═╝     ╚═╝ ╚═════╝   ╚═══╝  ",
            Style::default().fg(Theme::BORDER_MUTED),
        )]),
    ]
}
