use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use crate::app::{
    format_bytes, lang_name, search_settings_matches, SettingsCategory, SettingsConfirmation,
    SettingsModalState, SettingsPane,
};
use crate::config::Config;

pub fn render_settings_modal(
    f: &mut Frame,
    area: Rect,
    cfg: &Config,
    state: &SettingsModalState,
    poster_cache_size: u64,
    history_count: usize,
) {
    // 1. Calculate responsive modal area (e.g. 82% width, 78% height, with minimums)
    let modal_width = (area.width * 84 / 100).max(72).min(area.width);
    let modal_height = (area.height * 80 / 100).max(20).min(area.height);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(modal_width)) / 2,
        y: area.y + (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, popup_area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::ACCENT_GOLD))
        .style(Style::default().bg(Theme::BG))
        .title(Span::styled(
            " KittyMov • Settings & Preferences ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ));

    f.render_widget(main_block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header / Breadcrumbs & Search
            Constraint::Min(12),   // Body (Split: Sidebar + Content)
            Constraint::Length(3), // Footer (Hotkeys & Feedback)
        ])
        .margin(1)
        .split(popup_area);

    // ==========================================
    // 1. Header (Breadcrumbs + Search status)
    // ==========================================
    let cur_cat = state.current_category();
    let is_search_active = !state.search_query.trim().is_empty();

    let mut breadcrumbs = vec![
        Span::styled("Settings ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled("› ", Style::default().fg(Theme::ACCENT_GOLD)),
        Span::styled(
            format!("{} {} ", cur_cat.icon(), cur_cat.title()),
            Style::default()
                .fg(Theme::TEXT_TITLE)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if is_search_active {
        breadcrumbs.push(Span::styled("› ", Style::default().fg(Theme::ACCENT_GOLD)));
        breadcrumbs.push(Span::styled(
            format!("Filter: \"{}\" ", state.search_query),
            Style::default()
                .fg(Theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let header_line = Line::from(breadcrumbs);
    let search_hint_line = if state.is_searching {
        Line::from(vec![
            Span::styled(
                " / Search: ",
                Style::default()
                    .fg(Theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &state.search_query,
                Style::default()
                    .fg(Theme::TEXT_TITLE)
                    .bg(Theme::SELECTION_BG),
            ),
            Span::styled("█", Style::default().fg(Theme::ACCENT_PINK)),
            Span::styled(
                "  [Enter] Accept  [Esc] Clear",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                cur_cat.description(),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
            Span::styled(
                "  (Press '/' to search)",
                Style::default().fg(Theme::BORDER_MUTED),
            ),
        ])
    };

    let header_p = Paragraph::new(vec![header_line, search_hint_line]);
    f.render_widget(header_p, inner_chunks[0]);

    // ==========================================
    // 2. Body: Two-Pane Split Layout
    // ==========================================
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Left: Categories Sidebar
            Constraint::Min(45),    // Right: Content Pane
        ])
        .split(inner_chunks[1]);

    // A. Left Pane: Categories Sidebar
    let sidebar_is_active = state.active_pane == SettingsPane::Categories && !state.is_searching;
    let sidebar_border_color = if sidebar_is_active {
        Theme::ACCENT_GOLD
    } else {
        Theme::BORDER_MUTED
    };

    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(sidebar_border_color))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            " Categories ",
            Style::default()
                .fg(if sidebar_is_active {
                    Theme::ACCENT_GOLD
                } else {
                    Theme::TEXT_MUTED
                })
                .add_modifier(Modifier::BOLD),
        ));

    let mut cat_lines = Vec::new();
    cat_lines.push(Line::from(""));
    for (i, cat) in SettingsCategory::ALL.iter().enumerate() {
        let is_current = i == state.category_idx;
        if is_current {
            if sidebar_is_active {
                cat_lines.push(Line::from(vec![
                    Span::styled(
                        " ◆ ",
                        Style::default()
                            .fg(Theme::ACCENT_GOLD)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} ", cat.icon()),
                        Style::default().fg(Theme::ACCENT_GOLD),
                    ),
                    Span::styled(
                        format!("{:<15}", cat.title()),
                        Style::default()
                            .fg(Theme::TEXT_TITLE)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                cat_lines.push(Line::from(vec![
                    Span::styled(
                        " ▸ ",
                        Style::default()
                            .fg(Theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} ", cat.icon()),
                        Style::default().fg(Theme::ACCENT_CYAN),
                    ),
                    Span::styled(
                        format!("{:<15}", cat.title()),
                        Style::default()
                            .fg(Theme::TEXT_TITLE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        } else {
            cat_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(
                    format!("{} ", cat.icon()),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(
                    format!("{:<15}", cat.title()),
                    Style::default().fg(Theme::TEXT_BODY),
                ),
            ]));
        }
        cat_lines.push(Line::from(""));
    }

    let sidebar_p = Paragraph::new(cat_lines).block(sidebar_block);
    f.render_widget(sidebar_p, body_chunks[0]);

    // B. Right Pane: Content Pane
    let content_is_active = state.active_pane == SettingsPane::Content || state.is_searching;
    let content_border_color = if content_is_active {
        Theme::ACCENT_GOLD
    } else {
        Theme::BORDER_MUTED
    };

    let content_title = if is_search_active {
        " Search Results "
    } else {
        cur_cat.title()
    };

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(content_border_color))
        .style(Style::default().bg(Theme::SURFACE))
        .title(Span::styled(
            format!(" {} ", content_title),
            Style::default()
                .fg(if content_is_active {
                    Theme::ACCENT_GOLD
                } else {
                    Theme::TEXT_MUTED
                })
                .add_modifier(Modifier::BOLD),
        ));

    let content_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // List of items
            Constraint::Length(5), // Inline description inspector
        ])
        .margin(1)
        .split(body_chunks[1]);

    f.render_widget(content_block, body_chunks[1]);

    // Items Rendering
    let mut item_lines = Vec::new();
    let mut active_description =
        "Use arrow keys to highlight an option, then press Space or Enter to toggle/cycle values.";

    if is_search_active {
        let matches = search_settings_matches(cfg, &state.search_query);
        if matches.is_empty() {
            item_lines.push(Line::from(""));
            item_lines.push(Line::from(vec![
                Span::styled(
                    "   No settings found matching \"",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(&state.search_query, Style::default().fg(Theme::ACCENT_PINK)),
                Span::styled("\".", Style::default().fg(Theme::TEXT_MUTED)),
            ]));
            item_lines.push(Line::from("   Press [Esc] to clear the search filter."));
            active_description = "No results found. Try searching for 'audio', 'subtitles', 'cache', 'theme', or 'provider'.";
        } else {
            for (idx, m) in matches.iter().enumerate() {
                let is_sel = idx == state.item_idx;
                if is_sel {
                    active_description = m.description;
                    item_lines.push(Line::from(vec![
                        Span::styled(
                            " ◆ ",
                            Style::default()
                                .fg(Theme::ACCENT_GOLD)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("[{}] ", m.category.title()),
                            Style::default()
                                .fg(Theme::ACCENT_CYAN)
                                .bg(Theme::SELECTION_BG),
                        ),
                        Span::styled(
                            format!("{:<26} ", m.label),
                            Style::default()
                                .fg(Theme::TEXT_TITLE)
                                .bg(Theme::SELECTION_BG)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(": {}", m.value_str),
                            Style::default()
                                .fg(Theme::ACCENT_GOLD)
                                .bg(Theme::SELECTION_BG),
                        ),
                    ]));
                } else {
                    item_lines.push(Line::from(vec![
                        Span::styled("   ", Style::default()),
                        Span::styled(
                            format!("[{}] ", m.category.title()),
                            Style::default().fg(Theme::TEXT_MUTED),
                        ),
                        Span::styled(
                            format!("{:<26} ", m.label),
                            Style::default().fg(Theme::TEXT_BODY),
                        ),
                        Span::styled(
                            format!(": {}", m.value_str),
                            Style::default().fg(Theme::TEXT_MUTED),
                        ),
                    ]));
                }
            }
        }
    } else {
        // Normal Category View
        let rows_data = get_category_rows(cur_cat, cfg, poster_cache_size, history_count);
        for (i, (label, val_widget, desc)) in rows_data.iter().enumerate() {
            let is_sel = i == state.item_idx;
            if is_sel {
                active_description = desc;
                item_lines.push(Line::from(vec![
                    Span::styled(
                        if content_is_active { " ◆ " } else { "   " },
                        Style::default()
                            .fg(Theme::ACCENT_GOLD)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<30}", label),
                        Style::default()
                            .fg(Theme::TEXT_TITLE)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    val_widget.styled(true),
                ]));
            } else {
                item_lines.push(Line::from(vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(
                        format!("{:<30}", label),
                        Style::default().fg(Theme::TEXT_BODY),
                    ),
                    val_widget.styled(false),
                ]));
            }
            item_lines.push(Line::from(""));
        }
    }

    let items_p = Paragraph::new(item_lines);
    f.render_widget(items_p, content_inner[0]);

    // Inline Description Inspector Box
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_MUTED))
        .style(Style::default().bg(Theme::SURFACE_ALT))
        .title(Span::styled(
            " ℹ ABOUT THIS SETTING ",
            Style::default()
                .fg(Theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ));

    let desc_p = Paragraph::new(vec![Line::from(Span::styled(
        active_description,
        Style::default().fg(Theme::TEXT_TITLE),
    ))])
    .block(desc_block);
    f.render_widget(desc_p, content_inner[1]);

    // ==========================================
    // 3. Footer: Hotkey Guide & Feedback Message
    // ==========================================
    let mut footer_spans = Vec::new();
    if let Some(ref msg) = state.feedback_message {
        footer_spans.push(Span::styled(
            format!(" ✔ {}   ", msg),
            Style::default()
                .fg(Theme::ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if state.is_searching {
        footer_spans.push(Span::styled(
            "[Typing] Filter   [↑/↓] Select Result   [Enter] Jump To Setting   [Esc] Clear / Exit",
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    } else {
        footer_spans.push(Span::styled(
            "[Tab] Switch Pane   [↑/↓] Navigate   [←/→/Space] Change   [r] Reset Category   [/] Search   [Esc] Save & Close",
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    }

    let footer_p = Paragraph::new(Line::from(footer_spans))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER_MUTED)),
        )
        .alignment(Alignment::Center);
    f.render_widget(footer_p, inner_chunks[2]);

    // ==========================================
    // 4. Confirmation Dialog Overlay (if active)
    // ==========================================
    if let Some(ref conf) = state.confirmation {
        let (prompt_title, prompt_msg) = match conf {
            SettingsConfirmation::ClearPosterCache => (
                " ⚠ CLEAR POSTER CACHE ",
                format!("Are you sure you want to delete all cached posters ({})?\nThis will remove downloaded artwork and free up disk space.", format_bytes(poster_cache_size)),
            ),
            SettingsConfirmation::ClearWatchHistory => (
                " ⚠ CLEAR WATCH HISTORY ",
                format!("Are you sure you want to clear your watch history ({} items)?\nAll playback timestamps and progress records will be permanently erased.", history_count),
            ),
            SettingsConfirmation::ExportDiagnostics => (
                " ℹ EXPORT DIAGNOSTIC REPORT ",
                "Export a privacy-redacted diagnostic summary to ~/.local/state/kittymov/diagnostic_report.json?\nNo tokens or private passwords will be included.".to_string(),
            ),
            SettingsConfirmation::ResetDefaults => (
                " ⚠ RESET ALL FACTORY DEFAULTS ",
                "Are you sure you want to restore all settings to default?\nYour customized preferences will be reset to their factory defaults.".to_string(),
            ),
        };

        let conf_width = 56.min(popup_area.width.saturating_sub(4));
        let conf_height = 8.min(popup_area.height.saturating_sub(4));
        let conf_area = Rect {
            x: popup_area.x + (popup_area.width.saturating_sub(conf_width)) / 2,
            y: popup_area.y + (popup_area.height.saturating_sub(conf_height)) / 2,
            width: conf_width,
            height: conf_height,
        };

        f.render_widget(Clear, conf_area);

        let conf_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::ACCENT_PINK))
            .style(Style::default().bg(Theme::SURFACE_ALT))
            .title(Span::styled(
                prompt_title,
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ));

        let conf_inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)])
            .margin(1)
            .split(conf_area);

        f.render_widget(conf_block, conf_area);

        let msg_lines: Vec<Line> = prompt_msg
            .lines()
            .map(|l| Line::from(Span::styled(l, Style::default().fg(Theme::TEXT_TITLE))))
            .collect();
        let prompt_p = Paragraph::new(msg_lines).alignment(Alignment::Center);
        f.render_widget(prompt_p, conf_inner[0]);

        let action_line = Line::from(vec![
            Span::styled(
                " [ Y ] Confirm Action ",
                Style::default()
                    .fg(Theme::ACCENT_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("      ", Style::default()),
            Span::styled(
                " [ N / Esc ] Cancel ",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]);
        let action_p = Paragraph::new(action_line).alignment(Alignment::Center);
        f.render_widget(action_p, conf_inner[1]);
    }
}

enum SettingWidget {
    Toggle(bool),
    Select(String),
    Action(String),
    Info(String),
}

impl SettingWidget {
    fn styled(&self, is_selected: bool) -> Span<'static> {
        match self {
            SettingWidget::Toggle(val) => {
                if *val {
                    Span::styled(
                        " [ ON ] ",
                        Style::default()
                            .fg(Theme::ACCENT_GREEN)
                            .bg(if is_selected {
                                Theme::SELECTION_BG
                            } else {
                                Theme::SURFACE
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        " [ OFF ] ",
                        Style::default().fg(Theme::TEXT_MUTED).bg(if is_selected {
                            Theme::SELECTION_BG
                        } else {
                            Theme::SURFACE
                        }),
                    )
                }
            }
            SettingWidget::Select(val) => {
                let text = format!("◄  {}  ►", val);
                if is_selected {
                    Span::styled(
                        text,
                        Style::default()
                            .fg(Theme::ACCENT_GOLD)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(text, Style::default().fg(Theme::TEXT_MUTED))
                }
            }
            SettingWidget::Action(label) => {
                let text = format!(" [ {} ] ", label);
                if is_selected {
                    Span::styled(
                        text,
                        Style::default()
                            .fg(Theme::ACCENT_PINK)
                            .bg(Theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(text, Style::default().fg(Theme::ACCENT_GOLD))
                }
            }
            SettingWidget::Info(val) => {
                Span::styled(val.clone(), Style::default().fg(Theme::TEXT_MUTED))
            }
        }
    }
}

fn get_category_rows(
    cat: SettingsCategory,
    cfg: &Config,
    poster_cache_size: u64,
    history_count: usize,
) -> Vec<(&'static str, SettingWidget, &'static str)> {
    match cat {
        SettingsCategory::General => vec![
            (
                "Default Startup Tab",
                SettingWidget::Select(match cfg.default_tab.to_lowercase().as_str() {
                    "series" | "tv" => "Series".to_string(),
                    "history" => "History".to_string(),
                    "bookmarks" => "Bookmarks".to_string(),
                    _ => "Movies".to_string(),
                }),
                "Initial view presented upon launching kitty-mov without CLI arguments.",
            ),
            (
                "Resume on Startup",
                SettingWidget::Toggle(cfg.resume_on_startup),
                "Automatically resume playback of your most recently watched movie or series episode on launch.",
            ),
            (
                "TV Episode Order Mode",
                SettingWidget::Select(match cfg.episode_order.to_lowercase().as_str() {
                    "streaming" => "Streaming / Production (VixSrc Order)".to_string(),
                    _ => "Broadcast Order (Canonical / LookMovie)".to_string(),
                }),
                "TV episode order scheme. Broadcast matches canonical TV airings & LookMovie; Streaming matches VixSrc.",
            ),
            (
                "Player Engine",
                SettingWidget::Info("mpv (External High-Performance Player)".to_string()),
                "High-performance video renderer powering smooth playback, caching, and subtitle sync.",
            ),
            (
                "TMDB Metadata API",
                SettingWidget::Info("Connected (v3 API Active)".to_string()),
                "The Movie Database API supplying cast listings, overview blurbs, and backdrop artwork.",
            ),
        ],
        SettingsCategory::Playback => vec![
            (
                "Audio Preference Mode",
                SettingWidget::Select(match cfg.audio_mode.to_lowercase().as_str() {
                    "sub" => "Sub (Original Audio + Subtitles)".to_string(),
                    _ => "Dub (English Voice Tracks)".to_string(),
                }),
                "Dub prioritizes localized English tracks; Sub chooses original audio tracks with subtitles.",
            ),
            (
                "Default Audio Language",
                SettingWidget::Select(format!("{} ({})", cfg.audio_language, lang_name(&cfg.audio_language))),
                "Preferred audio language requested from streaming providers and selected in mpv.",
            ),
            (
                "Default Subtitle Language",
                SettingWidget::Select(format!("{} ({})", cfg.sub_language, lang_name(&cfg.sub_language))),
                "Preferred subtitle language automatically displayed when streaming media.",
            ),
            (
                "Preferred Subtitle Source",
                SettingWidget::Select(match cfg.subtitle_source.to_lowercase().as_str() {
                    "provider" => "Provider Native Only (LookMovie .vtt)".to_string(),
                    "opensubtitles" => "OpenSubtitles.org Only".to_string(),
                    "both" | "all" => "Both (Provider Native + OpenSubtitles)".to_string(),
                    _ => "Auto (Provider First, then OpenSubtitles)".to_string(),
                }),
                "Choose subtitle origin. Provider Native uses pre-synced provider tracks; OpenSubtitles offers broader languages.",
            ),
            (
                "Subtitle Timing Offset",
                SettingWidget::Select(format!("{:+.1}s (Use z/Z in MPV)", cfg.sub_delay)),
                "Default timing offset applied to external subtitles. Use [z] and [Z] inside MPV for on-the-fly +/-100ms sync.",
            ),
            (
                "Auto-Play Next Episode",
                SettingWidget::Toggle(cfg.auto_play_next),
                "Automatically queue and launch the next sequential episode when the current one concludes.",
            ),
            (
                "Hardware Decoding (hwdec)",
                SettingWidget::Select(match cfg.hwdec.as_str() {
                    "auto" => "auto (MPV auto detection)".to_string(),
                    "auto-safe" => "auto-safe (Safe hardware decoder)".to_string(),
                    "vaapi" => "vaapi (Intel / AMD Linux GPU)".to_string(),
                    "nvdec" => "nvdec (NVIDIA GPU)".to_string(),
                    "videotoolbox" => "videotoolbox (Apple Silicon / macOS)".to_string(),
                    "no" => "no (Software CPU decoding)".to_string(),
                    other => other.to_string(),
                }),
                "GPU hardware acceleration engine passed to mpv for efficient 1080p video decoding.",
            ),
        ],
        SettingsCategory::Providers => vec![
            (
                "Preferred Streaming Provider",
                SettingWidget::Select(match cfg.preferred_provider.to_lowercase().as_str() {
                    "vixsrc" => "VixSrc First (Ultra-Fast 1080p CDN)".to_string(),
                    "lookmovie" => "LookMovie First (Fallback CDN)".to_string(),
                    _ => "Auto Fallback (VixSrc → LookMovie)".to_string(),
                }),
                "Streaming server query order. Auto Fallback tries VixSrc first, then seamlessly switches to LookMovie.",
            ),
            (
                "Subtitle Source Strategy",
                SettingWidget::Select(match cfg.subtitle_source.to_lowercase().as_str() {
                    "provider" => "Provider Native (LookMovie)".to_string(),
                    "opensubtitles" => "OpenSubtitles.org".to_string(),
                    "both" | "all" => "Both (All Tracks Loaded)".to_string(),
                    _ => "Auto (Best Match First)".to_string(),
                }),
                "Controls whether subtitles are pulled from LookMovie's CDN or fetched externally from OpenSubtitles.",
            ),
            (
                "Subtitle Augmentation",
                SettingWidget::Toggle(cfg.subtitle_augmentation),
                "Queries OpenSubtitles for missing subtitle tracks when provider streams lack requested languages.",
            ),
            (
                "Network Timeout",
                SettingWidget::Info("10 seconds (Auto Failover)".to_string()),
                "Maximum duration kitty-mov waits for a provider response before failing over to the next source.",
            ),
        ],
        SettingsCategory::Appearance => vec![
            (
                "Color Theme Palette",
                SettingWidget::Select(match cfg.theme_name.as_str() {
                    "amber" => "Warm Amber (Cinematic)".to_string(),
                    "rose" => "Neon Rose (Cyberpunk)".to_string(),
                    "emerald" => "Emerald Matrix (Green)".to_string(),
                    _ => "Cyber Cyan (Default Glow)".to_string(),
                }),
                "Accent color palette for borders, badges, focus rings, and selection surfaces.",
            ),
            (
                "Poster Art Render Mode",
                SettingWidget::Select(match cfg.poster_render_mode.as_str() {
                    "off" => "Off (Text-Only Mode)".to_string(),
                    _ => "Half-Block 24-bit ANSI (High Quality)".to_string(),
                }),
                "Preview artwork format. Half-Block renders truecolor ANSI graphics; Off preserves terminal simplicity.",
            ),
            (
                "Interface Layout",
                SettingWidget::Info("Spacious Split-Pane (Adaptive)".to_string()),
                "Cinematic split-pane presentation with persistent navigation and contextual inspector.",
            ),
        ],
        SettingsCategory::Storage => vec![
            (
                "Configuration File",
                SettingWidget::Info(crate::config::get_config_dir().join("config.json").to_string_lossy().to_string()),
                "Absolute path where your user preferences and configuration are saved atomically.",
            ),
            (
                "Poster Artwork Cache",
                SettingWidget::Action(format!("Clear Cache ({})", format_bytes(poster_cache_size))),
                "Delete locally cached poster artwork thumbnails to reclaim system storage space.",
            ),
            (
                "Watch History Database",
                SettingWidget::Action(format!("Clear Watch History ({} records)", history_count)),
                "Clear playback timestamps, episode progress, and watch records from your local database.",
            ),
            (
                "Export Diagnostic Report",
                SettingWidget::Action("Export Sanitized Report".to_string()),
                "Generate a redacted report with system, dependency, and network diagnostics for bug reports.",
            ),
            (
                "Factory Reset",
                SettingWidget::Action("Reset All to Factory Defaults".to_string()),
                "Reset all configuration settings to original defaults without deleting watch history.",
            ),
        ],
    }
}
