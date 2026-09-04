use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService, TrainingSessionService};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    current_date::current_date,
    loading::LoadingFlag,
    page::training_sessions::start_training_session,
    session::Session,
    ui::element::{Block, Error, Icon, Loading, LoadingDialog, Title},
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn Home() -> Element {
    let cache = consume_context::<Cache>();
    let today = current_date();
    let session = consume_context::<Session>();
    let sex = use_memo(move || session.user().sex);
    let height = use_memo(move || session.user().height);

    let pending_today = use_memo(move || {
        let today = current_date();
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
        let today = current_date();
        if let (Some(height), CacheState::Ready(body_fat), CacheState::Ready(body_weight)) = (
            height(),
            &*cache.body_fat.read(),
            &*cache.body_weight.read(),
        ) {
            let avg_body_weight = DOMAIN_SERVICE().avg_body_weight(body_weight);
            domain::ffmi(&avg_body_weight, body_fat, sex(), height)
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
                if let Some(last) = training_sessions
                    .iter()
                    .map(|ts| ts.date)
                    .max()
                    .map(|date| last(date, today))
                {
                    rsx! { strong { {load} } " (last {last})" }
                } else {
                    rsx! { strong { {load} } }
                }
            })
        }
        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
        CacheState::Loading => Some(rsx! { Loading {} }),
    };

    let routines_subtitle = match &*cache.routines.read() {
        CacheState::Ready(_) => None,
        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
        CacheState::Loading => Some(rsx! { Loading {} }),
    };

    let exercises_subtitle = match &*cache.exercises.read() {
        CacheState::Ready(_) => None,
        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
        CacheState::Loading => Some(rsx! { Loading {} }),
    };

    let body_weight_subtitle = match &*cache.body_weight.read() {
        CacheState::Ready(body_weight) => body_weight
            .iter()
            .filter(|bw| bw.date <= today)
            .max_by(|a, b| a.date.cmp(&b.date))
            .map(|bw| rsx! { strong { "{bw.weight:.1} kg" } " ({last(bw.date, today)})" }),
        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
        CacheState::Loading => Some(rsx! { Loading {} }),
    };

    let body_fat_subtitle = match &*cache.body_fat.read() {
        CacheState::Ready(body_fat) => body_fat
            .iter()
            .filter(|bf| bf.date <= today)
            .max_by(|a, b| a.date.cmp(&b.date))
            .and_then(|bf| {
                bf.jp3(sex())
                    .map(|jp3| rsx! { strong { "{jp3:.1} %" } " ({last(bf.date, today)})" })
            }),
        CacheState::Error(err) => Some(rsx! { Error { message: "{err}" } }),
        CacheState::Loading => Some(rsx! { Loading {} }),
    };

    let ffmi_subtitle = if height().is_some() {
        match (&*cache.body_fat.read(), &*cache.body_weight.read()) {
            (CacheState::Ready(_), CacheState::Ready(_)) => latest_ffmi()
                .map(|(date, value)| rsx! { strong { "{value:.1}" } " ({last(date, today)})" }),
            (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
                Some(rsx! { Error { message: "{err}" } })
            }
            (CacheState::Loading, _) | (_, CacheState::Loading) => Some(rsx! { Loading {} }),
        }
    } else {
        None
    };

    let menstrual_cycle_subtitle = {
        if sex() == domain::Sex::FEMALE {
            match &*cache.period.read() {
                CacheState::Ready(period) => domain::current_cycle(&domain::cycles(period)).map(|current_cycle| rsx! {
                    strong { "{current_cycle.time_left.num_days()} (±{current_cycle.time_left_variation.num_days()}) days left" } " (day {(today - current_cycle.begin).num_days()})"
                }),
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
            Tile {
                title: "FFMI",
                testid: "home-ffmi",
                target: Route::Ffmi {},
                target_add: None,
                subtitle: if height().is_some() {
                    ffmi_subtitle
                } else {
                    Some(rsx! { "Set your height in the profile." })
                },
                disabled: height().is_none(),
            }
            if sex() == domain::Sex::FEMALE {
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
    let _loading = LoadingFlag::set(&IS_LOADING);
    start_training_session(Some(&routine), date).await;
}

#[component]
fn Tile(
    title: String,
    testid: String,
    target: Route,
    target_add: Option<Route>,
    subtitle: Option<Element>,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        div {
            class: "grid mx-3 my-3",
            div {
                class: "cell",
                a {
                    class: "box px-4 py-3",
                    "data-testid": "{testid}",
                    onclick: move |_| { if !disabled { navigator().push(target.clone()); } },
                    div {
                        class: "is-flex is-justify-content-space-between",
                        div {
                            a {
                                class: "title is-size-5",
                                class: if disabled { "has-text-grey-light" } else { "has-text-link" },
                                {title}
                            }
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
                        p {
                            "data-testid": "{testid}-subtitle",
                            class: if disabled { "has-text-grey" },
                            {subtitle}
                        }
                    }
                }
            }
        }
    }
}

fn last(date: chrono::NaiveDate, today: chrono::NaiveDate) -> String {
    let days = (today - date).num_days();

    if days == 0 {
        return "today".to_string();
    }

    if days == 1 {
        return "yesterday".to_string();
    }

    format!("{days} days ago")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    use crate::test_render::{TestCache, contains, provide_session, render, text_of};

    fn user(sex: domain::Sex, height: Option<u8>) -> domain::User {
        domain::User {
            id: 1.into(),
            name: domain::Name::new("Alice").unwrap(),
            sex,
            height,
            role: domain::Role::USER,
        }
    }

    fn render_home(user: domain::User, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_session(user.clone());
            rsx! { Home {} }
        })
    }

    #[test]
    fn test_menstrual_cycle_is_shown_for_a_female_user_only() {
        assert!(contains(
            &render_home(user(domain::Sex::FEMALE, None), TestCache::default),
            "home-menstrual-cycle"
        ));
        assert!(!contains(
            &render_home(user(domain::Sex::MALE, None), TestCache::default),
            "home-menstrual-cycle"
        ));
    }

    #[test]
    fn test_ffmi_without_a_height_asks_for_one() {
        let html = render_home(user(domain::Sex::FEMALE, None), TestCache::default);

        assert_eq!(
            text_of(&html, "home-ffmi-subtitle"),
            "Set your height in the profile."
        );
    }

    #[test]
    fn test_body_weight_shows_the_latest_entry() {
        let today = chrono::Local::now().date_naive();
        let html = render_home(user(domain::Sex::FEMALE, None), move || {
            TestCache::default().with_body_weight(vec![
                domain::BodyWeight {
                    date: today - chrono::Duration::days(1),
                    weight: 70.0,
                },
                domain::BodyWeight {
                    date: today,
                    weight: 71.5,
                },
            ])
        });

        assert_eq!(
            text_of(&html, "home-body-weight-subtitle"),
            "71.5 kg (today)"
        );
    }

    #[test]
    fn test_unread_data_is_shown_as_loading() {
        let html = render_home(user(domain::Sex::FEMALE, None), TestCache::loading);

        assert!(contains(&html, "loading"));
    }

    #[test]
    fn test_unreadable_data_is_reported_per_tile() {
        let html = render_home(user(domain::Sex::FEMALE, None), TestCache::failing);

        assert_eq!(text_of(&html, "home-body-weight-subtitle"), "No connection");
    }

    #[test]
    fn test_pending_routines_of_today_are_shown() {
        let today = chrono::Local::now().date_naive();
        let routine = domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        };
        let schedule = domain::Schedule::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::from([(
                domain::Weekday::from(chrono::Datelike::weekday(&today)),
                vec![domain::ScheduleSlot::Routine(1.into())],
            )]),
        )
        .unwrap();
        let html = render_home(user(domain::Sex::FEMALE, None), move || {
            TestCache::default()
                .with_routines(vec![routine.clone()])
                .with_schedule(schedule.clone())
        });

        assert_eq!(text_of(&html, "home-today-routine"), "A");
    }

    #[test]
    fn test_without_a_pending_routine_today_is_not_shown() {
        let html = render_home(user(domain::Sex::FEMALE, None), TestCache::default);

        assert!(!contains(&html, "home-today-entry"));
    }

    #[rstest]
    #[case(0, "today")]
    #[case(1, "yesterday")]
    #[case(7, "7 days ago")]
    #[case(-1, "-1 days ago")]
    fn test_last(#[case] days_ago: i64, #[case] expected: &str) {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        assert_eq!(
            last(today - chrono::Duration::days(days_ago), today),
            expected
        );
    }
}
