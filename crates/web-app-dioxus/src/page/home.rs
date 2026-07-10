use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService, TrainingSessionService};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    page::training_sessions::start_training_session,
    session::Session,
    ui::element::{Block, Error, Icon, Loading, LoadingDialog, Title},
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn Home() -> Element {
    let user = consume_context::<Session>().user;
    let cache = consume_context::<Cache>();
    let today = chrono::Local::now().date_naive();
    let sex = user.sex;
    let height = user.height;

    let pending_today = use_memo(move || {
        let (
            CacheState::Ready(schedule),
            CacheState::Ready(routines),
            CacheState::Ready(training_sessions),
        ) = (
            &*cache.schedule.read(),
            &*cache.routines.read(),
            &*cache.training_sessions.read(),
        )
        else {
            return vec![];
        };
        schedule
            .pending_routines(today, training_sessions)
            .into_iter()
            .filter_map(|(slot, routine_id)| {
                routines.iter().find(|r| r.id == routine_id).map(|routine| {
                    let rotation_name = if let domain::ScheduleSlot::Rotation(rotation_id) = slot {
                        schedule
                            .rotations()
                            .get(&rotation_id)
                            .map(|rotation| rotation.name.to_string())
                    } else {
                        None
                    };
                    (routine.clone(), rotation_name)
                })
            })
            .collect::<Vec<_>>()
    });

    let latest_ffmi = use_memo(move || {
        if let (Some(height), CacheState::Ready(body_fat), CacheState::Ready(body_weight)) =
            (height, &*cache.body_fat.read(), &*cache.body_weight.read())
        {
            let avg_body_weight = DOMAIN_SERVICE().avg_body_weight(body_weight);
            domain::ffmi(&avg_body_weight, body_fat, sex, height)
                .into_iter()
                .filter(|(date, _)| *date <= today)
                .max_by(|a, b| a.0.cmp(&b.0))
        } else {
            None
        }
    });

    let training_subtitle = match &*cache.training_sessions.read() {
        CacheState::Ready(training_sessions) => {
            let training_stats = DOMAIN_SERVICE().get_training_stats(training_sessions);
            training_stats.load_ratio().map(|load_ratio| {
                let load = String::from(if load_ratio > domain::TrainingStats::LOAD_RATIO_HIGH {
                    "high load"
                } else if load_ratio < domain::TrainingStats::LOAD_RATIO_LOW {
                    "low load"
                } else {
                    "optimal load"
                });
                if let Some(last) = training_sessions.iter().map(|ts| ts.date).max().map(last) {
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

    let ffmi_subtitle = if user.height.is_some() {
        match (&*cache.body_fat.read(), &*cache.body_weight.read()) {
            (CacheState::Ready(_), CacheState::Ready(_)) => latest_ffmi()
                .map(|(date, value)| rsx! { strong { "{value:.1}" } " ({last(date)})" }),
            (
                CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ),
                _,
            )
            | (
                _,
                CacheState::Error(
                    domain::ReadError::NotFound
                    | domain::ReadError::Storage(domain::StorageError::NoConnection),
                ),
            ) => None,
            (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
                Some(rsx! { Error { message: "{err}" } })
            }
            (CacheState::Loading, _) | (_, CacheState::Loading) => Some(rsx! { Loading {} }),
        }
    } else {
        None
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
        {view_today(&pending_today(), today)}
        Block {
            Title { "Training" },
            Tile {
                title: "Training sessions",
                testid: "home-training-sessions",
                target: Route::TrainingSessions { add: false },
                target_add: Some(Route::TrainingSessions { add: true }),
                subtitle: training_subtitle,
            }
            Tile {
                title: "Schedule",
                testid: "home-schedule",
                target: Route::Schedule {},
                target_add: None,
                subtitle: None,
            }
            Tile {
                title: "Routines",
                testid: "home-routines",
                target: Route::Routines { add: false, search: String::new() },
                target_add: Some(Route::Routines { add: true, search: String::new() }),
                subtitle: routines_subtitle,
            }
            Tile {
                title: "Exercises",
                testid: "home-exercises",
                target: Route::Exercises { add: false, filter: String::new() },
                target_add: Some(Route::Exercises { add: true, filter: String::new() }),
                subtitle: exercises_subtitle,
            }
            Tile {
                title: "Muscles",
                testid: "home-muscles",
                target: Route::Muscles {},
                target_add: None,
                subtitle: None,
            }
        }
        Block {
            Title { "Health" },
            Tile {
                title: "Body weight",
                testid: "home-body-weight",
                target: Route::BodyWeight { add: false },
                target_add: Some(Route::BodyWeight { add: true }),
                subtitle: body_weight_subtitle,
            }
            Tile {
                title: "Body fat",
                testid: "home-body-fat",
                target: Route::BodyFat { add: false },
                target_add: Some(Route::BodyFat { add: true }),
                subtitle: body_fat_subtitle,
            }
            if user.height.is_some() {
                Tile {
                    title: "FFMI",
                    testid: "home-ffmi",
                    target: Route::Ffmi {},
                    target_add: None,
                    subtitle: ffmi_subtitle,
                }
            }
            if user.sex == domain::Sex::FEMALE {
                Tile {
                    title: "Menstrual cycle",
                    testid: "home-menstrual-cycle",
                    target: Route::MenstrualCycle { add: false },
                    target_add: Some(Route::MenstrualCycle { add: true }),
                    subtitle: menstrual_cycle_subtitle,
                }
            }
        }
        if IS_LOADING() {
            LoadingDialog {}
        }
    }
}

fn view_today(pending: &[(domain::Routine, Option<String>)], today: chrono::NaiveDate) -> Element {
    if pending.is_empty() {
        return rsx! {};
    }

    rsx! {
        Block {
            Title { "Today" },
            for (routine, rotation_name) in pending.iter().cloned() {
                div {
                    class: "box px-4 py-3 mx-3 my-3",
                    "data-testid": "home-today-entry",
                    div {
                        class: "is-flex is-justify-content-space-between is-align-items-center",
                        div {
                            Link {
                                class: "title is-size-5 has-text-link",
                                to: Route::Routine { id: routine.id },
                                "data-testid": "home-today-routine",
                                "{routine.name}"
                            }
                            if let Some(rotation_name) = rotation_name {
                                p {
                                    class: "is-size-7 has-text-grey",
                                    "data-testid": "home-today-rotation",
                                    "{rotation_name}"
                                }
                            }
                        }
                        a {
                            class: "title is-size-5 has-text-link",
                            "data-testid": "home-today-start",
                            onclick: {
                                let routine = routine.clone();
                                move |_| {
                                    let routine = routine.clone();
                                    spawn(async move {
                                        start_pending_training_session(routine, today).await;
                                    });
                                }
                            },
                            Icon { name: "play-circle" }
                        }
                    }
                }
            }
        }
    }
}

async fn start_pending_training_session(routine: domain::Routine, date: chrono::NaiveDate) {
    if IS_LOADING() {
        return;
    }
    IS_LOADING.with_mut(|is_loading| *is_loading = true);
    start_training_session(Some(&routine), date).await;
    IS_LOADING.with_mut(|is_loading| *is_loading = false);
}

#[component]
fn Tile(
    title: String,
    testid: String,
    target: Route,
    target_add: Option<Route>,
    subtitle: Option<Element>,
) -> Element {
    rsx! {
        div {
            class: "grid mx-3 my-3",
            div {
                class: "cell",
                a {
                    class: "box px-4 py-3",
                    "data-testid": "{testid}",
                    onclick: move |_| { navigator().push(target.clone()); },
                    div {
                        class: "is-flex is-justify-content-space-between",
                        div {
                            a { class: "title is-size-5 has-text-link", {title} }
                        }
                        if let Some(target_add) = target_add {
                            div {
                                a {
                                    class: "title is-size-5 has-text-link",
                                    "data-testid": "{testid}-add",
                                    onclick: move |event| { navigator().push(target_add.clone()); event.stop_propagation(); },
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
