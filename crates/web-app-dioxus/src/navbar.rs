//! Navigation bar.

use dioxus::prelude::*;

use futures_util::StreamExt;
use gloo_timers::future::IntervalStream;
use log::warn;

use valens_domain::{self as domain, SessionService};

use crate::{
    DOMAIN_SERVICE, DROP_SET_CALCULATOR, METRONOME, NO_CONNECTION, ONE_REP_MAX_CALCULATOR, Route,
    notification::notify,
    ongoing_training_session::OngoingTrainingSession,
    page::common::{
        DropSetCalculator, Metronome, MutableTimer, OneRepMaxCalculator, Stopwatch,
        StopwatchService, TimerService,
    },
    session::Session,
    settings::{Settings, SettingsDialog},
    synchronization::Synchronization,
    ui::element::{ActivityBar, Dialog, ElementWithDescription, Icon},
};

#[component]
pub fn Navbar() -> Element {
    let ongoing = consume_context::<OngoingTrainingSession>();
    use_effect(|| {
        set_body_class("has-navbar-fixed-top", true);
        set_body_class("has-navbar-fixed-bottom", true);
    });
    use_effect(move || {
        set_body_class("has-activity-bar", ongoing.get().is_some());
    });
    use_drop(|| {
        set_body_class("has-navbar-fixed-top", false);
        set_body_class("has-navbar-fixed-bottom", false);
        set_body_class("has-activity-bar", false);
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
                        "data-testid": "navbar-menu",
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
                            "data-testid": "navbar-1rm-calculator",
                            onclick: move |_| {
                                ONE_REP_MAX_CALCULATOR.write().visible = true;
                                menu_visible.set(false);
                            },
                            Icon { name: "dumbbell", px: 5 }
                            "1RM calculator"
                        }
                        a {
                            class: "navbar-item",
                            "data-testid": "navbar-drop-set-calculator",
                            onclick: move |_| {
                                DROP_SET_CALCULATOR.write().visible = true;
                                menu_visible.set(false);
                            },
                            Icon { name: "arrow-down-wide-short", px: 5 }
                            "Drop set calculator"
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
                                        ongoing.clear().await;
                                        navigator().push(Route::Login {});
                                    }
                                    Err(err) => {
                                        notify("Failed to sign out", &err);
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

        if ONE_REP_MAX_CALCULATOR.read().visible {
            OneRepMaxCalculator {}
        }

        if DROP_SET_CALCULATOR.read().visible {
            DropSetCalculator {}
        }

        div {
            class: "container is-max-desktop py-4",
            Outlet::<Route> {}
        }

        ActivityBarNavigate { route: route.clone() }
    }
}

fn set_body_class(name: &str, present: bool) {
    let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    else {
        warn!("failed to access document body");
        return;
    };
    let result = if present {
        body.class_list().add_1(name)
    } else {
        body.class_list().remove_1(name)
    };
    if let Err(e) = result {
        warn!("failed to update body class `{name}`: {e:?}");
    }
}

#[component]
fn ActivityBarNavigate(route: Route) -> Element {
    let ongoing = consume_context::<OngoingTrainingSession>();
    let show_on_route = match &route {
        Route::Login {} | Route::NotFound { .. } => false,
        Route::TrainingSession { id } => ongoing.in_progress_other_than(id.as_u128()),
        _ => true,
    };
    if !show_on_route {
        return rsx! {};
    }
    let Some(ongoing) = ongoing.get() else {
        return rsx! {};
    };
    let target_id = domain::TrainingSessionID::from(ongoing.training_session_id);
    rsx! {
        ActivityBar {
            Link {
                "data-testid": "activity-bar",
                class: "is-flex is-align-items-center is-undecorated",
                to: Route::TrainingSession { id: target_id },
                span {
                    class: "icon has-text-info mr-3",
                    i { class: "fas fa-dumbbell" }
                }
                div {
                    class: "is-flex-grow-1 has-text-centered has-text-weight-bold",
                    "Training session in progress"
                }
                span {
                    class: "icon has-text-info ml-3",
                    i { class: "fas fa-chevron-right" }
                }
            }
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
