use dioxus::prelude::*;

use valens_web_app::{self as web_app, SettingsService};

use crate::{
    WEB_APP_SERVICE,
    notification::notify_error,
    ui::element::{Color, Dialog, Icon},
};

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

#[component]
pub fn SettingsDialog(on_close: EventHandler<MouseEvent>) -> Element {
    let settings = use_context::<Settings>();
    let notification_permission = web_sys::Notification::permission();
    let notifications_color = match notification_permission {
        web_sys::NotificationPermission::Granted if settings.notifications() => "is-link",
        web_sys::NotificationPermission::Denied => "is-danger",
        _ => "",
    };
    rsx! {
        Dialog {
            color: Color::Primary,
            title: rsx! { "Settings" },
            on_close,
            p {
                h1 { class: "subtitle", "Beep volume" }
                input {
                    class: "slider is-fullwidth is-info",
                    max: "100",
                    min: "0",
                    r#type: "range",
                    step: "10",
                    value: settings.beep_volume(),
                    oninput: move |event| {
                        let mut settings = settings;
                        if let Ok(value) = event.value().parse() {
                            settings.set_beep_volume(value);
                        }
                        async move {
                            settings.save().await;
                        }
                    },
                }
            }
            p {
                class: "mb-5",
                h1 { class: "subtitle", "Theme" }
                div {
                    class: "field has-addons",
                    p {
                        class: "control",
                        button {
                            class: "button",
                            class: if settings.theme() == web_app::Theme::Light { "is-link" },
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::Light);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            Icon { name: "sun", is_small: true }
                            span { "Light" }
                        }
                    }
                    p {
                        class: "control",
                        button {
                            class: "button",
                            class: if settings.theme() == web_app::Theme::Dark { "is-link" },
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::Dark);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            Icon { name: "moon", is_small: true }
                            span { "Dark" }
                        }
                    }
                    p { class: "control",
                        button {
                            class: "button",
                            class: if settings.theme() == web_app::Theme::System { "is-link" },
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::System);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            Icon { name: "desktop", is_small: true }
                            span { "System" }
                        }
                    }
                }
            }
            p {
                class: "mb-5",
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        settings.set_automatic_metronome(!settings.automatic_metronome());
                        async move {
                            settings.save().await;
                        }
                    }
                },
                h1 { class: "subtitle", "Metronome" }
                if settings.automatic_metronome() {
                    button { class: "button is-link", "Automatic" }
                } else {
                    button { class: "button", "Manual" }
                }
            }
            p {
                class: "mb-5",
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        settings.set_show_rpe(!settings.show_rpe());
                        async move {
                            settings.save().await;
                        }
                    }
                },
                h1 { class: "subtitle", "Rating of Perceived Exertion (RPE)" }
                if settings.show_rpe() {
                    button { class: "button is-link", "Enabled" }
                } else {
                    button { class: "button", "Disabled" }
                }
            }
            p {
                class: "mb-5",
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        settings.set_show_tut(!settings.show_tut());
                        async move {
                            settings.save().await;
                        }
                    }
                },
                h1 { class: "subtitle", "Time Under Tension (TUT)" }
                if settings.show_tut() {
                    button { class: "button is-link", "Enabled" }
                } else {
                    button { class: "button", "Disabled" }
                }
            }
            p {
                class: "mb-5",
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        settings.set_scroll_snapping(!settings.scroll_snapping());
                        async move {
                            settings.save().await;
                        }
                    }
                },
                h1 { class: "subtitle", "Scroll snapping" }
                if settings.scroll_snapping() {
                    button { class: "button is-link", "Enabled" }
                } else {
                    button { class: "button", "Disabled" }
                }
            }
            p {
                class: "mb-5",
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        async move {
                            match notification_permission {
                                web_sys::NotificationPermission::Granted => {
                                    settings.set_notifications(!settings.notifications());
                                    settings.save().await;
                                }
                                web_sys::NotificationPermission::Denied => {
                                }
                                _ => {
                                    match web_app::request_notification_permission().await {
                                        Ok(web_sys::NotificationPermission::Granted) => {
                                            settings.set_notifications(true);
                                        }
                                        Ok(_) => {}
                                        Err(err) => {
                                            notify_error(format!("Failed to enable notifications: {err}"));
                                        }
                                    }
                                    settings.save().await;
                                }
                            }
                        }
                    }
                },
                h1 { class: "subtitle", "Notifications" }
                button {
                    class: "button",
                    class: "{notifications_color}",
                    match notification_permission {
                        web_sys::NotificationPermission::Granted => {
                            if settings.notifications() {
                                "Enabled"
                            } else {
                                "Disabled"
                            }
                        }
                        web_sys::NotificationPermission::Denied => {
                            "Not allowed in browser settings"
                        }
                        _ => {
                            "Enable"
                        }
                    }
                }
                if let web_sys::NotificationPermission::Denied = notification_permission {
                    p {
                        class: "mt-3",
                        "To enable notifications, open the site settings from the address bar and allow notifications. If the app is installed and no address bar is visible, open it in your browser instead. Notifications are blocked in incognito or private browsing mode."
                    }
                }
            }
        }
    }
}
