use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    eh,
    loading::LoadingFlag,
    notification::notify,
    page::{
        self,
        common::{Chart, IntervalControl, SetsPerMuscle},
    },
    settings::Settings,
    ui::{
        drag_and_drop,
        element::{
            Block, CenteredBlock, DataBox, Dialog, Error, ErrorPage, FloatingActionButton, Icon,
            IconText, Loading, LoadingDialog, LoadingPage, MenuOption, NoConnection, NoData,
            OptionsMenu, SaveDialog, Title,
        },
        form::{ButtonSelectField, ButtonSelectOption, FieldValue, FieldValueState, InputField},
    },
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn Routine(id: domain::RoutineID) -> Element {
    let cache = consume_context::<Cache>();
    let mut current_interval = use_signal(domain::Interval::default);
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
                rsx! {
                    Title { "{routine.name}" }
                    {view_summary(routine)}
                    {view_routine(routine, exercises, edit_dialog, drag, cache)}
                    {drag_and_drop::view_drag_overlay(drag, &DropTarget::Remove)}
                    if let CacheState::Ready(training_sessions) = training_sessions {
                        {view_previous_exercises(routine, training_sessions, exercises)}
                    }
                    {view_muscles(routine, exercises)}
                    match training_sessions {
                        CacheState::Ready(training_sessions) => {
                            let training_sessions = training_sessions.iter()
                                .filter(|t| t.routine_id == id)
                                .cloned()
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
                                    current_interval.set(domain::init_interval(&dates, domain::DefaultInterval::All));
                                }
                                let interval = *current_interval.read();
                                let training_sessions = training_sessions
                                    .iter()
                                    .filter(|t| t.date >= interval.first && t.date <= interval.last)
                                    .cloned()
                                    .collect::<Vec<_>>();
                                rsx! {
                                    CenteredBlock {
                                        Title { "Training sessions" },
                                        IntervalControl { current_interval, all },
                                        if training_sessions.is_empty() {
                                            NoData {}
                                        } else {
                                            {view_charts(&training_sessions, interval, settings)}
                                            {page::training_sessions::view_calendar(&training_sessions, interval)}
                                            {page::training_sessions::view_table(&training_sessions, routines, interval, training_dialog, settings)}
                                            {page::training_sessions::view_dialog(training_dialog, &training_sessions, routines, None)}
                                        }
                                    }
                                }
                            }
                        }
                        CacheState::Error(err) => {
                            rsx! { Error { "{err}" } }
                        }
                        CacheState::Loading => {
                            rsx! {
                                Loading {}
                            }
                        }
                    }
                    {page::routines::view_dialog(routine_dialog, None)}
                    {view_edit_dialog(edit_dialog, cache)}
                    FloatingActionButton {
                        icon: "ellipsis-vertical",
                        on_click: eh!(routine; {
                            *routine_dialog.write() = page::routines::RoutineDialog::Options(routine.clone());
                        }),
                    }
                }
            } else {
                rsx! {
                    ErrorPage { "Routine not found" }
                }
            }
        }
        (
            CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)),
            _,
            _,
        ) => {
            rsx! { NoConnection {} }
        }
        (CacheState::Error(err), _, _) | (_, _, CacheState::Error(err)) => {
            rsx! { ErrorPage { "{err}" } }
        }
        (CacheState::Loading, _, _) | (_, _, CacheState::Loading) => {
            rsx! { LoadingPage {} }
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

fn view_routine(
    routine: &domain::Routine,
    exercises: &[domain::Exercise],
    edit_dialog: Signal<EditDialog>,
    drag: Signal<Option<Drag>>,
    cache: Cache,
) -> Element {
    let routine = Rc::new(routine.clone());
    rsx! {
        Block {
            div {
                class: "p-2",
                "data-testid": "routine-parts",
                "data-drop": "sections",
                for (i, section) in routine.sections.iter().enumerate() {
                    {view_routine_part(&routine, section, &vec![i].into(), routine.sections.len(), exercises, edit_dialog, drag)}
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

#[allow(clippy::too_many_lines)]
fn view_routine_part(
    routine: &Rc<domain::Routine>,
    part: &domain::RoutinePart,
    path: &domain::RoutinePartPath,
    num_siblings: usize,
    exercises: &[domain::Exercise],
    mut edit_dialog: Signal<EditDialog>,
    drag: Signal<Option<Drag>>,
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
                            {view_routine_part(routine, part, &[&[i], &path[..]].concat().into(), parts.len(), exercises, edit_dialog, drag)}
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
            time,
            weight,
            rpe,
            automatic,
        } => {
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
                                        to: Route::Exercise { id: exercise.id },
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
                                        if let Some(domain::RoutinePart::RoutineActivity {
                                            reps,
                                            time,
                                            weight,
                                            rpe,
                                            automatic,
                                            ..
                                        }) = routine.part(&path) {
                                            let routine = (*routine).clone();
                                            let reps = FieldValue::new_with_empty_default(*reps);
                                            let time = FieldValue::new_with_empty_default(*time);
                                            let weight = FieldValue::new_with_empty_default(*weight);
                                            let rpe = FieldValue::new_with_empty_default(*rpe);
                                            let automatic = FieldValue::new(*automatic);
                                            *edit_dialog.write() = EditDialog::EditActivity { routine, path, reps, time, weight, rpe, automatic };
                                        }
                                    }),
                                    span {
                                        class: "icon-text has-text-weight-bold mr-5",
                                        "data-testid": "rest-label",
                                        "Rest"
                                    }
                                    if *time != domain::Time::default() {
                                        span {
                                            class: "icon-text mr-4",
                                            "data-testid": "rest-time",
                                            span {
                                                class: "mr-2",
                                                Icon { name: "clock-rotate-left" }
                                                "{time.to_string()} s"
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
                                    if let Some(domain::RoutinePart::RoutineActivity {
                                        reps,
                                        time,
                                        weight,
                                        rpe,
                                        automatic,
                                        ..
                                    }) = routine.part(&path) {
                                        let routine = (*routine).clone();
                                        let reps = FieldValue::new_with_empty_default(*reps);
                                        let time = FieldValue::new_with_empty_default(*time);
                                        let weight = FieldValue::new_with_empty_default(*weight);
                                        let rpe = FieldValue::new_with_empty_default(*rpe);
                                        let automatic = FieldValue::new(*automatic);
                                        *edit_dialog.write() = EditDialog::EditActivity { routine, path, reps, time, weight, rpe, automatic };
                                    }
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
                                if *time != domain::Time::default() {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-time",
                                        span {
                                            class: "mr-2",
                                            Icon { name: "clock-rotate-left" }
                                            "{time} s"
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
                                if *rpe != domain::RPE::ZERO {
                                    span {
                                        class: "icon-text mr-4",
                                        "data-testid": "set-rpe",
                                        span {
                                            class: "mr-2",
                                            "@ {rpe}"
                                        }
                                    }
                                }
                                if *automatic {
                                    span {
                                        class: "icon",
                                        {automatic_icon()}
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
            class: if hovered { "has-text-text-bold" } else { "has-text-grey" },
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
    let previous_exercises = previous_exercises(routine, training_sessions, exercises);

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
                            to: Route::Exercise { id: exercise.id },
                            "{exercise.name}"
                        }
                    }
                }
            }
        }
    }
}

/// Returns the exercises used in training sessions of the routine that the routine no longer
/// contains, sorted by name.
fn previous_exercises<'a>(
    routine: &domain::Routine,
    training_sessions: &[domain::TrainingSession],
    exercises: &'a [domain::Exercise],
) -> Vec<&'a domain::Exercise> {
    let used_exercise_ids = training_sessions
        .iter()
        .filter(|t| t.routine_id == routine.id)
        .flat_map(domain::TrainingSession::exercises)
        .collect::<BTreeSet<_>>();
    let mut result = (&used_exercise_ids - &routine.exercises())
        .iter()
        .filter_map(|exercise_id| exercises.iter().find(|e| e.id == *exercise_id))
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn view_charts(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let params = web_app::chart::PlotParams::primary_range(0., 10.);
    let rpe_params = web_app::chart::PlotParams::primary_range(5., 10.);

    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut rpe_values: Vec<(NaiveDate, f32)> = vec![];
    for training_session in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        load.entry(training_session.date)
            .and_modify(|e| *e += training_session.load() as f32)
            .or_insert(training_session.load() as f32);
        #[allow(clippy::cast_precision_loss)]
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume() as f32)
            .or_insert(training_session.set_volume() as f32);
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
    }
}

fn view_muscles(routine: &domain::Routine, exercises: &[domain::Exercise]) -> Element {
    let stimulus_per_muscle = routine.stimulus_per_muscle(exercises);
    if stimulus_per_muscle.is_empty() {
        rsx! {}
    } else {
        rsx! {
            CenteredBlock {
                Title { "Sets per muscle" },
                SetsPerMuscle { stimulus_per_muscle: stimulus_per_muscle.clone() }
            }
        }
    }
}

fn view_edit_dialog(mut edit_dialog: Signal<EditDialog>, cache: Cache) -> Element {
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
                                            routine.add_activity(domain::ExerciseID::nil(), &path);
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
                                                reps,
                                                time,
                                                weight,
                                                rpe,
                                                automatic,
                                                ..
                                            }) => {
                                                let routine = routine.clone();
                                                let reps = FieldValue::new_with_empty_default(*reps);
                                                let time = FieldValue::new_with_empty_default(*time);
                                                let weight = FieldValue::new_with_empty_default(*weight);
                                                let rpe = FieldValue::new_with_empty_default(*rpe);
                                                let automatic = FieldValue::new(*automatic);
                                                *edit_dialog.write() = EditDialog::EditActivity { routine, path, reps, time, weight, rpe, automatic };
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
                            filter: String::new(),
                            on_exercise_click: {
                                let routine = routine.clone();
                                let path = path.clone();
                                move |(_, id)| {
                                    let mut routine = routine.clone();
                                    let path = path.clone();
                                    routine.add_activity(id, &path);
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
                            filter: String::new(),
                            on_exercise_click: {
                                let routine = routine.clone();
                                let path = path.clone();
                                move |(_, id)| {
                                    let mut routine = routine.clone();
                                    let path = path.clone();
                                    routine.update_activity(Some(id), None, None, None, None, None, &path);
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
                        Error { "Unexpected routine part type" }
                    }
                }
            }
        }
        EditDialog::EditActivity {
            routine,
            path,
            reps: reps_field,
            time: time_field,
            weight: weight_field,
            rpe: rpe_field,
            automatic: automatic_field,
        } => {
            fn validate_automatic(
                automatic: bool,
                exercise_id: domain::ExerciseID,
                reps: Option<domain::Reps>,
                time: Option<domain::Time>,
            ) -> Result<bool, String> {
                if !exercise_id.is_nil() && automatic {
                    if time.unwrap_or_default() == domain::Time::default() {
                        Err(
                            "Time must be greater than 0 to enable automatic start of timer"
                                .to_string(),
                        )
                    } else if reps.unwrap_or_default() != domain::Reps::default() {
                        Err("Reps must be undefined to enable automatic start of timer".to_string())
                    } else {
                        Ok(automatic)
                    }
                } else {
                    Ok(automatic)
                }
            }

            let save = eh!(mut routine; path, reps_field, time_field, weight_field, rpe_field, automatic_field, close_dialog; {
                routine.update_activity(None, reps_field.validated.ok(), time_field.validated.ok(), weight_field.validated.ok(), rpe_field.validated.ok(), automatic_field.validated.ok(), &path);
                modify_routine_sections(routine, cache, close_dialog)
            });
            match routine.part(path) {
                Some(domain::RoutinePart::RoutineActivity { exercise_id, .. }) => {
                    let validated_automatic = validate_automatic(
                        automatic_field.input == true.to_string(),
                        *exercise_id,
                        reps_field.validated.clone().ok(),
                        time_field.validated.clone().ok(),
                    );
                    rsx! {
                        SaveDialog {
                            on_close: eh!(mut close_dialog; { close_dialog(); }),
                            on_save: save,
                            is_loading: IS_LOADING(),
                            disabled: !FieldValue::has_valid_changes(&[reps_field as &dyn FieldValueState, time_field, weight_field, rpe_field, automatic_field]) || validated_automatic.is_err(),
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
                            InputField {
                                label: "Time",
                                right_icon: rsx! { "s" },
                                inputmode: "numeric",
                                value: time_field.input.clone(),
                                error: if let Err(err) = &time_field.validated { err.clone() },
                                has_changed: time_field.changed(),
                                autofocus: exercise_id.is_nil(),
                                on_input: move |event: FormEvent| {
                                    async move {
                                        if let EditDialog::EditActivity { time, .. } =  &mut *edit_dialog.write() {
                                            time.input = event.value();
                                            time.validated = if time.input.is_empty() {
                                                Ok(domain::Time::default())
                                            } else {
                                                domain::Time::try_from(time.input.as_ref())
                                                    .map_err(|err| err.to_string())
                                            };
                                        }
                                    }
                                },
                                "data-testid": "input-time",
                            }
                            if !exercise_id.is_nil() {
                                InputField {
                                    label: "Weight",
                                    right_icon: rsx! { "kg" },
                                    inputmode: "numeric",
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
                            if !exercise_id.is_nil() {
                                InputField {
                                    label: "RPE",
                                    left_icon: rsx! { "@" },
                                    inputmode: "numeric",
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
                                label: if exercise_id.is_nil() { "Transition to next part" } else { "Start of timer" },
                                options: vec![
                                    ButtonSelectOption {
                                        text: "Automatic".to_string(),
                                        value: true,
                                    },
                                    ButtonSelectOption {
                                        text: "Manual".to_string(),
                                        value: false,
                                    },
                                ],
                                selected: automatic_field.input == true.to_string(),
                                error: if let Err(err) = &validated_automatic { err.clone() },
                                has_changed: automatic_field.changed(),
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
                        }
                    }
                }
                _ => {
                    rsx! {
                        Error { "Unexpected routine part type" }
                    }
                }
            }
        }
    }
}

async fn modify_routine_sections(
    routine: domain::Routine,
    cache: Cache,
    mut close_dialog: impl FnMut(),
) {
    let _loading = LoadingFlag::set(&IS_LOADING);
    match DOMAIN_SERVICE()
        .modify_routine(routine.id, None, None, Some(routine.sections))
        .await
    {
        Ok(_) => {
            cache.refresh_routines();
        }
        Err(err) => {
            notify("Failed to modify routine", &err);
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
        time: FieldValue<domain::Time>,
        weight: FieldValue<domain::Weight>,
        rpe: FieldValue<domain::RPE>,
        automatic: FieldValue<bool>,
    },
}

#[cfg(test)]
mod tests {
    use crate::ui::drag_and_drop::DropTarget as _;

    use super::*;

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

    fn activity(reps: u32) -> domain::RoutinePart {
        domain::RoutinePart::RoutineActivity {
            exercise_id: 1.into(),
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::default(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    fn exercise(id: u128, name: &str) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(name).unwrap(),
            muscles: vec![],
        }
    }

    fn training_session(exercise_ids: &[u128]) -> domain::TrainingSession {
        domain::TrainingSession {
            id: 1.into(),
            routine_id: 1.into(),
            date: chrono::NaiveDate::default(),
            notes: String::new(),
            elements: exercise_ids
                .iter()
                .map(|exercise_id| domain::TrainingSessionElement::Set {
                    exercise_id: (*exercise_id).into(),
                    reps: domain::Reps::default(),
                    time: domain::Time::default(),
                    weight: domain::Weight::default(),
                    rpe: domain::RPE::default(),
                    target_reps: domain::Reps::default(),
                    target_time: domain::Time::default(),
                    target_weight: domain::Weight::default(),
                    target_rpe: domain::RPE::default(),
                    automatic: false,
                })
                .collect(),
            exercise_notes: BTreeMap::new(),
        }
    }

    #[test]
    fn test_previous_exercises_excludes_exercises_of_routine() {
        let exercises = [exercise(1, "A"), exercise(2, "B")];

        assert_eq!(
            previous_exercises(&routine(), &[training_session(&[1, 2])], &exercises),
            [&exercises[1]]
        );
    }

    #[test]
    fn test_previous_exercises_orders_by_name() {
        let exercises = [exercise(2, "C"), exercise(3, "B")];

        assert_eq!(
            previous_exercises(&routine(), &[training_session(&[2, 3])], &exercises),
            [&exercises[1], &exercises[0]]
        );
    }

    #[test]
    fn test_previous_exercises_ignores_other_routines() {
        let exercises = [exercise(2, "B")];
        let training_session = domain::TrainingSession {
            routine_id: 2.into(),
            ..training_session(&[2])
        };

        assert_eq!(
            previous_exercises(&routine(), &[training_session], &exercises),
            [] as [&domain::Exercise; 0]
        );
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
