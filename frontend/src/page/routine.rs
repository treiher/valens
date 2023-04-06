use std::collections::BTreeMap;
use std::collections::BTreeSet;

use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;
use crate::page::workouts;

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    let routine_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);
    let editing = url.next_hash_path_part() == Some("edit");

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Routine");

    let mut model = Model {
        interval: common::init_interval(&[], true),
        routine_id,
        sections: vec![],
        previous_exercises: BTreeSet::new(),
        dialog: Dialog::Hidden,
        editing,
        loading: false,
    };

    update_model(&mut model, data_model);

    model
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: common::Interval,
    routine_id: u32,
    sections: Vec<Form>,
    previous_exercises: BTreeSet<u32>,
    dialog: Dialog,
    editing: bool,
    loading: bool,
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.sections.iter().any(|s| s.changed())
    }

    pub fn mark_as_unchanged(&mut self) {
        for s in &mut self.sections {
            s.mark_as_unchanged();
        }
    }
}

enum Dialog {
    Hidden,
    SelectExercise(Vec<usize>, String),
    DeleteWorkout(u32),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum Form {
    Section {
        rounds: InputField<u32>,
        parts: Vec<Form>,
    },
    Activity {
        exercise_id: Option<u32>,
        reps: InputField<u32>,
        time: InputField<u32>,
        weight: InputField<f32>,
        rpe: InputField<f32>,
        automatic: bool,
    },
}

impl Form {
    fn changed(&self) -> bool {
        match self {
            Form::Section { rounds, parts } => rounds.changed || parts.iter().any(|p| p.changed()),
            Form::Activity {
                reps,
                time,
                weight,
                rpe,
                ..
            } => reps.changed || time.changed || weight.changed || rpe.changed,
        }
    }

    fn mark_as_unchanged(&mut self) {
        match self {
            Form::Section { rounds, parts } => {
                rounds.changed = false;
                for p in parts {
                    p.mark_as_unchanged();
                }
            }
            Form::Activity {
                reps,
                time,
                weight,
                rpe,
                ..
            } => {
                reps.changed = false;
                time.changed = false;
                weight.changed = false;
                rpe.changed = false;
            }
        }
    }

    fn valid(&self) -> bool {
        match self {
            Form::Section { rounds, parts } => rounds.valid && parts.iter().all(|p| p.valid()),
            Form::Activity {
                reps,
                time,
                weight,
                rpe,
                ..
            } => reps.valid && time.valid && weight.valid && rpe.valid,
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
struct InputField<T> {
    input: String,
    valid: bool,
    parsed: Option<T>,
    changed: bool,
}

impl<T> Default for InputField<T> {
    fn default() -> Self {
        InputField {
            input: String::new(),
            valid: true,
            parsed: None,
            changed: false,
        }
    }
}

impl From<&data::RoutinePart> for Form {
    fn from(part: &data::RoutinePart) -> Self {
        match part {
            data::RoutinePart::RoutineSection { rounds, parts, .. } => Form::Section {
                rounds: InputField {
                    input: if *rounds == 1 {
                        String::new()
                    } else {
                        rounds.to_string()
                    },
                    valid: true,
                    parsed: Some(*rounds),
                    changed: false,
                },
                parts: parts.iter().map(|p| p.into()).collect(),
            },
            data::RoutinePart::RoutineActivity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                automatic,
                ..
            } => Form::Activity {
                exercise_id: *exercise_id,
                reps: InputField {
                    input: if *reps == 0 {
                        String::new()
                    } else {
                        reps.to_string()
                    },
                    valid: true,
                    parsed: Some(*reps),
                    changed: false,
                },
                time: InputField {
                    input: if *time == 0 {
                        String::new()
                    } else {
                        time.to_string()
                    },
                    valid: true,
                    parsed: Some(*time),
                    changed: false,
                },
                weight: InputField {
                    input: if *weight == 0.0 {
                        String::new()
                    } else {
                        weight.to_string()
                    },
                    valid: true,
                    parsed: Some(*weight),
                    changed: false,
                },
                rpe: InputField {
                    input: if *rpe == 0.0 {
                        String::new()
                    } else {
                        rpe.to_string()
                    },
                    valid: true,
                    parsed: Some(*rpe),
                    changed: false,
                },
                automatic: *automatic,
            },
        }
    }
}

fn to_routine_parts(parts: &[Form]) -> Vec<data::RoutinePart> {
    parts
        .iter()
        .map(|p| match p {
            Form::Section { rounds, parts } => data::RoutinePart::RoutineSection {
                rounds: rounds.parsed.unwrap(),
                parts: to_routine_parts(parts),
            },
            Form::Activity {
                exercise_id,
                reps,
                time,
                weight,
                rpe,
                automatic,
            } => data::RoutinePart::RoutineActivity {
                exercise_id: *exercise_id,
                reps: reps.parsed.unwrap_or(0),
                time: time.parsed.unwrap_or(0),
                weight: weight.parsed.unwrap_or(0.0),
                rpe: rpe.parsed.unwrap_or(0.0),
                automatic: *automatic,
            },
        })
        .collect()
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    EditRoutine,
    SaveRoutine,

    ShowSelectExerciseDialog(Vec<usize>),
    ShowDeleteWorkoutDialog(u32),
    CloseDialog,

    AddSection(Vec<usize>),
    AddActivity(Vec<usize>, Option<u32>),
    RemovePart(Vec<usize>),
    MovePartDown(Vec<usize>),
    MovePartUp(Vec<usize>),
    RoundsChanged(Vec<usize>, String),
    ExerciseChanged(Vec<usize>, u32),
    RepsChanged(Vec<usize>, String),
    TimeChanged(Vec<usize>, String),
    WeightChanged(Vec<usize>, String),
    RPEChanged(Vec<usize>, String),
    AutomaticChanged(Vec<usize>),

    SearchTermChanged(String),

    CreateExercise,
    DeleteWorkout(u32),
    DataEvent(data::Event),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::EditRoutine => {
            model.editing = true;
            Url::go_and_push(
                &crate::Urls::new(&data_model.base_url)
                    .routine()
                    .add_hash_path_part(model.routine_id.to_string())
                    .add_hash_path_part("edit"),
            );
        }
        Msg::SaveRoutine => {
            model.loading = true;
            orders.notify(data::Msg::ModifyRoutine(
                model.routine_id,
                None,
                Some(to_routine_parts(&model.sections)),
            ));
        }

        Msg::ShowSelectExerciseDialog(part_id) => {
            model.dialog = Dialog::SelectExercise(part_id, String::new());
        }
        Msg::ShowDeleteWorkoutDialog(position) => {
            model.dialog = Dialog::DeleteWorkout(position);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
            model.loading = false;
            Url::go_and_replace(
                &crate::Urls::new(&data_model.base_url)
                    .routine()
                    .add_hash_path_part(model.routine_id.to_string()),
            );
        }

        Msg::AddSection(id) => {
            let new_section = Form::Section {
                rounds: InputField {
                    input: String::new(),
                    valid: true,
                    parsed: Some(1),
                    changed: false,
                },
                parts: vec![],
            };
            if id.is_empty() {
                model.sections.push(new_section);
            } else if let Some(Form::Section { parts, .. }) = get_part(&mut model.sections, &id) {
                parts.push(new_section);
            }
        }
        Msg::AddActivity(id, exercise_id) => {
            let new_activity = Form::Activity {
                exercise_id,
                reps: InputField {
                    input: String::new(),
                    valid: true,
                    parsed: Some(0),
                    changed: false,
                },
                time: if exercise_id.is_none() {
                    InputField {
                        input: String::from("60"),
                        valid: true,
                        parsed: Some(60),
                        changed: false,
                    }
                } else {
                    InputField {
                        input: String::new(),
                        valid: true,
                        parsed: Some(0),
                        changed: false,
                    }
                },
                weight: InputField {
                    input: String::new(),
                    valid: true,
                    parsed: Some(0.0),
                    changed: false,
                },
                rpe: InputField {
                    input: String::new(),
                    valid: true,
                    parsed: Some(0.0),
                    changed: false,
                },
                automatic: exercise_id.is_none(),
            };
            if let Some(Form::Section { parts, .. }) = get_part(&mut model.sections, &id) {
                parts.push(new_activity);
            }
        }
        Msg::RemovePart(id) => {
            if id.len() == 1 {
                model.sections.remove(id[0]);
            } else if let Some(Form::Section { parts, .. }) =
                get_part(&mut model.sections, &id[1..])
            {
                parts.remove(id[0]);
            }
        }
        Msg::MovePartDown(id) => {
            if id.len() == 1 {
                if id[0] == model.sections.len() - 1 {
                    model.sections.rotate_right(1);
                } else {
                    model.sections.swap(id[0], id[0] + 1);
                }
            } else if let Some(Form::Section { parts, .. }) =
                get_part(&mut model.sections, &id[1..])
            {
                if id[0] == parts.len() - 1 {
                    parts.rotate_right(1);
                } else {
                    parts.swap(id[0], id[0] + 1);
                }
            }
        }
        Msg::MovePartUp(id) => {
            if id.len() == 1 {
                if id[0] == 0 {
                    model.sections.rotate_left(1)
                } else {
                    model.sections.swap(id[0], id[0] - 1);
                }
            } else if let Some(Form::Section { parts, .. }) =
                get_part(&mut model.sections, &id[1..])
            {
                if id[0] == 0 {
                    parts.rotate_left(1);
                } else {
                    parts.swap(id[0], id[0] - 1);
                }
            }
        }
        Msg::RoundsChanged(id, input) => {
            if let Some(Form::Section { rounds, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *rounds = InputField {
                        input,
                        valid: true,
                        parsed: Some(1),
                        changed: true,
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_rounds) => {
                            *rounds = InputField {
                                input,
                                valid: true,
                                parsed: if parsed_rounds > 0 {
                                    Some(parsed_rounds)
                                } else {
                                    None
                                },
                                changed: true,
                            }
                        }
                        Err(_) => {
                            *rounds = InputField {
                                input,
                                valid: false,
                                parsed: None,
                                changed: true,
                            }
                        }
                    }
                }
            }
        }
        Msg::ExerciseChanged(id, input) => {
            if let Some(Form::Activity { exercise_id, .. }) = get_part(&mut model.sections, &id) {
                *exercise_id = Some(input);
            }
        }
        Msg::RepsChanged(id, input) => {
            if let Some(Form::Activity { reps, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *reps = InputField {
                        input,
                        valid: true,
                        parsed: Some(0),
                        changed: true,
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_reps) => {
                            let valid = common::valid_reps(parsed_reps);
                            *reps = InputField {
                                input,
                                valid,
                                parsed: if valid { Some(parsed_reps) } else { None },
                                changed: true,
                            }
                        }
                        Err(_) => {
                            *reps = InputField {
                                input,
                                valid: false,
                                parsed: None,
                                changed: true,
                            }
                        }
                    }
                }
            }
        }
        Msg::TimeChanged(id, input) => {
            if let Some(Form::Activity { time, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *time = InputField {
                        input,
                        valid: true,
                        parsed: Some(0),
                        changed: true,
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_time) => {
                            let valid = common::valid_time(parsed_time);
                            *time = InputField {
                                input,
                                valid,
                                parsed: if valid { Some(parsed_time) } else { None },
                                changed: true,
                            }
                        }
                        Err(_) => {
                            *time = InputField {
                                input,
                                valid: false,
                                parsed: None,
                                changed: true,
                            }
                        }
                    }
                }
            }
        }
        Msg::WeightChanged(id, input) => {
            if let Some(Form::Activity { weight, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *weight = InputField {
                        input,
                        valid: true,
                        parsed: Some(0.0),
                        changed: true,
                    };
                } else {
                    match input.parse::<f32>() {
                        Ok(parsed_weight) => {
                            let valid = common::valid_weight(parsed_weight);
                            *weight = InputField {
                                input,
                                valid,
                                parsed: if valid { Some(parsed_weight) } else { None },
                                changed: true,
                            }
                        }
                        Err(_) => {
                            *weight = InputField {
                                input,
                                valid: false,
                                parsed: None,
                                changed: true,
                            }
                        }
                    }
                }
            }
        }
        Msg::RPEChanged(id, input) => {
            if let Some(Form::Activity { rpe, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *rpe = InputField {
                        input,
                        valid: true,
                        parsed: Some(0.0),
                        changed: true,
                    };
                } else {
                    match input.parse::<f32>() {
                        Ok(parsed_rpe) => {
                            let valid = common::valid_rpe(parsed_rpe);
                            *rpe = InputField {
                                input,
                                valid,
                                parsed: if valid { Some(parsed_rpe) } else { None },
                                changed: true,
                            }
                        }
                        Err(_) => {
                            *rpe = InputField {
                                input,
                                valid: false,
                                parsed: None,
                                changed: true,
                            }
                        }
                    }
                }
            }
        }
        Msg::AutomaticChanged(id) => {
            if let Some(Form::Activity { automatic, .. }) = get_part(&mut model.sections, &id) {
                *automatic = not(*automatic);
            }
        }

        Msg::SearchTermChanged(search_term) => {
            if let Dialog::SelectExercise(_, dialog_search_term) = &mut model.dialog {
                *dialog_search_term = search_term;
            }
        }

        Msg::CreateExercise => {
            model.loading = true;
            if let Dialog::SelectExercise(_, search_term) = &model.dialog {
                orders.notify(data::Msg::CreateExercise(search_term.trim().to_string()));
            };
        }
        Msg::DeleteWorkout(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteWorkout(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    update_model(model, data_model);
                }
                data::Event::RoutineCreatedOk
                | data::Event::RoutineModifiedOk
                | data::Event::RoutineDeletedOk => {
                    model.editing = false;
                    model.mark_as_unchanged();
                    Url::go_and_push(
                        &crate::Urls::new(&data_model.base_url)
                            .routine()
                            .add_hash_path_part(model.routine_id.to_string()),
                    );
                }
                data::Event::WorkoutDeletedOk => {
                    orders.skip().send_msg(Msg::CloseDialog);
                }
                _ => {}
            };
        }

        Msg::ChangeInterval(first, last) => {
            model.interval.first = first;
            model.interval.last = last;
        }
    }
}

fn update_model(model: &mut Model, data_model: &data::Model) {
    model.interval = common::init_interval(
        &data_model
            .workouts
            .values()
            .filter(|w| w.routine_id == Some(model.routine_id))
            .map(|w| w.date)
            .collect::<Vec<NaiveDate>>(),
        true,
    );

    let routine = &data_model.routines.get(&model.routine_id);

    if let Some(routine) = routine {
        model.sections = routine.sections.iter().map(|p| p.into()).collect();
        let workouts = &data_model
            .workouts
            .values()
            .filter(|w| w.routine_id == Some(routine.id))
            .collect::<Vec<_>>();
        let all_exercises = &workouts
            .iter()
            .flat_map(|w| w.exercises())
            .collect::<BTreeSet<_>>();
        model.previous_exercises = all_exercises - &routine.exercises();
    } else {
        model.sections = vec![];
        model.previous_exercises = BTreeSet::new();
    };
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.routines.is_empty() && data_model.loading_routines {
        common::view_loading()
    } else if let Some(routine) = &data_model.routines.get(&model.routine_id) {
        let saving_disabled = not(model.sections.iter().all(|s| s.valid()));
        div![
            common::view_title(&span![&routine.name], 0),
            view_dialog(&model.dialog, &data_model.exercises, model.loading),
            view_routine(data_model, &model.sections, model.editing),
            if not(model.editing) {
                nodes![
                    view_previous_exercises(model, data_model),
                    view_workouts(model, data_model),
                    common::view_fab("edit", |_| Msg::EditRoutine),
                ]
            } else {
                nodes![button![
                    C!["button"],
                    C!["is-fab"],
                    C!["is-medium"],
                    C!["is-link"],
                    C![IF![model.loading => "is-loading"]],
                    attrs![
                        At::Disabled => saving_disabled.as_at_value(),
                    ],
                    ev(Ev::Click, |_| Msg::SaveRoutine),
                    span![C!["icon"], i![C!["fas fa-save"]]]
                ]]
            },
        ]
    } else {
        common::view_error_not_found("Routine")
    }
}

fn view_dialog(
    dialog: &Dialog,
    exercises: &BTreeMap<u32, data::Exercise>,
    loading: bool,
) -> Node<Msg> {
    match dialog {
        Dialog::SelectExercise(part_id, search_term) => {
            let mut exercises = exercises
                .values()
                .filter(|e| {
                    e.name
                        .to_lowercase()
                        .contains(search_term.to_lowercase().trim())
                })
                .collect::<Vec<_>>();
            exercises.sort_by(|a, b| a.name.cmp(&b.name));

            common::view_dialog(
                "primary",
                "Select exercise",
                nodes![
                    div![
                        C!["field"],
                        C!["is-grouped"],
                        common::view_search_box(search_term, Msg::SearchTermChanged),
                        {
                            let disabled = loading
                                || search_term.is_empty()
                                || exercises.iter().any(|e| e.name == *search_term.trim());
                            div![
                                C!["control"],
                                button![
                                    C!["button"],
                                    C!["is-link"],
                                    C![IF![loading => "is-loading"]],
                                    attrs! {
                                        At::Disabled => disabled.as_at_value()
                                    },
                                    ev(Ev::Click, |_| Msg::CreateExercise),
                                    span![C!["icon"], i![C!["fas fa-plus"]]]
                                ]
                            ]
                        }
                    ],
                    div![
                        C!["table-container"],
                        C!["mt-4"],
                        table![
                            C!["table"],
                            C!["is-fullwidth"],
                            C!["is-hoverable"],
                            tbody![&exercises
                                .iter()
                                .map(|e| {
                                    tr![td![
                                        C!["has-text-link"],
                                        ev(Ev::Click, {
                                            let part_id = part_id.clone();
                                            let exercise_id = e.id;
                                            move |_| Msg::ExerciseChanged(part_id, exercise_id)
                                        }),
                                        ev(Ev::Click, |_| Msg::CloseDialog),
                                        e.name.to_string(),
                                    ]]
                                })
                                .collect::<Vec<_>>()],
                        ]
                    ]
                ],
                &ev(Ev::Click, |_| Msg::CloseDialog),
            )
        }
        Dialog::DeleteWorkout(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            common::view_delete_confirmation_dialog(
                "workout",
                &ev(Ev::Click, move |_| Msg::DeleteWorkout(id)),
                &ev(Ev::Click, |_| Msg::CloseDialog),
                loading,
            )
        }
        Dialog::Hidden => {
            empty![]
        }
    }
}

fn view_routine(data_model: &data::Model, routine_sections: &[Form], editing: bool) -> Node<Msg> {
    div![
        C!["container"],
        C!["m-2"],
        &routine_sections
            .iter()
            .enumerate()
            .map(|(i, s)| { view_routine_part(data_model, s, vec![i], editing) })
            .collect::<Vec<_>>(),
        IF![editing => view_add_section_button(vec![])]
    ]
}

fn view_routine_part(
    data_model: &data::Model,
    part: &Form,
    id: Vec<usize>,
    editing: bool,
) -> Node<Msg> {
    match part {
        Form::Section { rounds, parts } => {
            div![
                C!["message"],
                IF![editing || id.first() != Some(&0) => C!["mt-3"]],
                C!["mb-0"],
                C!["is-grey"],
                C!["has-background-white-bis"],
                div![
                    C!["message-body"],
                    C!["p-3"],
                    if editing {
                        div![
                            C!["is-flex"],
                            C!["is-justify-content-space-between"],
                            div![
                                C!["field"],
                                C!["mb-0"],
                                div![
                                    C!["control"],
                                    C!["has-icons-left"],
                                    input_ev(Ev::Input, {
                                        let id = id.clone();
                                        move |v| Msg::RoundsChanged(id, v)
                                    }),
                                    span![
                                        C!["icon"],
                                        C!["is-small"],
                                        C!["is-left"],
                                        i![C!["fas fa-repeat"]]
                                    ],
                                    input![
                                        C!["input"],
                                        C!["has-text-right"],
                                        C![IF![not(rounds.valid) => "is-danger"]],
                                        C![IF![rounds.changed => "is-info"]],
                                        attrs! {
                                            At::Type => "number",
                                            At::Min => 1,
                                            At::Max => 999,
                                            At::Step => 1,
                                            At::Size => 2,
                                            At::Value => rounds.input,
                                            At::Placeholder => 1,
                                        }
                                    ]
                                ]
                            ],
                            view_position_buttons(id.clone())
                        ]
                    } else if let Some(rounds) = rounds.parsed {
                        if rounds > 1 {
                            span![
                                C!["icon-text"],
                                C!["mb-3"],
                                span![C!["icon"], i![C!["fas fa-repeat"]],],
                                span![rounds]
                            ]
                        } else {
                            empty![]
                        }
                    } else {
                        empty![]
                    },
                    parts
                        .iter()
                        .enumerate()
                        .map(|(i, p)| view_routine_part(
                            data_model,
                            p,
                            [&[i], &id[..]].concat(),
                            editing
                        ))
                        .collect::<Vec<_>>(),
                    IF![editing => view_add_part_buttons(data_model,id)]
                ],
            ]
        }
        Form::Activity {
            exercise_id,
            reps,
            time,
            weight,
            rpe,
            automatic,
        } => {
            div![
                C!["message"],
                IF![editing || id.first() != Some(&0) => C!["mt-3"]],
                C!["mb-0"],
                if exercise_id.is_some() {
                    C!["is-info"]
                } else {
                    C!["is-success"]
                },
                C!["has-background-white"],
                div![
                    C!["message-body"],
                    C!["p-3"],
                    if editing {
                        div![
                            C!["is-flex"],
                            C!["is-justify-content-space-between"],
                            if let Some(exercise_id) = exercise_id {
                                div![
                                    C!["field"],
                                    button![
                                        C!["input"],
                                        style! {St::Height => "auto"},
                                        input_ev(Ev::Click, {
                                            let id = id.clone();
                                            move |_| Msg::ShowSelectExerciseDialog(id)
                                        }),
                                        if let Some(exercise) =
                                            data_model.exercises.get(exercise_id)
                                        {
                                            exercise.name.clone()
                                        } else {
                                            format!("Exercise#{exercise_id}")
                                        }
                                    ]
                                ]
                            } else {
                                div![C!["field"], C!["has-text-weight-bold"], plain!["Rest"]]
                            },
                            view_position_buttons(id.clone())
                        ]
                    } else {
                        div![
                            C!["has-text-weight-bold"],
                            if let Some(exercise_id) = exercise_id {
                                if let Some(exercise) = data_model.exercises.get(exercise_id) {
                                    a![
                                        attrs! {
                                            At::Href => {
                                                crate::Urls::new(&data_model.base_url)
                                                    .exercise()
                                                    .add_hash_path_part(exercise_id.to_string())
                                            }
                                        },
                                        &exercise.name,
                                    ]
                                } else {
                                    plain![format!("Exercise#{exercise_id}")]
                                }
                            } else {
                                plain!["Rest"]
                            }
                        ]
                    },
                    if editing {
                        div![
                            C!["is-flex"],
                            C!["is-flex-wrap-wrap"],
                            C!["is-flex-gap-row-gap-2"],
                            C!["is-justify-content-flex-start"],
                            IF![
                                exercise_id.is_some() =>
                                div![
                                    C!["field"],
                                    C!["mb-0"],
                                    C!["mr-2"],
                                    div![
                                        C!["control"],
                                        C!["has-icons-left"],
                                        C!["has-icons-right"],
                                        input_ev(Ev::Input, {
                                            let id = id.clone();
                                            move |v| Msg::RepsChanged(id, v)
                                        }),
                                        span![
                                            C!["icon"],
                                            C!["is-small"],
                                            C!["is-left"],
                                            i![C!["fas fa-rotate-left"]]
                                        ],
                                        input![
                                            C!["input"],
                                            C!["has-text-right"],
                                            C![IF![not(reps.valid) => "is-danger"]],
                                            C![IF![reps.changed => "is-info"]],
                                            attrs! {
                                                At::Type => "number",
                                                At::Min => 0,
                                                At::Max => 999,
                                                At::Step => 1,
                                                At::Size => 2,
                                                At::Value => reps.input,
                                            }
                                        ],
                                        span![C!["icon"], C!["is-small"], C!["is-right"], "✕"],
                                    ]
                                ]
                            ],
                            div![
                                C!["field"],
                                C!["mb-0"],
                                C!["mr-2"],
                                div![
                                    C!["control"],
                                    C!["has-icons-left"],
                                    C!["has-icons-right"],
                                    input_ev(Ev::Input, {
                                        let id = id.clone();
                                        move |v| Msg::TimeChanged(id, v)
                                    }),
                                    span![
                                        C!["icon"],
                                        C!["is-small"],
                                        C!["is-left"],
                                        i![C!["fas fa-clock-rotate-left"]]
                                    ],
                                    input![
                                        C!["input"],
                                        C!["has-text-right"],
                                        C![IF![not(time.valid) => "is-danger"]],
                                        C![IF![time.changed => "is-info"]],
                                        attrs! {
                                            At::Type => "number",
                                            At::Min => 1,
                                            At::Max => 999,
                                            At::Step => 1,
                                            At::Size => 2,
                                            At::Value => time.input,
                                        }
                                    ],
                                    span![C!["icon"], C!["is-small"], C!["is-right"], "s"],
                                ]
                            ],
                            IF![
                                exercise_id.is_some() =>
                                div![
                                    C!["field"],
                                    C!["mb-0"],
                                    C!["mr-2"],
                                    div![
                                        C!["control"],
                                        C!["has-icons-left"],
                                        C!["has-icons-right"],
                                        input_ev(Ev::Input, {
                                            let id = id.clone();
                                            move |v| Msg::WeightChanged(id, v)
                                        }),
                                        span![
                                            C!["icon"],
                                            C!["is-small"],
                                            C!["is-left"],
                                            i![C!["fas fa-weight-hanging"]]
                                        ],
                                        input![
                                            C!["input"],
                                            C!["has-text-right"],
                                            C![IF![not(weight.valid) => "is-danger"]],
                                            C![IF![weight.changed => "is-info"]],
                                            attrs! {
                                                At::from("inputmode") => "numeric",
                                                At::Size => 3,
                                                At::Value => weight.input,
                                            }
                                        ],
                                        span![C!["icon"], C!["is-small"], C!["is-right"], "kg"],
                                    ]
                                ]
                            ],
                            IF![
                                exercise_id.is_some() =>
                                div![
                                    C!["field"],
                                    C!["mb-0"],
                                    C!["mr-2"],
                                    div![
                                        C!["control"],
                                        C!["has-icons-left"],
                                        C!["has-icons-right"],
                                        input_ev(Ev::Input, {
                                            let id = id.clone();
                                            move |v| Msg::RPEChanged(id, v)
                                        }),
                                        span![C!["icon"], C!["is-small"], C!["is-left"], "@"],
                                        input![
                                            C!["input"],
                                            C!["has-text-right"],
                                            C![IF![not(rpe.valid) => "is-danger"]],
                                            C![IF![rpe.changed => "is-info"]],
                                            attrs! {
                                                At::from("inputmode") => "numeric",
                                                At::Size => 3,
                                                At::Value => rpe.input,
                                            }
                                        ],
                                    ]
                                ]
                            ],
                            button![
                                C!["button"],
                                ev(Ev::Click, {
                                    let id = id;
                                    move |_| Msg::AutomaticChanged(id)
                                }),
                                span![
                                    C!["icon"],
                                    if *automatic {
                                        C!["has-text-dark"]
                                    } else {
                                        C!["has-text-grey-lighter"]
                                    },
                                    common::automatic_icon()
                                ]
                            ]
                        ]
                    } else {
                        div![
                            IF![
                                if let Some(reps) = reps.parsed { reps > 0 } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-rotate-left"]]],
                                        span![&reps.input]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(time) = time.parsed { time > 0 } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-clock-rotate-left"]]],
                                        span![&time.input, " s"]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(weight) = weight.parsed { weight > 0.0 } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-weight-hanging"]]],
                                        span![&weight.input, " kg"]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(rpe) = rpe.parsed { rpe > 0.0 } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], "@"],
                                        span![&rpe.input]
                                    ]
                                }
                            ],
                            IF![
                                *automatic => span![
                                    C!["icon"],
                                    common::automatic_icon(),
                                ]
                            ],
                        ]
                    }
                ]
            ]
        }
    }
}

fn view_previous_exercises(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if not(model.previous_exercises.is_empty()) {
        div![
            C!["container"],
            C!["has-text-centered"],
            C!["mt-6"],
            h1![C!["title"], C!["is-5"], "Previously used exercises"],
            &model
                .previous_exercises
                .iter()
                .map(|exercise_id| {
                    p![
                        C!["m-2"],
                        a![
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).exercise().add_hash_path_part(exercise_id.to_string()),
                            },
                            &data_model.exercises.get(exercise_id).unwrap().name
                        ]
                    ]
                })
            .collect::<Vec<_>>(),
        ]
    } else {
        empty![]
    }
}

fn view_workouts(model: &Model, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["container"],
        C!["has-text-centered"],
        C!["mt-6"],
        h1![C!["title"], C!["is-5"], "Workouts"],
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        workouts::view_charts(
            data_model
                .workouts
                .values()
                .filter(|w| {
                    w.routine_id == Some(model.routine_id)
                        && w.date >= model.interval.first
                        && w.date <= model.interval.last
                })
                .collect::<Vec<_>>()
                .as_slice(),
            &model.interval,
        ),
        workouts::view_table(
            &data_model
                .workouts
                .values()
                .filter(|w| w.routine_id == Some(model.routine_id))
                .collect::<Vec<&data::Workout>>(),
            &data_model.routines,
            &model.interval,
            &data_model.base_url,
            Msg::ShowDeleteWorkoutDialog
        ),
    ]
}

fn view_position_buttons(id: Vec<usize>) -> Node<Msg> {
    div![
        style! {St::WhiteSpace => "nowrap" },
        button![
            C!["button"],
            C!["is-small"],
            C!["ml-2"],
            ev(Ev::Click, {
                let id = id.clone();
                move |_| Msg::MovePartDown(id)
            }),
            span![C!["icon"], i![C!["fas fa-arrow-down"]],]
        ],
        button![
            C!["button"],
            C!["is-small"],
            C!["ml-2"],
            ev(Ev::Click, {
                let id = id.clone();
                move |_| Msg::MovePartUp(id)
            }),
            span![C!["icon"], i![C!["fas fa-arrow-up"]],]
        ],
        button![
            C!["button"],
            C!["is-small"],
            C!["is-danger"],
            C!["ml-2"],
            ev(Ev::Click, {
                let id = id;
                move |_| Msg::RemovePart(id)
            }),
            span![C!["icon"], i![C!["fas fa-remove"]],]
        ],
    ]
}

fn view_add_part_buttons(data_model: &data::Model, id: Vec<usize>) -> Node<Msg> {
    let mut exercises = data_model.exercises.values().collect::<Vec<_>>();
    exercises.sort_by(|a, b| a.name.cmp(&b.name));

    div![
        button![
            C!["button"],
            C!["is-link"],
            C!["mt-2"],
            C!["mr-2"],
            ev(Ev::Click, {
                let id = id.clone();
                let exercise_id = exercises.first().map(|e| e.id).unwrap_or(0);
                move |_| Msg::AddActivity(id, Some(exercise_id))
            }),
            span![
                i![C!["fas fa-person-running"]],
                i![C!["ml-1"], C!["is-small fas fa-plus-circle"]],
            ]
        ],
        button![
            C!["button"],
            C!["is-success"],
            C!["mt-2"],
            C!["mr-2"],
            ev(Ev::Click, {
                let id = id.clone();
                move |_| Msg::AddActivity(id, None)
            }),
            span![
                i![C!["fas fa-person"]],
                i![C!["ml-1"], C!["fas fa-plus-circle"]],
            ]
        ],
        view_add_section_button(id),
    ]
}

fn view_add_section_button(id: Vec<usize>) -> Node<Msg> {
    button![
        C!["button"],
        C!["has-text-light"],
        C!["has-background-grey"],
        C!["mt-2"],
        C!["mr-2"],
        ev(Ev::Click, {
            let id = id;
            move |_| Msg::AddSection(id)
        }),
        span![
            i![C!["fas fa-repeat"]],
            i![C!["ml-1"], C!["fas fa-plus-circle"]],
        ]
    ]
}

fn get_part<'a>(sections: &'a mut Vec<Form>, id: &[usize]) -> Option<&'a mut Form> {
    if let Some(i) = id.last() {
        if i < &sections.len() {
            let p = &mut sections[*i];
            if id.len() == 1 {
                return Some(p);
            }
            if let Form::Section { rounds: _, parts } = p {
                return get_part(parts, &id[..id.len() - 1]);
            }
        }
    };
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_part_in_sections() {
        let mut sections = vec![
            Form::Section {
                rounds: form_value(1),
                parts: vec![Form::Activity {
                    exercise_id: None,
                    reps: form_value(1),
                    time: form_value(2),
                    weight: form_value(4.0),
                    rpe: form_value(5.0),
                    automatic: false,
                }],
            },
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: None,
                    reps: form_value(2),
                    time: form_value(3),
                    weight: form_value(5.0),
                    rpe: form_value(6.0),
                    automatic: false,
                }],
            },
        ];
        assert_eq!(
            *get_part(&mut sections, &[0]).unwrap(),
            Form::Section {
                rounds: form_value(1),
                parts: vec![Form::Activity {
                    exercise_id: None,
                    reps: form_value(1),
                    time: form_value(2),
                    weight: form_value(4.0),
                    rpe: form_value(5.0),
                    automatic: false,
                }],
            }
        );
        assert_eq!(
            *get_part(&mut sections, &[1]).unwrap(),
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: None,
                    reps: form_value(2),
                    time: form_value(3),
                    weight: form_value(5.0),
                    rpe: form_value(6.0),
                    automatic: false,
                }],
            }
        );
        assert!(get_part(&mut sections, &[2]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 0]).unwrap(),
            Form::Activity {
                exercise_id: None,
                reps: form_value(1),
                time: form_value(2),
                weight: form_value(4.0),
                rpe: form_value(5.0),
                automatic: false,
            },
        );
        assert!(get_part(&mut sections, &[1, 0]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 1]).unwrap(),
            Form::Activity {
                exercise_id: None,
                reps: form_value(2),
                time: form_value(3),
                weight: form_value(5.0),
                rpe: form_value(6.0),
                automatic: false,
            },
        );
        assert!(get_part(&mut sections, &[1, 1]).is_none());
    }

    #[test]
    fn get_part_in_nested_sections() {
        let mut sections = vec![Form::Section {
            rounds: form_value(1),
            parts: vec![
                Form::Activity {
                    exercise_id: None,
                    reps: form_value(1),
                    time: form_value(2),
                    weight: form_value(4.0),
                    rpe: form_value(5.0),
                    automatic: false,
                },
                Form::Section {
                    rounds: form_value(2),
                    parts: vec![Form::Activity {
                        exercise_id: None,
                        reps: form_value(2),
                        time: form_value(3),
                        weight: form_value(5.0),
                        rpe: form_value(6.0),
                        automatic: false,
                    }],
                },
            ],
        }];
        assert_eq!(
            *get_part(&mut sections, &[0]).unwrap(),
            Form::Section {
                rounds: form_value(1),
                parts: vec![
                    Form::Activity {
                        exercise_id: None,
                        reps: form_value(1),
                        time: form_value(2),
                        weight: form_value(4.0),
                        rpe: form_value(5.0),
                        automatic: false,
                    },
                    Form::Section {
                        rounds: form_value(2),
                        parts: vec![Form::Activity {
                            exercise_id: None,
                            reps: form_value(2),
                            time: form_value(3),
                            weight: form_value(5.0),
                            rpe: form_value(6.0),
                            automatic: false,
                        }],
                    },
                ],
            }
        );
        assert!(get_part(&mut sections, &[1]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 0]).unwrap(),
            Form::Activity {
                exercise_id: None,
                reps: form_value(1),
                time: form_value(2),
                weight: form_value(4.0),
                rpe: form_value(5.0),
                automatic: false,
            },
        );
        assert_eq!(
            *get_part(&mut sections, &[1, 0]).unwrap(),
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: None,
                    reps: form_value(2),
                    time: form_value(3),
                    weight: form_value(5.0),
                    rpe: form_value(6.0),
                    automatic: false,
                }],
            },
        );
        assert!(get_part(&mut sections, &[2, 0]).is_none());
        assert!(get_part(&mut sections, &[0, 0, 0]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 1, 0]).unwrap(),
            Form::Activity {
                exercise_id: None,
                reps: form_value(2),
                time: form_value(3),
                weight: form_value(5.0),
                rpe: form_value(6.0),
                automatic: false,
            },
        );
        assert!(get_part(&mut sections, &[1, 1, 0]).is_none());
        assert!(get_part(&mut sections, &[0, 0, 1, 0]).is_none());
    }

    fn form_value<T: std::fmt::Display>(number: T) -> InputField<T> {
        InputField {
            input: number.to_string(),
            valid: true,
            parsed: Some(number),
            changed: false,
        }
    }
}
