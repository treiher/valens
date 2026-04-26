//! Navigation bar.

use dioxus::prelude::*;

use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::warn;

use valens_domain as domain;
use valens_domain::SessionService;
use valens_web_app as web_app;
use valens_web_app::SettingsService;

use crate::{
    DOMAIN_SERVICE, METRONOME, NO_CONNECTION, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    cache::Cache,
    page::common::{Metronome, MutableTimer, Stopwatch, StopwatchService, TimerService},
    synchronization::Synchronization,
    ui::element::{Color, Dialog, ElementWithDescription, ErrorMessage, Icon, Loading},
};

#[component]
pub fn Navbar() -> Element {
    let mut menu_visible = use_signal(|| false);
    let mut settings_visible = use_signal(|| false);
    let mut metronome_time_stopwatch_visible = use_signal(|| false);
    let mut session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
    let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
    let navigator = use_navigator();

    let mut stopwatch = use_signal(StopwatchService::new);
    let mut timer = use_signal(|| TimerService::new(60));
    use_effect(move || {
        if let Some(Ok(settings)) = settings.read().as_ref() {
            METRONOME.write().set_beep_volume(settings.beep_volume);
            timer.write().set_beep_volume(settings.beep_volume);
        }
    });
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(100);
        while interval.next().await.is_some() {
            METRONOME.write().update();
            stopwatch.write().update();
            timer.write().update();
        }
    });

    let user = match *session.read() {
        Some(Ok(ref user)) => Some(user.clone()),
        Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
            *NO_CONNECTION.write() = true;
            None
        }
        Some(Err(_)) | None => None,
    };
    let route = use_route::<Route>();
    let page_title = match route.clone() {
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
    let go_up_target = match route {
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
                    div {
                        class: "navbar-item is-clickable is-size-5",
                        class: if go_up_target.is_none() { "has-text-primary" },
                        "data-testid": "navbar-back",
                        onclick: {
                            let go_up_target = go_up_target.clone();
                            move |_| {
                                if let Some(go_up_target) = &go_up_target {
                                    navigator.push(go_up_target.clone());
                                }
                            }
                        },
                        Icon {
                            name: "chevron-left",
                        }
                    }
                    div { class: "navbar-item is-size-5", "data-testid": "page-title", "{page_title}" }
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
                                        *metronome_time_stopwatch_visible.write() = true;
                                        *menu_visible.write() = false;
                                    }
                                }
                            },
                            Icon { name: "stopwatch", px: 5 }
                            "Metronome · Stopwatch · Timer"
                        }
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
                                        Synchronization::sync();
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
                                "data-testid": "navbar-logout",
                                onclick: {
                                    move |_| {
                                        async move {
                                            let result = DOMAIN_SERVICE().delete_session().await;
                                            match result {
                                                Ok(()) => {
                                                    consume_context::<Cache>().clear();
                                                    session.restart();
                                                    navigator.push(Route::Root {});
                                                }
                                                Err(err) => {
                                                    NOTIFICATIONS
                                                        .write()
                                                        .push(format!("Failed to log out: {err}"));
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

        if *metronome_time_stopwatch_visible.read() {
            MetronomeTimerStopwatch {
                stopwatch,
                timer,
                close_event: {
                    move |_| {
                        async move {
                            *metronome_time_stopwatch_visible.write() = false;
                        }
                    }
                }
            }
        }

        if *settings_visible.read() {
            Settings {
                settings,
                close_event: {
                    move |_| {
                        async move {
                            *settings_visible.write() = false;
                        }
                    }
                }
            }
        }

        Outlet::<Route> {}
    }
}

#[component]
fn Settings(
    settings: Resource<Result<web_app::Settings, String>>,
    close_event: EventHandler<MouseEvent>,
) -> Element {
    match settings.read().clone() {
        Some(Ok(settings)) => {
            let notification_permission = web_sys::Notification::permission();
            let notifications_color = match notification_permission {
                web_sys::NotificationPermission::Granted => {
                    if settings.notifications {
                        "is-link"
                    } else {
                        ""
                    }
                }
                web_sys::NotificationPermission::Denied => "is-danger",
                _ => "",
            };
            rsx! {
                Dialog {
                    color: Color::Primary,
                    title: rsx! { "Settings" },
                    close_event,
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
                                            settings.theme.apply();
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
                                            settings.theme.apply();
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
                                            settings.theme.apply();
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
                                async move {
                                    match notification_permission {
                                        web_sys::NotificationPermission::Granted => {
                                            settings.notifications = !settings.notifications;
                                            let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
                                        }
                                        web_sys::NotificationPermission::Denied => {
                                        }
                                        _ => {
                                            match web_app::request_notification_permission().await {
                                                Ok(web_sys::NotificationPermission::Granted) => {
                                                    settings.notifications = true;
                                                }
                                                Ok(_) => {}
                                                Err(err) => {
                                                    warn!("failed to enable notifications: {err}");
                                                    NOTIFICATIONS
                                                        .write()
                                                        .push(format!("Failed to enable notifications: {err}"));
                                                }
                                            }
                                            let _ = WEB_APP_SERVICE.write().set_settings(settings).await;
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
                                    if settings.notifications {
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
        Some(Err(err)) => rsx! {
            ErrorMessage { message: "Failed to get settings: {err}" }
        },
        None => Loading(),
    }
}

#[component]
fn MetronomeTimerStopwatch(
    stopwatch: Signal<StopwatchService>,
    timer: Signal<TimerService>,
    close_event: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        Dialog {
            close_event,
            div { class: "block",
                label { class: "subtitle", "Metronome" }
                div {
                    class: "container has-text-centered p-4",
                    Metronome { }
                }
            }
            div { class: "block",
                label { class: "subtitle", "Stopwatch" }
                div {
                    class: "container has-text-centered p-4",
                    Stopwatch { stopwatch }
                }
            }
            div { class: "block",
                label { class: "subtitle", "Timer" }
                div {
                    class: "container has-text-centered p-4",
                    MutableTimer { timer }
                }
            }
        }
    }
}
