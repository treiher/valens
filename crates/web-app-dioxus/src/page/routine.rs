use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, ERRORS, Route,
    cache::{Cache, CacheState},
    eh,
    page::{
        self,
        common::{Chart, IntervalControl, SetsPerMuscle},
    },
    settings::Settings,
    ui::{
        element::{
            Block, CenteredBlock, DataBox, Dialog, Error, ErrorMessage, FloatingActionButton, Icon,
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
                    {view_routine(routine, exercises, edit_dialog, cache)}
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
                            rsx! { Error { message: err } }
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
                    ErrorMessage { message: "Routine not found" }
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
            rsx! { ErrorMessage { message: err } }
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
    cache: Cache,
) -> Element {
    rsx! {
        Block {
            div {
                class: "p-2",
                for (i, section) in routine.sections.iter().enumerate() {
                    {view_routine_part(routine, section, &vec![i].into(), exercises, edit_dialog)}
                }
            }
            div {
                class: "has-text-centered",
                button {
                    class: "button is-white-soft",
                    class: if IS_LOADING() && matches!(edit_dialog(), EditDialog::None) { "is-loading" },
                    "data-testid": "add-section",
                    onclick: eh!(mut routine; {
                        routine.add_section(&domain::RoutinePartPath::default());
                        modify_routine_sections(routine, cache, || {})
                    }),
                    Icon { name: "plus" }
                }
            }
        }
    }
}

fn view_routine_part(
    routine: &domain::Routine,
    part: &domain::RoutinePart,
    path: &domain::RoutinePartPath,
    exercises: &[domain::Exercise],
    mut edit_dialog: Signal<EditDialog>,
) -> Element {
    let show_options = {
        let routine = routine.clone();
        let path = path.clone();
        move || {
            *edit_dialog.write() = EditDialog::Options {
                routine: routine.clone(),
                path: path.clone(),
            }
        }
    };
    match part {
        domain::RoutinePart::RoutineSection { rounds, parts } => {
            rsx! {
                div {
                    class: "message",
                    div {
                        class: "message-body p-3 mb-3",
                        class: if path.first() != Some(&0) { "mt-3" },
                        "data-testid": "routine-section",
                        div {
                            class: "is-flex is-justify-content-space-between mb-3",
                            IconText {
                                icon: "repeat",
                                text: "{rounds}",
                                "data-testid": "section-rounds",
                                on_click: eh!(mut edit_dialog; routine, path; {
                                    if let Some(domain::RoutinePart::RoutineSection {
                                        rounds, ..
                                    }) = routine.part(&path) {
                                        let rounds = FieldValue::new_with_empty_default(*rounds);
                                        *edit_dialog.write() = EditDialog::EditSection { routine, path, rounds };
                                    }
                                })
                            }
                            Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "section-options" }
                        }
                        for (i, part) in parts.iter().enumerate() {
                            {view_routine_part(routine, part, &[&[i], &path[..]].concat().into(), exercises, edit_dialog)}
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
                    div {
                        class: "message-body has-background-scheme-main p-3",
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
                                Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "activity-options" }
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
                                            let routine = routine.clone();
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
                                Icon { name: "ellipsis-vertical", on_click: eh!(mut show_options; { show_options(); }), "data-testid": "activity-options" }
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
                                        let routine = routine.clone();
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

fn view_previous_exercises(
    routine: &domain::Routine,
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
) -> Element {
    let training_sessions = &training_sessions
        .iter()
        .filter(|t| t.routine_id == routine.id)
        .collect::<Vec<_>>();
    let all_exercise_ids = &training_sessions
        .iter()
        .flat_map(|t| t.exercises())
        .collect::<BTreeSet<_>>();
    let previous_exercise_ids = all_exercise_ids - &routine.exercises();

    if previous_exercise_ids.is_empty() {
        rsx! {}
    } else {
        let previous_exercises = previous_exercise_ids
            .iter()
            .filter_map(|exercise_id| exercises.iter().find(|e| e.id == *exercise_id))
            .collect::<Vec<_>>();
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
            if let domain::TrainingSessionElement::Set { rpe: Some(rpe), .. } = element {
                rpe_values.push((training_session.date, f32::from(*rpe)));
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
                                right_icon: rsx! { "✕" },
                                inputmode: "numeric",
                                value: rounds_field.input.clone(),
                                error: if let Err(err) = &rounds_field.validated { err.clone() },
                                has_changed: rounds_field.changed(),
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
                        ErrorMessage { message: "Unexpected routine part type" }
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
                        ErrorMessage { message: "Unexpected routine part type" }
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
    IS_LOADING.with_mut(|is_loading| *is_loading = true);
    match DOMAIN_SERVICE()
        .modify_routine(routine.id, None, None, Some(routine.sections))
        .await
    {
        Ok(_) => {
            cache.refresh_routines();
        }
        Err(err) => {
            ERRORS
                .write()
                .push(format!("Failed to modify routine: {err}"));
        }
    }
    IS_LOADING.with_mut(|is_loading| *is_loading = false);
    close_dialog();
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
