use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{SessionService, TrainingSessionService};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    ensure_session,
    ui::element::{Block, Error, ErrorMessage, Loading, LoadingPage, Title},
};

#[component]
pub fn Home() -> Element {
    let session = ensure_session!();
    let cache = consume_context::<Cache>();
    let today = chrono::Local::now().date_naive();

    match *session.read() {
        Some(Ok(ref user)) => {
            let training_subtitle = match &*cache.training_sessions.read() {
                CacheState::Ready(training_sessions) => {
                    let training_stats = DOMAIN_SERVICE().get_training_stats(training_sessions);
                    training_stats.load_ratio().map(|load_ratio| {
                        let load =
                            String::from(if load_ratio > domain::TrainingStats::LOAD_RATIO_HIGH {
                                "high load"
                            } else if load_ratio < domain::TrainingStats::LOAD_RATIO_LOW {
                                "low load"
                            } else {
                                "optimal load"
                            });
                        if let Some(last) =
                            training_sessions.iter().map(|ts| ts.date).max().map(last)
                        {
                            rsx! { strong { {load} } " (last {last})" }
                        } else {
                            rsx! { strong { {load} } }
                        }
                    })
                }
                CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ) => None,
                CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                CacheState::Loading => Some(rsx! { Loading {} }),
            };

            let routines_subtitle = match &*cache.routines.read() {
                CacheState::Ready(_)
                | CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ) => None,
                CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                CacheState::Loading => Some(rsx! { Loading {} }),
            };

            let exercises_subtitle = match &*cache.exercises.read() {
                CacheState::Ready(_)
                | CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ) => None,
                CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                CacheState::Loading => Some(rsx! { Loading {} }),
            };

            let body_weight_subtitle = match &*cache.body_weight.read() {
                CacheState::Ready(body_weight) => body_weight
                    .iter()
                    .filter(|bw| bw.date <= today)
                    .max_by(|a, b| a.date.cmp(&b.date))
                    .map(|bw| rsx! { strong { "{bw.weight:.1} kg" } " ({last(bw.date)})" }),
                CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ) => None,
                CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                CacheState::Loading => Some(rsx! { Loading {} }),
            };

            let body_fat_subtitle = match &*cache.body_fat.read() {
                CacheState::Ready(body_fat) => body_fat
                    .iter()
                    .filter(|bf| bf.date <= today)
                    .max_by(|a, b| a.date.cmp(&b.date))
                    .and_then(|bf| {
                        bf.jp3(user.sex)
                            .map(|jp3| rsx! { strong { "{jp3:.1} %" } " ({last(bf.date)})" })
                    }),
                CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ) => None,
                CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                CacheState::Loading => Some(rsx! { Loading {} }),
            };

            let menstrual_cycle_subtitle = {
                if user.sex == domain::Sex::FEMALE {
                    match &*cache.period.read() {
                        CacheState::Ready(period) => domain::current_cycle(&domain::cycles(period)).map(|current_cycle| rsx! {
                            strong { "{current_cycle.time_left.num_days()} (±{current_cycle.time_left_variation.num_days()}) days left" } " (day {(today - current_cycle.begin).num_days()})"
                        }),
                        CacheState::Error(
                            domain::ReadError::NotFound
                            | domain::ReadError::Storage(domain::StorageError::NoConnection),
                        ) => None,
                        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
                        CacheState::Loading => Some(rsx! { Loading {} }),
                    }
                } else {
                    None
                }
            };

            rsx! {
                Block {
                    Title { "Training" },
                    Tile {
                        title: "Training sessions",
                        target: Route::Training { add: false },
                        target_add: Some(Route::Training { add: true }),
                        subtitle: training_subtitle,
                    }
                    Tile {
                        title: "Routines",
                        target: Route::Routines { add: false, search: String::new() },
                        target_add: Some(Route::Routines { add: true, search: String::new() }),
                        subtitle: routines_subtitle,
                    }
                    Tile {
                        title: "Exercises",
                        target: Route::Exercises { add: false, filter: String::new() },
                        target_add: Some(Route::Exercises { add: true, filter: String::new() }),
                        subtitle: exercises_subtitle,
                    }
                    Tile {
                        title: "Muscles",
                        target: Route::Muscles { },
                        target_add: None,
                        subtitle: None,
                    }
                }
                Block {
                    Title { "Health" },
                    Tile {
                        title: "Body weight",
                        target: Route::BodyWeight { add: false },
                        target_add: Some(Route::BodyWeight { add: true }),
                        subtitle: body_weight_subtitle,
                    }
                    Tile {
                        title: "Body fat",
                        target: Route::BodyFat { add: false },
                        target_add: Some(Route::BodyFat { add: true }),
                        subtitle: body_fat_subtitle,
                    }
                    if user.sex == domain::Sex::FEMALE {
                        Tile {
                            title: "Menstrual cycle",
                            target: Route::MenstrualCycle { add: false },
                            target_add: Some(Route::MenstrualCycle { add: true }),
                            subtitle: menstrual_cycle_subtitle,
                        }
                    }
                }
            }
        }
        Some(Err(ref err)) => rsx! {
            ErrorMessage { message: err }
        },
        None => rsx! {
            LoadingPage {}
        },
    }
}

#[component]
fn Tile(
    title: String,
    target: Route,
    #[props(!optional)] target_add: Option<Route>,
    #[props(!optional)] subtitle: Option<Element>,
) -> Element {
    let navigator = use_navigator();

    rsx! {
        div {
            class: "grid mx-3 my-3",
            div {
                class: "cell",
                a {
                    class: "box px-4 py-3",
                    onclick: move |_| { navigator.push(target.clone()); },
                    div {
                        class: "is-flex is-justify-content-space-between",
                        div {
                            a { class: "title is-size-5 has-text-link", {title} }
                        }
                        if let Some(target_add) = target_add {
                            div {
                                a {
                                    class: "title is-size-5 has-text-link",
                                    onclick: move |event| { navigator.push(target_add.clone()); event.stop_propagation(); },
                                    span { class: "icon",
                                        i { class: "fas fa-plus-circle" }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(ref subtitle) = subtitle {
                        p { {subtitle} }
                    }
                }
            }
        }
    }
}

fn last(date: chrono::NaiveDate) -> String {
    let today = chrono::Local::now().date_naive();
    let days = (today - date).num_days();

    if days == 0 {
        return "today".to_string();
    }

    if days == 1 {
        return "yesterday".to_string();
    }

    format!("{days} days ago")
}
