use std::{borrow::Cow, collections::BTreeMap, rc::Rc};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{self, Cache, CacheState},
    chart::{Chart, IntervalControl, view_stretch_charts},
    eh,
    loading::LoadingFlag,
    muscle::SetsPerMuscle,
    notification::notify,
    page,
    routing::{ExerciseTab, NavigatorScrollExt, RoutineTab},
    settings::Settings,
    tempo::{phase_fields, update_phase_field, validate_tempo},
    ui::{
        drag_and_drop,
        element::{
            Block, CenteredBlock, DataBox, Dialog, Error, ErrorPage, FloatingActionButton, Icon,
            IconText, Loading, LoadingDialog, LoadingPage, MenuOption, NoData, OptionsMenu,
            SaveDialog, Tabs, Title,
        },
        form::{
            ButtonSelectField, ButtonSelectOption, FieldValue, FieldValueState, InputField,
            PhaseFields,
        },
    },
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn Routine(id: domain::RoutineID, tab: RoutineTab) -> Element {
    let cache = consume_context::<Cache>();
    let current_interval = use_signal(domain::Interval::default);
    let settings = use_context::<Settings>();
    let edit_dialog = use_signal(|| EditDialog::None);
    let mut routine_dialog = use_signal(|| page::routines::RoutineDialog::None);
    let training_dialog = use_signal(|| page::training_sessions::TrainingDialog::None);
    let drag = use_signal(|| None::<Drag>);

    match (
        &*cache.routines.read(),
        &*cache.training_sessions.read(),
        &*cache.exercises.read(),
    ) {
        (CacheState::Ready(routines), training_sessions, CacheState::Ready(exercises)) => {
            let routine = routines.iter().find(|e| e.id == id);
            if let Some(routine) = routine {
                let stretch_only = domain::StretchOnlyExercises::new(exercises);
                let content = match tab {
                    RoutineTab::Routine => rsx! {
                        {view_summary(routine)}
                        {view_notes(routine, routine_dialog)}
                        {view_routine(routine, exercises, edit_dialog, drag, cache, settings)}
                        {drag_and_drop::view_drag_overlay(drag, &DropTarget::Remove)}
                        if let CacheState::Ready(training_sessions) = training_sessions {
                            {view_previous_exercises(routine, training_sessions, exercises)}
                        }
                        {view_muscles(routine, exercises, &stretch_only)}
                    },
                    RoutineTab::Sessions | RoutineTab::Charts => view_training_sessions(
                        id,
                        tab,
                        training_sessions,
                        routines,
                        &stretch_only,
                        current_interval,
                        training_dialog,
                        settings,
                    ),
                };
                rsx! {
                    Tabs {
                        tabs: vec![("Routine", RoutineTab::Routine), ("Sessions", RoutineTab::Sessions), ("Charts", RoutineTab::Charts)],
                        selected: tab,
                        on_select: move |tab| {
                            navigator().replace_preserving_scroll(Route::Routine { id, tab });
                        },
                        is_page_top: true,
                    }
                    {content}
                    {page::routines::view_dialog(routine_dialog, None)}
                    {view_edit_dialog(edit_dialog, cache, settings)}
                    FloatingActionButton {
                        icon: "ellipsis-vertical",
                        on_click: eh!(routine; {
                            *routine_dialog.write() = page::routines::RoutineDialog::Options(routine.clone());
                        }),
                    }
                }
            } else {
                rsx! {
                    ErrorPage { message: "Routine not found" }
                }
            }
        }
        (CacheState::Error(err), _, _) | (_, _, CacheState::Error(err)) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        (CacheState::Loading, _, _) | (_, _, CacheState::Loading) => {
            rsx! { LoadingPage {} }
        }
    }
}

/// Show the sessions of the routine on the sessions tab, and their charts on the charts tab.
#[allow(clippy::too_many_arguments)]
fn view_training_sessions(
    id: domain::RoutineID,
    tab: RoutineTab,
    training_sessions: &CacheState<Vec<domain::TrainingSession>>,
    routines: &[domain::Routine],
    stretch_only: &domain::StretchOnlyExercises,
    mut current_interval: Signal<domain::Interval>,
    training_dialog: Signal<page::training_sessions::TrainingDialog>,
    settings: Settings,
) -> Element {
    match training_sessions {
        CacheState::Ready(training_sessions) => {
            let training_sessions = training_sessions
                .iter()
                .filter(|t| t.routine_id == id)
                .cloned()
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
                current_interval.set(domain::init_interval(&dates, domain::DefaultInterval::All));
            }
            let interval = *current_interval.read();
            let training_sessions = training_sessions
                .iter()
                .filter(|t| t.date >= interval.first && t.date <= interval.last)
                .cloned()
                .collect::<Vec<_>>();
            let metric_sessions = page::training_sessions::without_stretch_only_sets(
                &training_sessions,
                stretch_only,
            );
            rsx! {
                CenteredBlock {
                    IntervalControl { current_interval, all },
                    if training_sessions.is_empty() {
                        NoData {}
                    } else if tab == RoutineTab::Charts {
                        {view_charts(&training_sessions, &metric_sessions, stretch_only, interval, settings)}
                    } else {
                        {page::training_sessions::view_calendar(&training_sessions, stretch_only, interval)}
                        {page::training_sessions::view_table(&training_sessions, stretch_only, page::training_sessions::StretchMetrics::Exclude(&metric_sessions), settings.show_tut(), routines, interval, training_dialog, settings)}
                        {page::training_sessions::view_dialog(training_dialog, &training_sessions, routines, None)}
                    }
                }
            }
        }
        CacheState::Error(err) => {
            rsx! { Error { message: "{err}" } }
        }
        CacheState::Loading => {
            rsx! {
                Loading {}
            }
        }
    }
}

fn view_summary(routine: &domain::Routine) -> Element {
    rsx! {
        CenteredBlock {
            div {
                class: "columns is-gapless is-mobile",
                div {
                    class: "column",
                    DataBox {
                        title: "Duration",
                        "~ " strong { "{routine.duration().num_minutes()}" } " min"
                    }
                }
                div {
                    class: "column",
                    DataBox {
                        title: "Sets",
                        strong { "{routine.num_sets()}" }
                    }
                }
            }
        }
    }
}

fn view_notes(
    routine: &domain::Routine,
    mut routine_dialog: Signal<page::routines::RoutineDialog>,
) -> Element {
    if routine.notes.is_empty() {
        return rsx! {};
    }
    let notes = routine.notes.clone();
    let routine_id = routine.id;
    rsx! {
        CenteredBlock {
            div {
                class: "is-clickable is-italic has-text-centered is-preserving-line-breaks",
                "data-testid": "routine-notes",
                onclick: eh!(notes; {
                    *routine_dialog.write() = page::routines::RoutineDialog::EditNotes {
                        notes: FieldValue::new(notes.clone()),
                        routine_id,
                    };
                }),
                { notes.clone() }
            }
        }
    }
}

fn view_routine(
    routine: &domain::Routine,
    exercises: &[domain::Exercise],
    edit_dialog: Signal<EditDialog>,
    drag: Signal<Option<Drag>>,
    cache: Cache,
    settings: Settings,
) -> Element {
    let routine = Rc::new(routine.clone());
    rsx! {
        Block {
            div {
                class: "p-2",
                "data-testid": "routine-parts",
                "data-drop": "sections",
                for (i, section) in routine.sections.iter().enumerate() {
                    {view_routine_part(&routine, section, &vec![i].into(), routine.sections.len(), exercises, edit_dialog, drag, settings)}
                }
            }
            div {
                class: "has-text-centered",
                button {
                    class: "button is-white-soft",
                    class: if IS_LOADING() && matches!(edit_dialog(), EditDialog::None) { "is-loading" },
                    "data-testid": "add-section",
                    onclick: eh!(routine; {
                        let mut routine = (*routine).clone();
                        routine.add_section(&domain::RoutinePartPath::default());
                        modify_routine_sections(routine, cache, || {})
                    }),
                    Icon { name: "plus" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn view_routine_part(
    routine: &Rc<domain::Routine>,
    part: &domain::RoutinePart,
    path: &domain::RoutinePartPath,
    num_siblings: usize,
    exercises: &[domain::Exercise],
    mut edit_dialog: Signal<EditDialog>,
    drag: Signal<Option<Drag>>,
    settings: Settings,
) -> Element {
    let show_options = {
        let routine = Rc::clone(routine);
        let path = path.clone();
        move || {
            *edit_dialog.write() = EditDialog::Options {
                routine: (*routine).clone(),
                path: path.clone(),
            }
        }
    };
    let target = drag_and_drop::hovered_target(drag);
    let index = path.first().copied().unwrap_or_default();
    let parent = domain::RoutinePartPath::from(path[1..].to_vec());
    let insert_before = target == Some(DropTarget::Gap(parent.clone(), index));
    let insert_after =
        index + 1 == num_siblings && target == Some(DropTarget::Gap(parent, num_siblings));
    let drop_state = drag_and_drop::insertion_state(insert_before, insert_after);
    match part {
        domain::RoutinePart::RoutineSection { rounds, parts } => {
            rsx! {
                div {
                    class: "message",
                    "data-drag-state": drag_and_drop::drag_state(drag, path),
                    "data-drop-state": drop_state,
                    "data-testid": "routine-part",
                    div {
                        class: "message-body p-3 mb-3",
                        class: if path.first() != Some(&0) { "mt-3" },
                        "data-testid": "routine-section",
                        "data-drop": "section-{path_attribute(path)}",
                        div {
                            class: "is-flex is-justify-content-space-between mb-3",
                            "data-testid": "section-header",
                            "data-drop": "header-{path_attribute(path)}",
                            IconText {
                                icon: "repeat",
                                "data-testid": "section-rounds",
                                on_click: eh!(mut edit_dialog; routine, path; {
                                    if let Some(domain::RoutinePart::RoutineSection {
                                        rounds, ..
                                    }) = routine.part(&path) {
                                        let routine = (*routine).clone();
                                        let rounds = FieldValue::new_with_empty_default(*rounds);
                                        *edit_dialog.write() = EditDialog::EditSection { routine, path, rounds };
                                    }
                                }),
                                "{rounds}"
                            }
                            div {
                                class: "is-flex is-align-items-center",
                                Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "section-options" }
                                {view_drag_handle(routine, path, "section-handle", drag)}
                            }
                        }
                        for (i, part) in parts.iter().enumerate() {
                            {view_routine_part(routine, part, &[&[i], &path[..]].concat().into(), parts.len(), exercises, edit_dialog, drag, settings)}
                        }
                        if parts.is_empty() {
                            {view_empty_section_drop_zone(routine, path, drag)}
                        }
                    }
                }
            }
        }
        domain::RoutinePart::RoutineActivity {
            exercise_id,
            reps,
            tempo,
            weight,
            rpe,
            automatic,
            countdown,
            sides,
        } => {
            let laterality = cache::laterality(exercises, *exercise_id);
            let counted_down = domain::SetPacing::new(*reps, *tempo, *countdown)
                .countdown_seconds()
                .is_some();
            rsx! {
                div {
                    class: "message mb-0",
                    class: if path.first() != Some(&0) { "mt-3" },
                    class: if exercise_id.is_nil() {
                        "is-success"
                    } else {
                        "is-info"
                    },
                    "data-drag-state": drag_and_drop::drag_state(drag, path),
                    "data-drop-state": drop_state,
                    "data-testid": "routine-part",
                    "data-drop": "part-{path_attribute(path)}",
                    div {
                        class: "message-body has-background-scheme-main p-3",
                        "data-testid": "routine-part-body",
                        if !exercise_id.is_nil() {
                            div {
                                class: "is-flex is-justify-content-space-between has-text-weight-bold",
                                "data-testid": "set-exercise",
                                if let Some(exercise) = exercises.iter().find(|e| e.id == *exercise_id) {
                                    Link {
                                        to: Route::Exercise { id: exercise.id, tab: ExerciseTab::Exercise },
                                        "{exercise.name}"
                                    }
                                } else {
                                    "Exercise#{exercise_id.as_u128()}"
                                }
                                div {
                                    class: "is-flex is-align-items-center",
                                    Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "activity-options" }
                                    {view_drag_handle(routine, path, "activity-handle", drag)}
                                }
                            }
                        } else {
                            div {
                                class: "is-flex is-justify-content-space-between",
                                div {
                                    onclick: eh!(mut edit_dialog; routine, path; {
                                        *edit_dialog.write() = edit_activity_dialog(&routine, &path, laterality);
                                    }),
                                    span {
                                        class: "icon-text has-text-weight-bold mr-5",
                                        "data-testid": "rest-label",
                                        "Rest"
                                    }
                                    if tempo.non_zero().is_some() {
                                        span {
                                            class: "icon-text mr-4",
                                            "data-testid": "rest-time",
                                            span {
                                                class: "mr-2",
                                                Icon { name: "clock-rotate-left" }
                                                "{tempo}"
                                            }
                                        }
                                    }
                                    if *automatic {
                                        span {
                                            class: "icon-text",
                                            {automatic_icon()}
                                        }
                                    }
                                }
                                div {
                                    class: "is-flex is-align-items-center",
                                    Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "activity-options" }
                                    {view_drag_handle(routine, path, "activity-handle", drag)}
                                }
                            }
                        }
                        if !exercise_id.is_nil() {
                            div {
                                onclick: eh!(mut edit_dialog; routine, path; {
                                    *edit_dialog.write() = edit_activity_dialog(&routine, &path, laterality);
                                }),
                                if *reps != domain::Reps::default() {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-reps",
                                        span {
                                            class: "mr-2",
                                            Icon { name: "rotate-left" }
                                            "{reps}"
                                        }
                                    }
                                }
                                if *weight != domain::Weight::default() {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-weight",
                                        span {
                                            class: "mr-2",
                                            Icon { name: "weight-hanging" }
                                            "{weight} kg"
                                        }
                                    }
                                }
                                if settings.show_rpe() && *rpe != domain::RPE::ZERO {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-rpe",
                                        span {
                                            class: "mr-2",
                                            "@ {rpe}"
                                        }
                                    }
                                }
                                if tempo.non_zero().is_some() {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-tempo",
                                        span {
                                            class: "mr-2",
                                            Icon { name: "clock-rotate-left" }
                                            "{tempo}"
                                        }
                                    }
                                }
                                if counted_down {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-countdown",
                                        Icon { name: "hourglass-half" }
                                    }
                                }
                                if *automatic {
                                    span {
                                        class: "icon-text mr-4",
                                        {automatic_icon()}
                                    }
                                }
                                if *sides == domain::Sides::PerSide {
                                    span {
                                        class: "icon-text",
                                        "data-testid": "set-sides",
                                        "per side"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn automatic_icon() -> Element {
    rsx! {
        span { class: "fa-stack", style: "height: 1.5em; line-height: 1.5em",
            i { class: "fas fa-circle fa-stack-1x" }
            i {
                class: "fas fa-a fa-inverse fa-stack-1x",
                style: "color:var(--bulma-scheme-main)",
            }
        }
    }
}

fn view_drag_handle(
    routine: &Rc<domain::Routine>,
    path: &domain::RoutinePartPath,
    data_testid: &str,
    drag: Signal<Option<Drag>>,
) -> Element {
    drag_and_drop::view_drag_handle(
        drag,
        path.clone(),
        data_testid,
        drag_validator(routine),
        drop_handler(routine),
    )
}

fn view_empty_section_drop_zone(
    routine: &domain::Routine,
    path: &domain::RoutinePartPath,
    drag: Signal<Option<Drag>>,
) -> Element {
    let gap = DropTarget::Gap(path.clone(), 0);
    let droppable =
        drag().is_some_and(|drag| drag.active && is_valid_target(routine, &drag.source, &gap));
    if !droppable {
        return rsx! {};
    }
    let hovered = drag_and_drop::hovered_target(drag) == Some(gap);
    rsx! {
        div {
            class: "is-drop-zone is-active has-text-centered px-4 py-3",
            class: if hovered { "has-text-strong" } else { "has-text-weak" },
            "data-drop-state": drag_and_drop::drop_state(hovered),
            "data-testid": "empty-section-drop-zone",
            "Empty section"
        }
    }
}

fn drag_validator(
    routine: &Rc<domain::Routine>,
) -> impl Fn(domain::RoutinePartPath, DropTarget) -> bool + Clone + 'static {
    let routine = Rc::clone(routine);
    move |source, target| is_valid_target(&routine, &source, &target)
}

fn drop_handler(
    routine: &Rc<domain::Routine>,
) -> impl Fn(domain::RoutinePartPath, DropTarget) + 'static {
    let routine = Rc::clone(routine);
    move |source, target| {
        if let Some(modified) = apply_drop(&routine, &source, target)
            && modified != *routine
        {
            spawn(modify_routine_sections(
                modified,
                consume_context::<Cache>(),
                || {},
            ));
        }
    }
}

/// Whether dropping the part at `source` on `target` is allowed.
///
/// A part must not be dropped into itself or one of its descendants. The top level may only
/// contain sections.
fn is_valid_target(
    routine: &domain::Routine,
    source: &domain::RoutinePartPath,
    target: &DropTarget,
) -> bool {
    match target {
        DropTarget::Remove => true,
        DropTarget::Gap(parent, _) => {
            if parent.ends_with(source) {
                return false;
            }
            !parent.is_empty()
                || matches!(
                    routine.part(source),
                    Some(domain::RoutinePart::RoutineSection { .. })
                )
        }
        DropTarget::Part(_) | DropTarget::Header(_) | DropTarget::Section(_) => false,
    }
}

/// Apply the effect of dropping the part at `source` on `target`, or `None` if `target` is not a
/// resolved drop position.
fn apply_drop(
    routine: &domain::Routine,
    source: &domain::RoutinePartPath,
    target: DropTarget,
) -> Option<domain::Routine> {
    let mut routine = routine.clone();
    match target {
        DropTarget::Gap(parent, index) => routine.move_part(source, &parent, index),
        DropTarget::Remove => routine.remove_part(source),
        DropTarget::Part(_) | DropTarget::Header(_) | DropTarget::Section(_) => return None,
    }
    Some(routine)
}

fn path_attribute(path: &domain::RoutinePartPath) -> String {
    path.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("-")
}

fn parse_path(value: &str) -> Option<domain::RoutinePartPath> {
    value
        .split('-')
        .map(str::parse)
        .collect::<Result<Vec<usize>, _>>()
        .ok()
        .map(Into::into)
}

fn view_previous_exercises(
    routine: &domain::Routine,
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
) -> Element {
    let previous_exercises = routine.previously_used_exercises(training_sessions, exercises);

    if previous_exercises.is_empty() {
        rsx! {}
    } else {
        rsx! {
            CenteredBlock {
                Title { "Previously used exercises" }
                for exercise in previous_exercises {
                    p {
                        class: "m-2",
                        Link {
                            to: Route::Exercise { id: exercise.id, tab: ExerciseTab::Exercise },
                            "{exercise.name}"
                        }
                    }
                }
            }
        }
    }
}

fn view_charts(
    training_sessions: &[domain::TrainingSession],
    metric_sessions: &[Cow<'_, domain::TrainingSession>],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let params = web_app::chart::PlotParams::primary_range(0., 10.);
    let rpe_params = web_app::chart::PlotParams::primary_range(5., 10.);

    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut stretch_sets: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut stretch_minutes: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut rpe_values: Vec<(NaiveDate, f32)> = vec![];
    for (training_session, metric_session) in training_sessions.iter().zip(metric_sessions) {
        *load.entry(training_session.date).or_default() += training_session.load(stretch_only);
        *stretch_sets.entry(training_session.date).or_default() +=
            training_session.stretch_sets(stretch_only);
        *stretch_minutes.entry(training_session.date).or_default() +=
            training_session.stretch_time(stretch_only) / 60.;
        let training_session = metric_session;
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume())
            .or_insert(training_session.set_volume());
        for element in &training_session.elements {
            if let domain::TrainingSessionElement::Set { rpe, .. } = element
                && let Some(rpe) = rpe.non_zero()
            {
                rpe_values.push((training_session.date, f32::from(rpe)));
            }
        }
    }

    rsx! {
        Chart {
            series: vec![web_app::chart::LabeledSeries::new(
                "Load",
                web_app::chart::PlotData {
                    values_high: load.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_LOAD,
                    ),
                    params,
                },
            )],
            interval,
            no_data_label: false,
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
        if settings.show_rpe() {
            Chart {
                series: web_app::chart::labeled_min_avg_max(
                    "RPE",
                    &rpe_values,
                    interval,
                    rpe_params,
                    web_app::chart::COLOR_RPE,
                ),
                interval,
                no_data_label: false,
            }
        }
        if stretch_sets
            .iter()
            .any(|(date, sets)| (interval.first..=interval.last).contains(date) && *sets > 0.)
        {
            {view_stretch_charts(
                "Stretch sets",
                stretch_sets.into_iter().collect(),
                "Time under stretch (min)",
                stretch_minutes.into_iter().collect(),
                interval,
                false,
            )}
        }
    }
}

fn view_muscles(
    routine: &domain::Routine,
    exercises: &[domain::Exercise],
    stretch_only: &domain::StretchOnlyExercises,
) -> Element {
    let stretch_per_muscle = if routine.num_stretch_sets(stretch_only) > 0 {
        routine.stretch_per_muscle(exercises)
    } else {
        BTreeMap::new()
    };
    rsx! {
        SetsPerMuscle {
            stimulus_per_muscle: routine.stimulus_per_muscle(exercises),
            stretch_per_muscle,
            worked_caption: "Worked",
        }
    }
}

fn view_edit_dialog(
    mut edit_dialog: Signal<EditDialog>,
    cache: Cache,
    settings: Settings,
) -> Element {
    let close_dialog = move || {
        *edit_dialog.write() = EditDialog::None;
    };

    match &*edit_dialog.read() {
        EditDialog::None => rsx! {},
        EditDialog::Options { routine, path } => {
            #[derive(PartialEq)]
            enum PartType {
                Section,
                Exercise,
                Rest,
            }
            let part_type = match routine.part(path) {
                Some(domain::RoutinePart::RoutineActivity { exercise_id, .. }) => {
                    if exercise_id.is_nil() {
                        PartType::Rest
                    } else {
                        PartType::Exercise
                    }
                }
                _ => PartType::Section,
            };

            let routine = routine.clone();

            rsx! {
                if IS_LOADING() {
                    LoadingDialog {}
                } else {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                if part_type == PartType::Section {
                                    MenuOption {
                                        icon: "person-running",
                                        text: "Add exercise",
                                        "data-testid": "options-add-exercise",
                                        on_click: eh!(mut edit_dialog; routine, path; {
                                            *edit_dialog.write() = EditDialog::AddExercise { routine, path };
                                        })
                                    },
                                    MenuOption {
                                        icon: "person",
                                        text: "Add rest",
                                        "data-testid": "options-add-rest",
                                        on_click: eh!(mut routine; path, close_dialog; {
                                            routine.add_activity(domain::ExerciseID::nil(), None, &path);
                                            modify_routine_sections(routine, cache, close_dialog)
                                        })
                                    },
                                    MenuOption {
                                        icon: "repeat",
                                        text: "Add section",
                                        "data-testid": "options-add-section",
                                        on_click: eh!(mut routine; path, close_dialog; {
                                            routine.add_section(&path);
                                            modify_routine_sections(routine, cache, close_dialog)
                                        })
                                    },
                                }
                                MenuOption {
                                    icon: "arrow-up",
                                    text: "Move up",
                                    "data-testid": "options-move-up",
                                    on_click: eh!(mut routine; path, close_dialog; {
                                        routine.move_part_up(&path);
                                        modify_routine_sections(routine, cache, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "arrow-down",
                                    text: "Move down",
                                    "data-testid": "options-move-down",
                                    on_click: eh!(mut routine; path, close_dialog; {
                                        routine.move_part_down(&path);
                                        modify_routine_sections(routine, cache, close_dialog)
                                    })
                                },
                                if part_type == PartType::Exercise {
                                    MenuOption {
                                        icon: "arrow-right-arrow-left",
                                        text: "Replace exercise",
                                        "data-testid": "options-replace-exercise",
                                        on_click: eh!(mut edit_dialog; routine, path; {
                                            *edit_dialog.write() = EditDialog::ReplaceExercise { routine, path };
                                        })
                                    },
                                }
                                MenuOption {
                                    icon: "edit",
                                    text: match routine.part(path) {
                                        Some(domain::RoutinePart::RoutineSection { .. }) => {
                                            "Edit rounds"
                                        }
                                        Some(domain::RoutinePart::RoutineActivity {
                                            ..
                                        }) => {
                                            "Edit targets"
                                        }
                                        None => {
                                            "Edit"
                                        }
                                    },
                                    "data-testid": "options-edit",
                                    on_click: eh!(mut edit_dialog; routine, path; {
                                        match routine.part(&path) {
                                            Some(domain::RoutinePart::RoutineSection {
                                                rounds, ..
                                            }) => {
                                                let rounds = FieldValue::new_with_empty_default(*rounds);
                                                *edit_dialog.write() = EditDialog::EditSection { routine, path, rounds };
                                            }
                                            Some(domain::RoutinePart::RoutineActivity {
                                                exercise_id, ..
                                            }) => {
                                                *edit_dialog.write() = edit_activity_dialog(&routine, &path, cache.laterality(*exercise_id));
                                            }
                                            _ => {}
                                        }
                                    })
                                },
                                MenuOption {
                                    icon: "times",
                                    text: "Remove",
                                    "data-testid": "options-remove",
                                    on_click: eh!(mut routine; path, close_dialog; {
                                        routine.remove_part(&path);
                                        modify_routine_sections(routine, cache, close_dialog)
                                    })
                                },
                            },
                        ],
                        on_close: eh!(mut close_dialog; { close_dialog(); })
                    }
                }
            }
        }
        EditDialog::AddExercise { routine, path } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog {}
                } else {
                    Dialog {
                        on_close: eh!(mut close_dialog; { close_dialog(); }),
                        no_horizontal_padding: true,
                        page::exercises::ExerciseList {
                            add: false,
                            filter: domain::ExerciseFilter::default(),
                            on_exercise_click: {
                                let routine = routine.clone();
                                let path = path.clone();
                                move |(_, id)| {
                                    let mut routine = routine.clone();
                                    let path = path.clone();
                                    routine.add_activity(id, cache.laterality(id), &path);
                                    modify_routine_sections(routine, cache, close_dialog)
                                }
                            },
                            on_catalog_click: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::ReplaceExercise { routine, path } => {
            rsx! {
                if IS_LOADING() {
                    LoadingDialog {}
                } else {
                    Dialog {
                        on_close: eh!(mut close_dialog; { close_dialog(); }),
                        no_horizontal_padding: true,
                        page::exercises::ExerciseList {
                            add: false,
                            filter: page::exercises::replacement_filter(
                                match routine.part(path) {
                                    Some(domain::RoutinePart::RoutineActivity { exercise_id, .. }) => *exercise_id,
                                    _ => domain::ExerciseID::nil(),
                                },
                                &cache,
                            ),
                            on_exercise_click: {
                                let routine = routine.clone();
                                let path = path.clone();
                                move |(_, id)| {
                                    let mut routine = routine.clone();
                                    let path = path.clone();
                                    let sides = replacement_sides(cache.laterality(id));
                                    routine.update_activity(Some(id), None, None, None, None, None, None, sides, &path);
                                    modify_routine_sections(routine, cache, close_dialog)
                                }
                            },
                            on_catalog_click: |_| {}
                        }
                    }
                }
            }
        }
        EditDialog::EditSection {
            routine,
            path,
            rounds: rounds_field,
        } => {
            let save = eh!(mut routine; path, rounds_field, close_dialog; {
                routine.update_section(rounds_field.validated.ok(), &path);
                modify_routine_sections(routine, cache, close_dialog)
            });
            match routine.part(path) {
                Some(domain::RoutinePart::RoutineSection { .. }) => {
                    rsx! {
                        SaveDialog {
                            on_close: eh!(mut close_dialog; { close_dialog(); }),
                            on_save: save,
                            is_loading: IS_LOADING(),
                            disabled: !FieldValue::has_valid_changes(&[rounds_field as &dyn FieldValueState]),
                            InputField {
                                label: "Rounds",
                                "data-testid": "rounds",
                                right_icon: rsx! { "✕" },
                                inputmode: "numeric",
                                value: rounds_field.input.clone(),
                                error: if let Err(err) = &rounds_field.validated { err.clone() },
                                has_changed: rounds_field.changed(),
                                autofocus: true,
                                on_input: move |event: FormEvent| {
                                    async move {
                                        if let EditDialog::EditSection { rounds, .. } =  &mut *edit_dialog.write() {
                                            rounds.input = event.value();
                                            rounds.validated = if rounds.input.is_empty() {
                                                Ok(domain::Rounds::default())
                                            } else {
                                                domain::Rounds::try_from(rounds.input.as_ref())
                                                    .map_err(|err| err.to_string())
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    rsx! {
                        Error { message: "Unexpected routine part type" }
                    }
                }
            }
        }
        EditDialog::EditActivity {
            routine,
            path,
            reps: reps_field,
            tempo: tempo_fields,
            weight: weight_field,
            rpe: rpe_field,
            automatic: automatic_field,
            countdown: chosen_countdown,
            sides: sides_field,
            show_sides,
        } => {
            let validated_tempo = validate_tempo(tempo_fields);
            // Without reps, the single repetition is a hold.
            let is_hold = reps_field
                .validated
                .as_ref()
                .is_ok_and(|reps| reps.non_zero().is_none());
            let is_rest = matches!(
                routine.part(path),
                Some(domain::RoutinePart::RoutineActivity { exercise_id, .. }) if exercise_id.is_nil()
            );
            // The automatic start and the countdown of a set take effect only with a tempo.
            let needs_tempo = !is_rest
                && !validated_tempo
                    .as_ref()
                    .is_ok_and(|tempo| domain::SetPacing::countdown_applies(*tempo));
            let automatic = automatic_field.input == true.to_string() && !needs_tempo;
            let effective_automatic_field = FieldValue {
                input: automatic.to_string(),
                validated: Ok(automatic),
                orig: automatic_field.orig.clone(),
            };
            let stored_countdown = match routine.part(path) {
                Some(domain::RoutinePart::RoutineActivity { countdown, .. }) => *countdown,
                _ => false,
            };
            let countdown = edited_countdown(
                routine.part(path),
                reps_field.validated.clone().unwrap_or_default(),
                *chosen_countdown,
            );
            let countdown_field = FieldValue {
                input: countdown.to_string(),
                validated: Ok(countdown),
                orig: stored_countdown.to_string(),
            };
            let save = eh!(mut routine; path, reps_field, validated_tempo, weight_field, rpe_field, sides_field, close_dialog; {
                routine.update_activity(None, reps_field.validated.ok(), validated_tempo.ok(), weight_field.validated.ok(), rpe_field.validated.ok(), Some(automatic), Some(countdown), sides_field.validated.ok(), &path);
                modify_routine_sections(routine, cache, close_dialog)
            });
            match routine.part(path) {
                Some(domain::RoutinePart::RoutineActivity { exercise_id, .. }) => {
                    // A countdown without effect does not count as a change.
                    let field_states = [
                        reps_field as &dyn FieldValueState,
                        weight_field,
                        rpe_field,
                        &effective_automatic_field,
                        sides_field,
                    ]
                    .into_iter()
                    .chain(
                        (!is_rest && !needs_tempo)
                            .then_some(&countdown_field as &dyn FieldValueState),
                    )
                    .chain(
                        tempo_fields
                            .iter()
                            .map(|field| field as &dyn FieldValueState),
                    )
                    .collect::<Vec<_>>();
                    rsx! {
                        SaveDialog {
                            on_close: eh!(mut close_dialog; { close_dialog(); }),
                            on_save: save,
                            is_loading: IS_LOADING(),
                            disabled: !FieldValue::has_valid_changes(&field_states) || validated_tempo.is_err(),
                            if !exercise_id.is_nil() {
                                InputField {
                                    label: "Reps",
                                    right_icon: rsx! { "✕" },
                                    inputmode: "numeric",
                                    value: reps_field.input.clone(),
                                    error: if let Err(err) = &reps_field.validated { err.clone() },
                                    has_changed: reps_field.changed(),
                                    autofocus: true,
                                    on_input: move |event: FormEvent| {
                                        async move {
                                            if let EditDialog::EditActivity { reps, .. } =  &mut *edit_dialog.write() {
                                                reps.input = event.value();
                                                reps.validated = if reps.input.is_empty() {
                                                    Ok(domain::Reps::default())
                                                } else {
                                                    domain::Reps::try_from(reps.input.as_ref())
                                                        .map_err(|err| err.to_string())
                                                };
                                            }
                                        }
                                    },
                                    "data-testid": "input-reps",
                                }
                            }
                            if exercise_id.is_nil() {
                                InputField {
                                    label: "Time",
                                    right_icon: rsx! { "s" },
                                    inputmode: "numeric",
                                    value: tempo_fields[0].input.clone(),
                                    error: if let Err(err) = &tempo_fields[0].validated { err.clone() },
                                    has_changed: tempo_fields[0].changed(),
                                    autofocus: true,
                                    on_input: move |event: FormEvent| {
                                        async move {
                                            if let EditDialog::EditActivity { tempo, .. } =  &mut *edit_dialog.write() {
                                                update_phase_field(&mut tempo[0], &event.value());
                                            }
                                        }
                                    },
                                    "data-testid": "input-time",
                                }
                            } else {
                                PhaseFields {
                                    label: if is_hold { "Time" } else { "Time per rep" },
                                    help: (!is_hold).then(|| "Optionally split into the phases of a tempo".to_string()),
                                    unit: "s",
                                    phases: tempo_fields.iter().map(|field| field.input.clone()).collect::<Vec<_>>(),
                                    field_errors: tempo_fields.iter().map(|field| field.validated.clone().err()).collect::<Vec<_>>(),
                                    error: if let Err(err) = &validated_tempo { err.clone() },
                                    has_changed: tempo_fields.iter().any(FieldValueState::changed),
                                    field_testid: "input-tempo",
                                    on_input: move |(index, value): (usize, String)| {
                                        async move {
                                            if let EditDialog::EditActivity { tempo, .. } =  &mut *edit_dialog.write() {
                                                update_phase_field(&mut tempo[index], &value);
                                            }
                                        }
                                    },
                                }
                            }
                            if *show_sides {
                                ButtonSelectField {
                                    label: "Sides",
                                    options: vec![
                                        ButtonSelectOption {
                                            text: "Combined".to_string(),
                                            icon: None,
                                            value: domain::Sides::Combined,
                                            is_disabled: false,
                                        },
                                        ButtonSelectOption {
                                            text: "Per side".to_string(),
                                            icon: None,
                                            value: domain::Sides::PerSide,
                                            is_disabled: false,
                                        },
                                    ],
                                    selected: sides_field.validated.clone().unwrap_or_default(),
                                    has_changed: sides_field.changed(),
                                    on_click: {
                                        move |(_, value): (_, domain::Sides)| {
                                            async move {
                                                if let EditDialog::EditActivity { sides, .. } =  &mut *edit_dialog.write() {
                                                    *sides = sides_field_value(value, &sides.orig);
                                                }
                                            }
                                        }
                                    },
                                    "data-testid": "button-select-sides",
                                }
                            }
                            if !exercise_id.is_nil() {
                                InputField {
                                    label: "Weight",
                                    right_icon: rsx! { "kg" },
                                    inputmode: "decimal",
                                    value: weight_field.input.clone(),
                                    error: if let Err(err) = &weight_field.validated { err.clone() },
                                    has_changed: weight_field.changed(),
                                    on_input: move |event: FormEvent| {
                                        async move {
                                            if let EditDialog::EditActivity { weight, .. } =  &mut *edit_dialog.write() {
                                                weight.input = event.value();
                                                weight.validated = if weight.input.is_empty() {
                                                    Ok(domain::Weight::default())
                                                } else {
                                                    domain::Weight::try_from(weight.input.as_ref())
                                                        .map_err(|err| err.to_string())
                                                };
                                            }
                                        }
                                    },
                                    "data-testid": "input-weight",
                                }
                            }
                            if !exercise_id.is_nil() && (settings.show_rpe() || !rpe_field.orig.is_empty()) {
                                InputField {
                                    label: "RPE",
                                    left_icon: rsx! { "@" },
                                    inputmode: "decimal",
                                    value: rpe_field.input.clone(),
                                    error: if let Err(err) = &rpe_field.validated { err.clone() },
                                    has_changed: rpe_field.changed(),
                                    on_input: move |event: FormEvent| {
                                        async move {
                                            if let EditDialog::EditActivity { rpe, .. } =  &mut *edit_dialog.write() {
                                                rpe.input = event.value();
                                                rpe.validated = if rpe.input.is_empty() {
                                                    Ok(domain::RPE::default())
                                                } else {
                                                    domain::RPE::try_from(rpe.input.as_ref())
                                                        .map_err(|err| err.to_string())
                                                };
                                            }
                                        }
                                    },
                                    "data-testid": "input-rpe",
                                }
                            }
                            ButtonSelectField {
                                label: if exercise_id.is_nil() { "Transition to next part" } else { "Start" },
                                options: vec![
                                    ButtonSelectOption {
                                        text: "Automatic".to_string(),
                                        icon: None,
                                        value: true,
                                        is_disabled: needs_tempo,
                                    },
                                    ButtonSelectOption {
                                        text: "Manual".to_string(),
                                        icon: None,
                                        value: false,
                                        is_disabled: false,
                                    },
                                ],
                                selected: automatic,
                                has_changed: effective_automatic_field.changed(),
                                on_click: {
                                    move |(_, value): (_, bool)| {
                                        async move {
                                            if let EditDialog::EditActivity { automatic, .. } =  &mut *edit_dialog.write() {
                                                automatic.input = value.to_string();
                                                automatic.validated = Ok(value);
                                            }
                                        }
                                    }
                                },
                                "data-testid": "button-select-automatic",
                            }
                            if !is_rest {
                                ButtonSelectField {
                                    label: "Recording",
                                    options: vec![
                                        ButtonSelectOption {
                                            text: "Countdown".to_string(),
                                            icon: None,
                                            value: true,
                                            is_disabled: needs_tempo,
                                        },
                                        ButtonSelectOption {
                                            text: "Input fields".to_string(),
                                            icon: None,
                                            value: false,
                                            is_disabled: false,
                                        },
                                    ],
                                    selected: countdown && !needs_tempo,
                                    has_changed: countdown_field.changed() && !needs_tempo,
                                    on_click: {
                                        move |(_, value): (_, bool)| {
                                            async move {
                                                if let EditDialog::EditActivity { countdown, .. } =  &mut *edit_dialog.write() {
                                                    *countdown = Some(value);
                                                }
                                            }
                                        }
                                    },
                                    "data-testid": "button-select-countdown",
                                }
                            }
                        }
                    }
                }
                _ => {
                    rsx! {
                        Error { message: "Unexpected routine part type" }
                    }
                }
            }
        }
    }
}

/// The recording of an activity edited to `reps`: the tapped one, otherwise the stored one while the
/// activity stays a hold or a set with reps, otherwise the default of its kind.
fn edited_countdown(
    stored: Option<&domain::RoutinePart>,
    reps: domain::Reps,
    chosen: Option<bool>,
) -> bool {
    if let Some(chosen) = chosen {
        return chosen;
    }
    match stored {
        Some(domain::RoutinePart::RoutineActivity {
            reps: stored_reps,
            tempo,
            countdown,
            ..
        }) if domain::SetPacing::countdown_applies(*tempo)
            && stored_reps.non_zero().is_some() == reps.non_zero().is_some() =>
        {
            *countdown
        }
        _ => domain::SetPacing::default_countdown(reps),
    }
}

fn edit_activity_dialog(
    routine: &domain::Routine,
    path: &domain::RoutinePartPath,
    laterality: Option<domain::Laterality>,
) -> EditDialog {
    let Some(domain::RoutinePart::RoutineActivity {
        exercise_id,
        reps,
        tempo,
        weight,
        rpe,
        automatic,
        sides,
        ..
    }) = routine.part(path)
    else {
        return EditDialog::None;
    };
    EditDialog::EditActivity {
        routine: routine.clone(),
        path: path.clone(),
        reps: FieldValue::new_with_empty_default(*reps),
        tempo: phase_fields(tempo),
        weight: FieldValue::new_with_empty_default(*weight),
        rpe: FieldValue::new_with_empty_default(*rpe),
        automatic: FieldValue::new(*automatic),
        countdown: None,
        sides: sides_field_value(*sides, &(*sides as u8).to_string()),
        // A per-side activity keeps the field, so that it can be reset after the laterality of
        // its exercise changed.
        show_sides: !exercise_id.is_nil()
            && (laterality == Some(domain::Laterality::Unilateral)
                || *sides == domain::Sides::PerSide),
    }
}

/// The sides of an activity whose exercise is replaced by one of `laterality`, `None` keeping the
/// sides of the activity if the laterality is unknown.
fn replacement_sides(laterality: Option<domain::Laterality>) -> Option<domain::Sides> {
    laterality.map(|laterality| domain::Sides::for_laterality(Some(laterality)))
}

fn sides_field_value(sides: domain::Sides, orig: &str) -> FieldValue<domain::Sides> {
    FieldValue {
        input: (sides as u8).to_string(),
        validated: Ok(sides),
        orig: orig.to_string(),
    }
}

async fn modify_routine_sections(
    routine: domain::Routine,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    let _loading = LoadingFlag::set(&IS_LOADING);
    match DOMAIN_SERVICE()
        .modify_routine(routine.id, None, None, None, Some(routine.sections))
        .await
    {
        Ok(_) => {
            cache.refresh_routines();
        }
        Err(err) => {
            notify("modify routine", &err);
        }
    }
    close_dialog();
}

type Drag = drag_and_drop::Drag<domain::RoutinePartPath, DropTarget>;

/// The index of `Gap` is an insertion position between 0 and the number of parts of the section
/// at the path, referring to the gap before the part with the same index. An empty path refers to
/// the top level of the routine.
#[derive(Clone, Debug, PartialEq)]
enum DropTarget {
    Gap(domain::RoutinePartPath, usize),
    Part(domain::RoutinePartPath),
    Header(domain::RoutinePartPath),
    Section(domain::RoutinePartPath),
    Remove,
}

impl drag_and_drop::DropTarget for DropTarget {
    fn parse(value: &str) -> Option<Self> {
        if value == "remove" {
            return Some(Self::Remove);
        }
        if value == "sections" {
            return Some(Self::Section(domain::RoutinePartPath::default()));
        }
        if let Some(path) = value.strip_prefix("part-") {
            return Some(Self::Part(parse_path(path)?));
        }
        if let Some(path) = value.strip_prefix("header-") {
            return Some(Self::Header(parse_path(path)?));
        }
        if let Some(path) = value.strip_prefix("section-") {
            return Some(Self::Section(parse_path(path)?));
        }
        None
    }

    fn resolve(self, element: &web_sys::Element, y: f64) -> Self {
        match self {
            Self::Part(path) => insertion_target(&path, drag_and_drop::in_lower_half(element, y)),
            Self::Header(path) => header_target(&path, drag_and_drop::in_lower_half(element, y)),
            Self::Section(path) => {
                // The padding above the header belongs to the section element but is visually
                // outside of the section content, so pointing at it inserts before the section
                if let Ok(Some(header)) = element.query_selector(":scope > [data-drop^='header-']")
                    && y < header.get_bounding_client_rect().top()
                {
                    return insertion_target(&path, false);
                }
                let index =
                    drag_and_drop::insertion_index(element, ":scope > .message", y).unwrap_or(0);
                Self::Gap(path, index)
            }
            Self::Gap(..) | Self::Remove => self,
        }
    }

    fn suspends_auto_scroll(&self) -> bool {
        *self == Self::Remove
    }
}

/// Convert the path of a hovered part into an insertion position among its siblings.
///
/// Pointing at the upper half of an element inserts before it, pointing at the lower half inserts
/// after it.
fn insertion_target(path: &domain::RoutinePartPath, lower_half: bool) -> DropTarget {
    let Some((&index, parent)) = path.split_first() else {
        return DropTarget::Part(path.clone());
    };
    DropTarget::Gap(parent.to_vec().into(), index + usize::from(lower_half))
}

/// Convert the path of a hovered section header into an insertion position.
///
/// Pointing at the upper half of the header inserts before the section among its siblings,
/// pointing at the lower half inserts at the top of the section.
fn header_target(path: &domain::RoutinePartPath, lower_half: bool) -> DropTarget {
    if lower_half {
        DropTarget::Gap(path.clone(), 0)
    } else {
        insertion_target(path, false)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum EditDialog {
    None,
    Options {
        routine: domain::Routine,
        path: domain::RoutinePartPath,
    },
    AddExercise {
        routine: domain::Routine,
        path: domain::RoutinePartPath,
    },
    ReplaceExercise {
        routine: domain::Routine,
        path: domain::RoutinePartPath,
    },
    EditSection {
        routine: domain::Routine,
        path: domain::RoutinePartPath,
        rounds: FieldValue<domain::Rounds>,
    },
    EditActivity {
        routine: domain::Routine,
        path: domain::RoutinePartPath,
        reps: FieldValue<domain::Reps>,
        tempo: Vec<FieldValue<domain::Time>>,
        weight: FieldValue<domain::Weight>,
        rpe: FieldValue<domain::RPE>,
        automatic: FieldValue<bool>,
        /// The tapped recording, `None` until one is tapped.
        countdown: Option<bool>,
        sides: FieldValue<domain::Sides>,
        show_sides: bool,
    },
}

#[cfg(test)]
mod tests {
    use crate::ui::drag_and_drop::DropTarget as _;

    use super::*;

    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, all_attributes_of, all_text_of, attribute_of, contains, disabled_texts_of,
        provide_settings, render, rows_of, selected_text_of, text_of,
    };

    fn exercise(id: u128, name: &str) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            notes: String::new(),
            muscles: vec![],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn render_routine(id: u128, cache: impl Fn() -> TestCache + 'static) -> String {
        render_tab(id, RoutineTab::Routine, cache)
    }

    fn render_tab(id: u128, tab: RoutineTab, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_settings(web_app::Settings::default());
            rsx! { Routine { id: domain::RoutineID::from(id), tab } }
        })
    }

    fn with_routine() -> TestCache {
        TestCache::default()
            .with_routines(vec![routine()])
            .with_exercises(vec![exercise(1, "Squat")])
    }

    #[test]
    fn test_the_summary_is_shown() {
        let html = render_routine(1, with_routine);

        assert_eq!(
            all_text_of(&html, "data-box")[0..2],
            ["Duration~ 0 min", "Sets3"]
        );
    }

    #[test]
    fn test_the_nested_sections_and_their_exercises_are_shown() {
        let html = render_routine(1, with_routine);

        assert_eq!(all_text_of(&html, "set-exercise"), vec!["Squat"; 3]);
        assert_eq!(all_text_of(&html, "section-rounds").len(), 3);
    }

    #[test]
    fn test_the_tempo_of_an_activity_is_shown_independently_of_the_time_under_tension_setting() {
        let with_tempo = |show_tut| {
            render(move || {
                with_routine()
                    .with_routines(vec![domain::Routine {
                        sections: vec![section(vec![activity_with_tempo(8, &[3, 1, 1, 0])])],
                        ..routine()
                    }])
                    .provide();
                provide_settings(web_app::Settings {
                    show_tut,
                    ..web_app::Settings::default()
                });
                rsx! { Routine { id: domain::RoutineID::from(1), tab: RoutineTab::Routine } }
            })
        };

        assert_eq!(text_of(&with_tempo(true), "set-tempo"), "3·1·1·0");
        assert_eq!(text_of(&with_tempo(false), "set-tempo"), "3·1·1·0");
    }

    #[test]
    fn test_the_rpe_of_an_activity_is_shown_under_the_rpe_setting() {
        let with_rpe = |show_rpe| {
            render(move || {
                with_routine()
                    .with_routines(vec![domain::Routine {
                        sections: vec![section(vec![activity_with_rpe(8, domain::RPE::NINE)])],
                        ..routine()
                    }])
                    .provide();
                provide_settings(web_app::Settings {
                    show_rpe,
                    ..web_app::Settings::default()
                });
                rsx! { Routine { id: domain::RoutineID::from(1), tab: RoutineTab::Routine } }
            })
        };

        assert_eq!(text_of(&with_rpe(true), "set-rpe"), "@ 9");
        assert!(!contains(&with_rpe(false), "set-rpe"));
    }

    #[test]
    fn test_only_a_per_side_activity_is_marked() {
        let html = render_routine(1, || {
            with_routine().with_routines(vec![domain::Routine {
                sections: vec![section(vec![
                    activity(1),
                    activity_with_sides(2, domain::Sides::PerSide),
                ])],
                ..routine()
            }])
        });

        assert_eq!(all_text_of(&html, "set-sides"), vec!["per side"]);
    }

    #[rstest::rstest]
    #[case::unilateral(Some(domain::Laterality::Unilateral), domain::Sides::Combined, true)]
    #[case::without_laterality(None, domain::Sides::Combined, false)]
    #[case::without_laterality_already_per_side(None, domain::Sides::PerSide, true)]
    #[case::bilateral(Some(domain::Laterality::Bilateral), domain::Sides::Combined, false)]
    #[case::bilateral_already_per_side(
        Some(domain::Laterality::Bilateral),
        domain::Sides::PerSide,
        true
    )]
    fn test_the_sides_are_asked_for_a_unilateral_exercise_or_a_per_side_activity(
        #[case] laterality: Option<domain::Laterality>,
        #[case] sides: domain::Sides,
        #[case] expected: bool,
    ) {
        let routine = domain::Routine {
            sections: vec![section(vec![activity_with_sides(1, sides)])],
            ..routine()
        };

        assert!(matches!(
            edit_activity_dialog(&routine, &vec![0, 0].into(), laterality),
            EditDialog::EditActivity { show_sides, .. } if show_sides == expected
        ));
    }

    #[rstest::rstest]
    #[case::unilateral(Some(domain::Laterality::Unilateral), Some(domain::Sides::PerSide))]
    #[case::bilateral(Some(domain::Laterality::Bilateral), Some(domain::Sides::Combined))]
    #[case::unknown(None, None)]
    fn test_a_replacement_keeps_the_sides_only_without_a_laterality(
        #[case] laterality: Option<domain::Laterality>,
        #[case] expected: Option<domain::Sides>,
    ) {
        assert_eq!(replacement_sides(laterality), expected);
    }

    #[test]
    fn test_the_notes_are_shown_only_when_there_are_some() {
        let with_notes = render_routine(1, || {
            with_routine().with_routines(vec![domain::Routine {
                notes: "some notes".to_string(),
                ..routine()
            }])
        });
        let without_notes = render_routine(1, with_routine);

        assert_eq!(text_of(&with_notes, "routine-notes"), "some notes");
        assert!(!contains(&without_notes, "routine-notes"));
    }

    #[rstest::rstest]
    fn test_without_training_sessions_no_data_is_reported(
        #[values(RoutineTab::Sessions, RoutineTab::Charts)] tab: RoutineTab,
    ) {
        let html = render_tab(1, tab, with_routine);

        assert_eq!(text_of(&html, "no-data"), "No data");
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_the_routine_tab_shows_the_routine_and_no_sessions() {
        let html = render_routine(1, || with_stretching_routine(vec![timed_set(1, 5, 0)]));

        assert_eq!(attribute_of(&html, "tab-routine", "data-selected"), "true");
        assert!(contains(&html, "set-exercise"));
        assert!(!contains(&html, "table"));
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_the_sessions_tab_shows_the_sessions_and_no_routine() {
        let html = render_tab(1, RoutineTab::Sessions, || {
            with_stretching_routine(vec![timed_set(1, 5, 0)])
        });

        assert_eq!(attribute_of(&html, "tab-sessions", "data-selected"), "true");
        assert!(contains(&html, "table"));
        assert!(!contains(&html, "chart"));
        assert!(!contains(&html, "set-exercise"));
    }

    #[test]
    fn test_the_charts_tab_shows_the_charts_and_no_routine() {
        let html = render_tab(1, RoutineTab::Charts, || {
            with_stretching_routine(vec![timed_set(1, 5, 0)])
        });

        assert_eq!(attribute_of(&html, "tab-charts", "data-selected"), "true");
        assert!(contains(&html, "chart"));
        assert!(!contains(&html, "table"));
        assert!(!contains(&html, "set-exercise"));
    }

    #[test]
    fn test_an_unknown_routine_is_reported() {
        let html = render_routine(2, with_routine);

        assert_eq!(text_of(&html, "error-page"), "Routine not found");
    }

    #[test]
    fn test_unread_routines_are_shown_as_loading() {
        let html = render_routine(1, TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_routines_are_shown_as_an_error() {
        let html = render_routine(1, TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }

    fn routine() -> domain::Routine {
        domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![
                section(vec![activity(1), activity(2)]),
                section(vec![section(vec![activity(3)])]),
            ],
        }
    }

    fn section(parts: Vec<domain::RoutinePart>) -> domain::RoutinePart {
        domain::RoutinePart::RoutineSection {
            rounds: domain::Rounds::new(1).unwrap(),
            parts,
        }
    }

    fn activity_with_tempo(reps: u32, phases: &[u32]) -> domain::RoutinePart {
        let domain::RoutinePart::RoutineActivity {
            exercise_id,
            weight,
            rpe,
            automatic,
            sides,
            ..
        } = activity(reps)
        else {
            unreachable!()
        };
        domain::RoutinePart::RoutineActivity {
            exercise_id,
            reps: domain::Reps::new(reps).unwrap(),
            tempo: domain::Tempo::new(phases).unwrap(),
            weight,
            rpe,
            automatic,
            countdown: false,
            sides,
        }
    }

    fn activity_with_rpe(reps: u32, rpe: domain::RPE) -> domain::RoutinePart {
        let domain::RoutinePart::RoutineActivity {
            exercise_id,
            tempo,
            weight,
            automatic,
            sides,
            ..
        } = activity(reps)
        else {
            unreachable!()
        };
        domain::RoutinePart::RoutineActivity {
            exercise_id,
            reps: domain::Reps::new(reps).unwrap(),
            tempo,
            weight,
            rpe,
            automatic,
            countdown: false,
            sides,
        }
    }

    fn activity_with_sides(reps: u32, sides: domain::Sides) -> domain::RoutinePart {
        domain::RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: domain::Reps::new(reps).unwrap(),
            tempo: domain::Tempo::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
            sides,
        }
    }

    fn activity(reps: u32) -> domain::RoutinePart {
        domain::RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: domain::Reps::new(reps).unwrap(),
            tempo: domain::Tempo::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
            sides: domain::Sides::Combined,
        }
    }

    fn stretch(id: u128) -> domain::Exercise {
        domain::Exercise {
            muscles: vec![domain::ExerciseMuscle {
                muscle_id: domain::MuscleID::Hamstrings,
                stimulus: domain::Stimulus::NONE,
                stretch: domain::Stimulus::PRIMARY,
            }],
            ..exercise(id, "Forward Fold")
        }
    }

    fn hold(exercise_id: u128, seconds: u32) -> domain::RoutinePart {
        domain::RoutinePart::RoutineActivity {
            exercise_id: exercise_id.into(),
            reps: domain::Reps::default(),
            tempo: domain::Tempo::new(&[seconds]).unwrap(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            automatic: false,
            countdown: false,
            sides: domain::Sides::Combined,
        }
    }

    fn timed_set(exercise_id: u128, reps: u32, time: u32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::new(time).unwrap(),
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

    fn with_stretching_routine(elements: Vec<domain::TrainingSessionElement>) -> TestCache {
        TestCache::default()
            .with_routines(vec![domain::Routine {
                sections: vec![section(vec![hold(2, 60), hold(2, 60)])],
                ..routine()
            }])
            .with_exercises(vec![exercise(1, "Squat"), stretch(2)])
            .with_training_sessions(vec![domain::TrainingSession {
                id: 1.into(),
                routine_id: 1.into(),
                date: chrono::Local::now().date_naive(),
                notes: String::new(),
                elements,
                exercise_notes: std::collections::BTreeMap::new(),
                exercise_measurements: std::collections::BTreeMap::new(),
            }])
    }

    #[test]
    fn test_the_summary_of_a_stretching_routine_counts_its_stretch_sets() {
        let html = render_routine(1, || with_stretching_routine(vec![]));

        assert_eq!(
            all_text_of(&html, "data-box")[0..2],
            ["Duration~ 2 min", "Sets2"]
        );
        assert_eq!(
            text_of(&html, "stretch-sets-per-muscle"),
            "Hamstrings2Back of thighs"
        );
        assert_eq!(
            text_of(&html, "stretch-time-per-muscle"),
            "Hamstrings2 minBack of thighs"
        );
    }

    #[test]
    fn test_a_routine_without_stretches_shows_no_stretch() {
        let html = render_routine(1, with_routine);

        assert!(!contains(&html, "stretch-sets-per-muscle"));
    }

    #[test]
    fn test_the_stretch_sets_of_the_sessions_are_shown_in_columns_and_a_chart() {
        let cache = || with_stretching_routine(vec![timed_set(2, 0, 60), timed_set(1, 5, 0)]);
        let html = render_tab(1, RoutineTab::Sessions, cache);
        let charts = render_tab(1, RoutineTab::Charts, cache);
        let rows = rows_of(&html, "table");

        assert_eq!(
            rows[0][rows[0].len() - 3..],
            ["Stretch sets", "Stretch (min)", ""]
        );
        assert_eq!(rows[1][2..4], ["2", "1"]);
        assert_eq!(rows[1][rows[1].len() - 3..rows[1].len() - 1], ["1", "1"]);
        assert!(
            charts.contains("Time under stretch (min)</span>"),
            "{charts}"
        );
    }

    #[test]
    fn test_without_stretch_sets_no_stretch_columns_and_chart_are_shown() {
        let cache = || with_stretching_routine(vec![timed_set(1, 5, 0)]);
        let html = render_tab(1, RoutineTab::Sessions, cache);
        let charts = render_tab(1, RoutineTab::Charts, cache);

        assert!(!rows_of(&html, "table")[0].contains(&"Stretch sets".to_string()));
        assert!(!charts.contains("Time under stretch (min)"));
    }

    #[rstest::rstest]
    #[case::without_reps(0, "Time", false)]
    #[case::with_reps(5, "Time per rep", true)]
    fn test_the_time_of_an_activity_is_labelled_by_its_reps(
        #[case] reps: u32,
        #[case] label: &str,
        #[case] has_help: bool,
    ) {
        let html = render(move || {
            with_routine()
                .with_routines(vec![domain::Routine {
                    sections: vec![section(vec![activity_with_tempo(reps, &[30])])],
                    ..routine()
                }])
                .provide();
            provide_settings(web_app::Settings::default());
            rsx! { ActivityDialog {} }
        });

        assert!(html.contains(&format!(">{label}</label>")), "{html}");
        assert_eq!(html.contains("Optionally split"), has_help);
    }

    #[rstest::rstest]
    #[case::reps_and_tempo(activity_with_tempo(5, &[3]), false)]
    #[case::hold(activity_with_tempo(0, &[30]), false)]
    #[case::reps_without_tempo(activity_with_tempo(5, &[]), true)]
    fn test_the_options_of_a_set_that_need_a_tempo_are_disabled_without_one(
        #[case] part: domain::RoutinePart,
        #[case] disabled: bool,
    ) {
        let html = render_activity_dialog(part, |_| {});

        assert_eq!(
            disabled_texts_of(&html, "button-select-automatic"),
            if disabled { vec!["Automatic"] } else { vec![] }
        );
        assert_eq!(
            disabled_texts_of(&html, "button-select-countdown"),
            if disabled { vec!["Countdown"] } else { vec![] }
        );
    }

    #[rstest::rstest]
    #[case::with_time(rest(&[60]))]
    #[case::without_time(rest(&[]))]
    fn test_a_rest_can_move_on_automatically_and_has_no_recording(
        #[case] part: domain::RoutinePart,
    ) {
        let html = render_activity_dialog(part, |_| {});

        assert!(disabled_texts_of(&html, "button-select-automatic").is_empty());
        assert!(!contains(&html, "button-select-countdown"));
    }

    #[test]
    fn test_an_invalid_tempo_disables_the_options_that_need_a_tempo() {
        let html = render_activity_dialog(activity_with_tempo(5, &[3]), |dialog| {
            if let EditDialog::EditActivity { tempo, .. } = dialog {
                tempo[0].input = "x".to_string();
                tempo[0].validated = Err("time must be an integer".to_string());
            }
        });

        assert_eq!(
            disabled_texts_of(&html, "button-select-automatic"),
            ["Automatic"]
        );
        assert_eq!(
            disabled_texts_of(&html, "button-select-countdown"),
            ["Countdown"]
        );
    }

    #[test]
    fn test_a_set_without_a_tempo_shows_its_effective_start_and_recording() {
        let html =
            render_activity_dialog(with_countdown(with_automatic(activity(5), true)), |_| {});

        assert_eq!(selected_text_of(&html, "button-select-automatic"), "Manual");
        assert_eq!(
            selected_text_of(&html, "button-select-countdown"),
            "Input fields"
        );
    }

    #[rstest::rstest]
    #[case::with_tempo(activity_with_tempo(5, &[3]), true)]
    #[case::without_tempo(activity(5), false)]
    fn test_a_stored_automatic_start_without_effect_can_be_saved_as_manual(
        #[case] part: domain::RoutinePart,
        #[case] disabled: bool,
    ) {
        let html = render_activity_dialog(with_automatic(part, true), |_| {});

        assert_eq!(
            all_attributes_of(&html, "dialog-save", "disabled") == vec!["true"],
            disabled
        );
    }

    #[rstest::rstest]
    #[case::reps_and_tempo(activity_with_tempo(5, &[3]), false)]
    #[case::reps_without_tempo(activity_with_tempo(5, &[]), true)]
    fn test_a_change_of_a_countdown_without_effect_cannot_be_saved(
        #[case] part: domain::RoutinePart,
        #[case] disabled: bool,
    ) {
        let html = render_activity_dialog(part, |dialog| {
            if let EditDialog::EditActivity { countdown, .. } = dialog {
                *countdown = Some(true);
            }
        });

        assert_eq!(
            all_attributes_of(&html, "dialog-save", "disabled") == vec!["true"],
            disabled
        );
    }

    #[rstest::rstest]
    #[case::tapped(with_countdown(activity_with_tempo(5, &[3])), 5, Some(false), false)]
    #[case::stored_hold(activity_with_tempo(0, &[30]), 0, None, false)]
    #[case::stored_set_with_reps(with_countdown(activity_with_tempo(5, &[3])), 5, None, true)]
    #[case::new_hold(activity(0), 0, None, true)]
    #[case::new_set_with_reps(activity(0), 5, None, false)]
    #[case::cleared_reps(activity_with_tempo(5, &[3]), 0, None, true)]
    #[case::entered_reps(with_countdown(activity_with_tempo(0, &[30])), 5, None, false)]
    fn test_an_untapped_recording_follows_the_kind_of_the_activity(
        #[case] stored: domain::RoutinePart,
        #[case] reps: u32,
        #[case] chosen: Option<bool>,
        #[case] expected: bool,
    ) {
        assert_eq!(
            edited_countdown(Some(&stored), domain::Reps::new(reps).unwrap(), chosen),
            expected
        );
    }

    #[rstest::rstest]
    #[case::countdown_with_reps(with_countdown(activity_with_tempo(5, &[3])), true)]
    #[case::hold(with_countdown(activity_with_tempo(0, &[30])), true)]
    #[case::hold_with_input_fields(activity_with_tempo(0, &[30]), false)]
    #[case::input_fields(activity_with_tempo(5, &[3]), false)]
    #[case::untimed(with_countdown(activity(5)), false)]
    fn test_only_a_counted_down_activity_is_marked(
        #[case] part: domain::RoutinePart,
        #[case] expected: bool,
    ) {
        let html = render_routine(1, move || {
            with_routine().with_routines(vec![domain::Routine {
                sections: vec![section(vec![part.clone()])],
                ..routine()
            }])
        });

        assert_eq!(contains(&html, "set-countdown"), expected);
    }

    fn render_activity_dialog(part: domain::RoutinePart, modify: fn(&mut EditDialog)) -> String {
        render(move || {
            with_routine()
                .with_routines(vec![domain::Routine {
                    sections: vec![section(vec![part.clone()])],
                    ..routine()
                }])
                .provide();
            provide_settings(web_app::Settings::default());
            provide_context(DialogChange(modify));
            rsx! { ActivityDialog {} }
        })
    }

    /// A change applied to the dialog of [`ActivityDialog`] before it is rendered.
    #[derive(Clone, Copy)]
    struct DialogChange(fn(&mut EditDialog));

    fn with_countdown(mut part: domain::RoutinePart) -> domain::RoutinePart {
        if let domain::RoutinePart::RoutineActivity { countdown, .. } = &mut part {
            *countdown = true;
        }
        part
    }

    fn with_automatic(mut part: domain::RoutinePart, value: bool) -> domain::RoutinePart {
        if let domain::RoutinePart::RoutineActivity { automatic, .. } = &mut part {
            *automatic = value;
        }
        part
    }

    fn rest(phases: &[u32]) -> domain::RoutinePart {
        domain::RoutinePart::RoutineActivity {
            exercise_id: domain::ExerciseID::nil(),
            reps: domain::Reps::default(),
            tempo: domain::Tempo::new(phases).unwrap(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            automatic: true,
            countdown: false,
            sides: domain::Sides::Combined,
        }
    }

    #[component]
    fn ActivityDialog() -> Element {
        let cache = consume_context::<Cache>();
        let routine = match &*cache.routines.read() {
            CacheState::Ready(routines) => routines[0].clone(),
            CacheState::Loading | CacheState::Error(_) => unreachable!(),
        };
        let change = try_consume_context::<DialogChange>();
        let dialog = use_signal(|| {
            let mut dialog = edit_activity_dialog(&routine, &vec![0, 0].into(), None);
            if let Some(DialogChange(change)) = change {
                change(&mut dialog);
            }
            dialog
        });
        view_edit_dialog(dialog, cache, use_context::<Settings>())
    }

    #[test]
    fn test_parse_drop_target() {
        assert_eq!(DropTarget::parse("remove"), Some(DropTarget::Remove));
        assert_eq!(
            DropTarget::parse("sections"),
            Some(DropTarget::Section(domain::RoutinePartPath::default()))
        );
        assert_eq!(
            DropTarget::parse("section-1-0"),
            Some(DropTarget::Section(vec![1, 0].into()))
        );
        assert_eq!(
            DropTarget::parse("part-0"),
            Some(DropTarget::Part(vec![0].into()))
        );
        assert_eq!(
            DropTarget::parse("header-2-0"),
            Some(DropTarget::Header(vec![2, 0].into()))
        );
        assert_eq!(DropTarget::parse("part-x"), None);
        assert_eq!(DropTarget::parse("part-"), None);
        assert_eq!(DropTarget::parse("foo"), None);
    }

    #[test]
    fn test_insertion_target() {
        assert_eq!(
            insertion_target(&vec![2, 0].into(), false),
            DropTarget::Gap(vec![0].into(), 2)
        );
        assert_eq!(
            insertion_target(&vec![2, 0].into(), true),
            DropTarget::Gap(vec![0].into(), 3)
        );
        assert_eq!(
            insertion_target(&domain::RoutinePartPath::default(), true),
            DropTarget::Part(domain::RoutinePartPath::default())
        );
    }

    #[test]
    fn test_header_target() {
        assert_eq!(
            header_target(&vec![2, 0].into(), false),
            DropTarget::Gap(vec![0].into(), 2)
        );
        assert_eq!(
            header_target(&vec![2, 0].into(), true),
            DropTarget::Gap(vec![2, 0].into(), 0)
        );
    }

    #[test]
    fn test_is_valid_target() {
        let routine = routine();
        assert!(is_valid_target(
            &routine,
            &vec![0, 0].into(),
            &DropTarget::Remove
        ));
        assert!(is_valid_target(
            &routine,
            &vec![0].into(),
            &DropTarget::Gap(domain::RoutinePartPath::default(), 1)
        ));
        assert!(is_valid_target(
            &routine,
            &vec![0, 0].into(),
            &DropTarget::Gap(vec![1].into(), 0)
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![0, 0].into(),
            &DropTarget::Gap(domain::RoutinePartPath::default(), 0)
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![1].into(),
            &DropTarget::Gap(vec![1].into(), 0)
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![1].into(),
            &DropTarget::Gap(vec![0, 1].into(), 0)
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![0].into(),
            &DropTarget::Part(vec![1].into())
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![0].into(),
            &DropTarget::Header(vec![1].into())
        ));
        assert!(!is_valid_target(
            &routine,
            &vec![0].into(),
            &DropTarget::Section(vec![1].into())
        ));
    }

    #[test]
    fn test_apply_drop() {
        let routine = routine();
        assert_eq!(
            apply_drop(
                &routine,
                &vec![0, 0].into(),
                DropTarget::Gap(vec![0].into(), 2)
            )
            .unwrap()
            .sections,
            vec![
                section(vec![activity(2), activity(1)]),
                section(vec![section(vec![activity(3)])]),
            ]
        );
        assert_eq!(
            apply_drop(&routine, &vec![0, 0].into(), DropTarget::Remove)
                .unwrap()
                .sections,
            vec![
                section(vec![activity(2)]),
                section(vec![section(vec![activity(3)])]),
            ]
        );
        assert!(apply_drop(&routine, &vec![0].into(), DropTarget::Part(vec![1].into())).is_none());
    }

    #[test]
    fn test_path_attribute_and_parse_path() {
        assert_eq!(path_attribute(&vec![1, 0].into()), "1-0");
        assert_eq!(parse_path("1-0"), Some(vec![1, 0].into()));
        assert_eq!(parse_path(""), None);
    }
}
