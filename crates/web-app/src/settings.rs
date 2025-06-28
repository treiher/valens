use log::error;

#[allow(async_fn_in_trait)]
pub trait SettingsService {
    async fn get_settings(&self) -> Result<Settings, String>;
    async fn set_settings(&self, settings: Settings) -> Result<(), String>;
}

#[allow(async_fn_in_trait)]
pub trait SettingsRepository {
    async fn read_settings(&self) -> Result<Settings, String>;
    async fn write_settings(&self, settings: Settings) -> Result<(), String>;
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub beep_volume: u8,
    pub theme: Theme,
    pub automatic_metronome: bool,
    pub notifications: bool,
    pub show_rpe: bool,
    pub show_tut: bool,
}

impl Settings {
    #[must_use]
    pub fn current_theme(&self) -> Theme {
        match self.theme {
            Theme::System => {
                if let Some(window) = web_sys::window() {
                    if let Ok(prefers_dark_scheme) =
                        window.match_media("(prefers-color-scheme: dark)")
                    {
                        if let Some(media_query_list) = prefers_dark_scheme {
                            if media_query_list.matches() {
                                Theme::Dark
                            } else {
                                Theme::Light
                            }
                        } else {
                            error!("failed to determine preferred color scheme");
                            Theme::Light
                        }
                    } else {
                        error!("failed to match media to determine preferred color scheme");
                        Theme::Light
                    }
                } else {
                    error!("failed to access window to determine preferred color scheme");
                    Theme::Light
                }
            }
            Theme::Light | Theme::Dark => self.theme,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            beep_volume: 80,
            theme: Theme::Light,
            automatic_metronome: false,
            notifications: false,
            show_rpe: true,
            show_tut: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}
