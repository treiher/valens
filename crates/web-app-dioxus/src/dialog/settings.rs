use dioxus::prelude::*;

use valens_web_app as web_app;

use crate::{
    audio,
    notification::notify_error,
    settings::Settings,
    ui::{
        element::{Color, Dialog, Message, SectionCaption},
        form::{ButtonSelect, ButtonSelectOption, RowField, SwitchField},
    },
};

#[component]
pub fn SettingsDialog(on_close: EventHandler<MouseEvent>) -> Element {
    let settings = use_context::<Settings>();
    let notification_permission = web_app::notification_permission();
    rsx! {
        Dialog {
            title: rsx! { "Settings" },
            on_close,
            SectionCaption { "General" }
            div {
                class: "field-list",
                RowField {
                    label: "Theme",
                    ButtonSelect {
                        options: vec![
                            ButtonSelectOption {
                                text: "Light".to_string(),
                                icon: Some("sun"),
                                value: web_app::Theme::Light,
                            },
                            ButtonSelectOption {
                                text: "Dark".to_string(),
                                icon: Some("moon"),
                                value: web_app::Theme::Dark,
                            },
                            ButtonSelectOption {
                                text: "System".to_string(),
                                icon: Some("desktop"),
                                value: web_app::Theme::System,
                            },
                        ],
                        selected: settings.theme(),
                        is_icon_only: true,
                        on_click: move |(_, theme)| {
                            let mut settings = settings;
                            settings.set_theme(theme);
                            async move {
                                settings.save().await;
                            }
                        },
                        "data-testid": "settings-theme",
                    }
                }
            }
            SectionCaption { "Tracking" }
            div {
                class: "field-list",
                SwitchField {
                    label: "Rating of Perceived Exertion (RPE)",
                    help: "Track how hard each set felt",
                    is_on: settings.show_rpe(),
                    on_change: move |enabled| {
                        let mut settings = settings;
                        settings.set_show_rpe(enabled);
                        async move {
                            settings.save().await;
                        }
                    },
                    "data-testid": "settings-rpe",
                }
                SwitchField {
                    label: "Time Under Tension (TUT)",
                    help: "Track the duration of each set",
                    is_on: settings.show_tut(),
                    on_change: move |enabled| {
                        let mut settings = settings;
                        settings.set_show_tut(enabled);
                        async move {
                            settings.save().await;
                        }
                    },
                    "data-testid": "settings-tut",
                }
            }
            SectionCaption { "Training sessions" }
            div {
                class: "field-list",
                SwitchField {
                    label: "Scroll snapping",
                    help: "Center a set when scrolling",
                    is_on: settings.scroll_snapping(),
                    on_change: move |enabled| {
                        let mut settings = settings;
                        settings.set_scroll_snapping(enabled);
                        async move {
                            settings.save().await;
                        }
                    },
                    "data-testid": "settings-scroll-snapping",
                }
                div {
                    class: "field",
                    div {
                        class: "is-flex is-justify-content-space-between is-align-items-baseline",
                        label { class: "label mb-0", "Volume" }
                        span { "data-testid": "settings-volume", "{settings.beep_volume()} %" }
                    }
                    p { class: "help", "Timer and metronome beeps" }
                    div {
                        class: "control",
                        input {
                            class: "slider is-fullwidth is-info my-2",
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
                }
                if let Some(permission) = notification_permission {
                    div {
                        SwitchField {
                            label: "Notifications",
                            help: "Show the progress of a training session",
                            is_on: permission == web_sys::NotificationPermission::Granted
                                && settings.notifications(),
                            is_disabled: permission == web_sys::NotificationPermission::Denied,
                            on_change: move |enabled| {
                                let mut settings = settings;
                                async move {
                                    match permission {
                                        web_sys::NotificationPermission::Granted => {
                                            settings.set_notifications(enabled);
                                            settings.save().await;
                                        }
                                        web_sys::NotificationPermission::Denied => {}
                                        _ if enabled => {
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
                                        _ => {}
                                    }
                                }
                            },
                            "data-testid": "settings-notifications",
                        }
                        if permission == web_sys::NotificationPermission::Denied {
                            div {
                                class: "mt-2",
                                Message {
                                    color: Color::Warning,
                                    p {
                                        class: "is-size-7",
                                        "To enable notifications, open the site settings from the address bar and allow notifications. If the app is installed and no address bar is visible, open it in your browser instead. Notifications are blocked in incognito or private browsing mode."
                                    }
                                }
                            }
                        }
                    }
                } else {
                    SwitchField {
                        label: "Notifications",
                        help: "Not supported by this browser",
                        is_on: false,
                        is_disabled: true,
                        on_change: move |_| {},
                        "data-testid": "settings-notifications-unsupported",
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        attribute_of, contains, provide_settings, render, selected_text_of, text_of,
    };

    use super::*;

    fn render_settings(settings: web_app::Settings) -> String {
        render(move || {
            provide_settings(settings);
            rsx! { SettingsDialog { on_close: move |_| {} } }
        })
    }

    #[test]
    fn test_every_switch_shows_the_stored_setting() {
        let html = render_settings(web_app::Settings {
            show_rpe: true,
            show_tut: false,
            scroll_snapping: true,
            ..web_app::Settings::default()
        });

        assert_eq!(attribute_of(&html, "settings-rpe", "aria-checked"), "true");
        assert_eq!(attribute_of(&html, "settings-tut", "aria-checked"), "false");
        assert_eq!(
            attribute_of(&html, "settings-scroll-snapping", "aria-checked"),
            "true"
        );
    }

    #[test]
    fn test_the_chosen_theme_is_marked_as_selected() {
        let html = render_settings(web_app::Settings {
            theme: web_app::Theme::Dark,
            ..web_app::Settings::default()
        });

        assert_eq!(selected_text_of(&html, "settings-theme"), "Dark");
    }

    #[test]
    fn test_the_volume_is_shown_as_a_percentage() {
        let html = render_settings(web_app::Settings {
            beep_volume: 60,
            ..web_app::Settings::default()
        });

        assert_eq!(text_of(&html, "settings-volume"), "60 %");
    }

    #[test]
    fn test_without_notification_support_the_switch_is_disabled() {
        let html = render_settings(web_app::Settings::default());

        assert!(contains(&html, "settings-notifications-unsupported"));
        assert!(!contains(&html, "settings-notifications"));
        assert_eq!(
            attribute_of(&html, "settings-notifications-unsupported", "disabled"),
            "true"
        );
    }
}
