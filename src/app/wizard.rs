use std::sync::Arc;

use crate::providers::ProviderManager;

use super::mod_utils::cycle_list;
use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    Welcome = 0,
    Playback = 1,
    Subtitles = 2,
    Providers = 3,
    Appearance = 4,
    Complete = 5,
}

impl SetupStep {
    pub const ALL: [SetupStep; 6] = [
        SetupStep::Welcome,
        SetupStep::Playback,
        SetupStep::Subtitles,
        SetupStep::Providers,
        SetupStep::Appearance,
        SetupStep::Complete,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            SetupStep::Welcome => "Welcome",
            SetupStep::Playback => "Playback",
            SetupStep::Subtitles => "Audio & Subs",
            SetupStep::Providers => "Providers",
            SetupStep::Appearance => "Appearance",
            SetupStep::Complete => "Ready",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SetupStep::Welcome => "Welcome to KittyMov & Quick Setup",
            SetupStep::Playback => "Video player engine & hardware GPU acceleration",
            SetupStep::Subtitles => "Voice audio tracks, dubbing, and subtitle languages",
            SetupStep::Providers => "Streaming server prioritization and subtitle preferences",
            SetupStep::Appearance => "Color themes and terminal poster cover art",
            SetupStep::Complete => "Setup overview and essential keyboard shortcuts",
        }
    }

    pub fn item_count(&self) -> usize {
        match self {
            SetupStep::Welcome => 2,
            SetupStep::Playback => 4,
            SetupStep::Subtitles => 5,
            SetupStep::Providers => 3,
            SetupStep::Appearance => 4,
            SetupStep::Complete => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetupWizardState {
    pub current_step: SetupStep,
    pub selected_item_idx: usize,
}

impl Default for SetupWizardState {
    fn default() -> Self {
        Self {
            current_step: SetupStep::Welcome,
            selected_item_idx: 0,
        }
    }
}

impl App {
    pub fn open_setup_wizard(&mut self) {
        self.setup_wizard = Some(SetupWizardState::default());
        self.status_message = "Initial Setup Wizard Active".to_string();
    }

    pub fn setup_wizard_next_item(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let count = wizard.current_step.item_count();
            if count > 0 && wizard.selected_item_idx + 1 < count {
                wizard.selected_item_idx += 1;
            }
        }
    }

    pub fn setup_wizard_prev_item(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            if wizard.selected_item_idx > 0 {
                wizard.selected_item_idx -= 1;
            }
        }
    }

    pub fn setup_wizard_change_value(&mut self, forward: bool) {
        let (step, item_idx) = match self.setup_wizard {
            Some(ref w) => (w.current_step, w.selected_item_idx),
            None => return,
        };

        match step {
            SetupStep::Welcome => {
                if forward {
                    self.setup_wizard_next_step();
                }
            }
            SetupStep::Playback => match item_idx {
                0 => {
                    let modes = ["auto", "vaapi", "nvdec", "vdpau", "no"];
                    self.cfg.hwdec = cycle_list(&modes, &self.cfg.hwdec, forward);
                }
                1 => {
                    self.cfg.resume_on_startup = !self.cfg.resume_on_startup;
                }
                2 => {
                    self.cfg.auto_play_next = !self.cfg.auto_play_next;
                }
                3 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Subtitles => match item_idx {
                0 => {
                    let langs = [
                        "eng", "jpn", "spa", "fra", "deu", "ita", "por", "kor", "zho",
                    ];
                    self.cfg.audio_language = cycle_list(&langs, &self.cfg.audio_language, forward);
                }
                1 => {
                    let modes = ["sub", "dub"];
                    self.cfg.audio_mode = cycle_list(&modes, &self.cfg.audio_mode, forward);
                }
                2 => {
                    let langs = [
                        "eng", "spa", "fra", "deu", "ita", "por", "ara", "ind", "jpn",
                    ];
                    self.cfg.sub_language = cycle_list(&langs, &self.cfg.sub_language, forward);
                }
                3 => {
                    self.cfg.subtitle_augmentation = !self.cfg.subtitle_augmentation;
                }
                4 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Providers => match item_idx {
                0 => {
                    let provs = ["auto", "vidlink", "vixsrc", "hianime", "lookmovie"];
                    self.cfg.preferred_provider =
                        cycle_list(&provs, &self.cfg.preferred_provider, forward);
                }
                1 => {
                    let sources = ["auto", "provider", "wyzie"];
                    self.cfg.subtitle_source =
                        cycle_list(&sources, &self.cfg.subtitle_source, forward);
                }
                2 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Appearance => match item_idx {
                0 => {
                    let themes = ["cyber", "matrix", "synthwave", "dracula", "nord"];
                    self.cfg.theme_name = cycle_list(&themes, &self.cfg.theme_name, forward);
                }
                1 => {
                    let modes = ["auto", "kitty", "high-res", "half-block", "off"];
                    self.cfg.poster_render_mode =
                        cycle_list(&modes, &self.cfg.poster_render_mode, forward);
                }
                2 => {
                    let tabs = ["movies", "series", "bookmarks", "history"];
                    self.cfg.default_tab = cycle_list(&tabs, &self.cfg.default_tab, forward);
                }
                3 => {
                    if forward {
                        self.setup_wizard_next_step();
                    } else {
                        self.setup_wizard_prev_step();
                    }
                }
                _ => {}
            },
            SetupStep::Complete => {
                if item_idx == 0 {
                    self.setup_wizard_finish();
                } else {
                    self.setup_wizard_set_step(SetupStep::Welcome);
                }
            }
        }
    }

    pub fn setup_wizard_confirm(&mut self) {
        let (step, item_idx) = match self.setup_wizard {
            Some(ref w) => (w.current_step, w.selected_item_idx),
            None => return,
        };

        match step {
            SetupStep::Welcome => {
                if item_idx == 0 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_finish();
                }
            }
            SetupStep::Playback => {
                if item_idx == 3 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Subtitles => {
                if item_idx == 4 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Providers => {
                if item_idx == 2 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Appearance => {
                if item_idx == 3 {
                    self.setup_wizard_next_step();
                } else {
                    self.setup_wizard_change_value(true);
                }
            }
            SetupStep::Complete => {
                if item_idx == 0 {
                    self.setup_wizard_finish();
                } else {
                    self.setup_wizard_set_step(SetupStep::Welcome);
                }
            }
        }
    }

    pub fn setup_wizard_next_step(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let cur = wizard.current_step as usize;
            if cur + 1 < SetupStep::ALL.len() {
                wizard.current_step = SetupStep::ALL[cur + 1];
                wizard.selected_item_idx = 0;
            } else {
                self.setup_wizard_finish();
            }
        }
    }

    pub fn setup_wizard_prev_step(&mut self) {
        if let Some(ref mut wizard) = self.setup_wizard {
            let cur = wizard.current_step as usize;
            if cur > 0 {
                wizard.current_step = SetupStep::ALL[cur - 1];
                wizard.selected_item_idx = 0;
            }
        }
    }

    pub fn setup_wizard_set_step(&mut self, step: SetupStep) {
        if let Some(ref mut wizard) = self.setup_wizard {
            wizard.current_step = step;
            wizard.selected_item_idx = 0;
        }
    }

    pub fn setup_wizard_finish(&mut self) {
        let _ = self.cfg.save();
        self.providers = Arc::new(ProviderManager::with_preference(
            &self.cfg.preferred_provider,
        ));
        self.setup_wizard = None;
        self.status_message = "Welcome to KittyMov! Preferences saved.".to_string();
    }

    pub fn setup_wizard_cancel(&mut self) {
        let _ = self.cfg.save();
        self.setup_wizard = None;
        self.status_message = "Setup skipped. Default configuration active.".to_string();
    }
}
