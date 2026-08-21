use dioxus::{core::Runtime, prelude::*};
use log::warn;
use web_sys::wasm_bindgen::{JsCast, closure::Closure};

use valens_web_app::{self as web_app, SettingsService};

use crate::{WEB_APP_SERVICE, notification::notify_error};

/// Whether the system asks for a dark color scheme.
static PREFERS_DARK_SCHEME: GlobalSignal<bool> =
    Signal::global(|| color_scheme_query().is_some_and(|query| query.matches()));

#[derive(Clone, Copy, PartialEq)]
pub struct Settings {
    settings: Signal<web_app::Settings>,
}

impl Settings {
    pub fn provide() {
        use_hook(listen_for_color_scheme_changes);
        let settings = use_signal(web_app::Settings::default);
        use_context_provider(move || Self { settings });
        let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
        use_effect(move || match settings.read().as_ref() {
            Some(Ok(settings)) => {
                consume_context::<Self>().settings.set(*settings);
                settings.theme.apply();
            }
            Some(Err(err)) => {
                notify_error("load settings", err);
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
        self.settings.read().theme.resolve(PREFERS_DARK_SCHEME())
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
            notify_error("save settings", err);
        }
    }
}

/// Updates `PREFERS_DARK_SCHEME` on a change of the system color scheme.
fn listen_for_color_scheme_changes() {
    let Some(query) = color_scheme_query() else {
        return;
    };
    // Writing a signal requires the Dioxus runtime, which a browser callback does not run in.
    let on_change = Runtime::wrap_closure(move |event: web_sys::MediaQueryListEvent| {
        *PREFERS_DARK_SCHEME.write() = event.matches();
    });
    let closure =
        Closure::wrap(Box::new(on_change) as Box<dyn FnMut(web_sys::MediaQueryListEvent)>);
    if let Err(err) =
        query.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
    {
        warn!("failed to listen for color scheme changes: {err:?}");
        return;
    }
    closure.forget();
}

fn color_scheme_query() -> Option<web_sys::MediaQueryList> {
    let Some(window) = web_sys::window() else {
        warn!("failed to access window to determine preferred color scheme");
        return None;
    };
    match window.match_media("(prefers-color-scheme: dark)") {
        Ok(Some(query)) => Some(query),
        Ok(None) => {
            warn!("failed to determine preferred color scheme");
            None
        }
        Err(err) => {
            warn!("failed to match media to determine preferred color scheme: {err:?}");
            None
        }
    }
}
