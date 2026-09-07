use dioxus::prelude::*;

use valens_web_app as web_app;

use crate::{
    audio,
    notification::notify_error,
    settings::Settings,
    ui::element::{Dialog, Icon},
};

#[component]
pub fn SettingsDialog(on_close: EventHandler<MouseEvent>) -> Element {
    let settings = use_context::<Settings>();
    let notification_permission = web_app::notification_permission();
    let notifications_color = match notification_permission {
        Some(web_sys::NotificationPermission::Granted) if settings.notifications() => "is-link",
        Some(web_sys::NotificationPermission::Denied) => "is-danger",
        _ => "",
    };
    rsx! {
        Dialog {
            title: rsx! { "Settings" },
            on_close,
            p {
                class: "mb-5",
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
                    onchange: move |event| {
                        if let Ok(value) = event.value().parse() {
                            audio::play_volume_preview(value);
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
                            "data-selected": "{settings.theme() == web_app::Theme::Light}",
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::Light);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            "data-testid": "settings-theme-light",
                            Icon { name: "sun", is_small: true }
                            span { "Light" }
                        }
                    }
                    p {
                        class: "control",
                        button {
                            class: "button",
                            class: if settings.theme() == web_app::Theme::Dark { "is-link" },
                            "data-selected": "{settings.theme() == web_app::Theme::Dark}",
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::Dark);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            "data-testid": "settings-theme-dark",
                            Icon { name: "moon", is_small: true }
                            span { "Dark" }
                        }
                    }
                    p { class: "control",
                        button {
                            class: "button",
                            class: if settings.theme() == web_app::Theme::System { "is-link" },
                            "data-selected": "{settings.theme() == web_app::Theme::System}",
                            onclick: {
                                move |_| {
                                    let mut settings = settings;
                                    settings.set_theme(web_app::Theme::System);
                                    async move {
                                        settings.save().await;
                                    }
                                }
                            },
                            "data-testid": "settings-theme-system",
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
                        settings.set_show_rpe(!settings.show_rpe());
                        async move {
                            settings.save().await;
                        }
                    }
                },
                h1 { class: "subtitle", "Rating of Perceived Exertion (RPE)" }
                if settings.show_rpe() {
                    button { class: "button is-link", "data-testid": "settings-rpe", "Enabled" }
                } else {
                    button { class: "button", "data-testid": "settings-rpe", "Disabled" }
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
                    button { class: "button is-link", "data-testid": "settings-tut", "Enabled" }
                } else {
                    button { class: "button", "data-testid": "settings-tut", "Disabled" }
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
                    button { class: "button is-link", "data-testid": "settings-scroll-snapping", "Enabled" }
                } else {
                    button { class: "button", "data-testid": "settings-scroll-snapping", "Disabled" }
                }
            }
            p {
                onclick: {
                    move |_| {
                        let mut settings = settings;
                        async move {
                            match notification_permission {
                                Some(web_sys::NotificationPermission::Granted) => {
                                    settings.set_notifications(!settings.notifications());
                                    settings.save().await;
                                }
                                Some(web_sys::NotificationPermission::Denied) | None => {
                                }
                                _ => {
                                    match web_app::request_notification_permission().await {
                                        Ok(web_sys::NotificationPermission::Granted) => {
                                            settings.set_notifications(true);
                                        }
                                        Ok(_) => {}
                                        Err(err) => {
                                            notify_error("enable notifications", err);
                                        }
                                    }
                                    settings.save().await;
                                }
                            }
                        }
                    }
                },
                h1 { class: "subtitle", "Notifications" }
                if let Some(permission) = notification_permission {
                    button {
                        class: "button",
                        class: "{notifications_color}",
                        "data-testid": "settings-notifications",
                        match permission {
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
                    if let web_sys::NotificationPermission::Denied = permission {
                        p {
                            class: "mt-3",
                            "To enable notifications, open the site settings from the address bar and allow notifications. If the app is installed and no address bar is visible, open it in your browser instead. Notifications are blocked in incognito or private browsing mode."
                        }
                    }
                } else {
                    p { "data-testid": "settings-notifications-unsupported", "Not supported by this browser" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{attribute_of, contains, provide_settings, render, text_of};

    use super::*;

    fn render_settings(settings: web_app::Settings) -> String {
        render(move || {
            provide_settings(settings);
            rsx! { SettingsDialog { on_close: move |_| {} } }
        })
    }

    #[test]
    fn test_every_toggle_shows_the_stored_setting() {
        let html = render_settings(web_app::Settings {
            show_rpe: true,
            show_tut: false,
            scroll_snapping: true,
            ..web_app::Settings::default()
        });

        assert_eq!(text_of(&html, "settings-rpe"), "Enabled");
        assert_eq!(text_of(&html, "settings-tut"), "Disabled");
        assert_eq!(text_of(&html, "settings-scroll-snapping"), "Enabled");
    }

    #[test]
    fn test_the_chosen_theme_is_marked_as_selected() {
        let html = render_settings(web_app::Settings {
            theme: web_app::Theme::Dark,
            ..web_app::Settings::default()
        });

        assert_eq!(
            attribute_of(&html, "settings-theme-dark", "data-selected"),
            "true"
        );
        assert_eq!(
            attribute_of(&html, "settings-theme-light", "data-selected"),
            "false"
        );
    }

    #[test]
    fn test_without_notification_support_it_is_reported_as_unsupported() {
        let html = render_settings(web_app::Settings::default());

        assert!(contains(&html, "settings-notifications-unsupported"));
        assert!(!contains(&html, "settings-notifications"));
    }
}
