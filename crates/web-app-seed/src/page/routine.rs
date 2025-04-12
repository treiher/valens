use std::collections::{BTreeMap, BTreeSet};

use chrono::prelude::*;
use seed::{prelude::*, *};
use valens_domain as domain;
use valens_web_app as web_app;

use crate::{common, component, data, page::training};

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
        .parse::<u128>()
        .unwrap_or_default()
        .into();
    let editing = url.next_hash_path_part() == Some("edit");

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Routine");

    let mut model = Model {
        interval: domain::init_interval(&[], domain::DefaultInterval::All),
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
    interval: domain::Interval,
    routine_id: domain::RoutineID,
    name: common::InputField<domain::Name>,
    sections: Vec<Form>,
    previous_exercises: BTreeSet<domain::ExerciseID>,
    dialog: Dialog,
    editing: bool,
    loading: bool,
}

impl Model {
    pub fn has_unsaved_changes(&self) -> bool {
        self.name.changed() || self.sections.iter().any(Form::changed)
    }

    pub fn mark_as_unchanged(&mut self) {
        self.name.input = self.name.parsed.clone().unwrap().to_string();
        self.name.orig = self.name.parsed.clone().unwrap().to_string();
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
    SelectExercise(Vec<usize>, Box<component::exercise_list::Model>),
    DeleteTrainingSession(domain::TrainingSessionID),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum Form {
    Section {
        rounds: common::InputField<u32>,
        parts: Vec<Form>,
    },
    Activity {
        exercise_id: domain::ExerciseID,
        reps: common::InputField<domain::Reps>,
        time: common::InputField<domain::Time>,
        weight: common::InputField<domain::Weight>,
        rpe: common::InputField<domain::RPE>,
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

impl From<&domain::RoutinePart> for Form {
    fn from(part: &domain::RoutinePart) -> Self {
        match part {
            domain::RoutinePart::RoutineSection { rounds, parts, .. } => {
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
            domain::RoutinePart::RoutineActivity {
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
                    let reps_str = if *reps == domain::Reps::default() {
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
                    let time_str = if *time == domain::Time::default() {
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
                    let weight_str = if *weight == domain::Weight::default() {
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
                    let rpe_str = if *rpe == domain::RPE::ZERO {
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

fn to_routine_parts(parts: &[Form]) -> Vec<domain::RoutinePart> {
    parts
        .iter()
        .map(|p| match p {
            Form::Section { rounds, parts } => domain::RoutinePart::RoutineSection {
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
            } => domain::RoutinePart::RoutineActivity {
                exercise_id: *exercise_id,
                reps: reps.parsed.unwrap_or_default(),
                time: time.parsed.unwrap_or_default(),
                weight: weight.parsed.unwrap_or_default(),
                rpe: rpe.parsed.unwrap_or_default(),
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
    ShowDeleteTrainingSessionDialog(domain::TrainingSessionID),
    CloseDialog,

    NameChanged(String),
    AddSection(Vec<usize>),
    AddActivity(Vec<usize>, domain::ExerciseID),
    RemovePart(Vec<usize>),
    MovePartDown(Vec<usize>),
    MovePartUp(Vec<usize>),
    RoundsChanged(Vec<usize>, String),
    ExerciseChanged(Vec<usize>, domain::ExerciseID),
    RepsChanged(Vec<usize>, String),
    TimeChanged(Vec<usize>, String),
    WeightChanged(Vec<usize>, String),
    RPEChanged(Vec<usize>, String),
    AutomaticChanged(Vec<usize>),

    ExerciseList(component::exercise_list::Msg),

    DeleteTrainingSession(domain::TrainingSessionID),
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
                    .add_hash_path_part(model.routine_id.as_u128().to_string())
                    .add_hash_path_part("edit"),
            );
        }
        Msg::SaveRoutine => {
            model.loading = true;
            orders.notify(data::Msg::ModifyRoutine(
                model.routine_id,
                model.name.parsed.clone(),
                None,
                Some(to_routine_parts(&model.sections)),
            ));
        }

        Msg::ShowSelectExerciseDialog(part_id) => {
            model.dialog = Dialog::SelectExercise(
                part_id,
                Box::new(component::exercise_list::Model::new(
                    true, false, false, false,
                )),
            );
        }
        Msg::ShowDeleteTrainingSessionDialog(id) => {
            model.dialog = Dialog::DeleteTrainingSession(id);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
            model.loading = false;
            Url::go_and_replace(
                &crate::Urls::new(&data_model.base_url)
                    .routine()
                    .add_hash_path_part(model.routine_id.as_u128().to_string()),
            );
        }

        Msg::NameChanged(name) => {
            let parsed = domain::Name::new(&name).ok().and_then(|name| {
                if name.as_ref() == &model.name.orig
                    || data_model.routines.values().all(|r| r.name != name)
                {
                    Some(name)
                } else {
                    None
                }
            });
            model.name = common::InputField {
                input: name,
                parsed,
                orig: model.name.orig.clone(),
            };
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
                    parsed: Some(domain::Reps::default()),
                    orig: String::new(),
                },
                time: if exercise_id.is_nil() {
                    common::InputField {
                        input: String::from("60"),
                        parsed: Some(domain::Time::new(60).unwrap()),
                        orig: String::from("60"),
                    }
                } else {
                    common::InputField {
                        input: String::new(),
                        parsed: Some(domain::Time::default()),
                        orig: String::new(),
                    }
                },
                weight: common::InputField {
                    input: String::new(),
                    parsed: Some(domain::Weight::default()),
                    orig: String::new(),
                },
                rpe: common::InputField {
                    input: String::new(),
                    parsed: Some(domain::RPE::ZERO),
                    orig: String::new(),
                },
                automatic: exercise_id.is_nil(),
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
                *exercise_id = input;
            }
            orders.send_msg(Msg::CloseDialog);
        }
        Msg::RepsChanged(id, input) => {
            if let Some(Form::Activity { reps, .. }) = get_part(&mut model.sections, &id) {
                let parsed = if input.is_empty() {
                    Some(domain::Reps::default())
                } else {
                    domain::Reps::try_from(input.as_ref()).ok()
                };
                *reps = common::InputField {
                    input,
                    parsed,
                    orig: reps.orig.clone(),
                };
            }
        }
        Msg::TimeChanged(id, input) => {
            if let Some(Form::Activity { time, .. }) = get_part(&mut model.sections, &id) {
                let parsed = if input.is_empty() {
                    Some(domain::Time::default())
                } else {
                    domain::Time::try_from(input.as_ref()).ok()
                };
                *time = common::InputField {
                    input,
                    parsed,
                    orig: time.orig.clone(),
                };
            }
        }
        Msg::WeightChanged(id, input) => {
            if let Some(Form::Activity { weight, .. }) = get_part(&mut model.sections, &id) {
                let parsed = if input.is_empty() {
                    Some(domain::Weight::default())
                } else {
                    domain::Weight::try_from(input.as_ref()).ok()
                };
                *weight = common::InputField {
                    input,
                    parsed,
                    orig: weight.orig.clone(),
                };
            }
        }
        Msg::RPEChanged(id, input) => {
            if let Some(Form::Activity { rpe, .. }) = get_part(&mut model.sections, &id) {
                let parsed = if input.is_empty() {
                    Some(domain::RPE::default())
                } else {
                    domain::RPE::try_from(input.as_ref()).ok()
                };
                *rpe = common::InputField {
                    input,
                    parsed,
                    orig: rpe.orig.clone(),
                }
            }
        }
        Msg::AutomaticChanged(id) => {
            if let Some(Form::Activity { automatic, .. }) = get_part(&mut model.sections, &id) {
                *automatic = not(*automatic);
            }
        }

        Msg::ExerciseList(msg) => {
            if let Dialog::SelectExercise(part_id, exercise_list_model) = &mut model.dialog {
                match component::exercise_list::update(
                    msg,
                    exercise_list_model,
                    &mut orders.proxy(Msg::ExerciseList),
                ) {
                    component::exercise_list::OutMsg::None
                    | component::exercise_list::OutMsg::EditClicked(_)
                    | component::exercise_list::OutMsg::DeleteClicked(_)
                    | component::exercise_list::OutMsg::CatalogExerciseSelected(_) => {}
                    component::exercise_list::OutMsg::CreateClicked(name) => {
                        orders.notify(data::Msg::CreateExercise(name, vec![]));
                    }
                    component::exercise_list::OutMsg::Selected(exercise_id) => {
                        orders.send_msg(Msg::ExerciseChanged(part_id.clone(), exercise_id));
                    }
                };
            }
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
                            .add_hash_path_part(model.routine_id.as_u128().to_string()),
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
    model.interval = domain::init_interval(
        &data_model
            .training_sessions
            .values()
            .filter(|t| t.routine_id == model.routine_id)
            .map(|t| t.date)
            .collect::<Vec<NaiveDate>>(),
        domain::DefaultInterval::All,
    );

    let routine = &data_model.routines.get(&model.routine_id);

    if let Some(routine) = routine {
        model.name = common::InputField {
            input: routine.name.to_string(),
            parsed: Some(routine.name.clone()),
            orig: routine.name.to_string(),
        };
        model.sections = routine.sections.iter().map(Into::into).collect();
        let training_sessions = &data_model
            .training_sessions
            .values()
            .filter(|t| t.routine_id == routine.id)
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
    if data_model.routines.is_empty() && data_model.loading_routines > 0 {
        common::view_page_loading()
    } else if let Some(routine) = data_model.routines.get(&model.routine_id) {
        div![
            view_title(model),
            if not(model.editing) {
                view_summary(routine)
            } else {
                empty![]
            },
            view_dialog(&model.dialog, model.loading, data_model),
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
                    view_muscles(routine, data_model),
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
        C!["px-2"],
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

fn view_dialog(dialog: &Dialog, loading: bool, data_model: &data::Model) -> Node<Msg> {
    match dialog {
        Dialog::SelectExercise(_, exercise_list_model) => common::view_dialog(
            "primary",
            span!["Select exercise"],
            component::exercise_list::view(exercise_list_model, loading, data_model)
                .map_msg(Msg::ExerciseList),
            &ev(Ev::Click, |_| Msg::CloseDialog),
        ),
        Dialog::DeleteTrainingSession(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            let date = data_model
                .training_sessions
                .get(&id)
                .map(|t| t.date)
                .unwrap_or_default();
            common::view_delete_confirmation_dialog(
                "training session",
                &span!["of ", common::no_wrap(&date.to_string())],
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

fn view_summary(routine: &domain::Routine) -> Node<Msg> {
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
        C!["p-2"],
        &routine_sections
            .iter()
            .enumerate()
            .map(|(i, s)| {
                view_routine_part(
                    data_model,
                    s,
                    vec![i],
                    editing,
                    data_model.settings.show_rpe,
                    data_model.settings.show_tut,
                )
            })
            .collect::<Vec<_>>(),
        IF![editing => view_add_section_button(vec![])]
    ]
}

fn view_routine_part(
    data_model: &data::Model,
    part: &Form,
    id: Vec<usize>,
    editing: bool,
    show_rpe: bool,
    show_tut: bool,
) -> Node<Msg> {
    match part {
        Form::Section { rounds, parts } => {
            div![
                C!["message"],
                div![
                    C!["message-body"],
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
                            editing,
                            show_rpe,
                            show_tut,
                        ))
                        .collect::<Vec<_>>(),
                    IF![editing => view_add_part_buttons(data_model,id)]
                ]
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
                if exercise_id.is_nil() {
                    C!["is-success"]
                } else {
                    C!["is-info"]
                },
                div![
                    C!["message-body"],
                    C!["has-background-scheme-main"],
                    C!["p-3"],
                    if editing {
                        div![
                            C!["is-flex"],
                            C!["is-justify-content-space-between"],
                            if exercise_id.is_nil() {
                                div![C!["field"], C!["has-text-weight-bold"], plain!["Rest"]]
                            } else {
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
                                            exercise.name.to_string()
                                        } else {
                                            format!("Exercise#{}", exercise_id.as_u128())
                                        }
                                    ]
                                ]
                            },
                            view_position_buttons(id.clone())
                        ]
                    } else if !exercise_id.is_nil() {
                        div![
                            C!["has-text-weight-bold"],
                            if let Some(exercise) = data_model.exercises.get(exercise_id) {
                                a![
                                    attrs! {
                                        At::Href => {
                                            crate::Urls::new(&data_model.base_url)
                                                .exercise()
                                                .add_hash_path_part(exercise_id.as_u128().to_string())
                                        }
                                    },
                                    &exercise.name.as_ref(),
                                ]
                            } else {
                                plain![format!("Exercise#{}", exercise_id.as_u128())]
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
                                !exercise_id.is_nil() =>
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
                            IF![
                                show_tut || exercise_id.is_nil() =>
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
                                ]
                            ],
                            IF![
                                !exercise_id.is_nil() =>
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
                                show_rpe && !exercise_id.is_nil() =>
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
                                ev(Ev::Click, move |_| Msg::AutomaticChanged(id)),
                                span![
                                    C!["icon"],
                                    if *automatic {
                                        C!["has-text-dark-bold"]
                                    } else {
                                        C!["has-text-dark-soft"]
                                    },
                                    common::automatic_icon()
                                ]
                            ]
                        ]
                    } else if !exercise_id.is_nil() {
                        div![
                            IF![
                                if let Some(reps) = reps.parsed { reps > domain::Reps::default() } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-rotate-left"]]],
                                        span![&reps.input]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(time) = time.parsed { time > domain::Time::default() } else { false } && show_tut => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-clock-rotate-left"]]],
                                        span![&time.input, " s"]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(weight) = weight.parsed { weight > domain::Weight::default() } else { false } => {
                                    span![
                                        C!["icon-text"],
                                        C!["mr-4"],
                                        span![C!["mr-2"], i![C!["fas fa-weight-hanging"]]],
                                        span![&weight.input, " kg"]
                                    ]
                                }
                            ],
                            IF![
                                if let Some(rpe) = rpe.parsed { rpe > domain::RPE::ZERO } else { false } && show_rpe => {
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
                                At::Href => crate::Urls::new(&data_model.base_url).exercise().add_hash_path_part(exercise_id.as_u128().to_string()),
                            },
                            &data_model.exercises.get(exercise_id).unwrap().name.as_ref()
                        ]
                    ]
                })
            .collect::<Vec<_>>(),
        ]
    }
}

fn view_muscles(routine: &domain::Routine, data_model: &data::Model) -> Node<Msg> {
    let stimulus_per_muscle = routine.stimulus_per_muscle(&data_model.exercises);
    if stimulus_per_muscle.is_empty() {
        empty![]
    } else {
        div![
            C!["mt-6"],
            C!["has-text-centered"],
            common::view_title(&span!["Sets per muscle"], 3),
            common::view_sets_per_muscle(&stimulus_per_muscle)
        ]
    }
}

fn view_training_sessions(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let training_sessions = data_model
        .training_sessions
        .values()
        .filter(|t| {
            t.routine_id == model.routine_id
                && t.date >= model.interval.first
                && t.date <= model.interval.last
        })
        .collect::<Vec<_>>();
    let dates = training_sessions.iter().map(|t| t.date);
    let routine_interval = domain::Interval {
        first: dates.clone().min().unwrap_or_default(),
        last: dates.max().unwrap_or_default(),
    };
    div![
        C!["container"],
        C!["has-text-centered"],
        C!["mt-6"],
        common::view_title(&span!["Training sessions"], 5),
        common::view_interval_buttons(&model.interval, &routine_interval, Msg::ChangeInterval),
        view_charts(
            &training_sessions,
            &model.interval,
            data_model.theme(),
            data_model.settings.show_rpe,
        ),
        training::view_calendar(&training_sessions, &model.interval),
        training::view_table(
            &training_sessions,
            &data_model.routines,
            &data_model.base_url,
            Msg::ShowDeleteTrainingSessionDialog,
            data_model.settings.show_rpe,
            data_model.settings.show_tut,
        ),
    ]
}

pub fn view_charts<Ms>(
    training_sessions: &[&domain::TrainingSession],
    interval: &domain::Interval,
    theme: &web_app::Theme,
    show_rpe: bool,
) -> Vec<Node<Ms>> {
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
    nodes![
        common::view_chart(
            &[(
                "Load",
                web_app::chart::COLOR_LOAD,
                web_app::chart::OPACITY_LINE
            )],
            web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: load.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_LOAD,
                        web_app::chart::COLOR_LOAD
                    ),
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                }],
                interval,
                theme,
            ),
            false,
        ),
        common::view_chart(
            &[(
                "Set volume",
                web_app::chart::COLOR_SET_VOLUME,
                web_app::chart::OPACITY_LINE
            )],
            web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: set_volume.into_iter().collect::<Vec<_>>(),
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_SET_VOLUME,
                        web_app::chart::COLOR_SET_VOLUME,
                    ),
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                }],
                interval,
                theme,
            ),
            false,
        ),
        IF![
            show_rpe =>
            common::view_chart(
                &[
                    ("RPE", web_app::chart::COLOR_RPE, web_app::chart::OPACITY_AREA),
                    ("Avg. RPE", web_app::chart::COLOR_RPE, web_app::chart::OPACITY_LINE)
                ],
                web_app::chart::plot_min_avg_max(
                    &training_sessions
                        .iter()
                        .flat_map(|s| s
                            .elements
                            .iter()
                            .filter_map(|e| match e {
                                domain::TrainingSessionElement::Set { rpe, .. } =>
                                    rpe.map(|v| (s.date, v)),
                                _ => None,
                            })
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                    interval,
                    web_app::chart::PlotParams::primary_range(5., 10.),
                    web_app::chart::COLOR_RPE,
                    theme,
                ),
                false,
            )
        ],
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
            ev(Ev::Click, move |_| Msg::RemovePart(id)),
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
                let exercise_id = exercises.first().map(|e| e.id).unwrap_or_default();
                move |_| Msg::AddActivity(id, exercise_id)
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
                move |_| Msg::AddActivity(id, domain::ExerciseID::nil())
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
        ev(Ev::Click, move |_| Msg::AddSection(id)),
        span![
            i![C!["fas fa-repeat"]],
            i![C!["ml-1"], C!["fas fa-plus-circle"]],
        ]
    ]
}

fn get_part<'a>(sections: &'a mut [Form], id: &[usize]) -> Option<&'a mut Form> {
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
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(1).unwrap()),
                    time: form_value(domain::Time::new(2).unwrap()),
                    weight: form_value(domain::Weight::new(4.0).unwrap()),
                    rpe: form_value(domain::RPE::FIVE),
                    automatic: false,
                }],
            },
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(2).unwrap()),
                    time: form_value(domain::Time::new(3).unwrap()),
                    weight: form_value(domain::Weight::new(5.0).unwrap()),
                    rpe: form_value(domain::RPE::SIX),
                    automatic: false,
                }],
            },
        ];
        assert_eq!(
            *get_part(&mut sections, &[0]).unwrap(),
            Form::Section {
                rounds: form_value(1),
                parts: vec![Form::Activity {
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(1).unwrap()),
                    time: form_value(domain::Time::new(2).unwrap()),
                    weight: form_value(domain::Weight::new(4.0).unwrap()),
                    rpe: form_value(domain::RPE::FIVE),
                    automatic: false,
                }],
            }
        );
        assert_eq!(
            *get_part(&mut sections, &[1]).unwrap(),
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(2).unwrap()),
                    time: form_value(domain::Time::new(3).unwrap()),
                    weight: form_value(domain::Weight::new(5.0).unwrap()),
                    rpe: form_value(domain::RPE::SIX),
                    automatic: false,
                }],
            }
        );
        assert!(get_part(&mut sections, &[2]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 0]).unwrap(),
            Form::Activity {
                exercise_id: domain::ExerciseID::nil(),
                reps: form_value(domain::Reps::new(1).unwrap()),
                time: form_value(domain::Time::new(2).unwrap()),
                weight: form_value(domain::Weight::new(4.0).unwrap()),
                rpe: form_value(domain::RPE::FIVE),
                automatic: false,
            },
        );
        assert!(get_part(&mut sections, &[1, 0]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 1]).unwrap(),
            Form::Activity {
                exercise_id: domain::ExerciseID::nil(),
                reps: form_value(domain::Reps::new(2).unwrap()),
                time: form_value(domain::Time::new(3).unwrap()),
                weight: form_value(domain::Weight::new(5.0).unwrap()),
                rpe: form_value(domain::RPE::SIX),
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
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(1).unwrap()),
                    time: form_value(domain::Time::new(2).unwrap()),
                    weight: form_value(domain::Weight::new(4.0).unwrap()),
                    rpe: form_value(domain::RPE::FIVE),
                    automatic: false,
                },
                Form::Section {
                    rounds: form_value(2),
                    parts: vec![Form::Activity {
                        exercise_id: domain::ExerciseID::nil(),
                        reps: form_value(domain::Reps::new(2).unwrap()),
                        time: form_value(domain::Time::new(3).unwrap()),
                        weight: form_value(domain::Weight::new(5.0).unwrap()),
                        rpe: form_value(domain::RPE::SIX),
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
                        exercise_id: domain::ExerciseID::nil(),
                        reps: form_value(domain::Reps::new(1).unwrap()),
                        time: form_value(domain::Time::new(2).unwrap()),
                        weight: form_value(domain::Weight::new(4.0).unwrap()),
                        rpe: form_value(domain::RPE::FIVE),
                        automatic: false,
                    },
                    Form::Section {
                        rounds: form_value(2),
                        parts: vec![Form::Activity {
                            exercise_id: domain::ExerciseID::nil(),
                            reps: form_value(domain::Reps::new(2).unwrap()),
                            time: form_value(domain::Time::new(3).unwrap()),
                            weight: form_value(domain::Weight::new(5.0).unwrap()),
                            rpe: form_value(domain::RPE::SIX),
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
                exercise_id: domain::ExerciseID::nil(),
                reps: form_value(domain::Reps::new(1).unwrap()),
                time: form_value(domain::Time::new(2).unwrap()),
                weight: form_value(domain::Weight::new(4.0).unwrap()),
                rpe: form_value(domain::RPE::FIVE),
                automatic: false,
            },
        );
        assert_eq!(
            *get_part(&mut sections, &[1, 0]).unwrap(),
            Form::Section {
                rounds: form_value(2),
                parts: vec![Form::Activity {
                    exercise_id: domain::ExerciseID::nil(),
                    reps: form_value(domain::Reps::new(2).unwrap()),
                    time: form_value(domain::Time::new(3).unwrap()),
                    weight: form_value(domain::Weight::new(5.0).unwrap()),
                    rpe: form_value(domain::RPE::SIX),
                    automatic: false,
                }],
            },
        );
        assert!(get_part(&mut sections, &[2, 0]).is_none());
        assert!(get_part(&mut sections, &[0, 0, 0]).is_none());
        assert_eq!(
            *get_part(&mut sections, &[0, 1, 0]).unwrap(),
            Form::Activity {
                exercise_id: domain::ExerciseID::nil(),
                reps: form_value(domain::Reps::new(2).unwrap()),
                time: form_value(domain::Time::new(3).unwrap()),
                weight: form_value(domain::Weight::new(5.0).unwrap()),
                rpe: form_value(domain::RPE::SIX),
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
