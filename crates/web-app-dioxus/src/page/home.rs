use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService, TrainingSessionService};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    current_date::current_date,
    ongoing_training_session::OngoingTrainingSession,
    page::training_sessions::{
        IS_STARTING_TRAINING_SESSION, start_training_session_once, stretch_only_exercises,
    },
    routing::{RoutineTab, TrainingSessionsTab},
    session::Session,
    ui::element::{Block, Error, Icon, Loading, LoadingDialog, Title},
};

#[component]
pub fn Home() -> Element {
    let cache = consume_context::<Cache>();
    let today = current_date();
    let session = consume_context::<Session>();
    let ongoing = consume_context::<OngoingTrainingSession>();
    let sex = use_memo(move || session.user().sex);
    let height = use_memo(move || session.user().height);

    let today_entries = use_memo(move || {
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
            .day_entries(today, training_sessions)
            .into_iter()
            .filter_map(|entry| today_entry(&entry, schedule, routines, training_sessions))
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

    let training_subtitle = match (
        &*cache.training_sessions.read(),
        cache
            .exercises
            .read()
            .loaded()
            .map(|exercises| stretch_only_exercises(exercises.map(Vec::as_slice))),
    ) {
        (CacheState::Ready(training_sessions), Some(stretch_only)) => {
            let training_stats =
                DOMAIN_SERVICE().get_training_stats(training_sessions, &stretch_only);
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
        (CacheState::Error(err), _) => Some(rsx! { Error { message: "{err}" } }),
        (CacheState::Loading, _) | (_, None) => Some(rsx! { Loading {} }),
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
        {view_today(&today_entries(), today, ongoing)}
        Block {
            Title { "Training" },
            Tile {
                title: "Training sessions",
                testid: "home-training-sessions",
                target: Route::TrainingSessions { tab: TrainingSessionsTab::Sessions, muscle: None, add: false },
                target_add: Some(Route::TrainingSessions { tab: TrainingSessionsTab::Sessions, muscle: None, add: true }),
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
        if IS_STARTING_TRAINING_SESSION() {
            LoadingDialog {}
        }
    }
}

#[derive(Clone, PartialEq)]
struct TodayEntry {
    name: String,
    /// The routine the name links to
    target: Option<Route>,
    rotation_name: Option<String>,
    kind: TodayEntryKind,
}

#[derive(Clone, PartialEq)]
enum TodayEntryKind {
    Planned(domain::Routine),
    Session {
        id: domain::TrainingSessionID,
        recorded_sets: usize,
        sets: usize,
        /// `None` once every set is recorded
        first_unrecorded_set: Option<usize>,
    },
}

/// Returns the entry to show for `entry`, or `None` for a planned routine missing in `routines`.
fn today_entry(
    entry: &domain::DayEntry,
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
    training_sessions: &[domain::TrainingSession],
) -> Option<TodayEntry> {
    let rotation_name = |slot: Option<domain::ScheduleSlot>| {
        if let Some(domain::ScheduleSlot::Rotation(rotation_id)) = slot {
            schedule
                .rotations()
                .get(&rotation_id)
                .map(|rotation| rotation.name.to_string())
        } else {
            None
        }
    };
    let routine = |routine_id| routines.iter().find(|r| r.id == routine_id);
    let routine_route = |routine: &domain::Routine| Route::Routine {
        id: routine.id,
        tab: RoutineTab::Routine,
    };
    match *entry {
        domain::DayEntry::Planned { slot, routine_id } => {
            let routine = routine(routine_id)?;
            Some(TodayEntry {
                name: routine.name.to_string(),
                target: Some(routine_route(routine)),
                rotation_name: rotation_name(Some(slot)),
                kind: TodayEntryKind::Planned(routine.clone()),
            })
        }
        domain::DayEntry::Session {
            training_session_id,
            routine_id,
            slot,
            recorded_sets,
            sets,
        } => Some(TodayEntry {
            name: routine(routine_id).map_or_else(
                || "Training session".to_string(),
                |routine| routine.name.to_string(),
            ),
            target: routine(routine_id).map(routine_route),
            rotation_name: rotation_name(slot),
            kind: TodayEntryKind::Session {
                id: training_session_id,
                recorded_sets,
                sets,
                first_unrecorded_set: training_sessions
                    .iter()
                    .find(|t| t.id == training_session_id)
                    .and_then(domain::TrainingSession::first_unrecorded_set),
            },
        }),
    }
}

fn view_today(
    entries: &[TodayEntry],
    today: chrono::NaiveDate,
    ongoing: OngoingTrainingSession,
) -> Element {
    if entries.is_empty() {
        return rsx! {};
    }

    rsx! {
        Block {
            Title { "Today" },
            for entry in entries.iter().cloned() {
                div {
                    class: "box px-4 py-3 mx-3 my-3",
                    "data-testid": "home-today-entry",
                    div {
                        class: "is-flex is-justify-content-space-between is-align-items-center",
                        div {
                            if let Some(target) = entry.target.clone() {
                                Link {
                                    class: "title is-size-5 has-text-link",
                                    to: target,
                                    "data-testid": "home-today-name",
                                    "{entry.name}"
                                }
                            } else {
                                span {
                                    class: "title is-size-5",
                                    "data-testid": "home-today-name",
                                    "{entry.name}"
                                }
                            }
                            if let Some(rotation_name) = &entry.rotation_name {
                                p {
                                    class: "is-size-7 has-text-weak",
                                    "data-testid": "home-today-rotation",
                                    "{rotation_name}"
                                }
                            }
                        }
                        {view_today_action(&entry.kind, today, ongoing)}
                    }
                    if let TodayEntryKind::Session { recorded_sets, sets, first_unrecorded_set, .. } = entry.kind {
                        progress {
                            class: "progress is-thin is-link mt-2 mb-0",
                            value: "{recorded_sets}",
                            max: "{sets}",
                            aria_label: format!("{recorded_sets} of {sets} {}", if sets == 1 { "set" } else { "sets" }),
                            "data-testid": "home-today-progress",
                            "data-state": if first_unrecorded_set.is_some() { "in-progress" } else { "completed" },
                        }
                    }
                }
            }
        }
    }
}

fn view_today_action(
    kind: &TodayEntryKind,
    today: chrono::NaiveDate,
    ongoing: OngoingTrainingSession,
) -> Element {
    match kind.clone() {
        TodayEntryKind::Planned(routine) => rsx! {
            a {
                class: "title is-size-5 has-text-link",
                "data-testid": "home-today-start",
                onclick: move |_| {
                    let routine = routine.clone();
                    spawn(async move {
                        start_training_session_once(&routine, today).await;
                    });
                },
                Icon { name: "play" }
            }
        },
        // Until the ongoing state is loaded, it is unknown whether the session can be continued.
        TodayEntryKind::Session {
            first_unrecorded_set: Some(_),
            ..
        } if !ongoing.is_loaded() => rsx! {
            span {
                class: "title is-size-5 is-invisible",
                Icon { name: "play" }
            }
        },
        TodayEntryKind::Session {
            id,
            first_unrecorded_set: Some(_),
            ..
        } if ongoing.in_progress_other_than(id.as_u128()) => rsx! {
            Link {
                class: "title is-size-5 has-text-link",
                to: Route::TrainingSession { id },
                "data-testid": "home-today-view",
                Icon { name: "eye" }
            }
        },
        TodayEntryKind::Session {
            id,
            first_unrecorded_set: Some(element_idx),
            ..
        } => rsx! {
            a {
                class: "title is-size-5 has-text-link",
                "data-testid": "home-today-continue",
                onclick: move |_| {
                    ongoing.start_if_none(id.as_u128(), element_idx);
                    navigator().push(Route::TrainingSession { id });
                },
                Icon { name: "play" }
            }
        },
        TodayEntryKind::Session {
            id,
            first_unrecorded_set: None,
            ..
        } => rsx! {
            Link {
                class: "title is-size-5 has-text-link",
                to: Route::TrainingSession { id },
                "data-testid": "home-today-completed",
                Icon { name: "check" }
            }
        },
    }
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
                                class: if disabled { "has-text-weak" } else { "has-text-link" },
                                {title}
                            }
                        }
                        if let Some(target_add) = target_add {
                            div {
                                a {
                                    class: "title is-size-5 has-text-link",
                                    "data-testid": "{testid}-add",
                                    onclick: move |event| { navigator().push(target_add.clone()); event.stop_propagation(); },
                                    Icon { name: "plus" }
                                }
                            }
                        }
                    }
                    if let Some(ref subtitle) = subtitle {
                        p {
                            "data-testid": "{testid}-subtitle",
                            class: if disabled { "has-text-weak" },
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

    use crate::{
        ongoing_training_session::State,
        test_render::{
            TestCache, attribute_of, contains, links_of, provide_ongoing_training_session,
            provide_session, render, text_of,
        },
    };

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
        render_home_with_ongoing(user, cache, || State::None)
    }

    fn render_home_with_ongoing(
        user: domain::User,
        cache: impl Fn() -> TestCache + 'static,
        ongoing: fn() -> State,
    ) -> String {
        render(move || {
            cache().provide();
            provide_session(user.clone());
            provide_ongoing_training_session(ongoing());
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

    /// Sets at an RPE of nine of exercise 1, which works a muscle, from five weeks until a week
    /// ago, and of exercise 2, which only stretches, in the last week.
    fn lifting_followed_by_stretching() -> TestCache {
        let today = chrono::Local::now().date_naive();
        let exercise = |id: u128, stimulus, stretch| domain::Exercise {
            id: id.into(),
            name: domain::Name::new(&format!("Exercise {id}")).unwrap(),
            notes: String::new(),
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Hamstrings,
                stimulus,
                stretch,
            }],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        };
        let session = |days_ago: i64, exercise_id: u128| domain::TrainingSession {
            id: u128::try_from(days_ago).unwrap().into(),
            routine_id: domain::RoutineID::nil(),
            date: today - chrono::Duration::days(days_ago),
            notes: String::new(),
            elements: vec![domain::TrainingSessionElement::Set {
                exercise_id: exercise_id.into(),
                side: domain::Side::Unset,
                reps: domain::Reps::new(5).unwrap(),
                time: domain::Time::default(),
                weight: domain::Weight::default(),
                rpe: domain::RPE::new(9.0).unwrap(),
                target_reps: domain::Reps::default(),
                target_tempo: domain::Tempo::default(),
                target_weight: domain::Weight::default(),
                target_rpe: domain::RPE::ZERO,
                automatic: false,
                countdown: false,
            }],
            exercise_notes: std::collections::BTreeMap::new(),
            exercise_measurements: std::collections::BTreeMap::new(),
        };
        TestCache::default()
            .with_exercises(vec![
                exercise(1, domain::Stimulus::PRIMARY, domain::Stimulus::NONE),
                exercise(2, domain::Stimulus::NONE, domain::Stimulus::PRIMARY),
            ])
            .with_training_sessions(
                (0..35)
                    .map(|days_ago| session(days_ago, if days_ago < 7 { 2 } else { 1 }))
                    .collect(),
            )
    }

    #[test]
    fn test_the_load_counts_a_stretch_as_one_set_whatever_its_rpe() {
        let html = render_home(
            user(domain::Sex::FEMALE, None),
            lifting_followed_by_stretching,
        );

        assert_eq!(
            text_of(&html, "home-training-sessions-subtitle"),
            "low load (last today)"
        );
    }

    #[test]
    fn test_the_load_waits_for_the_exercises() {
        let html = render_home(user(domain::Sex::FEMALE, None), || {
            lifting_followed_by_stretching().with_unread_exercises()
        });

        assert!(!text_of(&html, "home-training-sessions-subtitle").contains("load"));
        assert!(contains(&html, "loading"));
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

        assert_eq!(text_of(&html, "home-today-name"), "A");
    }

    /// A session of today with `recorded_sets` of two sets recorded, of the routine with ID 1
    fn session_of_today(recorded_sets: u32) -> domain::TrainingSession {
        let set = |reps| domain::TrainingSessionElement::Set {
            exercise_id: 1.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::default(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::default(),
            automatic: false,
            countdown: false,
        };
        domain::TrainingSession {
            id: 1.into(),
            routine_id: 1.into(),
            date: chrono::Local::now().date_naive(),
            notes: String::new(),
            elements: (0..2).map(|i| set(u32::from(i < recorded_sets))).collect(),
            exercise_notes: std::collections::BTreeMap::new(),
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    fn render_home_with_session(
        training_session: domain::TrainingSession,
        routines: Vec<domain::Routine>,
        ongoing: fn() -> State,
    ) -> String {
        render_home_with_ongoing(
            user(domain::Sex::FEMALE, None),
            move || {
                TestCache::default()
                    .with_routines(routines.clone())
                    .with_training_sessions(vec![training_session.clone()])
            },
            ongoing,
        )
    }

    fn routine_a() -> domain::Routine {
        domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn routine_a_route() -> String {
        Route::Routine {
            id: 1.into(),
            tab: RoutineTab::Routine,
        }
        .to_string()
    }

    #[test]
    fn test_an_in_progress_session_shows_its_progress_and_a_continue_button() {
        let html = render_home_with_session(session_of_today(1), vec![routine_a()], || State::None);

        assert_eq!(text_of(&html, "home-today-name"), "A");
        assert_eq!(
            attribute_of(&html, "home-today-progress", "aria-label"),
            "1 of 2 sets"
        );
        assert_eq!(
            attribute_of(&html, "home-today-progress", "data-state"),
            "in-progress"
        );
        assert!(contains(&html, "home-today-continue"));
        assert!(!contains(&html, "home-today-view"));
        assert!(!contains(&html, "home-today-completed"));
        assert_eq!(links_of(&html, "home-today-entry"), [routine_a_route()]);
    }

    #[test]
    fn test_an_in_progress_session_can_only_be_viewed_while_another_is_ongoing() {
        let html = render_home_with_session(session_of_today(1), vec![routine_a()], || {
            State::InProgress(valens_web_app::OngoingTrainingSession::new(2))
        });

        assert!(contains(&html, "home-today-view"));
        assert!(!contains(&html, "home-today-continue"));
    }

    #[test]
    fn test_an_in_progress_session_has_no_action_while_the_ongoing_state_is_loading() {
        let html =
            render_home_with_session(session_of_today(1), vec![routine_a()], || State::Loading);

        assert!(contains(&html, "home-today-progress"));
        assert!(!contains(&html, "home-today-continue"));
        assert!(!contains(&html, "home-today-view"));
    }

    #[test]
    fn test_the_progress_of_a_single_set_is_labeled_in_singular() {
        let training_session = domain::TrainingSession {
            elements: session_of_today(0).elements[..1].to_vec(),
            ..session_of_today(0)
        };
        let html = render_home_with_session(training_session, vec![routine_a()], || State::None);

        assert_eq!(
            attribute_of(&html, "home-today-progress", "aria-label"),
            "0 of 1 set"
        );
    }

    #[test]
    fn test_a_completed_session_shows_a_full_bar_and_the_check_icon() {
        let html = render_home_with_session(session_of_today(2), vec![routine_a()], || State::None);

        assert_eq!(
            attribute_of(&html, "home-today-progress", "aria-label"),
            "2 of 2 sets"
        );
        assert_eq!(
            attribute_of(&html, "home-today-progress", "data-state"),
            "completed"
        );
        assert!(contains(&html, "home-today-completed"));
        assert!(!contains(&html, "home-today-continue"));
        assert!(!contains(&html, "home-today-start"));
        assert_eq!(
            links_of(&html, "home-today-entry"),
            [
                routine_a_route(),
                Route::TrainingSession { id: 1.into() }.to_string()
            ]
        );
    }

    #[test]
    fn test_a_session_without_a_routine_is_labeled_training_session() {
        let training_session = domain::TrainingSession {
            routine_id: domain::RoutineID::nil(),
            ..session_of_today(1)
        };
        let html = render_home_with_session(training_session, vec![routine_a()], || State::None);

        assert_eq!(text_of(&html, "home-today-name"), "Training session");
        assert_eq!(links_of(&html, "home-today-entry"), Vec::<String>::new());
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
