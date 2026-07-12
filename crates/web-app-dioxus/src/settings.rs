use dioxus::prelude::*;

use valens_web_app::{self as web_app, SettingsService};

use crate::{WEB_APP_SERVICE, notification::notify_error};

#[derive(Clone, Copy, PartialEq)]
pub struct Settings {
    settings: Signal<web_app::Settings>,
}

impl Settings {
    pub fn provide() {
        let settings = use_signal(web_app::Settings::default);
        use_context_provider(move || Self { settings });
        let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
        use_effect(move || match settings.read().as_ref() {
            Some(Ok(settings)) => {
                consume_context::<Self>().settings.set(*settings);
                settings.theme.apply();
            }
            Some(Err(err)) => {
                notify_error(format!("Failed to load settings: {err}"));
            }
            None => {}
        });
    }

    pub fn beep_volume(&self) -> u8 {
        self.settings.read().beep_volume
    }

    pub fn set_beep_volume(&mut self, beep_volume: u8) {
        self.settings.write().beep_volume = beep_volume.clamp(0, 100);
    }

    pub fn theme(&self) -> web_app::Theme {
        self.settings.read().theme
    }

    pub fn set_theme(&mut self, theme: web_app::Theme) {
        self.settings.write().theme = theme;
    }

    pub fn current_theme(&self) -> web_app::Theme {
        self.settings.read().current_theme()
    }

    pub fn automatic_metronome(&self) -> bool {
        self.settings.read().automatic_metronome
    }

    pub fn set_automatic_metronome(&mut self, automatic_metronome: bool) {
        self.settings.write().automatic_metronome = automatic_metronome;
    }

    pub fn notifications(&self) -> bool {
        self.settings.read().notifications
    }

    pub fn set_notifications(&mut self, notifications: bool) {
        self.settings.write().notifications = notifications;
    }

    pub fn show_rpe(&self) -> bool {
        self.settings.read().show_rpe
    }

    pub fn set_show_rpe(&mut self, show_rpe: bool) {
        self.settings.write().show_rpe = show_rpe;
    }

    pub fn show_tut(&self) -> bool {
        self.settings.read().show_tut
    }

    pub fn set_show_tut(&mut self, show_tut: bool) {
        self.settings.write().show_tut = show_tut;
    }

    pub fn scroll_snapping(&self) -> bool {
        self.settings.read().scroll_snapping
    }

    pub fn set_scroll_snapping(&mut self, scroll_snapping: bool) {
        self.settings.write().scroll_snapping = scroll_snapping;
    }

    pub async fn save(&self) {
        if let Err(err) = WEB_APP_SERVICE
            .write()
            .set_settings(self.settings.cloned())
            .await
        {
            notify_error(format!("Failed to save settings: {err}"));
        }
    }
}
