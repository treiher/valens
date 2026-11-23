use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, Property};
use valens_web_app as web_app;

use crate::{
    Route,
    cache::{Cache, CacheState},
    chart::{Calendar, Chart, IntervalControl},
    current_date::current_date,
    eh, page,
    routing::{ExerciseTab, NavigatorScrollExt, RoutineTab},
    settings::Settings,
    ui::element::{
        Caption, CenteredTags, ElementWithDescription, Error, ErrorPage, FloatingActionButton,
        Icon, Loading, LoadingPage, NoData, NoWrap, Table, Tabs, Title,
    },
};

#[component]
pub fn Exercise(id: domain::ExerciseID, tab: ExerciseTab) -> Element {
    let cache = consume_context::<Cache>();
    let current_interval = use_signal(domain::Interval::default);
    let settings = use_context::<Settings>();
    let mut exercise_dialog = use_signal(|| page::exercises::ExerciseDialog::None);
    let training_dialog = use_signal(|| page::training_sessions::TrainingDialog::None);
    let personal_records = use_memo(move || {
        let (CacheState::Ready(training_sessions), CacheState::Ready(exercises)) =
            (&*cache.training_sessions.read(), &*cache.exercises.read())
        else {
            return domain::PersonalRecords::default();
        };
        // Only the sessions are kept whole, as the record sets are indexed by their elements.
        let training_sessions = training_sessions
            .iter()
            .filter(|t| t.exercises().contains(&id))
            .cloned()
            .collect::<Vec<_>>();
        let exercises = exercises
            .iter()
            .filter(|e| e.id == id)
            .cloned()
            .collect::<Vec<_>>();
        domain::PersonalRecords::new(&training_sessions, &exercises)
    });

    match &*cache.exercises.read() {
        CacheState::Ready(exercises) => {
            let exercise = exercises.iter().find(|e| e.id == id);
            if let Some(exercise) = exercise {
                let content = match tab {
                    ExerciseTab::Exercise => {
                        view_exercise(exercise, &personal_records.read(), exercise_dialog)
                    }
                    ExerciseTab::Routines => view_routines(id, &cache),
                    ExerciseTab::Sessions | ExerciseTab::Charts => view_training_sessions(
                        exercise,
                        exercises,
                        tab,
                        &cache,
                        &personal_records.read(),
                        current_interval,
                        training_dialog,
                        settings,
                    ),
                };
                rsx! {
                    Tabs {
                        tabs: vec![("Exercise", ExerciseTab::Exercise), ("Routines", ExerciseTab::Routines), ("Sessions", ExerciseTab::Sessions), ("Charts", ExerciseTab::Charts)],
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

/// Show the properties, the muscles, the notes and the personal records of the exercise.
fn view_exercise(
    exercise: &domain::Exercise,
    personal_records: &domain::PersonalRecords,
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
    let records = personal_records.records(exercise.id, current_date());
    let sides = record_sides(&records);
    let on_edit: Rc<dyn Fn(MouseEvent)> = Rc::new(eh!(mut exercise_dialog; exercise; {
        *exercise_dialog.write() = page::exercises::ExerciseDialog::ChangeProperties {
            exercise: exercise.clone(),
        };
    }));
    let properties = if has_properties || has_muscles {
        view_exercise_properties(
            exercise.force,
            exercise.mechanic,
            exercise.laterality,
            exercise.assistance,
            &exercise.equipment,
            &muscles,
            &stretch,
            exercise.category,
            Some(&on_edit),
        )
    } else {
        let label = if exercise.notes.is_empty() && records.is_empty() {
            "No data"
        } else {
            "No properties"
        };
        rsx! {
            div {
                class: "is-clickable",
                onclick: move |event| on_edit(event),
                NoData { label }
            }
        }
    };
    rsx! {
        {properties}
        {view_notes(exercise, exercise_dialog)}
        {view_personal_records(&records, &sides)}
    }
}

/// The sides of the records table: the left and the right side if any record has one of them,
/// otherwise no side if any record has none, followed by the alternating side if any record has
/// it.
fn record_sides(
    records: &BTreeMap<(domain::Side, domain::RecordValue), domain::Records>,
) -> Vec<domain::Side> {
    let has = |side| records.keys().any(|(s, _)| *s == side);
    let mut sides = if has(domain::Side::Left) || has(domain::Side::Right) {
        vec![domain::Side::Left, domain::Side::Right]
    } else if has(domain::Side::Unset) {
        vec![domain::Side::Unset]
    } else {
        vec![]
    };
    if has(domain::Side::Alternating) {
        sides.push(domain::Side::Alternating);
    }
    sides
}

fn side_label(side: domain::Side) -> String {
    match side {
        domain::Side::Unset => "Combined".to_string(),
        side => side.to_string(),
    }
}

/// Show the all-time and the recent records in a row per value, holding the records of all
/// `sides` in one cell, in the order named below the column headers unless the only side is none.
fn view_personal_records(
    records: &BTreeMap<(domain::Side, domain::RecordValue), domain::Records>,
    sides: &[domain::Side],
) -> Element {
    if records.is_empty() {
        return rsx! {};
    }
    let values = records
        .keys()
        .map(|(_, value)| *value)
        .collect::<BTreeSet<_>>();
    let groups = [
        (
            "Body weight",
            [domain::RecordValue::Reps, domain::RecordValue::Time],
        ),
        (
            "With weight",
            [domain::RecordValue::Weight, domain::RecordValue::OneRepMax],
        ),
    ]
    .map(|(label, group)| {
        (
            label,
            group
                .into_iter()
                .filter(|value| values.contains(value))
                .collect::<Vec<_>>(),
        )
    });
    let is_grouped = groups.iter().all(|(_, group)| !group.is_empty());
    let header_class = page::training_session::COLUMN_HEADER_CLASS;
    let side_order = (sides != [domain::Side::Unset]).then(|| {
        sides
            .iter()
            .map(|side| side_label(*side))
            .collect::<Vec<_>>()
            .join(" / ")
    });
    rsx! {
        div {
            class: "block has-text-centered",
            Caption { "Personal records" }
            table {
                class: "mx-auto",
                "data-testid": "personal-records",
                tr {
                    class: if side_order.is_some() { "" } else { "has-border-bottom" },
                    td {}
                    for (kind, label) in [(domain::RecordKind::AllTime, "All-time"), (domain::RecordKind::Recent, "6 months")] {
                        th {
                            class: "px-2 pb-1 has-text-centered has-text-strong has-text-weight-bold",
                            scope: "col",
                            span {
                                class: "icon-text is-flex-wrap-nowrap is-align-items-center",
                                Icon {
                                    name: "trophy",
                                    is_small: true,
                                    class: page::training_session::record_class(kind),
                                }
                                span { "{label}" }
                            }
                        }
                    }
                }
                if let Some(side_order) = &side_order {
                    tr {
                        class: "has-border-bottom",
                        td {}
                        td { class: "px-2 has-text-centered is-size-7 has-text-weak", "{side_order}" }
                        td { class: "px-2 has-text-centered is-size-7 has-text-weak", "{side_order}" }
                    }
                }
                for (i, (label, group)) in groups.into_iter().enumerate() {
                    if is_grouped && !group.is_empty() {
                        tr {
                            class: if i > 0 { "has-border-top" },
                            th {
                                class: "{header_class} pt-2 has-text-left",
                                colspan: 3,
                                scope: "rowgroup",
                                "{label}"
                            }
                        }
                    }
                    for value in group {
                        tr {
                            th {
                                class: "px-2 has-text-left has-text-weight-normal",
                                scope: "row",
                                NoWrap { {value_label(value)} }
                            }
                            td {
                                class: "px-2 has-text-centered",
                                {view_record_cell(records, value, sides, |r| Some(r.all_time))}
                            }
                            td {
                                class: "px-2 has-text-centered",
                                {view_record_cell(records, value, sides, |r| r.recent)}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn value_label(value: domain::RecordValue) -> &'static str {
    match value {
        domain::RecordValue::Reps => "Reps",
        domain::RecordValue::Time => "Time (s)",
        domain::RecordValue::Weight => "Weight (kg)",
        domain::RecordValue::OneRepMax => "Est. 1RM (kg)",
    }
}

/// Show the record selected by `select` of each side, linking to the session that set it, with
/// its date below.
///
/// The records are a single value where the sides are equal, unless one of them is the alternating
/// side, as a single value would read as a record of one kind only.
fn view_record_cell(
    records: &BTreeMap<(domain::Side, domain::RecordValue), domain::Records>,
    value: domain::RecordValue,
    sides: &[domain::Side],
    select: impl Fn(&domain::Records) -> Option<domain::Record>,
) -> Element {
    let mergeable = sides.len() == 1 || !sides.contains(&domain::Side::Alternating);
    let records = sides
        .iter()
        .map(|side| {
            records
                .get(&(*side, value))
                .and_then(&select)
                .map(|record| (record_to_string(value, record.value), record))
        })
        .collect::<Vec<_>>();
    let link = |(text, record): &(String, domain::Record)| {
        rsx! {
            Link {
                to: Route::TrainingSession { id: record.session_id },
                "{text}"
            }
        }
    };
    let dash = rsx! { span { class: "has-text-weak", "–" } };
    let held = records.iter().flatten().collect::<Vec<_>>();
    let Some(latest) = held
        .iter()
        .max_by_key(|(_, record)| (record.date, record.session_id))
    else {
        return dash;
    };
    if mergeable && held.len() == records.len() && held.iter().all(|(text, _)| *text == latest.0) {
        // A single value links to the latest session reaching it.
        return rsx! {
            {link(latest)}
            {view_record_date("", latest.1.date)}
        };
    }
    rsx! {
        NoWrap {
            for (i, record) in records.iter().enumerate() {
                if i > 0 { " / " }
                match record {
                    Some(record) => link(record),
                    None => dash.clone(),
                }
            }
        }
        for (side, record) in sides.iter().zip(&records) {
            if let Some((_, record)) = record {
                {view_record_date(&format!("{} ", side_label(*side)), record.date)}
            }
        }
    }
}

fn view_record_date(prefix: &str, date: NaiveDate) -> Element {
    rsx! {
        div {
            class: "is-size-8 has-text-weak",
            "data-testid": "record-date",
            NoWrap { "{prefix}{date}" }
        }
    }
}

fn record_to_string(value: domain::RecordValue, number: f32) -> String {
    match value {
        domain::RecordValue::OneRepMax => format!("{number:.1}"),
        domain::RecordValue::Reps | domain::RecordValue::Time | domain::RecordValue::Weight => {
            format!("{number}")
        }
    }
}

/// Show the routines containing the exercise, the archived ones separately.
fn view_routines(id: domain::ExerciseID, cache: &Cache) -> Element {
    match (&*cache.routines.read(), &*cache.training_sessions.read()) {
        (CacheState::Ready(routines), CacheState::Ready(training_sessions)) => {
            let (archived_routines, current_routines): (Vec<_>, Vec<_>) =
                domain::routines_sorted_by_last_use(routines, training_sessions, |r| {
                    r.exercises().contains(&id)
                })
                .into_iter()
                .partition(|r| r.archived);
            let body = |routines: Vec<domain::Routine>| {
                routines
                    .into_iter()
                    .map(|r| {
                        vec![rsx! {
                            Link {
                                to: Route::Routine { id: r.id, tab: RoutineTab::Routine },
                                "{r.name}"
                            }
                        }]
                    })
                    .collect::<Vec<_>>()
            };
            let current_routines = body(current_routines);
            let archived_routines = body(archived_routines);
            if current_routines.is_empty() && archived_routines.is_empty() {
                return rsx! { NoData {} };
            }
            rsx! {
                if !current_routines.is_empty() {
                    Table { body: current_routines }
                }
                if !archived_routines.is_empty() {
                    Title { "Archive" }
                    Table { body: archived_routines }
                }
            }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { Error { message: "{err}" } }
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => rsx! { Loading {} },
    }
}

/// Show the sessions holding the exercise and its sets on the sessions tab, and their charts on
/// the charts tab.
#[allow(clippy::too_many_arguments)]
fn view_training_sessions(
    exercise: &domain::Exercise,
    exercises: &[domain::Exercise],
    tab: ExerciseTab,
    cache: &Cache,
    personal_records: &domain::PersonalRecords,
    mut current_interval: Signal<domain::Interval>,
    training_dialog: Signal<page::training_sessions::TrainingDialog>,
    settings: Settings,
) -> Element {
    let id = exercise.id;
    // A hold without its time reads as an empty set.
    let show_time = settings.show_tut() || exercise.is_stretch_only();
    let stretch_only = domain::StretchOnlyExercises::new(exercises);
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
                    {view_sets(id, &sessions_with_exercise, personal_records, routines, show_time, settings, training_dialog)}
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
    on_edit: Option<&Rc<dyn Fn(MouseEvent)>>,
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
            {view_block(
                "Worked muscles",
                "exercise-worked-muscles",
                on_edit.cloned(),
                view_muscles(muscles.iter().map(|(k, v)| (k, v)), "muscle-tag"),
            )}
        }
        if !stretch.is_empty() {
            {view_block(
                "Stretched muscles",
                "exercise-stretched-muscles",
                on_edit.cloned(),
                view_muscles(stretch.iter().map(|(k, v)| (k, v)), "stretch-tag"),
            )}
        }
        if !names.is_empty() {
            {view_block(
                "Properties",
                "exercise-properties",
                on_edit.cloned(),
                rsx! {
                    CenteredTags {
                        for name in names {
                            span { class: "tag", "data-testid": "property-tag", {name} }
                        }
                    }
                },
            )}
        }
        if !equipment.is_empty() {
            {view_block(
                "Equipment",
                "exercise-equipment",
                on_edit.cloned(),
                rsx! {
                    CenteredTags {
                        for e in equipment {
                            span { class: "tag", "data-testid": "property-tag", {e.name()} }
                        }
                    }
                },
            )}
        }
    }
}

pub fn view_block(
    caption: &str,
    test_id: &str,
    on_edit: Option<Rc<dyn Fn(MouseEvent)>>,
    content: Element,
) -> Element {
    rsx! {
        div {
            class: "block",
            class: if on_edit.is_some() { "is-clickable" },
            "data-testid": test_id,
            onclick: move |event| {
                if let Some(on_edit) = &on_edit {
                    on_edit(event);
                }
            },
            Caption { {caption} }
            {content}
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
                span {
                    class: "is-flex",
                    onclick: |event| event.stop_propagation(),
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
}

pub fn stimulus_level_class(level: domain::StimulusLevel) -> &'static str {
    match level {
        domain::StimulusLevel::Primary => "is-strong",
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
    let reps_values = reps_values(training_sessions);
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
            if let domain::TrainingSessionElement::Set { weight, time, .. } = element {
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

/// The reps of each set by date, half of them for an alternating set, so that they compare to the
/// estimated maximum reps.
fn reps_values(training_sessions: &[domain::TrainingSession]) -> Vec<(NaiveDate, f32)> {
    training_sessions
        .iter()
        .flat_map(|training_session| {
            training_session
                .elements
                .iter()
                .filter_map(|element| Some((training_session.date, element.reps_per_side()?)))
        })
        .collect()
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
                domain::Side::Unset | domain::Side::Alternating => {
                    ("Measurement", web_app::chart::COLOR_MEASUREMENT)
                }
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
        div {
            class: "block has-text-centered is-clickable",
            onclick: eh!(exercise; {
                *exercise_dialog.write() = page::exercises::ExerciseDialog::EditNotes {
                    exercise: exercise.clone(),
                };
            }),
            Caption { "Notes" }
            div {
                class: "is-italic has-text-centered is-preserving-line-breaks",
                "data-testid": "exercise-notes",
                { notes.clone() }
            }
        }
    }
}

/// Renders the performed sets of an exercise as a list of its training sessions.
///
/// The training sessions carry all their elements, since a left and a right set form a pair only
/// where they directly follow each other in the whole session, and the record sets are indexed by
/// the elements of the whole session.
#[allow(clippy::too_many_arguments)]
fn view_sets(
    exercise_id: domain::ExerciseID,
    training_sessions: &[&domain::TrainingSession],
    personal_records: &domain::PersonalRecords,
    routines: &[domain::Routine],
    show_time: bool,
    settings: Settings,
    training_dialog: Signal<page::training_sessions::TrainingDialog>,
) -> Element {
    let body = training_sessions
        .iter()
        .rev()
        .map(|t| {
            let record_sets = personal_records.record_sets(t.id);
            let note = t
                .exercise_notes
                .get(&exercise_id)
                .cloned()
                .unwrap_or_default();
            let measurements = t.measurements_of(exercise_id);
            let groups = t
                .groups()
                .into_iter()
                .filter_map(|group| performed_sets_of(group, exercise_id))
                .collect::<Vec<_>>();
            page::training_sessions::view_session_item(
                t,
                routines,
                training_dialog,
                rsx! {
                    if !measurements.is_empty() {
                        {page::training_session::view_measurement_row(
                            &measurements,
                            None::<fn(MouseEvent)>,
                        )}
                    }
                    if !note.is_empty() {
                        div {
                            class: "is-italic",
                            "data-testid": "session-exercise-notes",
                            {note}
                        }
                    }
                    table {
                        "data-testid": "set-history-sets",
                        {page::training_session::view_set_rows(
                            &groups,
                            &[],
                            &record_sets,
                            show_time,
                            settings.show_rpe(),
                        )}
                    }
                },
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { body }
    }
}

/// The performed sets of `exercise_id` in `group`, if any
fn performed_sets_of(
    group: domain::ElementGroup<'_>,
    exercise_id: domain::ExerciseID,
) -> Option<domain::ElementGroup<'_>> {
    let is_performed_set = |(_, element): &(usize, &domain::TrainingSessionElement)| {
        matches!(element, domain::TrainingSessionElement::Set { exercise_id: id, .. } if *id == exercise_id)
            && !element.is_empty()
    };
    match group {
        domain::ElementGroup::Sides { left, right } => {
            let left = left.filter(is_performed_set);
            let right = right.filter(is_performed_set);
            (left.is_some() || right.is_some())
                .then_some(domain::ElementGroup::Sides { left, right })
        }
        domain::ElementGroup::Single(idx, element) => {
            is_performed_set(&(idx, element)).then_some(group)
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, all_containing, all_text_of, attribute_of, contains, links_of, provide_settings,
        render, rows_containing, rows_of, rows_of_excluding, rows_of_nth, text_of,
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
        assert!(contains(&html, "set-history-sets"));
        assert!(!html.contains("<thead"));
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

    fn routine(id: u128, name: &str, archived: bool, exercise_id: u128) -> domain::Routine {
        domain::Routine {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: String::new(),
            archived,
            sections: vec![domain::RoutinePart::RoutineSection {
                rounds: domain::Rounds::new(1).unwrap(),
                parts: vec![domain::RoutinePart::RoutineSection {
                    rounds: domain::Rounds::new(1).unwrap(),
                    parts: vec![domain::RoutinePart::RoutineActivity {
                        exercise_id: exercise_id.into(),
                        reps: domain::Reps::default(),
                        tempo: domain::Tempo::default(),
                        weight: domain::Weight::default(),
                        rpe: domain::RPE::ZERO,
                        automatic: false,
                        countdown: false,
                        sides: domain::Sides::Combined,
                    }],
                }],
            }],
        }
    }

    #[test]
    fn test_the_routines_tab_lists_the_routines_containing_the_exercise() {
        let html = render_tab(1, ExerciseTab::Routines, || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat"), exercise(2, "Bench Press")])
                .with_routines(vec![
                    routine(1, "Legs", false, 1),
                    routine(2, "Push", false, 2),
                    routine(3, "Retired", true, 1),
                    routine(4, "Retired Push", true, 2),
                ])
        });

        assert_eq!(attribute_of(&html, "tab-routines", "data-selected"), "true");
        assert_eq!(rows_of(&html, "table"), vec![vec!["Legs"]]);
        assert_eq!(rows_of_nth(&html, "table", 1), vec![vec!["Retired"]]);
        assert!(html.contains("Archive"), "{html}");
        assert!(!contains(&html, "muscle-tag"));
    }

    #[test]
    fn test_the_routines_tab_without_archived_routines_shows_no_archive() {
        let html = render_tab(1, ExerciseTab::Routines, || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_routines(vec![routine(1, "Legs", false, 1)])
        });

        assert_eq!(rows_of(&html, "table"), vec![vec!["Legs"]]);
        assert!(!html.contains("Archive"), "{html}");
    }

    #[test]
    fn test_the_routines_tab_with_only_archived_routines_shows_only_the_archive() {
        let html = render_tab(1, ExerciseTab::Routines, || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_routines(vec![routine(1, "Retired", true, 1)])
        });

        assert_eq!(all_text_of(&html, "title"), vec!["Archive"]);
        assert_eq!(rows_of(&html, "table"), vec![vec!["Retired"]]);
    }

    #[test]
    fn test_the_routines_tab_without_routines_containing_the_exercise_reports_no_data() {
        let html = render_tab(1, ExerciseTab::Routines, || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Squat")])
                .with_routines(vec![routine(1, "Push", false, 2)])
        });

        assert_eq!(text_of(&html, "no-data"), "No data");
        assert!(!contains(&html, "table"));
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
            countdown: false,
        }
    }

    fn render_exercise_with_sets(elements: Vec<domain::TrainingSessionElement>) -> String {
        render_exercise_with_sets_and_settings(elements, web_app::Settings::default())
    }

    fn render_exercise_with_sets_and_settings(
        elements: Vec<domain::TrainingSessionElement>,
        settings: web_app::Settings,
    ) -> String {
        render(move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Lunge"), exercise(2, "Squat")])
                .with_training_sessions(vec![domain::TrainingSession {
                    id: 1.into(),
                    routine_id: domain::RoutineID::nil(),
                    date: chrono::NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
                    notes: String::new(),
                    elements: elements.clone(),
                    exercise_notes: BTreeMap::new(),
                    exercise_measurements: std::collections::BTreeMap::new(),
                }])
                .provide();
            provide_settings(settings);
            rsx! { Exercise { id: domain::ExerciseID::from(1), tab: ExerciseTab::Sessions } }
        })
    }

    fn set_of_other_exercise() -> domain::TrainingSessionElement {
        let mut set = performed_set(domain::Side::Unset, 5);
        if let domain::TrainingSessionElement::Set { exercise_id, .. } = &mut set {
            *exercise_id = 2.into();
        }
        set
    }

    fn rest() -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Rest {
            target_time: domain::Time::default(),
            automatic: false,
        }
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
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "Left", "Right", ""],
                vec!["", "10", "9", ""],
                vec!["", "", "8", ""],
                vec!["", "7", "", "", "", ""],
            ]
        );
    }

    #[test]
    fn test_the_reps_chart_plots_half_the_reps_of_an_alternating_set() {
        let session = domain::TrainingSession {
            id: 1.into(),
            routine_id: domain::RoutineID::nil(),
            date: chrono::Local::now().date_naive(),
            notes: String::new(),
            elements: vec![
                performed_set(domain::Side::Alternating, 11),
                performed_set(domain::Side::Unset, 6),
            ],
            exercise_notes: BTreeMap::new(),
            exercise_measurements: BTreeMap::new(),
        };

        assert_eq!(
            reps_values(&[session])
                .into_iter()
                .map(|(_, reps)| reps)
                .collect::<Vec<_>>(),
            [5.5, 6.0]
        );
    }

    #[test]
    fn test_the_set_history_of_alternating_sets_only_has_no_side_header() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Alternating, 20),
            performed_set(domain::Side::Unset, 10),
        ]);

        assert!(!contains(&html, "side-header"));
        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["L+R", "20", "", "", "", ""],
                vec!["", "10", "", "", "", ""],
            ]
        );
    }

    #[test]
    fn test_the_set_history_shows_an_alternating_set_after_a_pair_like_a_set_without_a_side() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Left, 10),
            performed_set(domain::Side::Right, 9),
            performed_set(domain::Side::Alternating, 20),
        ]);

        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "Left", "Right", ""],
                vec!["", "10", "9", ""],
                vec!["L+R", "20", "", "", "", ""],
            ]
        );
    }

    #[test]
    fn test_the_set_history_without_sides_has_no_side_header() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Unset, 10),
            performed_set(domain::Side::Unset, 9),
        ]);

        assert!(!contains(&html, "side-header"));
        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "10", "", "", "", ""],
                vec!["", "9", "", "", "", ""],
            ]
        );
    }

    #[test]
    fn test_a_row_of_the_set_history_has_the_date_and_a_delete_icon() {
        let html = render_exercise_with_sets(vec![performed_set(domain::Side::Unset, 10)]);

        assert_eq!(text_of(&html, "session-title"), "2026-11-02");
        assert!(contains(&html, "item-delete"));
    }

    #[rstest::rstest]
    #[case::shown(true, vec!["", "10 ×", "30 s", "", "@ 8", ""])]
    #[case::hidden(false, vec!["", "10", "", "", "", ""])]
    fn test_the_set_history_shows_the_time_and_the_rpe_following_the_settings(
        #[case] shown: bool,
        #[case] expected: Vec<&str>,
    ) {
        let mut set = performed_set(domain::Side::Unset, 10);
        if let domain::TrainingSessionElement::Set { time, rpe, .. } = &mut set {
            *time = domain::Time::new(30).unwrap();
            *rpe = domain::RPE::EIGHT;
        }
        let html = render_exercise_with_sets_and_settings(
            vec![set],
            web_app::Settings {
                show_rpe: shown,
                show_tut: shown,
                ..web_app::Settings::default()
            },
        );

        assert_eq!(rows_of(&html, "set-history-sets"), vec![expected]);
    }

    #[test]
    fn test_a_set_whose_values_are_all_hidden_shows_a_dash() {
        let sets = [domain::Side::Unset, domain::Side::Left].map(|side| {
            let mut set = performed_set(side, 0);
            if let domain::TrainingSessionElement::Set { time, rpe, .. } = &mut set {
                *time = domain::Time::new(30).unwrap();
                *rpe = domain::RPE::EIGHT;
            }
            set
        });
        let html = render_exercise_with_sets_and_settings(
            sets.to_vec(),
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
        );

        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["–"],
                vec!["", "Left", "Right", ""],
                vec!["", "–", "", ""],
            ]
        );
    }

    #[test]
    fn test_the_set_history_leaves_out_rests_and_unperformed_sets() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Unset, 10),
            rest(),
            performed_set(domain::Side::Unset, 0),
            performed_set(domain::Side::Left, 0),
            performed_set(domain::Side::Right, 8),
            performed_set(domain::Side::Left, 0),
            performed_set(domain::Side::Right, 0),
        ]);

        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "10", "", "", "", ""],
                vec!["", "Left", "Right", ""],
                vec!["", "", "8", ""],
            ]
        );
    }

    #[test]
    fn test_sides_separated_by_another_exercise_or_a_rest_stay_in_rows_of_their_own() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Left, 10),
            set_of_other_exercise(),
            performed_set(domain::Side::Right, 9),
            rest(),
            performed_set(domain::Side::Left, 8),
        ]);

        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "Left", "Right", ""],
                vec!["", "10", "", ""],
                vec!["", "", "9", ""],
                vec!["", "8", "", ""],
            ]
        );
    }

    #[test]
    fn test_a_session_with_pairs_in_two_sections_shows_one_side_header() {
        let html = render_exercise_with_sets(vec![
            performed_set(domain::Side::Left, 10),
            performed_set(domain::Side::Right, 9),
            rest(),
            set_of_other_exercise(),
            rest(),
            performed_set(domain::Side::Left, 8),
            performed_set(domain::Side::Right, 7),
        ]);

        assert_eq!(
            rows_of(&html, "set-history-sets"),
            vec![
                vec!["", "Left", "Right", ""],
                vec!["", "10", "9", ""],
                vec!["", "8", "7", ""],
            ]
        );
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
                        countdown: false,
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

        assert_eq!(text_of(&html, "set-row").contains("60 s"), expected);
        assert_eq!(charts.contains("Time under tension (s)"), expected);
    }

    #[rstest::rstest]
    #[case::stretch(domain::Stimulus::NONE, true)]
    #[case::hybrid(domain::Stimulus::PRIMARY, false)]
    fn test_stretch_sets_replace_the_volume_metrics_of_a_stretch(
        #[case] stimulus: domain::Stimulus,
        #[case] is_stretch_only: bool,
    ) {
        let html = render_stretch(stimulus, ExerciseTab::Charts);

        assert_eq!(html.contains("Stretch sets"), is_stretch_only);
        assert_eq!(html.contains("Set volume"), !is_stretch_only);
        if is_stretch_only {
            assert!(!html.contains("Volume load"));
        }
        assert!(!html.contains("Time under stretch (min)"));
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

    fn values(
        side: domain::Side,
        reps: u32,
        time: u32,
        weight: f32,
    ) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: 1.into(),
            side,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::new(time).unwrap(),
            weight: domain::Weight::new(weight).unwrap(),
            rpe: domain::RPE::ZERO,
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
        }
    }

    fn reps(reps: u32) -> domain::TrainingSessionElement {
        values(domain::Side::Unset, reps, 0, 0.0)
    }

    fn sided_reps(side: domain::Side, reps: u32) -> domain::TrainingSessionElement {
        values(side, reps, 0, 0.0)
    }

    fn hold(time: u32) -> domain::TrainingSessionElement {
        values(domain::Side::Unset, 0, time, 0.0)
    }

    fn lift(reps: u32, weight: f32) -> domain::TrainingSessionElement {
        values(domain::Side::Unset, reps, 0, weight)
    }

    fn session(
        id: u128,
        days_ago: i64,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: domain::RoutineID::nil(),
            date: chrono::Local::now().date_naive() - chrono::Duration::days(days_ago),
            notes: String::new(),
            elements,
            exercise_notes: BTreeMap::new(),
            exercise_measurements: BTreeMap::new(),
        }
    }

    fn session_link(id: u128) -> String {
        Route::TrainingSession { id: id.into() }.to_string()
    }

    fn render_records(
        tab: ExerciseTab,
        settings: web_app::Settings,
        sessions: Vec<domain::TrainingSession>,
    ) -> String {
        render(move || {
            TestCache::default()
                .with_exercises(vec![exercise(1, "Pull Up")])
                .with_training_sessions(sessions.clone())
                .provide();
            provide_settings(settings);
            rsx! { Exercise { id: domain::ExerciseID::from(1), tab } }
        })
    }

    fn records_table(sessions: Vec<domain::TrainingSession>) -> Vec<Vec<String>> {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            sessions,
        );
        rows_of_excluding(&html, "personal-records", "record-date")
    }

    fn set_history(sessions: Vec<domain::TrainingSession>) -> String {
        render_records(
            ExerciseTab::Sessions,
            web_app::Settings::default(),
            sessions,
        )
    }

    #[test]
    fn test_the_records_table_shows_a_value_per_kind_after_a_single_session() {
        assert_eq!(
            records_table(vec![session(
                1,
                0,
                vec![reps(10), hold(30), lift(5, 100.0)]
            )]),
            [
                vec!["", "All-time", "6 months"],
                vec!["Body weight"],
                vec!["Reps", "10", "10"],
                vec!["Time (s)", "30", "30"],
                vec!["With weight"],
                vec!["Weight (kg)", "100", "100"],
                vec!["Est. 1RM (kg)", "112.5", "112.5"],
            ]
        );
    }

    #[test]
    fn test_the_records_table_shows_a_recent_record_below_and_one_equal_to_the_all_time_record() {
        assert_eq!(
            records_table(vec![
                session(1, 400, vec![reps(20), hold(30)]),
                session(2, 10, vec![reps(12), hold(40)]),
            ])[1..],
            [vec!["Reps", "20", "12"], vec!["Time (s)", "40", "40"]]
        );
    }

    #[test]
    fn test_the_records_table_shows_a_dash_without_a_recent_record() {
        assert_eq!(
            records_table(vec![session(1, 400, vec![reps(20)])]),
            [vec!["", "All-time", "6 months"], vec!["Reps", "20", "–"]]
        );
    }

    #[rstest::rstest]
    #[case::body_weight(vec![reps(10), hold(30)], &["Reps", "Time (s)"])]
    #[case::with_weight(vec![lift(5, 100.0)], &["Weight (kg)", "Est. 1RM (kg)"])]
    fn test_the_records_table_has_no_group_headers_with_records_in_one_group(
        #[case] elements: Vec<domain::TrainingSessionElement>,
        #[case] expected: &[&str],
    ) {
        assert_eq!(
            records_table(vec![session(1, 0, elements)])[1..]
                .iter()
                .map(|row| row[0].clone())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn test_a_records_cell_shows_differing_sides_and_a_missing_side() {
        assert_eq!(
            records_table(vec![
                session(1, 400, vec![sided_reps(domain::Side::Left, 11)]),
                session(
                    2,
                    10,
                    vec![
                        sided_reps(domain::Side::Left, 10),
                        sided_reps(domain::Side::Right, 12)
                    ]
                ),
            ]),
            [
                vec!["", "All-time", "6 months"],
                vec!["", "L / R", "L / R"],
                vec!["Reps", "11 / 12", "10 / 12"],
            ]
        );
    }

    #[test]
    fn test_a_records_cell_shows_a_missing_side_as_a_dash() {
        assert_eq!(
            records_table(vec![session(
                1,
                0,
                vec![sided_reps(domain::Side::Left, 10)]
            )])[2],
            ["Reps", "10 / –", "10 / –"]
        );
    }

    #[test]
    fn test_a_records_cell_shows_equal_sides_as_a_single_value_linking_to_the_later_session() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 20, vec![sided_reps(domain::Side::Left, 8)]),
                session(2, 10, vec![sided_reps(domain::Side::Right, 8)]),
            ],
        );

        assert_eq!(
            rows_of_excluding(&html, "personal-records", "record-date")[2],
            ["Reps", "8", "8"]
        );
        assert_eq!(
            links_of(&html, "personal-records"),
            [session_link(2), session_link(2)]
        );
    }

    #[test]
    fn test_records_without_a_side_count_for_both_sides_once_records_per_side_exist() {
        assert_eq!(
            records_table(vec![
                session(1, 20, vec![reps(10), hold(30)]),
                session(
                    2,
                    10,
                    vec![
                        sided_reps(domain::Side::Left, 12),
                        sided_reps(domain::Side::Right, 9)
                    ]
                ),
            ]),
            [
                vec!["", "All-time", "6 months"],
                vec!["", "L / R", "L / R"],
                vec!["Reps", "12 / 10", "12 / 10"],
                vec!["Time (s)", "30", "30"],
            ]
        );
    }

    #[test]
    fn test_a_records_cell_adds_the_alternating_record_to_the_records_per_side_unmerged() {
        assert_eq!(
            records_table(vec![session(
                1,
                0,
                vec![
                    sided_reps(domain::Side::Left, 10),
                    sided_reps(domain::Side::Right, 10),
                    sided_reps(domain::Side::Alternating, 22),
                ]
            )]),
            [
                vec!["", "All-time", "6 months"],
                vec!["", "L / R / L+R", "L / R / L+R"],
                vec!["Reps", "10 / 10 / 22", "10 / 10 / 22"],
            ]
        );
    }

    #[test]
    fn test_a_records_cell_adds_the_alternating_record_to_the_record_without_a_side_unmerged() {
        assert_eq!(
            records_table(vec![session(
                1,
                0,
                vec![reps(20), sided_reps(domain::Side::Alternating, 20),]
            )]),
            [
                vec!["", "All-time", "6 months"],
                vec!["", "Combined / L+R", "Combined / L+R"],
                vec!["Reps", "20 / 20", "20 / 20"],
            ]
        );
    }

    #[test]
    fn test_the_dates_of_a_combined_and_an_alternating_record_are_labelled() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 20, vec![reps(20)]),
                session(2, 10, vec![sided_reps(domain::Side::Alternating, 22)]),
            ],
        );

        let dates = [
            format!("Combined {}", date(20)),
            format!("L+R {}", date(10)),
        ];
        assert_eq!(
            all_text_of(&html, "record-date"),
            [dates.clone(), dates].concat()
        );
    }

    #[test]
    fn test_the_records_table_of_alternating_records_only_shows_single_values() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![session(
                1,
                10,
                vec![sided_reps(domain::Side::Alternating, 22)],
            )],
        );

        assert_eq!(
            rows_of_excluding(&html, "personal-records", "record-date"),
            [
                vec!["", "All-time", "6 months"],
                vec!["", "L+R", "L+R"],
                vec!["Reps", "22", "22"],
            ]
        );
        assert_eq!(all_text_of(&html, "record-date"), [date(10), date(10)]);
    }

    fn date(days_ago: i64) -> String {
        (chrono::Local::now().date_naive() - chrono::Duration::days(days_ago)).to_string()
    }

    #[test]
    fn test_the_date_of_a_record_is_shown_below_its_value() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 400, vec![reps(12)]),
                session(2, 10, vec![reps(10)]),
            ],
        );

        assert_eq!(all_text_of(&html, "record-date"), [date(400), date(10)]);
    }

    #[test]
    fn test_the_records_per_side_show_the_date_of_each_side() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 20, vec![sided_reps(domain::Side::Left, 12)]),
                session(2, 10, vec![sided_reps(domain::Side::Right, 10)]),
            ],
        );

        let dates = [format!("L {}", date(20)), format!("R {}", date(10))];
        assert_eq!(
            all_text_of(&html, "record-date"),
            [dates.clone(), dates].concat()
        );
    }

    #[test]
    fn test_an_equal_record_of_both_sides_shows_the_date_of_the_later_session() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 20, vec![sided_reps(domain::Side::Left, 10)]),
                session(2, 10, vec![sided_reps(domain::Side::Right, 10)]),
            ],
        );

        assert_eq!(all_text_of(&html, "record-date"), [date(10), date(10)]);
    }

    #[test]
    fn test_an_equal_record_without_a_side_links_to_the_earlier_session() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 20, vec![reps(10)]),
                session(2, 10, vec![sided_reps(domain::Side::Left, 10)]),
            ],
        );

        assert_eq!(
            rows_of_excluding(&html, "personal-records", "record-date")[2],
            ["Reps", "10", "10"]
        );
        assert_eq!(
            links_of(&html, "personal-records"),
            [session_link(1), session_link(1)]
        );
    }

    #[test]
    fn test_a_recent_record_held_by_two_sessions_links_to_the_first() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings::default(),
            vec![
                session(1, 400, vec![reps(20)]),
                session(2, 20, vec![reps(10)]),
                session(3, 10, vec![reps(10)]),
            ],
        );

        assert_eq!(
            links_of(&html, "personal-records"),
            [session_link(1), session_link(2)]
        );
    }

    #[test]
    fn test_estimated_one_rep_maxes_of_both_sides_rounding_to_the_same_value_show_a_single_value() {
        let sided_lift = |side, weight| values(side, 5, 0, weight);
        assert_eq!(
            records_table(vec![session(
                1,
                0,
                vec![
                    sided_lift(domain::Side::Left, 100.0),
                    sided_lift(domain::Side::Right, 100.01)
                ]
            )])[2..],
            [
                vec!["Weight (kg)", "100 / 100.01", "100 / 100.01"],
                vec!["Est. 1RM (kg)", "112.5", "112.5"],
            ]
        );
    }

    #[test]
    fn test_the_records_table_shows_the_time_while_the_time_under_tension_is_hidden() {
        let html = render_records(
            ExerciseTab::Exercise,
            web_app::Settings {
                show_tut: false,
                ..web_app::Settings::default()
            },
            vec![session(1, 0, vec![hold(30)])],
        );

        assert_eq!(
            rows_of_excluding(&html, "personal-records", "record-date")[1],
            ["Time (s)", "30", "30"]
        );
    }

    #[test]
    fn test_the_records_table_is_shown_below_a_placeholder_for_missing_properties() {
        let html = render(|| {
            TestCache::default()
                .with_exercises(vec![domain::Exercise {
                    notes: String::new(),
                    muscles: vec![],
                    force: None,
                    ..exercise(1, "Pull Up")
                }])
                .with_training_sessions(vec![session(1, 0, vec![reps(10)])])
                .provide();
            provide_settings(web_app::Settings::default());
            rsx! { Exercise { id: domain::ExerciseID::from(1), tab: ExerciseTab::Exercise } }
        });

        assert!(contains(&html, "personal-records"));
        assert_eq!(text_of(&html, "no-data"), "No properties");
    }

    #[test]
    fn test_a_set_that_a_session_before_the_interval_beats_carries_no_marker() {
        let html = set_history(vec![
            session(1, 100, vec![reps(20)]),
            session(2, 50, vec![reps(10)]),
            session(3, 0, vec![reps(15)]),
        ]);

        assert_eq!(all_text_of(&html, "set-row"), ["15", "10"]);
        assert!(!contains(&html, "personal-record"));
        assert!(!contains(&html, "recent-personal-record"));
    }

    #[test]
    fn test_the_marker_ends_the_line_of_the_record_set_only() {
        let html = set_history(vec![
            session(1, 10, vec![reps(10)]),
            session(2, 0, vec![reps(12), reps(12)]),
        ]);

        assert_eq!(
            all_containing(&html, "set-row", "personal-record"),
            [true, false, false]
        );
    }

    #[test]
    fn test_a_recent_record_set_carries_the_recent_marker() {
        let html = set_history(vec![
            session(1, 400, vec![reps(20)]),
            session(2, 20, vec![reps(10)]),
            session(3, 0, vec![reps(12)]),
        ]);

        assert_eq!(
            all_containing(&html, "set-row", "recent-personal-record"),
            [true, false]
        );
        assert!(!contains(&html, "personal-record"));
    }

    #[test]
    fn test_the_marker_of_a_pair_is_in_the_cell_of_its_side() {
        let html = set_history(vec![
            session(
                1,
                10,
                vec![
                    sided_reps(domain::Side::Left, 10),
                    sided_reps(domain::Side::Right, 10),
                ],
            ),
            session(
                2,
                0,
                vec![
                    sided_reps(domain::Side::Right, 12),
                    sided_reps(domain::Side::Left, 10),
                ],
            ),
        ]);

        assert_eq!(
            rows_containing(&html, "set-history-sets", "personal-record")[1],
            [false, false, true, false]
        );
    }

    #[test]
    fn test_the_marker_is_found_after_another_exercise_and_a_rest() {
        let mut other_exercise = reps(5);
        if let domain::TrainingSessionElement::Set { exercise_id, .. } = &mut other_exercise {
            *exercise_id = 2.into();
        }
        let html = set_history(vec![
            session(1, 10, vec![reps(10)]),
            session(
                2,
                0,
                vec![
                    other_exercise,
                    domain::TrainingSessionElement::Rest {
                        target_time: domain::Time::default(),
                        automatic: false,
                    },
                    reps(9),
                    reps(12),
                ],
            ),
        ]);

        assert_eq!(
            all_containing(&html, "set-row", "personal-record"),
            [false, true, false]
        );
    }
}
