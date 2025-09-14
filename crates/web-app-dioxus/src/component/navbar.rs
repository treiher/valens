use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::SessionService;
use valens_web_app as web_app;
use valens_web_app::SettingsService;

use crate::{
    DOMAIN_SERVICE, NO_CONNECTION, NOTIFICATIONS, Route, SYNC_TRIGGER, WEB_APP_SERVICE,
    component::element::{Color, Dialog, ElementWithDescription, ErrorMessage, Icon},
    signal_changed_data, trigger_sync,
};

use super::element::Loading;

#[component]
pub fn Navbar() -> Element {
    let mut menu_visible = use_signal(|| false);
    let mut settings_visible = use_signal(|| false);
    let mut session = use_resource(|| async { DOMAIN_SERVICE.read().get_session().await });
    use_effect(|| {
        spawn(async {
            let _ = SYNC_TRIGGER.read();
            let _ = DOMAIN_SERVICE.read().sync().await;
            signal_changed_data();
        });
    });
    let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
    let navigator = use_navigator();

    let user = match *session.read() {
        Some(Ok(ref user)) => Some(user.clone()),
        Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
            *NO_CONNECTION.write() = true;
            None
        }
        Some(Err(_)) | None => None,
    };
    let page_title = match use_route::<Route>() {
        Route::Root {} | Route::Login {} => "Valens".to_string(),
        Route::Home {} => {
            if let Some(ref user) = user {
                user.name.to_string()
            } else {
                "Home".to_string()
            }
        }
        Route::Admin {} => "Administration".to_string(),
        Route::Training { .. } => "Training sessions".to_string(),
        Route::TrainingSession { .. } => "Training session".to_string(),
        Route::Routines { .. } => "Routines".to_string(),
        Route::Routine { .. } => "Routine".to_string(),
        Route::Exercises { .. } => "Exercises".to_string(),
        Route::Exercise { .. } => "Exercise".to_string(),
        Route::Catalog { .. } => "Catalog exercise".to_string(),
        Route::Muscles { .. } => "Muscles".to_string(),
        Route::BodyWeight { .. } => "Body weight".to_string(),
        Route::BodyFat { .. } => "Body fat".to_string(),
        Route::MenstrualCycle { .. } => "Menstrual cycle".to_string(),
        Route::NotFound { .. } => String::new(),
    };
    let go_up_target = match use_route::<Route>() {
        Route::Root {} | Route::Login {} | Route::Home {} => None,
        Route::Admin {}
        | Route::Training { .. }
        | Route::Routines { .. }
        | Route::Exercises { .. }
        | Route::Muscles { .. }
        | Route::BodyWeight { .. }
        | Route::BodyFat { .. }
        | Route::MenstrualCycle { .. }
        | Route::NotFound { .. } => Some(Route::Root {}),
        Route::TrainingSession { .. } => Some(Route::Training { add: false }),
        Route::Routine { .. } => Some(Route::Routines {
            add: false,
            search: String::new(),
        }),
        Route::Exercise { .. } | Route::Catalog { .. } => Some(Route::Exercises {
            add: false,
            filter: String::new(),
        }),
    };

    rsx! {
        nav {
            class: "navbar is-fixed-top is-primary has-shadow has-text-weight-bold",
            div {
                class: "container",
                div {
                    class: "navbar-brand is-flex-grow-1",
                    a {
                        class: "navbar-item is-size-5",
                        class: if go_up_target.is_none() { "has-text-primary" },
                        Icon {
                            name: "chevron-left",
                            onclick: {
                                let go_up_target = go_up_target.clone();
                                move |_| {
                                    if let Some(go_up_target) = &go_up_target {
                                        navigator.push(go_up_target.clone());
                                    }
                                }
                            },
                        }
                    }
                    div { class: "navbar-item is-size-5", "{page_title}" }
                    div { class: "mx-auto" }
                    if NO_CONNECTION() {
                        a {
                            class: "navbar-item",
                            class: "is-size-5",
                            class: "mx-1",
                            ElementWithDescription {
                                description: "No connection to server",
                                right_aligned: true,
                                Icon { name: "plug-circle-xmark" }
                            }
                        }
                    }
                    a {
                        aria_expanded: menu_visible(),
                        aria_label: "menu",
                        class: "navbar-burger ml-0",
                        class: if menu_visible() { "is-active" },
                        role: "button",
                        onclick: move |_| { *menu_visible.write() = !menu_visible() },
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                        span { aria_hidden: "true" }
                    }
                }
                div {
                    class: "navbar-menu is-flex-grow-0",
                    class: if menu_visible() { "is-active" },
                    div {
                        class: "navbar-end",
                        a {
                            class: "navbar-item",
                            onclick: {
                                move |_| {
                                    async move {
                                        *settings_visible.write() = true;
                                        *menu_visible.write() = false;
                                    }
                                }
                            },
                            Icon { name: "gear", px: 5 }
                            "Settings"
                        }
                        a {
                            class: "navbar-item",
                            onclick: {
                                move |_| {
                                    async move {
                                        trigger_sync();
                                        *menu_visible.write() = false;
                                    }
                                }
                            },
                            Icon { name: "rotate", px: 5 }
                            if user.is_some() {
                                "Refresh data"
                            } else {
                                "Refresh user data"
                            }
                        }
                        if let Some(user) = user {
                            a {
                                class: "navbar-item",
                                onclick: {
                                    move |_| {
                                        async move {
                                            let result = DOMAIN_SERVICE.read().delete_session().await;
                                            match result {
                                                Ok(()) => {
                                                    session.restart();
                                                    navigator.push(Route::Root {});
                                                }
                                                Err(err) => {
                                                    NOTIFICATIONS
                                                        .write()
                                                        .push(format!("Failed to switch user: {err}"));
                                                }
                                            }
                                            *menu_visible.write() = false;
                                        }
                                    }
                                },
                                Icon { name: "sign-out-alt", px: 5 }
                                "Log out ({user.name})"
                            }
                        }
                        a {
                            class: "navbar-item",
                            onclick: {
                                move |_| {
                                    async move {
                                        *menu_visible.write() = false;
                                        navigator.push(Route::Admin {});
                                    }
                                }
                            },
                            Icon { name: "gears", px: 5 }
                            "Administration"
                        }
                    }
                }
            }
        }

        if *settings_visible.read() {
            Settings { settings, settings_visible }
        }

        Outlet::<Route> {}
    }
}

#[component]
fn Settings(
    settings: Resource<Result<web_app::Settings, String>>,
    settings_visible: Signal<bool>,
) -> Element {
    match settings.read().clone() {
        Some(Ok(settings)) => rsx! {
            Dialog {
                color: Color::Primary,
                title: rsx! { "Settings" },
                close_event: {
                    move |_| {
                        async move {
                            *settings_visible.write() = false;
                        }
                    }
                },
                p {
                    h1 { class: "subtitle", "Beep volume" }
                    input {
                        class: "slider is-fullwidth is-info",
                        max: "100",
                        min: "0",
                        r#type: "range",
                        step: "10",
                        value: settings.beep_volume,
                        oninput: move |event| {
                            let mut settings = settings;
                            settings.beep_volume = event.value().parse().unwrap_or(100);
                            async move {
                                let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
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
                                class: if settings.theme == web_app::Theme::Light { "is-link" },
                                onclick: {
                                    move |_| {
                                        let mut settings = settings;
                                        settings.theme = web_app::Theme::Light;
                                        async move {
                                            let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                                        }
                                    }
                                },
                                Icon { name: "sun", is_small: true }
                                span { "Light" }
                            }
                        }
                        p {
                            class: "control",
                            span {
                                class: "button",
                                class: if settings.theme == web_app::Theme::Dark { "is-link" },
                                onclick: {
                                    move |_| {
                                        let mut settings = settings;
                                        settings.theme = web_app::Theme::Dark;
                                        async move {
                                            let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                                        }
                                    }
                                },
                                Icon { name: "moon", is_small: true }
                                span { "Dark" }
                            }
                        }
                        p { class: "control",
                            span {
                                class: "button",
                                class: if settings.theme == web_app::Theme::System { "is-link" },
                                onclick: {
                                    move |_| {
                                        let mut settings = settings;
                                        settings.theme = web_app::Theme::System;
                                        async move {
                                            let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
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
                            settings.automatic_metronome = !settings.automatic_metronome;
                            async move {
                                let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                            }
                        }
                    },
                    h1 { class: "subtitle", "Metronome" }
                    if settings.automatic_metronome {
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
                            settings.show_rpe = !settings.show_rpe;
                            async move {
                                let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                            }
                        }
                    },
                    h1 { class: "subtitle", "Rating of Perceived Exertion (RPE)" }
                    if settings.show_rpe {
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
                            settings.show_tut = !settings.show_tut;
                            async move {
                                let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                            }
                        }
                    },
                    h1 { class: "subtitle", "Time Under Tension (TUT)" }
                    if settings.show_tut {
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
                            settings.notifications = !settings.notifications;
                            async move {
                                let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                            }
                        }
                    },
                    h1 { class: "subtitle", "Notifications" }
                    if settings.notifications {
                        button { class: "button is-link", "Enabled" }
                    } else {
                        button { class: "button", "Disabled" }
                    }
                }
            }
        },
        Some(Err(err)) => rsx! {
            ErrorMessage { message: "Failed to get settings: {err}" }
        },
        None => Loading(),
    }
}
