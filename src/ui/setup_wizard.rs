use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::{SetupStep, SetupWizardState};
use crate::config::Config;

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

pub fn render_setup_wizard(f: &mut Frame, area: Rect, state: &SetupWizardState, cfg: &Config) {
    // 1. Full-screen background clear
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::BG))
        .title(" 🐱 KittyMov • Initial Setup Wizard ")
        .title_style(
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        );

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // 2. Main vertical layout: Top stepper, Content Card, Bottom Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Stepper Header
            Constraint::Min(12),   // Step Content Body
            Constraint::Length(3), // Navigation Footer
        ])
        .split(inner);

    // Render Stepper Header
    render_stepper_header(f, chunks[0], state.current_step);

    // Render Step Content
    render_step_content(f, chunks[1], state, cfg);

    // Render Bottom Footer
    render_wizard_footer(f, chunks[2], state.current_step);
}

fn render_stepper_header(f: &mut Frame, area: Rect, current: SetupStep) {
    let mut spans = Vec::new();
    let current_idx = current as usize;

    for (idx, step) in SetupStep::ALL.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(
                " ─ ",
                Style::default().fg(Theme::BORDER_MUTED),
            ));
        }

        let is_current = idx == current_idx;
        let is_completed = idx < current_idx;

        let num_str = format!("{}. ", idx + 1);
        let title_str = step.title();

        if is_current {
            spans.push(Span::styled(
                format!(" [ {}{} ] ", num_str, title_str),
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if is_completed {
            spans.push(Span::styled(
                format!(" ✓ {} ", title_str),
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {}{} ", num_str, title_str),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }
    }

    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(spans).alignment(Alignment::Center),
    ]);
    f.render_widget(p, area);
}

fn render_step_content(f: &mut Frame, area: Rect, state: &SetupWizardState, cfg: &Config) {
    let content_area = centered_rect(area, 86, 92);

    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE))
        .title(format!(
            " Step {}: {} ",
            (state.current_step as usize) + 1,
            state.current_step.description()
        ))
        .title_style(
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        );

    let card_inner = card_block.inner(content_area);
    f.render_widget(card_block, content_area);

    match state.current_step {
        SetupStep::Welcome => render_welcome_step(f, card_inner, state.selected_item_idx),
        SetupStep::Playback => render_playback_step(f, card_inner, state.selected_item_idx, cfg),
        SetupStep::Subtitles => render_subtitles_step(f, card_inner, state.selected_item_idx, cfg),
        SetupStep::Providers => render_providers_step(f, card_inner, state.selected_item_idx, cfg),
        SetupStep::Appearance => {
            render_appearance_step(f, card_inner, state.selected_item_idx, cfg)
        }
        SetupStep::Complete => render_complete_step(f, card_inner, state.selected_item_idx, cfg),
    }
}

fn render_welcome_step(f: &mut Frame, area: Rect, selected_idx: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Introduction text
            Constraint::Length(7), // Feature highlights
            Constraint::Length(5), // Action options
        ])
        .margin(1)
        .split(area);

    let welcome_text = vec![
        Line::from(vec![
            Span::styled("✨ Welcome to ", Style::default().fg(Theme::TEXT_TITLE)),
            Span::styled(
                "KittyMov",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" — Terminal Cinema Streaming Suite", Style::default().fg(Theme::TEXT_TITLE)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "KittyMov is designed for fast, seamless movies and series playback in modern terminals.",
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "This quick 1-minute setup wizard guides you through essential preferences so you get the",
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "best performance, audio dubbing, subtitle sync, and artwork rendering right away.",
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(welcome_text), chunks[0]);

    let features_text = vec![
        Line::from(vec![
            Span::styled(
                "  ⚡ Ultra-Fast Streaming: ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "VixSrc & LookMovie high-speed CDNs with automatic failover",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  🎧 Smart Audio & Subs:  ",
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Native pre-synced .vtt tracks with automatic Wyzie Subs fallback",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  📺 TV Episode Tracking: ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Canonical broadcast sequence, resume points, and season progress",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  🎨 Cyberpunk Aesthetic:  ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Multiple themes, glowing borders, and ANSI / Kitty / Sixel posters",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(features_text), chunks[1]);

    let btn0_style = if selected_idx == 0 {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };

    let btn1_style = if selected_idx == 1 {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_MUTED)
            .bg(Theme::SURFACE_ALT)
    };

    let action_lines = vec![
        Line::from(vec![
            Span::styled(
                if selected_idx == 0 { " › " } else { "   " },
                Style::default().fg(Theme::ACCENT_CYAN),
            ),
            Span::styled(" [ 1. Begin Guided Setup (Recommended) ] ", btn0_style),
            Span::styled(
                "  Customize acceleration, audio, and subtitle preferences",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                if selected_idx == 1 { " › " } else { "   " },
                Style::default().fg(Theme::ACCENT_GOLD),
            ),
            Span::styled(" [ 2. Quick Start with Safe Defaults ] ", btn1_style),
            Span::styled(
                "     Launch KittyMov immediately with optimal pre-configurations",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(action_lines), chunks[2]);
}

fn render_playback_step(f: &mut Frame, area: Rect, selected_idx: usize, cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Options table
            Constraint::Length(5), // Explanation box
            Constraint::Length(3), // Continue button
        ])
        .margin(1)
        .split(area);

    let rows = [
        (
            "1. Hardware Video Acceleration",
            match cfg.hwdec.as_str() {
                "auto" => "auto (Auto-negotiate GPU decoding • Recommended)",
                "vaapi" => "vaapi (Intel / AMD Linux GPU)",
                "nvdec" => "nvdec (NVIDIA Linux GPU)",
                "vdpau" => "vdpau (Legacy Video Decode)",
                _ => "no (Software CPU decoding)",
            },
        ),
        (
            "2. Resume Playback on Startup",
            if cfg.resume_on_startup {
                "ON (Auto-resume last watched title)"
            } else {
                "OFF (Start at catalog view)"
            },
        ),
        (
            "3. Continuous Episode Autoplay",
            if cfg.auto_play_next {
                "ON (Play next episode automatically)"
            } else {
                "OFF (Prompt when episode ends)"
            },
        ),
    ];

    let mut row_lines = Vec::new();
    for (idx, (label, val)) in rows.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { " › " } else { "   " };
        let prefix_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let label_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let val_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_BODY)
        };

        row_lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(format!("{:<32} ", label), label_style),
            Span::styled(format!("◂ {} ▸", val), val_style),
        ]));
        row_lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(row_lines), chunks[0]);

    // Hint text based on selected item
    let hint = match selected_idx {
        0 => "Hardware Acceleration offloads H.264/HEVC decoding to your GPU via VA-API or NVDEC, reducing CPU usage and fan noise during 1080p playback.",
        1 => "When enabled, launching KittyMov without search flags immediately resumes where you left off in your most recently watched movie or series.",
        2 => "When an episode ends in mpv, KittyMov seamlessly queues and streams the next episode without requiring manual selection.",
        _ => "Press [Enter] or [→] to proceed to the Audio & Subtitles configuration step.",
    };

    let hint_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Tip ");
    let hint_p = Paragraph::new(hint)
        .block(hint_box)
        .wrap(Wrap { trim: true });
    f.render_widget(hint_p, chunks[1]);

    let is_btn_sel = selected_idx == 3;
    let btn_style = if is_btn_sel {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };
    let btn_line = Line::from(vec![
        Span::styled(
            if is_btn_sel { " › " } else { "   " },
            Style::default().fg(Theme::ACCENT_CYAN),
        ),
        Span::styled(" [ Continue to Audio & Subtitles → ] ", btn_style),
    ]);
    f.render_widget(Paragraph::new(btn_line), chunks[2]);
}

fn render_subtitles_step(f: &mut Frame, area: Rect, selected_idx: usize, cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Options table
            Constraint::Length(5),  // Explanation box
            Constraint::Length(3),  // Continue button
        ])
        .margin(1)
        .split(area);

    let rows = [
        (
            "1. Preferred Voice Language",
            cfg.audio_language.to_uppercase(),
        ),
        (
            "2. Audio Mode Preference",
            if cfg.audio_mode.to_lowercase() == "sub" {
                "Subtitled (Original Audio + Subtitles • Recommended)"
            } else {
                "Dubbed (Local Voiceover Dub)"
            }
            .to_string(),
        ),
        (
            "3. Preferred Subtitle Language",
            cfg.sub_language.to_uppercase(),
        ),
        (
            "4. Cloud Subtitle Augmentation",
            if cfg.subtitle_augmentation {
                "Enabled (Auto-fetch Wyzie Subs if missing • Recommended)"
            } else {
                "Disabled (Provider native only)"
            }
            .to_string(),
        ),
    ];

    let mut row_lines = Vec::new();
    for (idx, (label, val)) in rows.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { " › " } else { "   " };
        let prefix_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let label_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let val_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_BODY)
        };

        row_lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(format!("{:<32} ", label), label_style),
            Span::styled(format!("◂ {} ▸", val), val_style),
        ]));
        row_lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(row_lines), chunks[0]);

    let hint = match selected_idx {
        0 => "Default voice language requested when multi-audio tracks (e.g. English, Japanese, French) are available in the stream.",
        1 => "Subtitled mode plays original audio with synchronized text subtitles. Dubbed mode selects localized audio dub tracks.",
        2 => "Default subtitle language automatically loaded and rendered on video start.",
        3 => "Cloud Augmentation automatically fetches high-accuracy ad-free subtitles from Wyzie Subs when streams lack your language.",
        _ => "Press [Enter] or [→] to proceed to the Streaming Providers configuration step.",
    };

    let hint_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Subtitle Intelligence ");
    let hint_p = Paragraph::new(hint)
        .block(hint_box)
        .wrap(Wrap { trim: true });
    f.render_widget(hint_p, chunks[1]);

    let is_btn_sel = selected_idx == 4;
    let btn_style = if is_btn_sel {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };
    let btn_line = Line::from(vec![
        Span::styled(
            if is_btn_sel { " › " } else { "   " },
            Style::default().fg(Theme::ACCENT_CYAN),
        ),
        Span::styled(" [ Continue to Streaming Providers → ] ", btn_style),
    ]);
    f.render_widget(Paragraph::new(btn_line), chunks[2]);
}

fn render_providers_step(f: &mut Frame, area: Rect, selected_idx: usize, cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Options table
            Constraint::Length(5), // Explanation box
            Constraint::Length(3), // Continue button
        ])
        .margin(1)
        .split(area);

    let rows = [
        (
            "1. Primary Streaming Engine",
            match cfg.preferred_provider.as_str() {
                "vidlink" => "VidLink (HD 1080p Direct + Subtitles • Recommended)",
                "vixsrc" => "VixSrc (High-speed adaptive CDN)",
                "hianime" => "HiAnime (Dedicated anime CDN)",
                "lookmovie" => "LookMovie (Reliable backup CDN)",
                _ => "Auto Fallback Engine (VidLink Primary → VixSrc → HiAnime → LookMovie)",
            },
        ),
        (
            "2. TV Episode Order Mode",
            match cfg.episode_order.as_str() {
                "streaming" => "Alternate / Production Order (Digital Volumes)",
                _ => "Broadcast Order (Canonical TV Airings • Recommended)",
            },
        ),
        (
            "3. Subtitle Source Preference",
            match cfg.subtitle_source.as_str() {
                "provider" => "Provider Native Only",
                "wyzie" | "opensubtitles" => "Wyzie Subs Cloud Only",
                _ => "Auto (Provider Native pre-synced, with Cloud fallback)",
            },
        ),
    ];

    let mut row_lines = Vec::new();
    for (idx, (label, val)) in rows.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { " › " } else { "   " };
        let prefix_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let label_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let val_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_BODY)
        };

        row_lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(format!("{:<32} ", label), label_style),
            Span::styled(format!("◂ {} ▸", val), val_style),
        ]));
        row_lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(row_lines), chunks[0]);

    let hint = match selected_idx {
        0 => "Auto Fallback queries VixSrc first for ultra-fast startup, and automatically fails over to LookMovie if an endpoint is offline.",
        1 => "Broadcast Order matches standard TV air dates across all providers, ensuring consistent season and episode alignments.",
        2 => "Auto subtitle mode prioritizes pre-synced native .vtt tracks from the stream encode, seamlessly augmenting with Wyzie Subs when absent.",
        _ => "Press [Enter] or [→] to proceed to the Appearance & Theme configuration step.",
    };

    let hint_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Reliability & Source Priority ");
    let hint_p = Paragraph::new(hint)
        .block(hint_box)
        .wrap(Wrap { trim: true });
    f.render_widget(hint_p, chunks[1]);

    let is_btn_sel = selected_idx == 3;
    let btn_style = if is_btn_sel {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };
    let btn_line = Line::from(vec![
        Span::styled(
            if is_btn_sel { " › " } else { "   " },
            Style::default().fg(Theme::ACCENT_CYAN),
        ),
        Span::styled(" [ Continue to Appearance & Themes → ] ", btn_style),
    ]);
    f.render_widget(Paragraph::new(btn_line), chunks[2]);
}

fn render_appearance_step(f: &mut Frame, area: Rect, selected_idx: usize, cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Options table
            Constraint::Length(5), // Explanation box
            Constraint::Length(3), // Continue button
        ])
        .margin(1)
        .split(area);

    let rows = [
        (
            "1. Interface Color Theme",
            match cfg.theme_name.as_str() {
                "matrix" => "Matrix (Phosphor Hacker Green)",
                "synthwave" => "Synthwave (Neon Purple & Pink)",
                "dracula" => "Dracula (Vampire Dark & Purple)",
                "nord" => "Nord (Arctic Frost Blue)",
                _ => "Cyber (Cyber Cyan & Amber Gold • Default)",
            },
        ),
        (
            "2. Poster Artwork Render Mode",
            match cfg.poster_render_mode.as_str() {
                "off" => "Off (Clean Text-Only Minimalist)",
                "kitty" => "Kitty Protocol (Pixel-Perfect Native Graphics)",
                "half-block" => "Half-Block (ANSI Unicode ▀▄ • Universally Supported)",
                "high-res" => "High-Resolution TrueColor (Sextant & Quad • Cinematic)",
                _ => "Auto (Native Kitty Protocol if Supported, High-Res Fallback)",
            },
        ),
        (
            "3. Default Catalog Landing Tab",
            match cfg.default_tab.as_str() {
                "series" => "Trending Series",
                "bookmarks" => "Bookmarks",
                "history" => "Watch History",
                _ => "Trending Movies",
            },
        ),
    ];

    let mut row_lines = Vec::new();
    for (idx, (label, val)) in rows.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { " › " } else { "   " };
        let prefix_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let label_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED)
        };
        let val_style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_BODY)
        };

        row_lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(format!("{:<32} ", label), label_style),
            Span::styled(format!("◂ {} ▸", val), val_style),
        ]));
        row_lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(row_lines), chunks[0]);

    let hint = match selected_idx {
        0 => "Custom color schemes tint borders, badges, focus cursors, and highlight surfaces. Changes apply immediately across the interface.",
        1 => "Half-Block works on 100% of Linux terminal emulators without graphical plugins. Kitty mode delivers crisp 300dpi covers if supported.",
        2 => "Select whether KittyMov presents trending theatrical movies or television series upon launch.",
        _ => "Press [Enter] or [→] to review your configuration summary and finish setup.",
    };

    let hint_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Visual Experience ");
    let hint_p = Paragraph::new(hint)
        .block(hint_box)
        .wrap(Wrap { trim: true });
    f.render_widget(hint_p, chunks[1]);

    let is_btn_sel = selected_idx == 3;
    let btn_style = if is_btn_sel {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };
    let btn_line = Line::from(vec![
        Span::styled(
            if is_btn_sel { " › " } else { "   " },
            Style::default().fg(Theme::ACCENT_CYAN),
        ),
        Span::styled(" [ Review Summary & Finish Setup → ] ", btn_style),
    ]);
    f.render_widget(Paragraph::new(btn_line), chunks[2]);
}

fn render_complete_step(f: &mut Frame, area: Rect, selected_idx: usize, cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Configuration Summary Card
            Constraint::Length(6), // Essential Shortcut Cheatsheet
            Constraint::Length(4), // Finish Action Buttons
        ])
        .margin(1)
        .split(area);

    let summary_lines = vec![
        Line::from(vec![
            Span::styled(
                "✓ Video Engine:     ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("mpv (Hardware Acceleration: {})", cfg.hwdec),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "✓ Audio & Dubs:     ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} [Mode: {}]",
                    cfg.audio_language.to_uppercase(),
                    cfg.audio_mode.to_uppercase()
                ),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "✓ Subtitles:        ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} [Cloud Augmentation: {}]",
                    cfg.sub_language.to_uppercase(),
                    if cfg.subtitle_augmentation {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "✓ Streaming Engine: ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} (TV Order: {})",
                    cfg.preferred_provider.to_uppercase(),
                    cfg.episode_order.to_uppercase()
                ),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "✓ Terminal Theme:   ",
                Style::default()
                    .fg(Theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} (Posters: {})",
                    cfg.theme_name.to_uppercase(),
                    cfg.poster_render_mode.to_uppercase()
                ),
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
    ];

    let summary_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_GREEN))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Selected Configuration Summary ");
    f.render_widget(Paragraph::new(summary_lines).block(summary_box), chunks[0]);

    let cheatsheet_lines = vec![
        Line::from(vec![
            Span::styled(
                "  [Enter] / [s]  ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Open Tracks Inspector (Servers, Quality, Subs)   ",
                Style::default().fg(Theme::TEXT_BODY),
            ),
            Span::styled(
                "[p]     ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Instant Quick Play", Style::default().fg(Theme::TEXT_BODY)),
        ]),
        Line::from(vec![
            Span::styled(
                "  [/]            ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Search movies & TV series                        ",
                Style::default().fg(Theme::TEXT_BODY),
            ),
            Span::styled(
                "[Tab]   ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Switch Movies / Series tabs",
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  [,]            ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Open Settings (Change anything anytime)          ",
                Style::default().fg(Theme::TEXT_BODY),
            ),
            Span::styled(
                "[?]     ",
                Style::default()
                    .fg(Theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Full Keyboard Help Overlay",
                Style::default().fg(Theme::TEXT_BODY),
            ),
        ]),
    ];

    let cheat_box = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(" Essential Keyboard Cheatsheet ");
    f.render_widget(Paragraph::new(cheatsheet_lines).block(cheat_box), chunks[1]);

    let btn0_style = if selected_idx == 0 {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_GREEN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_TITLE)
            .bg(Theme::SURFACE_ALT)
    };

    let btn1_style = if selected_idx == 1 {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Theme::TEXT_MUTED)
            .bg(Theme::SURFACE_ALT)
    };

    let finish_lines = vec![
        Line::from(vec![
            Span::styled(
                if selected_idx == 0 { " › " } else { "   " },
                Style::default().fg(Theme::ACCENT_GREEN),
            ),
            Span::styled(" [ Complete Setup & Launch KittyMov ] ", btn0_style),
            Span::styled(
                "  Save configuration and enter terminal streaming catalog",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                if selected_idx == 1 { " › " } else { "   " },
                Style::default().fg(Theme::ACCENT_GOLD),
            ),
            Span::styled(" [ Back to Review Settings ] ", btn1_style),
            Span::styled(
                "          Return to step 1 to revise any choices",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(finish_lines), chunks[2]);
}

fn render_wizard_footer(f: &mut Frame, area: Rect, step: SetupStep) {
    let step_info = format!("Step {} / 6: {}", (step as usize) + 1, step.title());
    let left_spans = vec![
        Span::styled(
            " 🐱 KittyMov ",
            Style::default()
                .fg(Theme::ACCENT_PINK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" • ", Style::default().fg(Theme::BORDER_MUTED)),
        Span::styled(step_info, Style::default().fg(Theme::ACCENT_CYAN)),
    ];

    let right_spans = vec![
        Span::styled("[↑/↓] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("select item   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[←/→] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("change value   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("confirm / next   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[1-6] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("step jump   ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled("skip wizard ", Style::default().fg(Theme::TEXT_MUTED)),
    ];

    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Theme::BORDER_MUTED));

    let f_inner = footer_block.inner(area);
    f.render_widget(footer_block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(35), Constraint::Min(20)])
        .split(f_inner);

    f.render_widget(Paragraph::new(Line::from(left_spans)), cols[0]);
    f.render_widget(
        Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
        cols[1],
    );
}
