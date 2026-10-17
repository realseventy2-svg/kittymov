use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::config::Config;

pub fn render_settings_modal(
    f: &mut Frame,
    area: Rect,
    cfg: &Config,
    selected_idx: usize,
) {
    let popup_area = centered_rect(area, 72, 65);
    f.render_widget(Clear, popup_area);

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(" KittyMov • Configuration & Settings ", Style::default().fg(Theme::ACCENT_CYAN).add_modifier(Modifier::BOLD)));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Subtitle / description
            Constraint::Min(8),    // Settings fields
            Constraint::Length(3), // Hotkey guide footer
        ])
        .margin(1)
        .split(popup_area);

    f.render_widget(modal_block, popup_area);

    // 1. Header subtitle
    let desc = Line::from(vec![
        Span::styled("Configure playback defaults, audio preferences, and video engines.", Style::default().fg(Theme::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(desc), inner_chunks[0]);

    // 2. Settings Rows
    let audio_lang_desc = match cfg.audio_language.to_lowercase().as_str() {
        "eng" => "eng (English)".to_string(),
        "jpn" => "jpn (Japanese)".to_string(),
        "spa" => "spa (Spanish)".to_string(),
        "fra" => "fra (French)".to_string(),
        "deu" => "deu (German)".to_string(),
        "ita" => "ita (Italian)".to_string(),
        "por" => "por (Portuguese)".to_string(),
        "rus" => "rus (Russian)".to_string(),
        "kor" => "kor (Korean)".to_string(),
        "zho" => "zho (Chinese)".to_string(),
        other => other.to_string(),
    };

    let sub_lang_desc = match cfg.sub_language.to_lowercase().as_str() {
        "eng" => "eng (English)".to_string(),
        "spa" => "spa (Spanish)".to_string(),
        "fra" => "fra (French)".to_string(),
        "deu" => "deu (German)".to_string(),
        "ita" => "ita (Italian)".to_string(),
        "jpn" => "jpn (Japanese)".to_string(),
        "por" => "por (Portuguese)".to_string(),
        "rus" => "rus (Russian)".to_string(),
        "kor" => "kor (Korean)".to_string(),
        "zho" => "zho (Chinese)".to_string(),
        other => other.to_string(),
    };

    let rows_data: Vec<(&str, String)> = vec![
        (
            "Audio Mode",
            match cfg.audio_mode.to_lowercase().as_str() {
                "sub" => "Sub (Original Audio + Subtitles)".to_string(),
                _ => "Dub (English / Native Audio Track)".to_string(),
            },
        ),
        ("Default Audio Language", audio_lang_desc),
        ("Default Subtitle Language", sub_lang_desc),
        (
            "Auto-Play Next Episode",
            if cfg.auto_play_next { "ON (Seamless background prefetch)".to_string() } else { "OFF".to_string() },
        ),
        (
            "Preferred Provider",
            match cfg.preferred_provider.to_lowercase().as_str() {
                "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)".to_string(),
                "lookmovie" => "LookMovie First (Fallback CDN)".to_string(),
                _ => "Auto Fallback (VixSrc → LookMovie)".to_string(),
            },
        ),
        (
            "Hardware Decoding (hwdec)",
            match cfg.hwdec.as_str() {
                "auto" => "auto (MPV auto detection)".to_string(),
                "auto-safe" => "auto-safe (Safe hardware decoder)".to_string(),
                "vaapi" => "vaapi (Intel / AMD Linux GPU)".to_string(),
                "nvdec" => "nvdec (NVIDIA GPU)".to_string(),
                "videotoolbox" => "videotoolbox (Apple Silicon / macOS)".to_string(),
                "no" => "no (Software CPU decoding)".to_string(),
                other => other.to_string(),
            },
        ),
    ];

    let mut fields_lines = Vec::new();
    for (i, (label, val_str)) in rows_data.iter().enumerate() {
        let is_sel = i == selected_idx;

        let selector = format!("◄  {}  ►", val_str);
        if is_sel {
            fields_lines.push(Line::from(vec![
                Span::styled(" ◆ ", Style::default().fg(Theme::ACCENT_PINK).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<30}", label), Style::default().fg(Theme::TEXT_TITLE).bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD)),
                Span::styled(selector, Style::default().fg(Theme::ACCENT_CYAN).bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            fields_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{:<30}", label), Style::default().fg(Theme::TEXT_BODY)),
                Span::styled(selector, Style::default().fg(Theme::TEXT_MUTED)),
            ]));
        }
        fields_lines.push(Line::from(""));
    }

    let fields_p = Paragraph::new(fields_lines);
    f.render_widget(fields_p, inner_chunks[1]);

    // 3. Hotkey guide footer
    let hints = Line::from(vec![
        Span::styled(" [↑/↓ / k/j] Navigate   [←/→ / Space] Cycle Option   [Esc] Save & Close ", Style::default().fg(Theme::TEXT_MUTED)),
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
