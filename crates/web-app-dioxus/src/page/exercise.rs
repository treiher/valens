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
    routing::{ExerciseTab, NavigatorScrollExt, RoutineTab},
    settings::Settings,
    ui::element::{
        Block, Caption, CenteredBlock, CenteredTags, ElementWithDescription, Error, ErrorPage,
        FloatingActionButton, Loading, LoadingPage, NoData, NoWrap, Tabs,
    },
};

#[component]
pub fn Exercise(id: domain::ExerciseID, tab: ExerciseTab) -> Element {
    let cache = consume_context::<Cache>();
    let current_interval = use_signal(domain::Interval::default);
    let settings = use_context::<Settings>();
    let mut exercise_dialog = use_signal(|| page::exercises::ExerciseDialog::None);
    let training_dialog = use_signal(|| page::training_sessions::TrainingDialog::None);

    match &*cache.exercises.read() {
        CacheState::Ready(exercises) => {
            let exercise = exercises.iter().find(|e| e.id == id);
            if let Some(exercise) = exercise {
                let content = match tab {
                    ExerciseTab::Exercise => view_exercise(exercise, exercise_dialog),
                    ExerciseTab::Sessions | ExerciseTab::Charts => view_training_sessions(
                        exercise,
                        exercises,
                        tab,
                        &cache,
                        current_interval,
                        training_dialog,
                        settings,
                    ),
                };
                rsx! {
                    Tabs {
                        tabs: vec![("Exercise", ExerciseTab::Exercise), ("Sessions", ExerciseTab::Sessions), ("Charts", ExerciseTab::Charts)],
                        selected: tab,
                        on_select: move |tab| {
                            navigator().replace_preserving_scroll(Route::Exercise { id, tab });
                        },
                        is_page_top: true,
                    }
                    {content}
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

/// Show the properties, the muscles and the notes of the exercise.
fn view_exercise(
    exercise: &domain::Exercise,
    exercise_dialog: Signal<page::exercises::ExerciseDialog>,
) -> Element {
    let muscles = exercise.muscle_stimulus().into_iter().collect::<Vec<_>>();
    let stretch = exercise.muscle_stretch().into_iter().collect::<Vec<_>>();
    let has_properties = exercise.force.is_some()
        || exercise.mechanic.is_some()
        || exercise.laterality.is_some()
        || exercise.assistance.is_some()
        || exercise.category.is_some()
        || !exercise.equipment.is_empty();
    let has_muscles = !muscles.is_empty() || !stretch.is_empty();
    if !has_properties && !has_muscles && exercise.notes.is_empty() {
        return rsx! { NoData {} };
    }
    rsx! {
        {view_exercise_properties(
            exercise.force,
            exercise.mechanic,
            exercise.laterality,
            exercise.assistance,
            &exercise.equipment,
            &muscles,
            &stretch,
            exercise.category,
        )},
        {view_notes(exercise, exercise_dialog)}
    }
}

/// Show the sessions holding the exercise and its sets on the sessions tab, and their charts on
/// the charts tab.
fn view_training_sessions(
    exercise: &domain::Exercise,
    exercises: &[domain::Exercise],
    tab: ExerciseTab,
    cache: &Cache,
    mut current_interval: Signal<domain::Interval>,
    training_dialog: Signal<page::training_sessions::TrainingDialog>,
    settings: Settings,
) -> Element {
    let id = exercise.id;
    // A hold without its time reads as an empty set.
    let show_time = settings.show_tut() || exercise.is_stretch_only();
    let stretch_only = domain::StretchOnlyExercises::new(exercises);
    let stretch_metrics = if exercise.is_stretch_only() {
        page::training_sessions::StretchMetrics::Only
    } else {
        page::training_sessions::StretchMetrics::Include
    };
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
                return rsx! { NoData {} };
            }
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
                } else if tab == ExerciseTab::Charts {
                    {view_charts(id, &training_sessions, &stretch_only, interval, show_time, settings)}
                } else {
                    {view_calendar(id, &training_sessions, &stretch_only, interval)}
                    {page::training_sessions::view_table(&training_sessions, &stretch_only, stretch_metrics, show_time, routines, interval, training_dialog, settings)}
                    {view_sets(id, &sessions_with_exercise, routines, show_time, settings)}
                    {page::training_sessions::view_dialog(training_dialog, &training_sessions, routines, None)}
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
}

#[allow(clippy::too_many_arguments)]
pub fn view_exercise_properties(
    force: Option<domain::Force>,
    mechanic: Option<domain::Mechanic>,
    laterality: Option<domain::Laterality>,
    assistance: Option<domain::Assistance>,
    equipment: &[domain::Equipment],
    muscles: &[(domain::MuscleID, domain::Stimulus)],
    stretch: &[(domain::MuscleID, domain::Stimulus)],
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
        if !muscles.is_empty() {
            Block {
                Caption { "Worked muscles" }
                {view_muscles(muscles.iter().map(|(k, v)| (k, v)), "muscle-tag")}
            }
        }
        if !stretch.is_empty() {
            Block {
                Caption { "Stretched muscles" }
                {view_muscles(stretch.iter().map(|(k, v)| (k, v)), "stretch-tag")}
            }
        }
        if !names.is_empty() {
            Block {
                Caption { "Properties" }
                CenteredTags {
                    for name in names {
                        span { class: "tag", "data-testid": "property-tag", {name} }
                    }
                }
            }
        }
        if !equipment.is_empty() {
            Block {
                Caption { "Equipment" }
                CenteredTags {
                    for e in equipment {
                        span { class: "tag", "data-testid": "property-tag", {e.name()} }
                    }
                }
            }
        }
    }
}

pub fn view_muscles<M, I, S>(muscles: M, test_id: &str) -> Element
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
                    description: (!m.description().is_empty()).then(|| rsx! { {m.description()} }),
                    span {
                        class: "tag {class}",
                        "data-testid": test_id,
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

/// The text color matching the tag color of [`stimulus_level_class`].
pub fn stimulus_level_text_class(level: domain::StimulusLevel) -> String {
    stimulus_level_class(level).replacen("is-", "has-text-", 1)
}

fn view_charts(
    exercise_id: domain::ExerciseID,
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
    show_time: bool,
    settings: Settings,
) -> Element {
    let params = web_app::chart::PlotParams::primary_range(0., 10.);

    let is_stretch_only = stretch_only.contains(&exercise_id);
    let mut sets: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_values: Vec<(NaiveDate, f32)> = vec![];
    let mut weight_values: Vec<(NaiveDate, f32)> = vec![];
    let mut time_values: Vec<(NaiveDate, f32)> = vec![];
    let mut estimated_max_reps_by_date: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut one_rep_max_by_date: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    for training_session in training_sessions {
        *sets.entry(training_session.date).or_default() +=
            sets_of(training_session, stretch_only, is_stretch_only);
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

    let measurement_series = measurement_series(exercise_id, training_sessions);

    let time_series = web_app::chart::labeled_min_avg_max(
        "time (s)",
        &time_values,
        interval,
        params,
        web_app::chart::COLOR_TIME,
    );

    rsx! {
        if !measurement_series.is_empty() {
            Chart {
                series: measurement_series,
                interval,
                no_data_label: false,
            }
        }
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
        if show_time {
            Chart {
                series: time_series,
                interval,
                no_data_label: false,
            }
        }
        Chart {
            series: vec![web_app::chart::LabeledSeries::new(
                if is_stretch_only { "Stretch sets" } else { "Set volume" },
                web_app::chart::PlotData {
                    values_high: sets.into_iter().collect::<Vec<_>>(),
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
        if !is_stretch_only {
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
        }
        if show_time {
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

/// A series per side of the measurements, the value of a date taken from its session with the
/// greatest ID, which is the last created one.
fn measurement_series(
    exercise_id: domain::ExerciseID,
    training_sessions: &[domain::TrainingSession],
) -> Vec<web_app::chart::LabeledSeries> {
    let mut training_sessions = training_sessions.iter().collect::<Vec<_>>();
    training_sessions.sort_by_key(|training_session| training_session.id);
    let mut values: BTreeMap<domain::Side, BTreeMap<NaiveDate, f32>> = BTreeMap::new();
    for training_session in training_sessions {
        for (side, measurement) in training_session.measurements_of(exercise_id) {
            #[allow(clippy::cast_possible_truncation)]
            values
                .entry(side)
                .or_default()
                .insert(training_session.date, f64::from(measurement) as f32);
        }
    }
    values
        .into_iter()
        .map(|(side, values)| {
            let (name, color) = match side {
                domain::Side::Unset => ("Measurement", web_app::chart::COLOR_MEASUREMENT),
                domain::Side::Left => ("Left", web_app::chart::COLOR_MEASUREMENT_LEFT),
                domain::Side::Right => ("Right", web_app::chart::COLOR_MEASUREMENT_RIGHT),
            };
            web_app::chart::LabeledSeries::new(
                name,
                web_app::chart::PlotData {
                    values_high: values.into_iter().collect(),
                    values_low: None,
                    plots: web_app::chart::plot_line_with_markers(color),
                    params: web_app::chart::PlotParams::default(),
                },
            )
        })
        .collect()
}

fn view_calendar(
    exercise_id: domain::ExerciseID,
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
) -> Element {
    let is_stretch_only = stretch_only.contains(&exercise_id);
    let mut sets: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            *sets.entry(training_session.date).or_default() +=
                sets_of(training_session, stretch_only, is_stretch_only);
        }
    }
    let min = sets
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
    let max = sets
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
    let entries = sets
        .iter()
        .map(|(date, sets)| {
            (
                *date,
                web_app::chart::COLOR_SET_VOLUME,
                if max > min {
                    (f64::from(sets - min) / f64::from(max - min)) * 0.8 + 0.2
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

/// The stretch sets of a session for a stretch-only exercise, its set volume otherwise.
fn sets_of(
    training_session: &domain::TrainingSession,
    stretch_only: &domain::StretchOnlyExercises,
    is_stretch_only: bool,
) -> f32 {
    if is_stretch_only {
        training_session.stretch_sets(stretch_only)
    } else {
        training_session.set_volume()
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
            Caption { "Notes" }
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
    show_time: bool,
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
        let measurements = t.measurements_of(exercise_id);
        let rows = t.set_history_rows(exercise_id);
        let has_sides = rows
            .iter()
            .any(|row| matches!(row, domain::SetHistoryRow::Sides { .. }));
        let values = |set: &Option<domain::Set>| {
            set.as_ref()
                .map(|set| set.to_string(show_time, settings.show_rpe()))
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
                            to: Route::Routine { id: routine_id, tab: RoutineTab::Routine },
                            match routine {
                                Some(routine) => rsx! { {routine.name.to_string()} },
                                None => rsx! { "-" }
                            }
                        }
                    }
                }
                if !measurements.is_empty() {
                    div {
                        class: "mb-2",
                        {page::training_session::view_measurement_row(
                            &measurements,
                            None::<fn(MouseEvent)>,
                        )}
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
        TestCache, all_text_of, attribute_of, contains, provide_settings, render, rows_of, text_of,
    };

    fn exercise(id: u128, name: &str) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: "some notes".to_string(),
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Pecs,
                stimulus: domain::Stimulus::PRIMARY,
                stretch: domain::Stimulus::NONE,
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
        render_tab(id, ExerciseTab::Exercise, cache)
    }

    fn render_tab(id: u128, tab: ExerciseTab, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_settings(web_app::Settings::default());
            rsx! { Exercise { id: domain::ExerciseID::from(id), tab } }
        })
    }

    #[test]
    fn test_the_properties_and_notes_are_shown() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![exercise(1, "Squat")])
        });

        assert_eq!(text_of(&html, "property-tag"), "Push");
        assert_eq!(text_of(&html, "muscle-tag"), "Pecs");
        assert_eq!(text_of(&html, "exercise-notes"), "some notes");
    }

    #[test]
    fn test_each_group_is_captioned() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![domain::Exercise {
                equipment: vec![domain::Equipment::Barbell],
                ..exercise(1, "Squat")
            }])
        });

        assert_eq!(
            all_text_of(&html, "caption"),
            vec!["Worked muscles", "Properties", "Equipment", "Notes"]
        );
        assert!(!contains(&html, "stretch-tag"));
    }

    #[test]
    fn test_the_stretched_muscles_are_shown_in_a_row_of_their_own() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![domain::Exercise {
                muscles: vec![
                    domain::ExerciseMuscle {
                        muscle_id: domain::MuscleID::ErectorSpinae,
                        stimulus: domain::Stimulus::PRIMARY,
                        stretch: domain::Stimulus::SECONDARY,
                    },
                    domain::ExerciseMuscle {
                        muscle_id: domain::MuscleID::Hamstrings,
                        stimulus: domain::Stimulus::NONE,
                        stretch: domain::Stimulus::PRIMARY,
                    },
                ],
                ..exercise(1, "Jefferson Curl")
            }])
        });

        assert_eq!(
            all_text_of(&html, "caption"),
            vec!["Worked muscles", "Stretched muscles", "Properties", "Notes"]
        );
        assert_eq!(all_text_of(&html, "muscle-tag"), vec!["Erector Spinae"]);
        assert_eq!(
            all_text_of(&html, "stretch-tag"),
            vec!["Hamstrings", "Erector Spinae"]
        );
    }

    #[test]
    fn test_a_stretch_only_exercise_without_other_properties_shows_its_stretched_muscles() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![domain::Exercise {
                muscles: vec![domain::ExerciseMuscle {
                    muscle_id: domain::MuscleID::HipFlexors,
                    stimulus: domain::Stimulus::NONE,
                    stretch: domain::Stimulus::PRIMARY,
                }],
                force: None,
                ..exercise(1, "Couch Stretch")
            }])
        });

        assert_eq!(
            all_text_of(&html, "caption"),
            vec!["Stretched muscles", "Notes"]
        );
        assert!(!contains(&html, "muscle-tag"));
        assert!(!contains(&html, "property-tag"));
        assert_eq!(text_of(&html, "stretch-tag"), "Hip Flexors");
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
    fn test_without_properties_muscles_and_notes_no_data_is_reported() {
        let html = render_exercise(1, || {
            TestCache::default().with_exercises(vec![domain::Exercise {
                notes: String::new(),
                muscles: vec![],
                force: None,
                ..exercise(1, "Squat")
            }])
        });

        assert_eq!(attribute_of(&html, "tab-exercise", "data-selected"), "true");
        assert_eq!(text_of(&html, "no-data"), "No data");
    }

    #[rstest::rstest]
    fn test_without_training_sessions_no_data_is_reported(
        #[values(ExerciseTab::Sessions, ExerciseTab::Charts)] tab: ExerciseTab,
    ) {
        let html = render_tab(1, tab, || {
            TestCache::default().with_exercises(vec![exercise(1, "Squat")])
        });

        assert_eq!(text_of(&html, "no-data"), "No data");
        assert!(!contains(&html, "chart"));
    }

    fn with_a_set() -> TestCache {
        TestCache::default()
            .with_exercises(vec![exercise(1, "Squat")])
            .with_training_sessions(vec![session_with_measurements(1, &[])])
    }

    #[test]
    fn test_the_exercise_tab_shows_the_exercise_and_no_sessions() {
        let html = render_exercise(1, with_a_set);

        assert!(contains(&html, "muscle-tag"));
        assert!(!contains(&html, "table"));
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_the_sessions_tab_shows_the_sessions_and_sets_and_no_exercise() {
        let html = render_tab(1, ExerciseTab::Sessions, with_a_set);

        assert_eq!(attribute_of(&html, "tab-sessions", "data-selected"), "true");
        assert!(contains(&html, "table"));
        assert!(contains(&html, "set-history-set"));
        assert!(!contains(&html, "chart"));
        assert!(!contains(&html, "muscle-tag"));
    }

    #[test]
    fn test_the_charts_tab_shows_the_charts_and_no_exercise() {
        let html = render_tab(1, ExerciseTab::Charts, with_a_set);

        assert_eq!(attribute_of(&html, "tab-charts", "data-selected"), "true");
        assert!(contains(&html, "chart"));
        assert!(!contains(&html, "table"));
        assert!(!contains(&html, "muscle-tag"));
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
        render_tab(1, ExerciseTab::Sessions, move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Lunge")])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: domain::RoutineID::nil(),
                    date: chrono::Local::now().date_naive(),
                    notes: String::new(),
                    elements: elements.clone(),
                    exercise_notes: BTreeMap::new(),
                    exercise_measurements: std::collections::BTreeMap::new(),
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

    /// Render `tab` of the page of an exercise stretching the hamstrings and working them at
    /// `stimulus`, with a 60 s hold and TUT hidden.
    fn render_stretch(stimulus: domain::Stimulus, tab: ExerciseTab) -> String {
        render(move || {
            TestCache::default()
                .with_exercises(vec![domain::Exercise {
                    muscles: vec![domain::ExerciseMuscle {
                        muscle_id: domain::MuscleID::Hamstrings,
                        stimulus,
                        stretch: domain::Stimulus::PRIMARY,
                    }],
                    ..exercise(1, "Forward Fold")
                }])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: domain::RoutineID::nil(),
                    date: chrono::Local::now().date_naive(),
                    notes: String::new(),
                    elements: vec![domain::TrainingSessionElement::Set {
                        exercise_id: 1.into(),
                        side: domain::Side::Unset,
                        reps: domain::Reps::default(),
                        time: domain::Time::new(60).unwrap(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        target_reps: domain::Reps::default(),
                        target_tempo: domain::Tempo::default(),
                        target_weight: domain::Weight::default(),
                        target_rpe: domain::RPE::ZERO,
                        automatic: false,
                    }],
                    exercise_notes: BTreeMap::new(),
                    exercise_measurements: std::collections::BTreeMap::new(),
                }])
                .provide();
            provide_settings(web_app::Settings {
                show_tut: false,
                ..web_app::Settings::default()
            });
            rsx! { Exercise { id: domain::ExerciseID::from(1), tab } }
        })
    }

    #[rstest::rstest]
    #[case::stretch(domain::Stimulus::NONE, true)]
    #[case::hybrid(domain::Stimulus::PRIMARY, false)]
    fn test_the_time_of_a_stretch_is_shown_with_tut_hidden(
        #[case] stimulus: domain::Stimulus,
        #[case] expected: bool,
    ) {
        let html = render_stretch(stimulus, ExerciseTab::Sessions);
        let charts = render_stretch(stimulus, ExerciseTab::Charts);
        let head = &rows_of(&html, "table")[0];

        assert_eq!(text_of(&html, "set-history-set").contains("60 s"), expected);
        assert_eq!(charts.contains("Time under tension (s)"), expected);
        assert_eq!(head.contains(&"TUT".to_string()), expected);
        assert_eq!(head.contains(&"Time (s)".to_string()), expected);
    }

    #[rstest::rstest]
    #[case::stretch(domain::Stimulus::NONE, true)]
    #[case::hybrid(domain::Stimulus::PRIMARY, false)]
    fn test_stretch_sets_replace_the_volume_metrics_of_a_stretch(
        #[case] stimulus: domain::Stimulus,
        #[case] is_stretch_only: bool,
    ) {
        let html = render_stretch(stimulus, ExerciseTab::Sessions);
        let table = rows_of(&html, "table");
        let column = |name: &str| table[0].iter().position(|h| h == name);

        assert_eq!(html.contains("Stretch sets"), is_stretch_only);
        assert_eq!(html.contains("Set volume"), !is_stretch_only);
        assert_eq!(html.contains("Volume load"), !is_stretch_only);
        assert!(!html.contains("Time under stretch (min)"));
        if is_stretch_only {
            assert_eq!(table[1][column("Stretch sets").unwrap()], "1");
        }
    }

    fn session_with_measurements(
        id: u128,
        measurements: &[(domain::Side, f64)],
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: domain::RoutineID::nil(),
            date: chrono::Local::now().date_naive(),
            notes: String::new(),
            elements: vec![performed_set(domain::Side::Unset, 5)],
            exercise_notes: BTreeMap::new(),
            exercise_measurements: measurements
                .iter()
                .map(|(side, value)| ((1.into(), *side), domain::Measurement::new(*value).unwrap()))
                .collect(),
        }
    }

    #[rstest::rstest]
    #[case::without_measurements(&[], &[], None)]
    #[case::side_less(&[(domain::Side::Unset, 4.5)], &["Measurement"], Some("4.5"))]
    #[case::per_side(
        &[(domain::Side::Left, 12.5), (domain::Side::Right, 10.0)],
        &["Left", "Right"],
        Some("L 12.5 · R 10")
    )]
    #[case::negative(&[(domain::Side::Unset, -3.25)], &["Measurement"], Some("-3.25"))]
    fn test_the_measurements_are_shown_in_the_set_history_and_a_chart(
        #[case] measurements: &'static [(domain::Side, f64)],
        #[case] series: &[&str],
        #[case] row: Option<&str>,
    ) {
        let html = render_tab(1, ExerciseTab::Sessions, move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Pancake Stretch")])
                .with_training_sessions(vec![session_with_measurements(1, measurements)])
        });

        assert_eq!(
            contains(&html, "session-exercise-measurement")
                .then(|| text_of(&html, "session-exercise-measurement")),
            row.map(str::to_string)
        );
        assert_eq!(
            measurement_series(1.into(), &[session_with_measurements(1, measurements)])
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>(),
            series
        );
    }

    #[test]
    fn test_the_measurement_of_a_date_is_taken_from_its_last_created_session() {
        let series = measurement_series(
            1.into(),
            &[
                session_with_measurements(2, &[(domain::Side::Unset, 2.0)]),
                session_with_measurements(1, &[(domain::Side::Unset, 1.0)]),
            ],
        );

        assert_eq!(
            series[0].data[0].values_high,
            vec![(chrono::Local::now().date_naive(), 2.0)]
        );
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
