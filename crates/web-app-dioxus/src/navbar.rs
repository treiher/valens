//! Navigation bar.

use dioxus::prelude::*;

use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::warn;

use valens_domain::SessionService;

use crate::{
    DOMAIN_SERVICE, ERRORS, METRONOME, NO_CONNECTION, Route,
    page::common::{Metronome, MutableTimer, Stopwatch, StopwatchService, TimerService},
    session::Session,
    settings::{Settings, SettingsDialog},
    synchronization::Synchronization,
    ui::element::{Dialog, ElementWithDescription, Icon},
};

#[component]
pub fn Navbar() -> Element {
    use_effect(|| {
        let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        else {
            warn!("failed to access document body");
            return;
        };
        if let Err(e) = body
            .class_list()
            .add_2("has-navbar-fixed-top", "has-navbar-fixed-bottom")
        {
            warn!("failed to add body classes: {e:?}");
        }
    });
    use_drop(|| {
        let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        else {
            warn!("failed to access document body");
            return;
        };
        if let Err(e) = body
            .class_list()
            .remove_2("has-navbar-fixed-top", "has-navbar-fixed-bottom")
        {
            warn!("failed to remove body classes: {e:?}");
        }
    });

    let mut menu_visible = use_signal(|| false);
    let mut settings_visible = use_signal(|| false);
    let mut metronome_time_stopwatch_visible = use_signal(|| false);
    let session = consume_context::<Session>();
    let settings = use_context::<Settings>();

    let mut stopwatch = use_signal(StopwatchService::new);
    let mut timer = use_signal(|| TimerService::new(60));
    use_effect(move || {
        METRONOME.write().set_beep_volume(settings.beep_volume());
        timer.write().set_beep_volume(settings.beep_volume());
    });
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut interval = IntervalStream::new(100);
        while interval.next().await.is_some() {
            METRONOME.write().update();
            stopwatch.write().update();
            timer.write().update();
        }
    });

    let user = session.user;
    let route = use_route::<Route>();
    let page_title = match route.clone() {
        Route::Login {} => "Valens".to_string(),
        Route::Home {} => user.name.to_string(),
        Route::Admin {} => "Administration".to_string(),
        Route::TrainingSessions { .. } => "Training sessions".to_string(),
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
        Route::Login {} | Route::Home {} => None,
        Route::Admin {}
        | Route::TrainingSessions { .. }
        | Route::Routines { .. }
        | Route::Exercises { .. }
        | Route::Muscles { .. }
        | Route::BodyWeight { .. }
        | Route::BodyFat { .. }
        | Route::MenstrualCycle { .. }
        | Route::NotFound { .. } => Some(Route::Home {}),
        Route::TrainingSession { .. } => Some(Route::TrainingSessions { add: false }),
        Route::Routine { .. } => Some(Route::Routines {
            add: false,
            search: String::new(),
        }),
        Route::Exercise { .. } | Route::Catalog { .. } => Some(Route::Exercises {
            add: false,
            filter: String::new(),
        }),
    };

    let mut synchronization = consume_context::<Synchronization>();

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
                                    navigator().push(go_up_target.clone());
                                }
                            }
                        },
                        Icon {
                            name: "chevron-left",
                        }
                    }
                    div { class: "navbar-item is-size-5", "data-testid": "page-title", "{page_title}" }
                    div { class: "mx-auto" }
                    if synchronization.in_progress() {
                        a {
                            class: "navbar-item is-size-5 mx-1",
                            "data-testid": "navbar-sync-indicator",
                            ElementWithDescription {
                                description: "Synchronization in progress",
                                right_aligned: true,
                                Icon { name: "rotate fa-pulse" }
                            }
                        }
                    }
                    if synchronization.has_error() {
                        a {
                            class: "navbar-item is-size-5 mx-1",
                            ElementWithDescription {
                                description: synchronization.error(),
                                right_aligned: true,
                                Icon { name: "circle-xmark" }
                            }
                        }
                    }
                    if NO_CONNECTION() {
                        a {
                            class: "navbar-item is-size-5 mx-1",
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
                        onclick: move |_| { menu_visible.toggle() },
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
                            onclick: move |_| {
                                metronome_time_stopwatch_visible.set(true);
                                menu_visible.set(false);
                            },
                            Icon { name: "stopwatch", px: 5 }
                            "Metronome · Stopwatch · Timer"
                        }
                        a {
                            class: "navbar-item",
                            onclick: move |_| {
                                settings_visible.set(true);
                                menu_visible.set(false);
                            },
                            Icon { name: "gear", px: 5 }
                            "Settings"
                        }
                        a {
                            class: "navbar-item",
                            onclick: move |_| {
                                synchronization.sync();
                                menu_visible.set(false);
                            },
                            Icon { name: "rotate", px: 5 }
                            "Refresh data"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-logout",
                            onclick: move |_| async move {
                                let result = DOMAIN_SERVICE().delete_session().await;
                                match result {
                                    Ok(()) => {
                                        navigator().push(Route::Login {});
                                    }
                                    Err(err) => {
                                        ERRORS
                                            .write()
                                            .push(format!("Failed to sign out: {err}"));
                                        }
                                }
                                menu_visible.set(false);
                            },
                            Icon { name: "sign-out-alt", px: 5 }
                            "Sign out ({user.name})"
                        }
                        a {
                            class: "navbar-item",
                            onclick: move |_| {
                                menu_visible.set(false);
                                navigator().push(Route::Admin {});
                            },
                            Icon { name: "gears", px: 5 }
                            "Administration"
                        }
                    }
                }
            }
        }

        if metronome_time_stopwatch_visible() {
            MetronomeTimerStopwatch {
                stopwatch,
                timer,
                on_close: move |_| { metronome_time_stopwatch_visible.set(false); }
            }
        }

        if settings_visible() {
            SettingsDialog {
                on_close: move |_| { settings_visible.set(false); }
            }
        }

        div {
            class: "container is-max-desktop py-4",
            Outlet::<Route> {}
        }
    }
}

#[component]
fn MetronomeTimerStopwatch(
    stopwatch: Signal<StopwatchService>,
    timer: Signal<TimerService>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        Dialog {
            on_close,
            div { class: "block",
                label { class: "subtitle", "Metronome" }
                div {
                    class: "container has-text-centered p-4",
                    Metronome {}
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
