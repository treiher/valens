use log::warn;

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
    #[serde(default)]
    pub scroll_snapping: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            beep_volume: 80,
            theme: Theme::System,
            automatic_metronome: false,
            notifications: false,
            show_rpe: true,
            show_tut: true,
            scroll_snapping: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    /// The theme to render, with `System` resolved to the system color scheme.
    #[must_use]
    pub fn resolve(self, prefers_dark_scheme: bool) -> Theme {
        match self {
            Theme::System => {
                if prefers_dark_scheme {
                    Theme::Dark
                } else {
                    Theme::Light
                }
            }
            Theme::Light | Theme::Dark => self,
        }
    }

    pub fn apply(self) {
        if let Some(html) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
            && let Err(err) = match self {
                Theme::System => html.remove_attribute("data-theme"),
                Theme::Light => html.set_attribute("data-theme", "light"),
                Theme::Dark => html.set_attribute("data-theme", "dark"),
            }
        {
            warn!("failed to apply theme: {err:?}");
        }
    }
}
