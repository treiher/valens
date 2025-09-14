use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{
    self as domain, ExerciseService, Property, RoutineService, SessionService,
    TrainingSessionService,
};
use valens_web_app::{self as web_app, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        element::{
            Block, CenteredBlock, Chart, ChartLabel, DataBox, Dialog, Error, ErrorMessage,
            FloatingActionButton, Icon, IconText, IntervalControl, Loading, LoadingPage,
            MenuOption, NoConnection, NoData, OptionsMenu, TagsWithAddon, Title,
        },
        form::{ButtonSelectField, ButtonSelectOption, FieldValue, FieldValueState, InputField},
    },
    eh, ensure_session, page, signal_changed_data,
};

#[component]
pub fn Routine(id: domain::RoutineID) -> Element {
    ensure_session!();

    let routine = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_routine(id).await
    });
    let memorized_routine = use_memo(move || {
        routine
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .and_then(std::clone::Clone::clone)
    });
    let training_sessions = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        if let Some(r) = memorized_routine() {
            Some(
                DOMAIN_SERVICE
                    .read()
                    .get_training_sessions_by_routine_id(r.id)
                    .await,
            )
        } else {
            None
        }
    });
    let exercises = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_exercises().await
    });
    let dates = use_memo(move || {
        if let Some(Some(Ok(training_session))) = &*training_sessions.read() {
            training_session
                .iter()
                .map(|ts| ts.date)
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    });
    let current_interval =
        use_signal(|| domain::init_interval(&dates.read(), domain::DefaultInterval::_3M));
    let all = *use_memo(move || domain::Interval {
        first: dates.read().iter().min().copied().unwrap_or_default(),
        last: dates.read().iter().max().copied().unwrap_or_default(),
    })
    .read();
    let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
    let edit_dialog = use_signal(|| EditDialog::None);
    let mut routine_dialog = use_signal(|| page::routines::RoutineDialog::None);
    let training_dialog = use_signal(|| page::training::TrainingDialog::None);

    match (
        &*routine.read(),
        &*training_sessions.read(),
        &*exercises.read(),
        &*settings.read(),
    ) {
        (
            Some(Ok(Some(routine))),
            Some(training_sessions),
            Some(Ok(exercises)),
            Some(Ok(settings)),
        ) => {
            rsx! {
                Title { title: "{routine.name}", x_padding: 2 }
                {view_summary(routine)}
                {view_routine(routine, exercises, edit_dialog)}
                if let Some(Ok(training_sessions)) = training_sessions {
                    {view_previous_exercises(routine, training_sessions, exercises)}
                }
                {view_muscles(routine, exercises)}
                match training_sessions {
                    Some(Ok(training_sessions)) => {
                        let routines = &[routine.clone()];
                        if training_sessions.is_empty() {
                            rsx! {
                                NoData {}
                            }
                        } else {
                            let interval = *current_interval.read();
                            let training_sessions = training_sessions
                                .iter()
                                .filter(|t| t.date >= interval.first && t.date <= interval.last)
                                .cloned()
                                .collect::<Vec<_>>();
                            rsx! {
                                CenteredBlock {
                                    Title { title: "Training sessions" },
                                    IntervalControl { current_interval, all },
                                    if training_sessions.is_empty() {
                                        NoData {}
                                    } else {
                                        {view_charts(&training_sessions, interval, *settings)}
                                        {page::training::view_calendar(&training_sessions, interval)}
                                        {page::training::view_table(&training_sessions, routines, interval, training_dialog, *settings)}
                                        {page::training::view_dialog(training_dialog, &training_sessions, routines, None)}
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(err)) => {
                        rsx! { Error { message: err } }
                    }
                    None => {
                        rsx! {
                            Loading {}
                        }
                    }
                }
                {page::routines::view_dialog(routine_dialog, None)}
                {view_edit_dialog(edit_dialog)}
                FloatingActionButton {
                    icon: "edit".to_string(),
                    onclick: eh!(routine; {
                        *routine_dialog.write() = page::routines::RoutineDialog::Options(routine.clone());
                    }),
                }
            }
        }
        (Some(Ok(None)), _, _, _) => {
            rsx! {
                ErrorMessage { message: "Routine not found" }
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _, _) | (_, _, Some(Err(err)), _) => {
            rsx! { ErrorMessage { message: err } }
        }
        (_, _, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _, _, _) | (_, None, _, _) | (_, _, None, _) | (_, _, _, None) => {
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
                    onclick: eh!(mut routine; {
                        routine.add_section(&domain::RoutinePartPath::default());
                        modify_routine_sections(routine, || {})
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
                        div {
                            class: "is-flex is-justify-content-space-between mb-3",
                            IconText {
                                icon: "repeat",
                                text: "{rounds}",
                                onclick: eh!(mut edit_dialog; routine, path; {
                                    if let Some(domain::RoutinePart::RoutineSection {
                                        rounds, ..
                                    }) = routine.part(&path) {
                                        let rounds = FieldValue::new_with_empty_default(*rounds);
                                        *edit_dialog.write() = EditDialog::EditSection { routine, path, rounds };
                                    }
                                }),
                            }
                            Icon { name: "ellipsis-vertical", onclick: eh!(mut show_options; { show_options(); }) }
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
                                if let Some(exercise) = exercises.iter().find(|e| e.id == *exercise_id) {
                                    Link {
                                        to: Route::Exercise { id: exercise.id },
                                        "{exercise.name}"
                                    }
                                } else {
                                    "Exercise#{exercise_id.as_u128()}"
                                }
                                Icon { name: "ellipsis-vertical", onclick: eh!(mut show_options; { show_options(); }) }
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
                                        "Rest"
                                    }
                                    if *time != domain::Time::default() {
                                        span {
                                            class: "icon-text mr-4",
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
                                Icon { name: "ellipsis-vertical", onclick: eh!(mut show_options; { show_options(); }) }
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
                Title { title: "Previously used exercises" }
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
    settings: web_app::Settings,
) -> Element {
    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
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
    }

    let theme = settings.current_theme();

    rsx! {
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Load".to_string(),
                    color: web_app::chart::COLOR_LOAD,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: load.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(
                            web_app::chart::COLOR_LOAD,
                            web_app::chart::COLOR_LOAD,
                        ),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }
                ],
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Set volume".to_string(),
                    color: web_app::chart::COLOR_SET_VOLUME,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: set_volume.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(
                            web_app::chart::COLOR_SET_VOLUME,
                            web_app::chart::COLOR_SET_VOLUME,
                        ),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }
                ],
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        if settings.show_rpe {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "RPE".to_string(),
                        color: web_app::chart::COLOR_RPE,
                        opacity: web_app::chart::OPACITY_AREA,
                    },
                    ChartLabel {
                        name: "Avg. RPE".to_string(),
                        color: web_app::chart::COLOR_RPE,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot_min_avg_max(
                    &training_sessions
                        .iter()
                        .flat_map(|s| s
                            .elements
                            .iter()
                            .filter_map(|e| match e {
                                domain::TrainingSessionElement::Set { rpe, .. } =>
                                    rpe.map(|v| (s.date, v)),
                                domain::TrainingSessionElement::Rest { .. } => None,
                            })
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                    interval,
                    web_app::chart::PlotParams::primary_range(5., 10.),
                    web_app::chart::COLOR_RPE,
                    theme,
                ).map_err(|err| err.to_string()),
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
                Title { title: "Sets per muscle" },
                {view_sets_per_muscle(&stimulus_per_muscle)}
            }
        }
    }
}

pub fn view_sets_per_muscle(
    stimulus_per_muscle: &BTreeMap<domain::MuscleID, domain::Stimulus>,
) -> Element {
    let mut stimulus_per_muscle = stimulus_per_muscle
        .iter()
        .map(|(muscle_id, stimulus)| (*muscle_id, *stimulus))
        .collect::<Vec<_>>();
    stimulus_per_muscle.sort_by(|a, b| b.1.cmp(&a.1));
    let mut groups = [vec![], vec![], vec![], vec![]];
    for (muscle, stimulus) in stimulus_per_muscle {
        let name = muscle.name();
        let description = muscle.description();
        let sets = f64::from(*stimulus) / 100.0;
        let sets_str = format!("{:.1$}", sets, usize::from(sets.fract() != 0.0));
        if sets > 10.0 {
            groups[0].push((name, description, sets_str, vec!["is-dark"]));
        } else if sets >= 3.0 {
            groups[1].push((name, description, sets_str, vec!["is-dark", "is-link"]));
        } else if sets > 0.0 {
            groups[2].push((name, description, sets_str, vec!["is-light", "is-link"]));
        } else {
            groups[3].push((name, description, sets_str, vec![]));
        }
    }
    rsx! {
        for tags in groups {
            if !tags.is_empty() {
                TagsWithAddon { tags }
            }
        }
    }
}

fn view_edit_dialog(mut edit_dialog: Signal<EditDialog>) -> Element {
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
                OptionsMenu {
                    options: vec![
                        rsx! {
                            if part_type == PartType::Section {
                                MenuOption {
                                    icon: "person-running".to_string(),
                                    text: "Add exercise".to_string(),
                                    onclick: eh!(mut edit_dialog; routine, path; {
                                        *edit_dialog.write() = EditDialog::AddExercise { routine, path };
                                    })
                                },
                                MenuOption {
                                    icon: "person".to_string(),
                                    text: "Add rest".to_string(),
                                    onclick: eh!(mut routine; path, close_dialog; {
                                        routine.add_activity(domain::ExerciseID::nil(), &path);
                                        modify_routine_sections(routine, close_dialog)
                                    })
                                },
                                MenuOption {
                                    icon: "repeat".to_string(),
                                    text: "Add section".to_string(),
                                    onclick: eh!(mut routine; path, close_dialog; {
                                        routine.add_section(&path);
                                        modify_routine_sections(routine, close_dialog)
                                    })
                                },
                            }
                            MenuOption {
                                icon: "arrow-up".to_string(),
                                text: "Move up".to_string(),
                                onclick: eh!(mut routine; path, close_dialog; {
                                    routine.move_part_up(&path);
                                    modify_routine_sections(routine, close_dialog)
                                })
                            },
                            MenuOption {
                                icon: "arrow-down".to_string(),
                                text: "Move down".to_string(),
                                onclick: eh!(mut routine; path, close_dialog; {
                                    routine.move_part_down(&path);
                                    modify_routine_sections(routine, close_dialog)
                                })
                            },
                            if part_type == PartType::Exercise {
                                MenuOption {
                                    icon: "arrow-right-arrow-left".to_string(),
                                    text: "Replace exercise".to_string(),
                                    onclick: eh!(mut edit_dialog; routine, path; {
                                        *edit_dialog.write() = EditDialog::ReplaceExercise { routine, path };
                                    })
                                },
                            }
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Edit".to_string(),
                                onclick: eh!(mut edit_dialog; routine, path; {
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
                                        _ => { }
                                    }
                                })
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Remove".to_string(),
                                onclick: eh!(mut routine; path, close_dialog; {
                                    routine.remove_part(&path);
                                    modify_routine_sections(routine, close_dialog)
                                })
                            },
                        },
                    ],
                    close_event: eh!(mut close_dialog; { close_dialog(); })
                }
            }
        }
        EditDialog::AddExercise { routine, path } => {
            rsx! {
                Dialog {
                    title: rsx! { "Add exercise" },
                    close_event: eh!(mut close_dialog; { close_dialog(); }),
                    page::exercises::ExerciseList {
                        add: false,
                        filter: String::new(),
                        change_route: false,
                        exercise_onclick: {
                            let routine = routine.clone();
                            let path = path.clone();
                            move |(_, id)| {
                                let mut routine = routine.clone();
                                let path = path.clone();
                                routine.add_activity(id, &path);
                                modify_routine_sections(routine, close_dialog)
                            }
                        },
                        catalog_onclick: |_| {}
                    }
                }
            }
        }
        EditDialog::ReplaceExercise { routine, path } => {
            rsx! {
                Dialog {
                    title: rsx! { "Replace exercise" },
                    close_event: eh!(mut close_dialog; { close_dialog(); }),
                    page::exercises::ExerciseList {
                        add: false,
                        filter: String::new(),
                        change_route: false,
                        exercise_onclick: {
                            let routine = routine.clone();
                            let path = path.clone();
                            move |(_, id)| {
                                let mut routine = routine.clone();
                                let path = path.clone();
                                routine.update_activity(Some(id), None, None, None, None, None, &path);
                                modify_routine_sections(routine, close_dialog)
                            }
                        },
                        catalog_onclick: |_| {}
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
                modify_routine_sections(routine, close_dialog)
            });
            match routine.part(path) {
                Some(domain::RoutinePart::RoutineSection { .. }) => {
                    rsx! {
                        Dialog {
                            close_event: eh!(mut close_dialog; { close_dialog(); }),
                            Block {
                                InputField {
                                    label: "Rounds",
                                    right_icon: rsx! { "✕" },
                                    inputmode: "numeric",
                                    value: rounds_field.input.clone(),
                                    error: if let Err(err) = &rounds_field.validated { err.clone() },
                                    has_changed: rounds_field.changed(),
                                    oninput: move |event: FormEvent| {
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
                            Block {
                                div {
                                    class: "field is-grouped is-grouped-centered",
                                    div {
                                        class: "control",
                                        onclick: eh!(mut close_dialog; { close_dialog(); }),
                                        button { class: "button is-light is-soft", "Cancel" }
                                    }
                                    div {
                                        class: "control",
                                        onclick: save,
                                        button {
                                            class: "button is-primary",
                                            // TODO: class: if is_loading() { "is-loading" },
                                            disabled: !FieldValue::has_valid_changes(&[rounds_field as &dyn FieldValueState]),
                                            "Save"
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
                modify_routine_sections(routine, close_dialog)
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
                        Dialog {
                            close_event: eh!(mut close_dialog; { close_dialog(); }),
                            Block {
                                if !exercise_id.is_nil() {
                                    InputField {
                                        label: "Reps",
                                        right_icon: rsx! { "✕" },
                                        inputmode: "numeric",
                                        value: reps_field.input.clone(),
                                        error: if let Err(err) = &reps_field.validated { err.clone() },
                                        has_changed: reps_field.changed(),
                                        oninput: move |event: FormEvent| {
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
                                        }
                                    }
                                }
                                InputField {
                                    label: "Time",
                                    right_icon: rsx! { "s" },
                                    inputmode: "numeric",
                                    value: time_field.input.clone(),
                                    error: if let Err(err) = &time_field.validated { err.clone() },
                                    has_changed: time_field.changed(),
                                    oninput: move |event: FormEvent| {
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
                                    }
                                }
                                if !exercise_id.is_nil() {
                                    InputField {
                                        label: "Weight",
                                        right_icon: rsx! { "kg" },
                                        inputmode: "numeric",
                                        value: weight_field.input.clone(),
                                        error: if let Err(err) = &weight_field.validated { err.clone() },
                                        has_changed: weight_field.changed(),
                                        oninput: move |event: FormEvent| {
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
                                        }
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
                                        oninput: move |event: FormEvent| {
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
                                        }
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
                                    onclick: {
                                        move |(_, value): (_, bool)| {
                                            async move {
                                                if let EditDialog::EditActivity { automatic, .. } =  &mut *edit_dialog.write() {
                                                    automatic.input = value.to_string();
                                                    automatic.validated = Ok(value);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Block {
                                div {
                                    class: "field is-grouped is-grouped-centered",
                                    div {
                                        class: "control",
                                        onclick: eh!(mut close_dialog; { close_dialog(); }),
                                        button { class: "button is-light is-soft", "Cancel" }
                                    }
                                    div {
                                        class: "control",
                                        onclick: save,
                                        button {
                                            class: "button is-primary",
                                            // TODO: class: if is_loading() { "is-loading" },
                                            disabled: !FieldValue::has_valid_changes(&[reps_field as &dyn FieldValueState, time_field, weight_field, rpe_field, automatic_field]) || validated_automatic.is_err(),
                                            "Save"
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
    }
}

async fn modify_routine_sections(routine: domain::Routine, mut close_dialog: impl FnMut()) {
    match DOMAIN_SERVICE
        .read()
        .modify_routine(routine.id, None, None, Some(routine.sections))
        .await
    {
        Ok(_) => {
            signal_changed_data();
        }
        Err(err) => {
            NOTIFICATIONS
                .write()
                .push(format!("Failed to modify routine: {err}"));
        }
    };
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
