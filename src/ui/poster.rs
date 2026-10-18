use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn calculate_poster_dimensions_for_panel(panel_w: u16, panel_h: u16) -> (u16, u16) {
    let max_w = panel_w.saturating_sub(4).max(12);

    let poster_frame_h = if panel_h < 25 {
        (panel_h / 2).max(6)
    } else {
        (panel_h.saturating_sub(15)).clamp(14, 28)
    };
    let poster_h = poster_frame_h.saturating_sub(2).max(6);

    // Standard movie/series poster ratio is 2:3. In character cells (1:2 aspect ratio),
    // target width is ~ 1.35 * height, bounded by available panel width.
    let target_w = ((poster_h as f32 * 1.35) as u16).clamp(16, max_w);
    (target_w, poster_h)
}

pub fn calculate_poster_screen_rect(term_w: u16, term_h: u16) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::widgets::{Block, Borders};

    let outer_block = Block::default().borders(Borders::ALL);
    let inner_area = outer_block.inner(Rect::new(0, 0, term_w, term_h));

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(inner_area);

    let (target_w, target_h) =
        calculate_poster_dimensions_for_panel(columns[0].width, columns[0].height);
    let poster_frame_height = target_h + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(poster_frame_height),
            Constraint::Length(3),
            Constraint::Min(8),
        ])
        .split(columns[0]);

    let poster_block = Block::default().borders(Borders::ALL);
    let poster_inner = poster_block.inner(chunks[0]);

    let x = poster_inner.x + (poster_inner.width.saturating_sub(target_w)) / 2;
    let y = poster_inner.y + (poster_inner.height.saturating_sub(target_h)) / 2;

    Rect {
        x,
        y,
        width: target_w,
        height: target_h,
    }
}

pub fn calculate_resume_modal_poster_screen_rect(term_w: u16, term_h: u16) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};

    let area = Rect::new(0, 0, term_w, term_h);
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

    let body_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(chunks[3]);

    let (target_w, target_h) = calculate_poster_dimensions_for_panel(
        (term_w.saturating_sub(2) as f32 * 0.38) as u16,
        term_h.saturating_sub(2),
    );

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),        // "Current Selection" header
            Constraint::Length(target_h), // Poster artwork
            Constraint::Min(4),           // Progress & details
        ])
        .split(body_cols[1]);

    let poster_chunk = right_chunks[1];
    let x = poster_chunk.x + (poster_chunk.width.saturating_sub(target_w)) / 2;
    let y = poster_chunk.y;

    Rect {
        x,
        y,
        width: target_w,
        height: target_h,
    }
}

pub fn is_kitty_graphics_supported() -> bool {
    if std::env::var("KITTY_WINDOW_ID").is_ok() || std::env::var("KITTY_PID").is_ok() {
        return true;
    }
    if let Ok(term) = std::env::var("TERM") {
        let t = term.to_lowercase();
        if t.contains("kitty") || t.contains("ghostty") {
            return true;
        }
    }
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return true;
    }
    if let Ok(prog) = std::env::var("TERM_PROGRAM") {
        let p = prog.to_lowercase();
        if p.contains("wezterm") || p.contains("rio") {
            return true;
        }
    }
    if std::env::var("WEZTERM_PANE").is_ok() {
        return true;
    }
    if let Ok(ver) = std::env::var("KONSOLE_VERSION") {
        if let Ok(v) = ver.parse::<u64>() {
            if v >= 220000 {
                return true;
            }
        }
    }
    false
}

pub fn render_poster_to_kitty(path: &Path, width: u16, height: u16) -> Option<Vec<u8>> {
    let size_arg = format!("{}x{}", width, height);
    let output = Command::new("chafa")
        .arg("--probe=off")
        .arg("--polite=on")
        .arg("-f")
        .arg("kitty")
        .arg(format!("--size={}", size_arg))
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    Some(output.stdout)
}

pub fn clear_kitty_graphics() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b_Ga=d,d=a\x1b\\");
    let _ = stdout.flush();
}

pub fn draw_kitty_graphics(raw_escape: &[u8], col: u16, row: u16) {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "\x1b[{};{}H", row + 1, col + 1);
    let _ = stdout.write_all(raw_escape);
    let _ = stdout.write_all(b"\x1b[?25l");
    let _ = stdout.flush();
}

pub fn render_poster_to_text(
    path: &Path,
    width: u16,
    height: u16,
    mode: &str,
) -> Option<Text<'static>> {
    if mode == "off" {
        return None;
    }

    let symbols = match mode {
        "half-block" => "vhalf",
        _ => "block+border+space+vhalf+quad+sextant",
    };

    let size_arg = format!("{}x{}", width, height);
    let output = Command::new("chafa")
        .arg("--probe=off")
        .arg("--format=symbols")
        .arg(format!("--symbols={}", symbols))
        .arg("--color-space=din99d")
        .arg("--colors=full")
        .arg("--work=7")
        .arg(format!("--size={}", size_arg))
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw_str = String::from_utf8_lossy(&output.stdout);
    Some(parse_ansi_to_text(&raw_str))
}

fn parse_ansi_to_text(input: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut current_fg = None;
    let mut current_bg = None;

    for raw_line in input.lines() {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = raw_line.chars().collect();
        let mut idx = 0;

        while idx < chars.len() {
            if chars[idx] == '\x1b' && idx + 1 < chars.len() && chars[idx + 1] == '[' {
                // Flush accumulated text with current style
                if !current_text.is_empty() {
                    let mut style = Style::default();
                    if let Some(fg) = current_fg {
                        style = style.fg(fg);
                    }
                    if let Some(bg) = current_bg {
                        style = style.bg(bg);
                    }
                    spans.push(Span::styled(current_text.clone(), style));
                    current_text.clear();
                }

                idx += 2; // skip '\x1b['
                let start_seq = idx;
                while idx < chars.len() && !chars[idx].is_ascii_alphabetic() {
                    idx += 1;
                }
                if idx < chars.len() {
                    let cmd = chars[idx];
                    let seq: String = chars[start_seq..idx].iter().collect();
                    idx += 1; // consume command char

                    if cmd == 'm' {
                        // SGR sequence
                        let tokens: Vec<&str> = seq.split(';').collect();
                        let mut t_idx = 0;
                        while t_idx < tokens.len() {
                            match tokens[t_idx] {
                                "" | "0" => {
                                    current_fg = None;
                                    current_bg = None;
                                    t_idx += 1;
                                }
                                "39" => {
                                    current_fg = None;
                                    t_idx += 1;
                                }
                                "49" => {
                                    current_bg = None;
                                    t_idx += 1;
                                }
                                "38" => {
                                    // 38;2;r;g;b or 38;5;n
                                    if t_idx + 4 < tokens.len() && tokens[t_idx + 1] == "2" {
                                        if let (Ok(r), Ok(g), Ok(b)) = (
                                            tokens[t_idx + 2].parse::<u8>(),
                                            tokens[t_idx + 3].parse::<u8>(),
                                            tokens[t_idx + 4].parse::<u8>(),
                                        ) {
                                            current_fg = Some(Color::Rgb(r, g, b));
                                        }
                                        t_idx += 5;
                                    } else if t_idx + 2 < tokens.len() && tokens[t_idx + 1] == "5" {
                                        if let Ok(n) = tokens[t_idx + 2].parse::<u8>() {
                                            current_fg = Some(Color::Indexed(n));
                                        }
                                        t_idx += 3;
                                    } else {
                                        t_idx += 1;
                                    }
                                }
                                "48" => {
                                    // 48;2;r;g;b or 48;5;n
                                    if t_idx + 4 < tokens.len() && tokens[t_idx + 1] == "2" {
                                        if let (Ok(r), Ok(g), Ok(b)) = (
                                            tokens[t_idx + 2].parse::<u8>(),
                                            tokens[t_idx + 3].parse::<u8>(),
                                            tokens[t_idx + 4].parse::<u8>(),
                                        ) {
                                            current_bg = Some(Color::Rgb(r, g, b));
                                        }
                                        t_idx += 5;
                                    } else if t_idx + 2 < tokens.len() && tokens[t_idx + 1] == "5" {
                                        if let Ok(n) = tokens[t_idx + 2].parse::<u8>() {
                                            current_bg = Some(Color::Indexed(n));
                                        }
                                        t_idx += 3;
                                    } else {
                                        t_idx += 1;
                                    }
                                }
                                _ => {
                                    t_idx += 1;
                                }
                            }
                        }
                    }
                }
            } else {
                current_text.push(chars[idx]);
                idx += 1;
            }
        }

        if !current_text.is_empty() {
            let mut style = Style::default();
            if let Some(fg) = current_fg {
                style = style.fg(fg);
            }
            if let Some(bg) = current_bg {
                style = style.bg(bg);
            }
            spans.push(Span::styled(current_text, style));
        }

        if !spans.is_empty() {
            lines.push(Line::from(spans));
        }
    }
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_parsing_colors() {
        let sample = "\x1b[?25l\x1b[0m\x1b[38;2;5;13;26;48;2;5;20;39m▄\x1b[0m\n\x1b[?25h";
        let text = parse_ansi_to_text(sample);
        assert_eq!(text.lines.len(), 1);
        let spans = &text.lines[0].spans;
        let block_span = spans
            .iter()
            .find(|s| s.content == "▄")
            .expect("Span with block character should exist");
        assert_eq!(block_span.style.fg, Some(Color::Rgb(5, 13, 26)));
        assert_eq!(block_span.style.bg, Some(Color::Rgb(5, 20, 39)));
    }

    #[test]
    fn test_ansi_parsing_sextant_characters() {
        let sample = "\x1b[38;2;255;100;50;48;2;10;20;30m🬆🬀\x1b[0m";
        let text = parse_ansi_to_text(sample);
        assert_eq!(text.lines.len(), 1);
        assert_eq!(text.lines[0].spans[0].content, "🬆🬀");
        assert_eq!(
            text.lines[0].spans[0].style.fg,
            Some(Color::Rgb(255, 100, 50))
        );
        assert_eq!(
            text.lines[0].spans[0].style.bg,
            Some(Color::Rgb(10, 20, 30))
        );
    }

    #[test]
    fn test_calculate_poster_dimensions() {
        // Small terminal panel (width 29, height 22)
        let (w1, h1) = calculate_poster_dimensions_for_panel(29, 22);
        assert!((14..=25).contains(&w1));
        assert!((6..=12).contains(&h1));

        // Standard terminal panel (width 44, height 38)
        let (w2, h2) = calculate_poster_dimensions_for_panel(44, 38);
        assert_eq!(h2, 21);
        assert_eq!(w2, 28);

        // Large terminal panel (width 60, height 48)
        let (w3, h3) = calculate_poster_dimensions_for_panel(60, 48);
        assert_eq!(h3, 26);
        assert_eq!(w3, 35);
    }

    #[test]
    fn test_calculate_poster_screen_rect() {
        let rect = calculate_poster_screen_rect(120, 40);
        assert!(rect.width >= 16);
        assert!(rect.height >= 10);
        assert!(rect.x >= 1);
        assert!(rect.y >= 1);
    }

    #[test]
    fn test_calculate_resume_modal_poster_screen_rect() {
        let rect = calculate_resume_modal_poster_screen_rect(120, 40);
        assert!(rect.width >= 16);
        assert!(rect.height >= 10);
        // The right column starts at 66% of terminal (120 * 0.66 ~ 79)
        assert!(rect.x > 70);
        assert!(rect.y >= 10);
    }

    #[test]
    fn test_is_kitty_graphics_supported() {
        // If KITTY_WINDOW_ID is set (like in this session), returns true
        let supported = is_kitty_graphics_supported();
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            assert!(supported);
        }
    }
}
