#[allow(async_fn_in_trait)]
pub trait SettingsRepository {
    async fn read_settings(&self) -> Result<Settings, String>;
    async fn write_settings(&self, settings: Settings) -> Result<(), String>;
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub beep_volume: u8,
    pub theme: Theme,
    pub automatic_metronome: bool,
    pub notifications: bool,
    pub show_rpe: bool,
    pub show_tut: bool,
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

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}
