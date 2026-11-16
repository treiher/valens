use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    chart::{Calendar, IntervalControl},
    eh,
    loading::LoadingFlag,
    notification::{notify, notify_error},
    page::training_charts,
    routing::{NavigatorScrollExt, RoutineTab, TrainingSessionsTab},
    settings::Settings,
    ui::{
        element::{
            DeleteConfirmationDialog, ErrorPage, FloatingActionButton, Icon, LoadingPage, NoWrap,
            SaveDialog, Table, Tabs, count_to_string,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

pub static IS_STARTING_TRAINING_SESSION: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn TrainingSessions(tab: TrainingSessionsTab, muscle: Option<u8>, add: bool) -> Element {
    let tab = if add {
        TrainingSessionsTab::Sessions
    } else {
        tab
    };
    let cache = consume_context::<Cache>();
    let dates = use_memo(move || {
        if let CacheState::Ready(training_session) = &*cache.training_sessions.read() {
            training_session
                .iter()
                .map(|ts| ts.date)
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    });
    let current_interval =
        use_signal(|| domain::init_interval(&dates.read(), domain::DefaultInterval::_1M));
    let all = *use_memo(move || domain::Interval {
        first: dates.read().iter().min().copied().unwrap_or_default(),
        last: dates.read().iter().max().copied().unwrap_or_default(),
    })
    .read();
    let settings = use_context::<Settings>();
    let mut dialog = use_signal(|| TrainingDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE()
            .validate_training_session_date(&date.input)
            .map_err(|err| err.to_string());
        dialog.set(TrainingDialog::Add {
            date,
            routine_id: FieldValue {
                input: String::new(),
                validated: Ok(domain::RoutineID::nil()),
                orig: String::new(),
            },
        });
        navigator().replace_preserving_scroll(Route::TrainingSessions {
            tab,
            muscle,
            add: true,
        });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match (
        &*cache.training_sessions.read(),
        &*cache.routines.read(),
        cache.exercises.read().loaded(),
    ) {
        (CacheState::Ready(training_sessions), CacheState::Ready(routines), Some(exercises)) => {
            let exercises = exercises.map(Vec::as_slice);
            let stretch_only = stretch_only_exercises(exercises);
            let interval = *current_interval.read();
            let content = match tab {
                TrainingSessionsTab::Sessions => {
                    let shown_sessions = sessions_in(training_sessions, interval);
                    rsx! {
                        {view_calendar(training_sessions, &stretch_only, interval)}
                        {view_list(shown_sessions, &stretch_only, routines, dialog)}
                    }
                }
                TrainingSessionsTab::Charts => training_charts::view_charts(
                    training_sessions,
                    exercises,
                    &stretch_only,
                    muscle.and_then(|muscle| domain::MuscleID::try_from(muscle).ok()),
                    interval,
                    settings,
                ),
            };
            rsx! {
                Tabs {
                    tabs: vec![("Sessions", TrainingSessionsTab::Sessions), ("Charts", TrainingSessionsTab::Charts)],
                    selected: tab,
                    on_select: move |tab| {
                        navigator().replace_preserving_scroll(Route::TrainingSessions { tab, muscle, add: false });
                    },
                    is_page_top: true,
                }
                IntervalControl { current_interval, all }
                {content}
                {view_dialog(dialog, training_sessions, routines, Some(Route::TrainingSessions { tab, muscle, add: false }))}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    on_click: move |_| { show_add_dialog() },
                }
            }
        }
        (CacheState::Error(err), _, _) | (_, CacheState::Error(err), _) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        _ => {
            rsx! { LoadingPage {} }
        }
    }
}

/// The stretch-only exercises.
///
/// If the exercises cannot be read, no exercise is considered stretch-only.
pub fn stretch_only_exercises(
    exercises: Result<&[domain::Exercise], &domain::ReadError>,
) -> domain::StretchOnlyExercises {
    match exercises {
        Ok(exercises) => domain::StretchOnlyExercises::new(exercises),
        Err(_) => domain::StretchOnlyExercises::default(),
    }
}

/// The training sessions of `interval`, from training sessions sorted by date.
fn sessions_in(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
) -> &[domain::TrainingSession] {
    let start = training_sessions.partition_point(|t| t.date < interval.first);
    let end = training_sessions.partition_point(|t| t.date <= interval.last);
    &training_sessions[start..end.max(start)]
}

/// The sessions without the sets of stretch-only exercises, in the same order.
pub fn without_stretch_only_sets<'a>(
    training_sessions: &'a [domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
) -> Vec<Cow<'a, domain::TrainingSession>> {
    training_sessions
        .iter()
        .map(|t| t.without_stretch_only_sets(stretch_only))
        .collect()
}

pub fn view_calendar(
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
) -> Element {
    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            let session_load = training_session.load(stretch_only);
            load.entry(training_session.date)
                .and_modify(|e| *e += session_load)
                .or_insert(session_load);
        }
    }
    let min = load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
    let max = load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
    let entries = load
        .iter()
        .map(|(date, load)| {
            (
                *date,
                web_app::chart::COLOR_LOAD,
                if max > min {
                    (f64::from(load - min) / f64::from(max - min)) * 0.8 + 0.2
                } else {
                    1.0
                },
            )
        })
        .collect();

    rsx! {
        Calendar { entries, interval }
    }
}

/// Show an item per training session, newest first, with a summary of its load and set volume.
///
/// The set volume leaves out the sets of stretch-only exercises.
pub fn view_list(
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    routines: &[domain::Routine],
    dialog: Signal<TrainingDialog>,
) -> Element {
    let body = training_sessions
        .iter()
        .rev()
        .map(|t| {
            let load = count_to_string(f64::from(t.load(stretch_only)));
            let set_volume = count_to_string(f64::from(
                t.without_stretch_only_sets(stretch_only).set_volume(),
            ));
            view_session_item(
                t,
                routines,
                dialog,
                rsx! {
                    div {
                        "data-testid": "session-summary",
                        "Load {load} · Set volume {set_volume}"
                    }
                },
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { body }
    }
}

/// The cells of a session in a `Table`: the date and the routine of the session above `body`, and
/// a delete icon
///
/// The routine is shown if it is one of `routines`.
pub fn view_session_item(
    training_session: &domain::TrainingSession,
    routines: &[domain::Routine],
    mut dialog: Signal<TrainingDialog>,
    body: Element,
) -> Vec<Element> {
    let id = training_session.id;
    let date = training_session.date;
    let routine = routines
        .iter()
        .find(|r| r.id == training_session.routine_id);
    vec![
        rsx! {
            div {
                class: "has-text-left",
                "data-testid": "session-item",
                div {
                    class: "has-text-weight-bold",
                    "data-testid": "session-title",
                    Link {
                        to: Route::TrainingSession { id },
                        NoWrap { "{date}" }
                    }
                    if let Some(routine) = routine {
                        " · "
                        Link {
                            to: Route::Routine { id: routine.id, tab: RoutineTab::Routine },
                            "{routine.name}"
                        }
                    }
                }
                {body}
            }
        },
        rsx! {
            div {
                class: "has-text-right",
                a {
                    class: "mx-2",
                    "data-testid": "item-delete",
                    onclick: move |_| { *dialog.write() = TrainingDialog::Delete { id, date }; },
                    Icon { name: "xmark"}
                }
            }
        },
    ]
}

pub fn view_dialog(
    mut dialog: Signal<TrainingDialog>,
    training_sessions: &[domain::TrainingSession],
    routines: &[domain::Routine],
    closed_dialog_route: Option<Route>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let close_dialog = move || {
        dialog.set(TrainingDialog::None);
        if let Some(route) = closed_dialog_route {
            navigator().replace_preserving_scroll(route);
        }
    };

    let save = move |()| async move {
        is_loading! {
            if let TrainingDialog::Add { date, routine_id } = &*dialog.read()
                && let (Ok(date), Ok(routine_id)) = (date.validated.clone(), routine_id.validated.clone()) {
                    match DOMAIN_SERVICE().get_routines().await {
                        Ok(routines) => {
                            let routine = routines.iter().find(|r| r.id == routine_id);
                            if routine.is_none() && !routine_id.is_nil() {
                                notify_error("add training session", "unknown routine");
                            } else {
                                start_training_session(routine, date).await;
                            }
                        }
                        Err(err) => {
                            notify("add training session", &err);
                        }
                    }
                }
        }
    };
    let delete = eh!(close_dialog; {
        async move {
            let mut deleted = false;
            is_loading! {
                if let TrainingDialog::Delete { id, .. } = *dialog.read() {
                    match DOMAIN_SERVICE().delete_training_session(id).await {
                        Ok(()) => {
                            deleted = true;
                            consume_context::<Cache>().refresh_training_sessions();
                        },
                        Err(err) => notify("delete training session", &err)
                    }
                }
            }
            if deleted {
                close_dialog();
            }
        }
    });
    let close = move |_| {
        close_dialog.clone()();
    };

    match &*dialog.read() {
        TrainingDialog::None => rsx! {},
        TrainingDialog::Add { date, routine_id } => {
            rsx! {
                SaveDialog {
                    title: rsx! { "Add training session" },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: !date.valid() || !routine_id.valid(),
                    InputField {
                        label: "Date".to_string(),
                        "data-testid": "date",
                        r#type: "date".to_string(),
                        max: Local::now().date_naive().to_string(),
                        value: date.input.clone(),
                        error: if let Err(err) = &date.validated { err.clone() },
                        has_changed: date.changed(),
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated_date = DOMAIN_SERVICE()
                                .validate_training_session_date(&input)
                                .map_err(|err| err.to_string());
                            if let TrainingDialog::Add { date, .. } = &mut *dialog.write() {
                                date.input = input;
                                date.validated = validated_date;
                            }
                        },
                    }
                    SelectField {
                        label: "Routine".to_string(),
                        "data-testid": "routine",
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: String::new(),
                                    value: String::new(),
                                    selected: routine_id.validated == Ok(domain::RoutineID::nil()),
                                }
                            }
                        ].into_iter().chain(domain::routines_sorted_by_last_use(routines, training_sessions, |r: &domain::Routine| !r.archived).iter().map(|r| {
                            rsx! {
                                SelectOption {
                                    text: r.name.to_string(),
                                    value: r.id.to_string(),
                                    selected: routine_id.validated == Ok(r.id),
                                }
                            }
                        })).collect::<Vec<_>>(),
                        has_changed: routine_id.changed(),
                        on_change: move |event: FormEvent| {
                            if let TrainingDialog::Add { routine_id, .. } = &mut *dialog.write() {
                                routine_id.input = event.value();
                                routine_id.validated = Ok(domain::RoutineID::from_str(&routine_id.input).unwrap_or(domain::RoutineID::nil()));
                            }
                        }
                    }
                }
            }
        }
        TrainingDialog::Delete { date, .. } => rsx! {
            DeleteConfirmationDialog {
                element_type: "training session".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{date}" } } },
                on_delete: delete,
                on_cancel: close,
                is_loading: is_loading(),
            }
        },
    }
}

/// Starts a training session of `routine`, unless a previous start is still in progress.
pub async fn start_training_session_once(routine: &domain::Routine, date: NaiveDate) {
    if IS_STARTING_TRAINING_SESSION() {
        return;
    }
    let _loading = LoadingFlag::set(&IS_STARTING_TRAINING_SESSION);
    start_training_session(Some(routine), date).await;
}

/// Creates a training session on `date`, prefilled with the elements of `routine`, and navigates
/// to it.
async fn start_training_session(routine: Option<&domain::Routine>, date: NaiveDate) {
    let elements = routine
        .map(|routine| {
            routine
                .sections
                .iter()
                .flat_map(domain::RoutinePart::to_training_session_elements)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match DOMAIN_SERVICE()
        .create_training_session(
            routine.map_or_else(domain::RoutineID::nil, |routine| routine.id),
            date,
            String::new(),
            elements,
        )
        .await
    {
        Ok(training_session) => {
            let id = training_session.id;
            consume_context::<Cache>().load_training_sessions().await;
            navigator().push(Route::TrainingSession { id });
        }
        Err(err) => {
            notify("add training session", &err);
        }
    }
}

pub enum TrainingDialog {
    None,
    Add {
        date: FieldValue<NaiveDate>,
        routine_id: FieldValue<domain::RoutineID>,
    },
    Delete {
        id: domain::TrainingSessionID,
        date: NaiveDate,
    },
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, all_text_of, attribute_of, contains, links_of, provide_settings, render,
        rows_of, text_of,
    };

    use super::*;

    fn training_session(id: u128, date: NaiveDate) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date,
            notes: String::new(),
            elements: vec![domain::TrainingSessionElement::Set {
                exercise_id: 1.into(),
                side: domain::Side::Unset,
                reps: domain::Reps::new(10).unwrap(),
                time: domain::Time::new(30).unwrap(),
                weight: domain::Weight::default(),
                rpe: domain::RPE::new(8.0).unwrap(),
                target_reps: domain::Reps::default(),
                target_tempo: domain::Tempo::default(),
                target_weight: domain::Weight::default(),
                target_rpe: domain::RPE::ZERO,
                automatic: false,
                countdown: false,
            }],
            exercise_notes: BTreeMap::new(),
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    fn routine() -> domain::Routine {
        domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn sessions() -> TestCache {
        let today = Local::now().date_naive();
        TestCache::default()
            .with_routines(vec![routine()])
            .with_training_sessions(vec![
                training_session(1, today - chrono::Duration::days(1)),
                training_session(2, today),
            ])
    }

    fn render_training_sessions(
        settings: web_app::Settings,
        cache: impl Fn() -> TestCache + 'static,
    ) -> String {
        render_tab(TrainingSessionsTab::Sessions, settings, cache)
    }

    fn render_charts(cache: impl Fn() -> TestCache + 'static) -> String {
        render_tab(
            TrainingSessionsTab::Charts,
            web_app::Settings::default(),
            cache,
        )
    }

    fn render_tab(
        tab: TrainingSessionsTab,
        settings: web_app::Settings,
        cache: impl Fn() -> TestCache + 'static,
    ) -> String {
        render(move || {
            cache().provide();
            provide_settings(settings);
            rsx! { TrainingSessions { tab, muscle: None, add: false } }
        })
    }

    #[test]
    fn test_sessions_in_includes_the_first_and_the_last_day() {
        let day = |d| NaiveDate::from_ymd_opt(2026, 1, d).unwrap();
        let training_sessions = (1..=5)
            .map(|d| training_session(u128::from(d), day(d)))
            .collect::<Vec<_>>();

        assert_eq!(
            sessions_in(
                &training_sessions,
                domain::Interval {
                    first: day(2),
                    last: day(4)
                }
            )
            .iter()
            .map(|t| t.date)
            .collect::<Vec<_>>(),
            vec![day(2), day(3), day(4)]
        );
    }

    #[test]
    fn test_the_sessions_tab_shows_the_table_and_no_chart() {
        let html = render_training_sessions(web_app::Settings::default(), sessions);

        assert_eq!(attribute_of(&html, "tab-sessions", "data-selected"), "true");
        assert_eq!(attribute_of(&html, "tab-charts", "data-selected"), "false");
        assert!(contains(&html, "table"));
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_the_charts_tab_shows_the_charts_and_no_table() {
        let html = render_charts(sessions);

        assert_eq!(
            attribute_of(&html, "tab-sessions", "data-selected"),
            "false"
        );
        assert_eq!(attribute_of(&html, "tab-charts", "data-selected"), "true");
        assert!(contains(&html, "chart"));
        assert!(!contains(&html, "table"));
    }

    #[test]
    fn test_the_add_dialog_opens_the_sessions_tab() {
        let html = render(move || {
            sessions().provide();
            provide_settings(web_app::Settings::default());
            rsx! { TrainingSessions { tab: TrainingSessionsTab::Charts, muscle: None, add: true } }
        });

        assert_eq!(attribute_of(&html, "tab-sessions", "data-selected"), "true");
        assert!(contains(&html, "table"));
    }

    #[test]
    fn test_sessions_are_listed_newest_first_with_their_date_and_routine_links() {
        let html = render_training_sessions(web_app::Settings::default(), sessions);
        let today = Local::now().date_naive();

        assert_eq!(
            all_text_of(&html, "session-title"),
            [
                format!("{today} · A"),
                format!("{} · A", today - chrono::Duration::days(1))
            ]
        );
        assert_eq!(
            links_of(&html, "session-title"),
            [
                Route::TrainingSession { id: 2.into() }.to_string(),
                Route::Routine {
                    id: 1.into(),
                    tab: RoutineTab::Routine
                }
                .to_string()
            ]
        );
    }

    #[test]
    fn test_a_session_without_a_routine_shows_its_date_alone() {
        let today = Local::now().date_naive();
        let html = render_training_sessions(web_app::Settings::default(), move || {
            TestCache::default()
                .with_routines(vec![routine()])
                .with_training_sessions(vec![domain::TrainingSession {
                    routine_id: domain::RoutineID::nil(),
                    ..training_session(1, today)
                }])
        });

        assert_eq!(text_of(&html, "session-title"), today.to_string());
        assert_eq!(
            links_of(&html, "session-title"),
            [Route::TrainingSession { id: 1.into() }.to_string()]
        );
    }

    #[test]
    fn test_an_item_shows_the_load_and_the_set_volume_of_its_session() {
        let html = render_training_sessions(web_app::Settings::default(), sessions);

        assert_eq!(text_of(&html, "session-summary"), "Load 8 · Set volume 1");
    }

    fn exercise(id: u128, muscles: Vec<domain::ExerciseMuscle>) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(&format!("Exercise {id}")).unwrap(),
            notes: String::new(),
            muscles,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn muscle(
        muscle_id: domain::MuscleID,
        stimulus: domain::Stimulus,
        stretch: domain::Stimulus,
    ) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus,
            stretch,
        }
    }

    /// A session of today holding a hybrid exercise 1 and a hold of a stretch-only exercise 2.
    fn sessions_with_stretch() -> TestCache {
        stretch_cache(vec![session_with_stretch(1, Local::now().date_naive())])
    }

    fn session_with_stretch(id: u128, date: NaiveDate) -> domain::TrainingSession {
        let mut session = training_session(id, date);
        session.elements.push(domain::TrainingSessionElement::Set {
            exercise_id: 2.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::default(),
            time: domain::Time::new(90).unwrap(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::new(9.0).unwrap(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
        });
        session
    }

    /// The `training_sessions` with a hybrid exercise 1 and a stretch-only exercise 2.
    fn stretch_cache(training_sessions: Vec<domain::TrainingSession>) -> TestCache {
        TestCache::default()
            .with_routines(vec![routine()])
            .with_exercises(vec![
                exercise(
                    1,
                    vec![
                        muscle(
                            domain::MuscleID::ErectorSpinae,
                            domain::Stimulus::PRIMARY,
                            domain::Stimulus::NONE,
                        ),
                        muscle(
                            domain::MuscleID::Hamstrings,
                            domain::Stimulus::NONE,
                            domain::Stimulus::PRIMARY,
                        ),
                    ],
                ),
                exercise(
                    2,
                    vec![muscle(
                        domain::MuscleID::Hamstrings,
                        domain::Stimulus::NONE,
                        domain::Stimulus::PRIMARY,
                    )],
                ),
            ])
            .with_training_sessions(training_sessions)
    }

    #[test]
    fn test_a_session_of_stretches_alone_shows_its_load_and_a_set_volume_of_0() {
        let html = render_training_sessions(web_app::Settings::default(), || {
            let mut session = session_with_stretch(1, Local::now().date_naive());
            session.elements.remove(0);
            stretch_cache(vec![session])
        });

        assert_eq!(text_of(&html, "session-summary"), "Load 1 · Set volume 0");
    }

    #[test]
    fn test_a_stretch_shows_the_stretch_charts() {
        let html = render_charts(sessions_with_stretch);

        assert!(html.contains("7-day time under stretch (min)"));
    }

    fn stretch_outside_the_interval() -> TestCache {
        let today = Local::now().date_naive();
        stretch_cache(vec![
            session_with_stretch(1, today - chrono::Duration::days(365)),
            training_session(2, today),
        ])
    }

    #[test]
    fn test_a_stretch_outside_the_interval_shows_no_stretch_charts() {
        let charts = render_charts(stretch_outside_the_interval);

        assert!(!charts.contains("7-day stretch"));
    }

    #[test]
    fn test_with_unreadable_exercises_stretches_count_like_other_sets() {
        let html = render_training_sessions(web_app::Settings::default(), || {
            sessions_with_stretch().with_unreadable_exercises()
        });

        assert_eq!(text_of(&html, "session-summary"), "Load 24 · Set volume 2");
    }

    #[test]
    fn test_without_stretch_sets_no_stretch_charts_are_shown() {
        let charts = render_charts(sessions);

        assert!(!charts.contains("7-day stretch"));
    }

    #[test]
    fn test_without_sessions_no_chart_and_an_empty_list_are_shown() {
        let list = render_training_sessions(web_app::Settings::default(), TestCache::default);
        let charts = render_charts(TestCache::default);

        assert!(rows_of(&list, "table").is_empty());
        assert!(!contains(&charts, "chart"));
    }

    #[test]
    fn test_unread_sessions_are_shown_as_loading() {
        let html = render_training_sessions(web_app::Settings::default(), TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_sessions_are_shown_as_an_error() {
        let html = render_training_sessions(web_app::Settings::default(), TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }
}
