use std::{borrow::Borrow, collections::BTreeMap};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, Property};
use valens_web_app as web_app;

use crate::{
    Route,
    cache::{Cache, CacheState},
    chart::{Calendar, Chart, IntervalControl},
    eh, page,
    settings::Settings,
    ui::element::{
        Block, CenteredBlock, CenteredTags, ElementWithDescription, Error, ErrorPage,
        FloatingActionButton, Loading, LoadingPage, NoData, NoWrap, Title,
    },
};

#[component]
pub fn Exercise(id: domain::ExerciseID) -> Element {
    let cache = consume_context::<Cache>();
    let mut current_interval = use_signal(domain::Interval::default);
    let settings = use_context::<Settings>();
    let mut exercise_dialog = use_signal(|| page::exercises::ExerciseDialog::None);
    let training_dialog = use_signal(|| page::training_sessions::TrainingDialog::None);

    match &*cache.exercises.read() {
        CacheState::Ready(exercises) => {
            let exercise = exercises.iter().find(|e| e.id == id);
            if let Some(exercise) = exercise {
                let muscles = exercise.muscle_stimulus().into_iter().collect::<Vec<_>>();
                let has_properties = exercise.force.is_some()
                    || exercise.mechanic.is_some()
                    || exercise.laterality.is_some()
                    || exercise.assistance.is_some()
                    || exercise.category.is_some()
                    || !exercise.equipment.is_empty();
                rsx! {
                    Title { "{exercise.name}" },
                    if has_properties || !muscles.is_empty() {
                        Block {
                            {view_exercise_properties(
                                exercise.force,
                                exercise.mechanic,
                                exercise.laterality,
                                exercise.assistance,
                                &exercise.equipment,
                                &muscles,
                                exercise.category,
                            )},
                        }
                    }
                    {view_notes(exercise, exercise_dialog)}
                    match (&*cache.training_sessions.read(), &*cache.routines.read()) {
                        (CacheState::Ready(training_sessions), CacheState::Ready(routines)) => {
                            let sessions_with_exercise = training_sessions
                                .iter()
                                .filter(|t| t.exercises().contains(&id))
                                .collect::<Vec<_>>();
                            let training_sessions = sessions_with_exercise
                                .iter()
                                .map(|t| t.restricted_to(id))
                            .collect::<Vec<_>>();
                            if training_sessions.is_empty() {
                                rsx! {
                                    NoData {}
                                }
                            } else {
                                let dates = training_sessions
                                    .iter()
                                    .map(|ts| ts.date)
                                    .collect::<Vec<_>>();
                                let all = domain::Interval {
                                    first: dates.iter().min().copied().unwrap_or_default(),
                                    last: dates.iter().max().copied().unwrap_or_default(),
                                };
                                if *current_interval.read() == domain::Interval::default() {
                                    current_interval.set(domain::init_interval(&dates, domain::DefaultInterval::_3M));
                                }
                                let interval = *current_interval.read();
                                let training_sessions = training_sessions
                                    .iter()
                                    .filter(|t| t.date >= interval.first && t.date <= interval.last)
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let sessions_with_exercise = sessions_with_exercise
                                    .into_iter()
                                    .filter(|t| t.date >= interval.first && t.date <= interval.last)
                                    .collect::<Vec<_>>();
                                rsx! {
                                    IntervalControl { current_interval, all },
                                    if training_sessions.is_empty() {
                                        NoData {}
                                    } else {
                                        {view_charts(id, &training_sessions, interval, settings)}
                                        {view_calendar(&training_sessions, interval)}
                                        {page::training_sessions::view_table(&training_sessions, routines, interval, training_dialog, settings)}
                                        {view_sets(id, &sessions_with_exercise, routines, settings)}
                                        {page::training_sessions::view_dialog(training_dialog, &training_sessions, routines, None)}
                                    }
                                }
                            }
                        }
                        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
                            rsx! { Error { message: "{err}" } }
                        }
                        (CacheState::Loading, _) | (_, CacheState::Loading) => {
                            rsx! {
                                Loading {}
                            }
                        }
                    }
                    {page::exercises::view_dialog(exercise_dialog, None)}
                    FloatingActionButton {
                        icon: "edit".to_string(),
                        on_click: eh!(exercise; {
                            *exercise_dialog.write() = page::exercises::ExerciseDialog::Options(exercise.clone());
                        }),
                    }
                }
            } else {
                rsx! {
                    ErrorPage { message: "Exercise not found" }
                }
            }
        }
        CacheState::Error(err) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        CacheState::Loading => {
            rsx! { LoadingPage {} }
        }
    }
}

pub fn view_exercise_properties(
    force: Option<domain::Force>,
    mechanic: Option<domain::Mechanic>,
    laterality: Option<domain::Laterality>,
    assistance: Option<domain::Assistance>,
    equipment: &[domain::Equipment],
    muscles: &[(domain::MuscleID, domain::Stimulus)],
    category: Option<domain::Category>,
) -> Element {
    let names = domain::ExerciseProperty::iter()
        .filter_map(|property| match property {
            domain::ExerciseProperty::Force => force.map(domain::Force::name),
            domain::ExerciseProperty::Mechanic => mechanic.map(domain::Mechanic::name),
            domain::ExerciseProperty::Laterality => laterality.map(domain::Laterality::name),
            domain::ExerciseProperty::Assistance => assistance.map(domain::Assistance::name),
            domain::ExerciseProperty::Category => category.map(domain::Category::name),
            domain::ExerciseProperty::Muscles | domain::ExerciseProperty::Equipment => None,
        })
        .collect::<Vec<_>>();
    let equipment = equipment.to_vec();
    rsx! {
        {view_muscles(muscles.iter().map(|(k, v)| (k, v)))}
        if !names.is_empty() {
            CenteredTags {
                for name in names {
                    span { class: "tag", "data-testid": "property-tag", {name} }
                }
            }
        }
        if !equipment.is_empty() {
            CenteredTags {
                for e in equipment {
                    span { class: "tag", "data-testid": "property-tag", {e.name()} }
                }
            }
        }
    }
}

pub fn view_muscles<M, I, S>(muscles: M) -> Element
where
    M: IntoIterator<Item = (I, S)>,
    I: Borrow<domain::MuscleID>,
    S: Borrow<domain::Stimulus>,
{
    let mut muscles = muscles
        .into_iter()
        .filter_map(|(k, v)| {
            domain::StimulusLevel::from_stimulus(*v.borrow()).map(|level| (*k.borrow(), level))
        })
        .collect::<Vec<_>>();
    muscles.sort_by_key(|b| std::cmp::Reverse(b.1));
    let muscles = muscles
        .into_iter()
        .map(|(m, level)| (m, stimulus_level_class(level)))
        .collect::<Vec<_>>();
    rsx! {
        CenteredTags {
            for (m, class) in muscles {
                ElementWithDescription {
                    description: m.description(),
                    span {
                        class: "tag {class}",
                        "data-testid": "muscle-tag",
                        {m.name()}
                    }
                }
            }
        }
    }
}

pub fn stimulus_level_class(level: domain::StimulusLevel) -> &'static str {
    match level {
        domain::StimulusLevel::Primary => "is-dark",
        domain::StimulusLevel::Secondary => "is-link",
    }
}

fn view_charts(
    exercise_id: domain::ExerciseID,
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let params = web_app::chart::PlotParams::primary_range(0., 10.);

    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_values: Vec<(NaiveDate, f32)> = vec![];
    let mut weight_values: Vec<(NaiveDate, f32)> = vec![];
    let mut time_values: Vec<(NaiveDate, f32)> = vec![];
    let mut estimated_max_reps_by_date: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut one_rep_max_by_date: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    for training_session in training_sessions {
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume())
            .or_insert(training_session.set_volume());
        #[allow(clippy::cast_precision_loss)]
        volume_load
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.volume_load() as f32)
            .or_insert(training_session.volume_load() as f32);
        #[allow(clippy::cast_precision_loss)]
        tut.entry(training_session.date)
            .and_modify(|e| *e += training_session.tut().unwrap_or_default() as f32)
            .or_insert(training_session.tut().unwrap_or_default() as f32);
        if let Some(v) = training_session.estimated_max_reps() {
            estimated_max_reps_by_date
                .entry(training_session.date)
                .and_modify(|e| {
                    if v > *e {
                        *e = v;
                    }
                })
                .or_insert(v);
        }
        if let Some(v) = training_session.one_rep_max(exercise_id) {
            one_rep_max_by_date
                .entry(training_session.date)
                .and_modify(|e| {
                    if v > *e {
                        *e = v;
                    }
                })
                .or_insert(v);
        }
        for element in &training_session.elements {
            if let domain::TrainingSessionElement::Set {
                reps, weight, time, ..
            } = element
            {
                if let Some(reps) = reps.non_zero() {
                    #[allow(clippy::cast_precision_loss)]
                    reps_values.push((training_session.date, u32::from(reps) as f32));
                }
                if let Some(weight) = weight.non_zero() {
                    weight_values.push((training_session.date, f32::from(weight)));
                }
                if let Some(time) = time.non_zero() {
                    #[allow(clippy::cast_precision_loss)]
                    time_values.push((training_session.date, u32::from(time) as f32));
                }
            }
        }
    }

    let mut reps_series = web_app::chart::labeled_min_avg_max(
        "reps",
        &reps_values,
        interval,
        params,
        web_app::chart::COLOR_REPS,
    );
    if settings.show_rpe() && !estimated_max_reps_by_date.is_empty() {
        reps_series.insert(
            1,
            web_app::chart::LabeledSeries::new(
                "Est. max. reps",
                web_app::chart::PlotData {
                    values_high: estimated_max_reps_by_date.into_iter().collect(),
                    values_low: None,
                    plots: web_app::chart::plot_dotted_line(web_app::chart::COLOR_REPS),
                    params,
                },
            ),
        );
    }

    let mut weight_series = web_app::chart::labeled_min_avg_max(
        "weight (kg)",
        &weight_values,
        interval,
        params,
        web_app::chart::COLOR_WEIGHT,
    );
    if !one_rep_max_by_date.is_empty() {
        weight_series.insert(
            1,
            web_app::chart::LabeledSeries::new(
                "Est. 1RM (kg)",
                web_app::chart::PlotData {
                    values_high: one_rep_max_by_date.into_iter().collect(),
                    values_low: None,
                    plots: web_app::chart::plot_dotted_line(web_app::chart::COLOR_WEIGHT),
                    params,
                },
            ),
        );
    }

    let time_series = web_app::chart::labeled_min_avg_max(
        "time (s)",
        &time_values,
        interval,
        params,
        web_app::chart::COLOR_TIME,
    );

    rsx! {
        Chart {
            series: reps_series,
            interval,
            no_data_label: false,
        }
        Chart {
            series: weight_series,
            interval,
            no_data_label: false,
        }
        if settings.show_tut() {
            Chart {
                series: time_series,
                interval,
                no_data_label: false,
            }
        }
        Chart {
            series: vec![web_app::chart::LabeledSeries::new(
                "Set volume",
                web_app::chart::PlotData {
                    values_high: set_volume.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_SET_VOLUME,
                    ),
                    params,
                },
            )],
            interval,
            no_data_label: false,
        }
        Chart {
            series: vec![web_app::chart::LabeledSeries::new(
                "Volume load",
                web_app::chart::PlotData {
                    values_high: volume_load.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_VOLUME_LOAD,
                    ),
                    params,
                },
            )],
            interval,
            no_data_label: false,
        }
        if settings.show_tut() {
            Chart {
                series: vec![web_app::chart::LabeledSeries::new(
                    "Time under tension (s)",
                    web_app::chart::PlotData {
                        values_high: tut.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(
                            web_app::chart::COLOR_TUT,
                        ),
                        params,
                    },
                )],
                interval,
                no_data_label: false,
            }
        }
    }
}

fn view_calendar(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
) -> Element {
    let mut volume_load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            volume_load
                .entry(training_session.date)
                .and_modify(|e| *e += training_session.volume_load())
                .or_insert(training_session.volume_load());
        }
    }
    let min = volume_load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = volume_load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let entries = volume_load
        .iter()
        .map(|(date, volume_load)| {
            (
                *date,
                web_app::chart::COLOR_VOLUME_LOAD,
                if max > min {
                    (f64::from(volume_load - min) / f64::from(max - min)) * 0.8 + 0.2
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

fn view_notes(
    exercise: &domain::Exercise,
    mut exercise_dialog: Signal<page::exercises::ExerciseDialog>,
) -> Element {
    if exercise.notes.is_empty() {
        return rsx! {};
    }
    let notes = exercise.notes.clone();
    rsx! {
        CenteredBlock {
            div {
                class: "is-clickable is-italic has-text-centered is-preserving-line-breaks",
                "data-testid": "exercise-notes",
                onclick: eh!(exercise; {
                    *exercise_dialog.write() = page::exercises::ExerciseDialog::EditNotes {
                        exercise: exercise.clone(),
                    };
                }),
                { notes.clone() }
            }
        }
    }
}

/// Renders the sets of an exercise per training session, the sets of a session holding sets with
/// a side in a left and a right column.
///
/// The training sessions carry all their elements, since a left and a right set form a pair only
/// where they directly follow each other in the whole session.
fn view_sets(
    exercise_id: domain::ExerciseID,
    training_sessions: &[&domain::TrainingSession],
    routines: &[domain::Routine],
    settings: Settings,
) -> Element {
    let blocks = training_sessions.iter().rev().flat_map(|t| {
        let routine = routines.iter().find(|r| r.id == t.routine_id);
        let routine_id = routine.map(|r| r.id).unwrap_or_default();
        let note = t
            .exercise_notes
            .get(&exercise_id)
            .cloned()
            .unwrap_or_default();
        let rows = t.set_history_rows(exercise_id);
        let has_sides = rows
            .iter()
            .any(|row| matches!(row, domain::SetHistoryRow::Sides { .. }));
        let values = |set: &Option<domain::Set>| {
            set.as_ref()
                .map(|set| set.to_string(settings.show_tut(), settings.show_rpe()))
                .unwrap_or_default()
        };
        let sets = if has_sides {
            rsx! {
                table {
                    class: "mx-auto",
                    "data-testid": "set-history-sides",
                    tr {
                        th { class: page::training_session::COLUMN_HEADER_CLASS, "Left" }
                        th { class: page::training_session::COLUMN_HEADER_CLASS, "Right" }
                    }
                    for row in rows {
                        tr {
                            match row {
                                domain::SetHistoryRow::Sides { left, right } => rsx! {
                                    td { class: "px-2 has-text-centered", NoWrap { {values(&left)} } }
                                    td { class: "px-2 has-text-centered", NoWrap { {values(&right)} } }
                                },
                                domain::SetHistoryRow::Combined(set) => rsx! {
                                    td {
                                        class: "px-2 has-text-centered",
                                        colspan: 2,
                                        NoWrap { {values(&Some(set))} }
                                    }
                                },
                            }
                        }
                    }
                }
            }
        } else {
            rsx! {
                for row in rows {
                    if let domain::SetHistoryRow::Combined(set) = row {
                        div {
                            "data-testid": "set-history-set",
                            NoWrap { {values(&Some(set))} }
                        }
                    }
                }
            }
        };
        [
            rsx! {
                div {
                    class: "block has-text-centered has-text-weight-bold mb-2",
                    Link {
                        to: Route::TrainingSession { id: t.id },
                        NoWrap { "{t.date}" }
                    }
                    " "
                    if routine_id.is_nil() {
                        "-"
                    } else {
                        Link {
                            to: Route::Routine { id: routine_id },
                            match routine {
                                Some(routine) => rsx! { {routine.name.to_string()} },
                                None => rsx! { "-" }
                            }
                        }
                    }
                }
                if !note.is_empty() {
                    div {
                        class: "is-italic has-text-centered mb-2",
                        "data-testid": "session-exercise-notes",
                        { note.clone() }
                    }
                }
            },
            rsx! {
                div {
                    class: "block has-text-centered",
                    {sets}
                }
            },
        ]
    });

    rsx! {
        for b in blocks {
            {b}
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, all_text_of, contains, provide_settings, render, rows_of, text_of,
    };

    fn exercise(id: u128, name: &str) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: "some notes".to_string(),
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Pecs,
                stimulus: domain::Stimulus::PRIMARY,
            }],
            force: Some(domain::Force::Push),
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn render_exercise(id: u128, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_settings(web_app::Settings::default());
            rsx! { Exercise { id: domain::ExerciseID::from(id) } }
        })
    }

    #[test]
    fn test_the_name_properties_and_notes_are_shown() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![exercise(1, "Squat")])
        });

        assert_eq!(text_of(&html, "title"), "Squat");
        assert_eq!(text_of(&html, "property-tag"), "Push");
        assert_eq!(text_of(&html, "muscle-tag"), "Pecs");
        assert_eq!(text_of(&html, "exercise-notes"), "some notes");
    }

    #[test]
    fn test_without_notes_none_are_shown() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![domain::Exercise {
                notes: String::new(),
                ..exercise(1, "Squat")
            }])
        });

        assert!(!contains(&html, "exercise-notes"));
    }

    #[test]
    fn test_without_training_sessions_no_data_is_reported() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![exercise(1, "Squat")])
        });

        assert_eq!(text_of(&html, "no-data"), "No data");
        assert!(!contains(&html, "chart"));
    }

    fn performed_set(side: domain::Side, reps: u32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: 1.into(),
            side,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    fn render_exercise_with_sets(elements: Vec<domain::TrainingSessionElement>) -> String {
        render_exercise(1, move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Lunge")])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: domain::RoutineID::nil(),
                    date: chrono::Local::now().date_naive(),
                    notes: String::new(),
                    elements: elements.clone(),
                    exercise_notes: BTreeMap::new(),
                }])
        })
    }

    #[test]
    fn test_the_set_history_shows_the_sides_in_a_left_and_a_right_column() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Left, 10),
            performed_set(domain::Side::Right, 9),
            performed_set(domain::Side::Right, 8),
            performed_set(domain::Side::Unset, 7),
        ]);

        assert_eq!(
            rows_of(&html, "set-history-sides"),
            vec![
                vec!["Left", "Right"],
                vec!["10", "9"],
                vec!["", "8"],
                vec!["7"],
            ]
        );
    }

    #[test]
    fn test_the_set_history_without_sides_is_a_single_column() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Unset, 10),
            performed_set(domain::Side::Unset, 9),
        ]);

        assert!(!contains(&html, "set-history-sides"));
        assert_eq!(all_text_of(&html, "set-history-set"), vec!["10", "9"]);
    }

    #[test]
    fn test_an_unknown_exercise_is_reported() {
        let html = render_exercise(2, || {
            TestCache::default().with_exercises(vec![exercise(1, "Squat")])
        });

        assert_eq!(text_of(&html, "error-page"), "Exercise not found");
    }

    #[test]
    fn test_unread_exercises_are_shown_as_loading() {
        let html = render_exercise(1, TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_exercises_are_shown_as_an_error() {
        let html = render_exercise(1, TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn stimulus_levels_are_visually_distinct() {
        let levels = [
            domain::StimulusLevel::Primary,
            domain::StimulusLevel::Secondary,
        ];

        assert_eq!(
            levels
                .into_iter()
                .map(stimulus_level_class)
                .collect::<HashSet<_>>()
                .len(),
            levels.len()
        );
    }
}
