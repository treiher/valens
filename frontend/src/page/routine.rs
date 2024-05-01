use std::collections::BTreeMap;
use std::collections::BTreeSet;

use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;
use crate::page::training;

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
        name: common::InputField::default(),
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
    name: common::InputField<String>,
    sections: Vec<Form>,
    previous_exercises: BTreeSet<u32>,
    dialog: Dialog,
    editing: bool,
    loading: bool,
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.name.changed() || self.sections.iter().any(Form::changed)
    }

    pub fn mark_as_unchanged(&mut self) {
        self.name.input = self.name.parsed.clone().unwrap();
        self.name.orig = self.name.parsed.clone().unwrap();
        for s in &mut self.sections {
            s.mark_as_unchanged();
        }
    }

    fn saving_disabled(&self) -> bool {
        self.loading || not(self.name.valid()) || not(self.sections.iter().all(Form::valid))
    }
}

enum Dialog {
    Hidden,
    SelectExercise(Vec<usize>, String),
    DeleteTrainingSession(u32),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum Form {
    Section {
        rounds: common::InputField<u32>,
        parts: Vec<Form>,
    },
    Activity {
        exercise_id: Option<u32>,
        reps: common::InputField<u32>,
        time: common::InputField<u32>,
        weight: common::InputField<f32>,
        rpe: common::InputField<f32>,
        automatic: bool,
    },
}

impl Form {
    fn changed(&self) -> bool {
        match self {
            Form::Section { rounds, parts } => rounds.changed() || parts.iter().any(Form::changed),
            Form::Activity {
                reps,
                time,
                weight,
                rpe,
                ..
            } => reps.changed() || time.changed() || weight.changed() || rpe.changed(),
        }
    }

    fn mark_as_unchanged(&mut self) {
        match self {
            Form::Section { rounds, parts } => {
                rounds.orig = rounds.input.clone();
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
                reps.orig = reps.input.clone();
                time.orig = time.input.clone();
                weight.orig = weight.input.clone();
                rpe.orig = rpe.input.clone();
            }
        }
    }

    fn valid(&self) -> bool {
        match self {
            Form::Section { rounds, parts } => rounds.valid() && parts.iter().all(Form::valid),
            Form::Activity {
                reps,
                time,
                weight,
                rpe,
                ..
            } => reps.valid() && time.valid() && weight.valid() && rpe.valid(),
        }
    }
}

impl From<&data::RoutinePart> for Form {
    fn from(part: &data::RoutinePart) -> Self {
        match part {
            data::RoutinePart::RoutineSection { rounds, parts, .. } => {
                let rounds_str = if *rounds == 1 {
                    String::new()
                } else {
                    rounds.to_string()
                };
                Form::Section {
                    rounds: common::InputField {
                        input: rounds_str.clone(),
                        parsed: Some(*rounds),
                        orig: rounds_str,
                    },
                    parts: parts.iter().map(Into::into).collect(),
                }
            }
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
                reps: {
                    let reps_str = if *reps == 0 {
                        String::new()
                    } else {
                        reps.to_string()
                    };
                    common::InputField {
                        input: reps_str.clone(),
                        parsed: Some(*reps),
                        orig: reps_str,
                    }
                },
                time: {
                    let time_str = if *time == 0 {
                        String::new()
                    } else {
                        time.to_string()
                    };
                    common::InputField {
                        input: time_str.clone(),
                        parsed: Some(*time),
                        orig: time_str,
                    }
                },
                weight: {
                    let weight_str = if *weight == 0.0 {
                        String::new()
                    } else {
                        weight.to_string()
                    };
                    common::InputField {
                        input: weight_str.clone(),
                        parsed: Some(*weight),
                        orig: weight_str,
                    }
                },
                rpe: {
                    let rpe_str = if *rpe == 0.0 {
                        String::new()
                    } else {
                        rpe.to_string()
                    };
                    common::InputField {
                        input: rpe_str.clone(),
                        parsed: Some(*rpe),
                        orig: rpe_str,
                    }
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
    ShowDeleteTrainingSessionDialog(u32),
    CloseDialog,

    NameChanged(String),
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
    DeleteTrainingSession(u32),
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
                model.name.parsed.clone(),
                Some(to_routine_parts(&model.sections)),
            ));
        }

        Msg::ShowSelectExerciseDialog(part_id) => {
            model.dialog = Dialog::SelectExercise(part_id, String::new());
        }
        Msg::ShowDeleteTrainingSessionDialog(position) => {
            model.dialog = Dialog::DeleteTrainingSession(position);
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

        Msg::NameChanged(name) => {
            let trimmed_name = name.trim();
            if not(trimmed_name.is_empty())
                && (trimmed_name == model.name.orig
                    || data_model.routines.values().all(|e| e.name != trimmed_name))
            {
                model.name = common::InputField {
                    input: name.clone(),
                    parsed: Some(trimmed_name.to_string()),
                    orig: model.name.orig.clone(),
                };
            } else {
                model.name = common::InputField {
                    input: name,
                    parsed: None,
                    orig: model.name.orig.clone(),
                }
            }
        }
        Msg::AddSection(id) => {
            let new_section = Form::Section {
                rounds: common::InputField {
                    input: String::new(),
                    parsed: Some(1),
                    orig: String::new(),
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
                reps: common::InputField {
                    input: String::new(),
                    parsed: Some(0),
                    orig: String::new(),
                },
                time: if exercise_id.is_none() {
                    common::InputField {
                        input: String::from("60"),
                        parsed: Some(60),
                        orig: String::from("60"),
                    }
                } else {
                    common::InputField {
                        input: String::new(),
                        parsed: Some(0),
                        orig: String::new(),
                    }
                },
                weight: common::InputField {
                    input: String::new(),
                    parsed: Some(0.0),
                    orig: String::new(),
                },
                rpe: common::InputField {
                    input: String::new(),
                    parsed: Some(0.0),
                    orig: String::new(),
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
                    model.sections.rotate_left(1);
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
                    *rounds = common::InputField {
                        input,
                        parsed: Some(1),
                        orig: rounds.orig.clone(),
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_rounds) => {
                            *rounds = common::InputField {
                                input,
                                parsed: if parsed_rounds > 0 {
                                    Some(parsed_rounds)
                                } else {
                                    None
                                },
                                orig: rounds.orig.clone(),
                            }
                        }
                        Err(_) => {
                            *rounds = common::InputField {
                                input,
                                parsed: None,
                                orig: rounds.orig.clone(),
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
            orders.send_msg(Msg::CloseDialog);
        }
        Msg::RepsChanged(id, input) => {
            if let Some(Form::Activity { reps, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *reps = common::InputField {
                        input,
                        parsed: Some(0),
                        orig: reps.orig.clone(),
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_reps) => {
                            let valid = common::valid_reps(parsed_reps);
                            *reps = common::InputField {
                                input,
                                parsed: if valid { Some(parsed_reps) } else { None },
                                orig: reps.orig.clone(),
                            }
                        }
                        Err(_) => {
                            *reps = common::InputField {
                                input,
                                parsed: None,
                                orig: reps.orig.clone(),
                            }
                        }
                    }
                }
            }
        }
        Msg::TimeChanged(id, input) => {
            if let Some(Form::Activity { time, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *time = common::InputField {
                        input,
                        parsed: Some(0),
                        orig: time.orig.clone(),
                    };
                } else {
                    match input.parse::<u32>() {
                        Ok(parsed_time) => {
                            let valid = common::valid_time(parsed_time);
                            *time = common::InputField {
                                input,
                                parsed: if valid { Some(parsed_time) } else { None },
                                orig: time.orig.clone(),
                            }
                        }
                        Err(_) => {
                            *time = common::InputField {
                                input,
                                parsed: None,
                                orig: time.orig.clone(),
                            }
                        }
                    }
                }
            }
        }
        Msg::WeightChanged(id, input) => {
            if let Some(Form::Activity { weight, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *weight = common::InputField {
                        input,
                        parsed: Some(0.0),
                        orig: weight.orig.clone(),
                    };
                } else {
                    match input.parse::<f32>() {
                        Ok(parsed_weight) => {
                            let valid = common::valid_weight(parsed_weight);
                            *weight = common::InputField {
                                input,
                                parsed: if valid { Some(parsed_weight) } else { None },
                                orig: weight.orig.clone(),
                            }
                        }
                        Err(_) => {
                            *weight = common::InputField {
                                input,
                                parsed: None,
                                orig: weight.orig.clone(),
                            }
                        }
                    }
                }
            }
        }
        Msg::RPEChanged(id, input) => {
            if let Some(Form::Activity { rpe, .. }) = get_part(&mut model.sections, &id) {
                if input.is_empty() {
                    *rpe = common::InputField {
                        input,
                        parsed: Some(0.0),
                        orig: rpe.orig.clone(),
                    };
                } else {
                    match input.parse::<f32>() {
                        Ok(parsed_rpe) => {
                            let valid = common::valid_rpe(parsed_rpe);
                            *rpe = common::InputField {
                                input,
                                parsed: if valid { Some(parsed_rpe) } else { None },
                                orig: rpe.orig.clone(),
                            }
                        }
                        Err(_) => {
                            *rpe = common::InputField {
                                input,
                                parsed: None,
                                orig: rpe.orig.clone(),
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
        Msg::DeleteTrainingSession(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteTrainingSession(id));
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
                data::Event::TrainingSessionDeletedOk => {
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
            .training_sessions
            .values()
            .filter(|t| t.routine_id == Some(model.routine_id))
            .map(|t| t.date)
            .collect::<Vec<NaiveDate>>(),
        true,
    );

    let routine = &data_model.routines.get(&model.routine_id);

    if let Some(routine) = routine {
        model.name = common::InputField {
            input: routine.name.clone(),
            parsed: Some(routine.name.clone()),
            orig: routine.name.clone(),
        };
        model.sections = routine.sections.iter().map(Into::into).collect();
        let training_sessions = &data_model
            .training_sessions
            .values()
            .filter(|t| t.routine_id == Some(routine.id))
            .collect::<Vec<_>>();
        let all_exercises = &training_sessions
            .iter()
            .flat_map(|t| t.exercises())
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
        common::view_page_loading()
    } else if let Some(routine) = data_model.routines.get(&model.routine_id) {
        div![
            view_title(model),
            view_summary(routine),
            view_dialog(&model.dialog, &data_model.exercises, model.loading),
            view_routine(data_model, &model.sections, model.editing),
            if model.editing {
                nodes![button![
                    C!["button"],
                    C!["is-fab"],
                    C!["is-medium"],
                    C!["is-link"],
                    C![IF![model.loading => "is-loading"]],
                    attrs![
                        At::Disabled => model.saving_disabled().as_at_value(),
                    ],
                    ev(Ev::Click, |_| Msg::SaveRoutine),
                    span![C!["icon"], i![C!["fas fa-save"]]]
                ]]
            } else {
                nodes![
                    view_previous_exercises(model, data_model),
                    view_training_sessions(model, data_model),
                    common::view_fab("edit", |_| Msg::EditRoutine),
                ]
            },
        ]
    } else {
        common::view_error_not_found("Routine")
    }
}

fn view_title(model: &Model) -> Node<Msg> {
    div![
        C!["mx-2"],
        if model.editing {
            let saving_disabled = model.saving_disabled();
            div![
                C!["field"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::NameChanged),
                    keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                        IF!(
                            not(saving_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                Msg::SaveRoutine
                            }
                        )
                    }),
                    input![
                        C!["input"],
                        C![IF![not(model.name.valid()) => "is-danger"]],
                        C![IF![model.name.changed() => "is-info"]],
                        attrs! {
                            At::Type => "text",
                            At::Value => model.name.input,
                        }
                    ]
                ],
            ]
        } else {
            common::view_title(&span![&model.name.input], 0)
        }
    ]
}

fn view_dialog(
    dialog: &Dialog,
    exercises: &BTreeMap<u32, data::Exercise>,
    loading: bool,
) -> Node<Msg> {
    match dialog {
        Dialog::SelectExercise(part_id, search_term) => {
            let part_id = part_id.clone();

            common::view_dialog(
                "primary",
                "Select exercise",
                common::view_exercises_with_search(
                    exercises,
                    search_term,
                    Msg::SearchTermChanged,
                    |_| Msg::CreateExercise,
                    loading,
                    |exercise_id| Msg::ExerciseChanged(part_id, exercise_id),
                ),
                &ev(Ev::Click, |_| Msg::CloseDialog),
            )
        }
        Dialog::DeleteTrainingSession(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            common::view_delete_confirmation_dialog(
                "training_session",
                &ev(Ev::Click, move |_| Msg::DeleteTrainingSession(id)),
                &ev(Ev::Click, |_| Msg::CloseDialog),
                loading,
            )
        }
        Dialog::Hidden => {
            empty![]
        }
    }
}

fn view_summary(routine: &data::Routine) -> Node<Msg> {
    div![
        C!["columns"],
        C!["is-gapless"],
        C!["is-mobile"],
        C!["mt-4"],
        [
            (
                "Duration",
                format!(
                    "~ <strong>{}</strong> min",
                    routine.duration().num_minutes(),
                )
            ),
            ("Sets", format!("<strong>{}</strong>", routine.num_sets()))
        ]
        .iter()
        .map(|(title, subtitle)| { div![C!["column"], common::view_box(title, subtitle)] }),
    ]
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
                C!["has-background-white-bis"],
                C!["p-3"],
                IF![editing || id.first() != Some(&0) => C!["mt-3"]],
                C!["mb-3"],
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
                                    C![IF![not(rounds.valid()) => "is-danger"]],
                                    C![IF![rounds.changed() => "is-info"]],
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
                    } else if let Some(exercise_id) = exercise_id {
                        div![
                            C!["has-text-weight-bold"],
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
                        ]
                    } else {
                        common::view_rest(time.parsed.unwrap_or_default(), *automatic)
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
                                            C![IF![not(reps.valid()) => "is-danger"]],
                                            C![IF![reps.changed() => "is-info"]],
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
                                        C![IF![not(time.valid()) => "is-danger"]],
                                        C![IF![time.changed() => "is-info"]],
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
                                            C![IF![not(weight.valid()) => "is-danger"]],
                                            C![IF![weight.changed() => "is-info"]],
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
                                            C![IF![not(rpe.valid()) => "is-danger"]],
                                            C![IF![rpe.changed() => "is-info"]],
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
                    } else if exercise_id.is_some() {
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
                    } else {
                        empty![]
                    }
                ]
            ]
        }
    }
}

fn view_previous_exercises(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if model.previous_exercises.is_empty() {
        empty![]
    } else {
        div![
            C!["container"],
            C!["has-text-centered"],
            C!["mt-6"],
            common::view_title(&span!["Previously used exercises"], 3),
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
    }
}

fn view_training_sessions(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let training_sessions = data_model
        .training_sessions
        .values()
        .filter(|t| {
            t.routine_id == Some(model.routine_id)
                && t.date >= model.interval.first
                && t.date <= model.interval.last
        })
        .collect::<Vec<_>>();
    let dates = training_sessions.iter().map(|t| t.date);
    let routine_interval = common::Interval {
        first: dates.clone().min().unwrap_or_default(),
        last: dates.max().unwrap_or_default(),
    };
    div![
        C!["container"],
        C!["has-text-centered"],
        C!["mt-6"],
        common::view_title(&span!["Training sessions"], 5),
        common::view_interval_buttons(&model.interval, &routine_interval, Msg::ChangeInterval),
        view_charts(&training_sessions, &model.interval),
        training::view_calendar(&training_sessions, &model.interval),
        training::view_table(
            &training_sessions,
            &data_model.routines,
            &data_model.base_url,
            Msg::ShowDeleteTrainingSessionDialog
        ),
    ]
}
pub fn view_charts<Ms>(
    training_sessions: &[&data::TrainingSession],
    interval: &common::Interval,
) -> Vec<Node<Ms>> {
    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut intensity: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();
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
        if let Some(avg_rpe) = training_session.avg_rpe() {
            intensity
                .entry(training_session.date)
                .and_modify(|e| e.push(avg_rpe))
                .or_insert(vec![avg_rpe]);
        }
    }
    nodes![
        common::view_chart(
            &[("Load", common::COLOR_LOAD)],
            common::plot_line_chart(
                &[(load.into_iter().collect::<Vec<_>>(), common::COLOR_LOAD)],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Set volume", common::COLOR_SET_VOLUME)],
            common::plot_line_chart(
                &[(
                    set_volume.into_iter().collect::<Vec<_>>(),
                    common::COLOR_SET_VOLUME,
                )],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Intensity (RPE)", common::COLOR_INTENSITY)],
            common::plot_line_chart(
                &[(
                    intensity
                        .into_iter()
                        .map(|(date, values)| {
                            #[allow(clippy::cast_precision_loss)]
                            (
                                date,
                                if values.is_empty() {
                                    0.
                                } else {
                                    values.iter().sum::<f32>() / values.len() as f32
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                    common::COLOR_INTENSITY,
                )],
                interval.first,
                interval.last,
                Some(5.),
                None,
            )
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
                let exercise_id = exercises.first().map_or(0, |e| e.id);
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

    fn form_value<T: std::fmt::Display + std::marker::Copy>(number: T) -> common::InputField<T> {
        common::InputField {
            input: number.to_string(),
            parsed: Some(number),
            orig: number.to_string(),
        }
    }
}
