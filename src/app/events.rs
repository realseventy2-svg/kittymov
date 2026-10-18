use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{
    ActiveTab, ProviderModalCategory, SettingsCategory, SettingsPane, SetupStep,
};
use super::App;

impl App {
    pub async fn handle_key_event(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        // -3. If Error Dialog is active, allow dismissal or jumping to provider selector
        if self.error_dialog.is_some() {
            match code {
                KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Char(' ')
                | KeyCode::Char('q')
                | KeyCode::Char('x') => {
                    self.error_dialog = None;
                }
                KeyCode::Char('s') => {
                    self.error_dialog = None;
                    self.open_provider_modal_for_selected();
                }
                _ => {}
            }
            return;
        }

        // -2. If Initial Setup Wizard is active
        if self.setup_wizard.is_some() {
            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.setup_wizard_cancel();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.setup_wizard_prev_item();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.setup_wizard_next_item();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.setup_wizard_change_value(false);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.setup_wizard_change_value(true);
                }
                KeyCode::Tab => {
                    self.setup_wizard_next_step();
                }
                KeyCode::BackTab => {
                    self.setup_wizard_prev_step();
                }
                KeyCode::Char('1') => {
                    self.setup_wizard_set_step(SetupStep::Welcome);
                }
                KeyCode::Char('2') => {
                    self.setup_wizard_set_step(SetupStep::Playback);
                }
                KeyCode::Char('3') => {
                    self.setup_wizard_set_step(SetupStep::Subtitles);
                }
                KeyCode::Char('4') => {
                    self.setup_wizard_set_step(SetupStep::Providers);
                }
                KeyCode::Char('5') => {
                    self.setup_wizard_set_step(SetupStep::Appearance);
                }
                KeyCode::Char('6') => {
                    self.setup_wizard_set_step(SetupStep::Complete);
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.setup_wizard_confirm();
                    if self.setup_wizard.is_none() {
                        self.update_poster().await;
                    }
                }
                _ => {}
            }
            return;
        }

        // -1. If Playback Loading / Bootstrap screen is active
        if self.playback_loading.is_some() {
            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.cancel_playback_loading();
                }
                KeyCode::Char('F') | KeyCode::Char('f') => {
                    self.force_playback_fallback().await;
                }
                _ => {}
            }
            return;
        }

        // 0. If Help modal is active
        if self.show_help {
            match code {
                KeyCode::Esc
                | KeyCode::Char('?')
                | KeyCode::Char('q')
                | KeyCode::Enter
                | KeyCode::F(1) => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // 1. If Settings Modal is open
        if let Some(ref s) = self.settings_modal.clone() {
            // A. If confirmation dialog is active
            if s.confirmation.is_some() {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        self.settings_execute_confirmation().await;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.settings_cancel_confirmation();
                    }
                    _ => {}
                }
                return;
            }

            // B. If in search input mode
            if s.is_searching {
                match code {
                    KeyCode::Char(c) => {
                        if let Some(ref mut sm) = self.settings_modal {
                            sm.search_query.push(c);
                            sm.item_idx = 0;
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(ref mut sm) = self.settings_modal {
                            sm.search_query.pop();
                            sm.item_idx = 0;
                        }
                    }
                    KeyCode::Up => {
                        self.settings_prev();
                    }
                    KeyCode::Down => {
                        self.settings_next();
                    }
                    KeyCode::Enter => {
                        self.settings_change_value(true);
                    }
                    KeyCode::Esc => {
                        if let Some(ref mut sm) = self.settings_modal {
                            if !sm.search_query.is_empty() {
                                sm.search_query.clear();
                            } else {
                                sm.is_searching = false;
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }

            // C. Normal Settings Navigation
            match code {
                KeyCode::Esc => {
                    if let Some(ref mut sm) = self.settings_modal {
                        if !sm.search_query.is_empty() {
                            sm.search_query.clear();
                        } else if sm.active_pane == SettingsPane::Content {
                            sm.active_pane = SettingsPane::Categories;
                        } else {
                            self.toggle_settings();
                            if self.episode_modal.is_some() {
                                self.refresh_episode_modal().await;
                            }
                            self.update_poster().await;
                        }
                    }
                }
                KeyCode::Char('S') => {
                    self.toggle_settings();
                    if self.episode_modal.is_some() {
                        self.refresh_episode_modal().await;
                    }
                    self.update_poster().await;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    self.settings_switch_pane();
                }
                KeyCode::Char('/') => {
                    if let Some(ref mut sm) = self.settings_modal {
                        sm.is_searching = true;
                    }
                }
                KeyCode::Char('r') => {
                    self.settings_reset_current_category();
                }
                KeyCode::Char('?') => {
                    if let Some(ref mut sm) = self.settings_modal {
                        sm.feedback_message = Some(
                            "Keys: Tab switch pane, ↑/↓ navigate, ←/→/Space change, r reset, / search, Esc close"
                                .to_string(),
                        );
                    }
                }
                KeyCode::F(1) => {
                    self.toggle_help();
                }
                KeyCode::Home => {
                    if let Some(ref mut sm) = self.settings_modal {
                        sm.item_idx = 0;
                    }
                }
                KeyCode::End => {
                    if let Some(ref mut sm) = self.settings_modal {
                        match sm.active_pane {
                            SettingsPane::Categories => {
                                sm.category_idx = SettingsCategory::ALL.len().saturating_sub(1);
                            }
                            SettingsPane::Content => {
                                let cat = sm.current_category();
                                sm.item_idx = cat.item_count().saturating_sub(1);
                            }
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.settings_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.settings_next();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if let Some(ref sm) = self.settings_modal {
                        if sm.active_pane == SettingsPane::Content {
                            self.settings_change_value(false);
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(ref mut sm) = self.settings_modal {
                        if sm.active_pane == SettingsPane::Categories {
                            sm.active_pane = SettingsPane::Content;
                        } else {
                            self.settings_change_value(true);
                        }
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.settings_change_value(true);
                }
                _ => {}
            }
            return;
        }

        // 1.5. If Resume / "Where to start?" modal is open
        if self.resume_modal.is_some() {
            let is_filtering = self
                .resume_modal
                .as_ref()
                .map(|m| m.is_filtering)
                .unwrap_or(false);

            if is_filtering {
                match code {
                    KeyCode::Esc => {
                        if let Some(ref mut m) = self.resume_modal {
                            m.is_filtering = false;
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(ref mut m) = self.resume_modal {
                            m.filter_query.pop();
                            m.selected_idx = 0;
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(ref mut m) = self.resume_modal {
                            m.filter_query.push(c);
                            m.selected_idx = 0;
                        }
                    }
                    KeyCode::Up => {
                        if let Some(ref mut m) = self.resume_modal {
                            let count = m.filtered_indices().len();
                            if count > 0 {
                                if m.selected_idx == 0 {
                                    m.selected_idx = count - 1;
                                } else {
                                    m.selected_idx -= 1;
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(ref mut m) = self.resume_modal {
                            let count = m.filtered_indices().len();
                            if count > 0 {
                                m.selected_idx = (m.selected_idx + 1) % count;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let opt_kind = self
                            .resume_modal
                            .as_ref()
                            .and_then(|m| m.get_selected_option().map(|o| o.kind));
                        if let Some(kind) = opt_kind {
                            self.handle_resume_action(kind).await;
                        }
                    }
                    _ => {}
                }
                return;
            }

            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.resume_modal = None;
                }
                KeyCode::Char('/') => {
                    if let Some(ref mut m) = self.resume_modal {
                        m.is_filtering = true;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(ref mut m) = self.resume_modal {
                        let count = m.filtered_indices().len();
                        if count > 0 {
                            if m.selected_idx == 0 {
                                m.selected_idx = count - 1;
                            } else {
                                m.selected_idx -= 1;
                            }
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref mut m) = self.resume_modal {
                        let count = m.filtered_indices().len();
                        if count > 0 {
                            m.selected_idx = (m.selected_idx + 1) % count;
                        }
                    }
                }
                KeyCode::Enter => {
                    let opt_kind = self
                        .resume_modal
                        .as_ref()
                        .and_then(|m| m.get_selected_option().map(|o| o.kind));
                    if let Some(kind) = opt_kind {
                        self.handle_resume_action(kind).await;
                    }
                }
                _ => {}
            }
            return;
        }

        // 2. If Subtitle Language Selector modal is open
        if self.subtitle_modal.is_some() {
            match code {
                KeyCode::Esc => {
                    self.subtitle_modal = None;
                }
                KeyCode::Char('?') | KeyCode::F(1) => {
                    self.toggle_help();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.subtitle_modal_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.subtitle_modal_next();
                }
                KeyCode::Enter => {
                    self.subtitle_modal_confirm();
                }
                KeyCode::Char('s') => {
                    self.cycle_subtitle_source();
                }
                KeyCode::Char('d') => {
                    self.toggle_audio_mode();
                }
                _ => {}
            }
            return;
        }

        // 3. If Provider / Tracks selector screen is open
        if self.provider_modal.is_some() {
            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.provider_modal = None;
                }
                KeyCode::Char('?') | KeyCode::F(1) => {
                    self.toggle_help();
                }
                KeyCode::Tab => {
                    self.provider_modal_switch_pane();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.provider_modal_left();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.provider_modal_right();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.provider_modal_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.provider_modal_next();
                }
                KeyCode::Char('1') => {
                    self.provider_modal_set_category(ProviderModalCategory::Provider);
                }
                KeyCode::Char('2') => {
                    self.provider_modal_set_category(ProviderModalCategory::Source);
                }
                KeyCode::Char('3') => {
                    self.provider_modal_set_category(ProviderModalCategory::Quality);
                }
                KeyCode::Char('4') => {
                    self.provider_modal_set_category(ProviderModalCategory::Subtitles);
                }
                KeyCode::Char('d') => {
                    self.toggle_audio_mode();
                }
                KeyCode::Char('f') => {
                    self.toggle_favorite().await;
                }
                KeyCode::Enter => {
                    self.provider_modal_confirm().await;
                }
                _ => {}
            }
            return;
        }

        // 4. If in search input mode
        if self.is_searching {
            match code {
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Enter => {
                    self.perform_search().await;
                }
                KeyCode::Esc => {
                    self.is_searching = false;
                }
                _ => {}
            }
            return;
        }

        // 5. If Episode Modal is open
        if let Some(ref mut modal) = self.episode_modal {
            match code {
                KeyCode::Esc => {
                    self.episode_modal = None;
                }
                KeyCode::Char('?') | KeyCode::F(1) => {
                    self.toggle_help();
                }
                KeyCode::Char('s') => {
                    self.open_provider_modal_for_selected();
                }
                KeyCode::Char('o') => {
                    self.cycle_preferred_provider(true).await;
                }
                KeyCode::Char('t') => {
                    self.open_subtitle_modal().await;
                }
                KeyCode::Char('d') => {
                    self.toggle_audio_mode();
                }
                KeyCode::Char('S') => {
                    self.toggle_settings();
                    if self.episode_modal.is_some() {
                        self.refresh_episode_modal().await;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if modal.selected_episode_idx > 0 {
                        modal.selected_episode_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !modal.episodes.is_empty()
                        && modal.selected_episode_idx + 1 < modal.episodes.len()
                    {
                        modal.selected_episode_idx += 1;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('[') => {
                    if modal.selected_season_idx > 0 {
                        modal.selected_season_idx -= 1;
                        modal.selected_episode_idx = 0;
                        if let Some(s) = modal.seasons.get(modal.selected_season_idx) {
                            let tmdb_id = modal.series.tmdb_id;
                            let s_num = s.season_number;
                            if let Some(ref map) = modal.all_season_episodes {
                                modal.episodes = map.get(&s_num).cloned().unwrap_or_default();
                            } else {
                                modal.episodes = self
                                    .tmdb
                                    .get_season_episodes(tmdb_id, s_num)
                                    .await
                                    .unwrap_or_default();
                            }
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(']') => {
                    if !modal.seasons.is_empty()
                        && modal.selected_season_idx + 1 < modal.seasons.len()
                    {
                        modal.selected_season_idx += 1;
                        modal.selected_episode_idx = 0;
                        if let Some(s) = modal.seasons.get(modal.selected_season_idx) {
                            let tmdb_id = modal.series.tmdb_id;
                            let s_num = s.season_number;
                            if let Some(ref map) = modal.all_season_episodes {
                                modal.episodes = map.get(&s_num).cloned().unwrap_or_default();
                            } else {
                                modal.episodes = self
                                    .tmdb
                                    .get_season_episodes(tmdb_id, s_num)
                                    .await
                                    .unwrap_or_default();
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    self.on_enter().await;
                }
                KeyCode::Char('p') => {
                    self.quick_play_selected().await;
                }
                KeyCode::Char('a') => {
                    self.cfg.auto_play_next = !self.cfg.auto_play_next;
                    let _ = self.cfg.save();
                    self.status_message = format!(
                        "Auto-Play Next: {}",
                        if self.cfg.auto_play_next {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                }
                _ => {}
            }
            return;
        }

        // 6. Main view navigation
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.toggle_help();
            }
            KeyCode::Char('/') => {
                self.is_searching = true;
                self.search_query.clear();
            }
            KeyCode::Char('1') => {
                self.load_tab(ActiveTab::Movies).await;
            }
            KeyCode::Char('2') => {
                self.load_tab(ActiveTab::Series).await;
            }
            KeyCode::Char('3') => {
                self.load_tab(ActiveTab::History).await;
            }
            KeyCode::Char('4') => {
                self.load_tab(ActiveTab::Bookmarks).await;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev().await;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next().await;
            }
            KeyCode::Enter => {
                self.on_enter().await;
            }
            KeyCode::Char('p') => {
                self.quick_play_selected().await;
            }
            KeyCode::Char('s') => {
                self.open_provider_modal_for_selected();
            }
            KeyCode::Char('t') => {
                self.open_subtitle_modal().await;
            }
            KeyCode::Char('d') => {
                self.toggle_audio_mode();
            }
            KeyCode::Char('o') => {
                self.cycle_preferred_provider(true).await;
            }
            KeyCode::Char('S') | KeyCode::F(2) | KeyCode::Char(',') => {
                self.toggle_settings();
                if self.episode_modal.is_some() {
                    self.refresh_episode_modal().await;
                }
            }
            KeyCode::Char('f') => {
                self.toggle_favorite().await;
            }
            KeyCode::Char('a') => {
                self.cfg.auto_play_next = !self.cfg.auto_play_next;
                let _ = self.cfg.save();
                self.status_message = format!(
                    "Auto-Play Next: {}",
                    if self.cfg.auto_play_next {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            _ => {}
        }
    }
}
