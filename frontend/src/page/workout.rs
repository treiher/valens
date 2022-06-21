use std::collections::HashMap;

use seed::{prelude::*, *};
use slice_group_by::GroupBy;

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, data_model: &data::Model) -> Model {
    let workout_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);

    orders.subscribe(Msg::DataEvent);

    let workout = &data_model.workouts.iter().find(|w| w.id == workout_id);

    Model {
        workout_id,
        form: init_form(workout),
        previous_sets: init_previous_sets(workout, data_model),
        loading: false,
    }
}

fn init_form(workout: &Option<&data::Workout>) -> Form {
    if let Some(workout) = workout {
        Form {
            notes: workout.notes.clone().unwrap_or_default(),
            sets: workout
                .sets
                .iter()
                .map(|s| SetForm {
                    position: s.position,
                    exercise_id: s.exercise_id,
                    reps: (
                        s.reps.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.reps,
                        false,
                    ),
                    time: (
                        s.time.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.time,
                        false,
                    ),
                    weight: (
                        s.weight.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.weight,
                        false,
                    ),
                    rpe: (
                        s.rpe.map(|v| v.to_string()).unwrap_or_default(),
                        true,
                        s.rpe,
                        false,
                    ),
                })
                .collect::<Vec<_>>(),
        }
    } else {
        Form {
            notes: String::new(),
            sets: vec![],
        }
    }
}

fn init_previous_sets(
    workout: &Option<&data::Workout>,
    data_model: &data::Model,
) -> HashMap<u32, Vec<data::WorkoutSet>> {
    let mut sets: HashMap<u32, Vec<data::WorkoutSet>> = HashMap::new();
    if let Some(workout) = workout {
        if let Some(previous_workout) = &data_model
            .workouts
            .iter()
            .filter(|w| w.id != workout.id && w.date <= workout.date)
            .last()
        {
            for s in &previous_workout.sets {
                sets.entry(s.exercise_id).or_default().push(s.clone());
            }
        }
    }
    sets
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    workout_id: u32,
    form: Form,
    previous_sets: HashMap<u32, Vec<data::WorkoutSet>>,
    loading: bool,
}

struct Form {
    notes: String,
    sets: Vec<SetForm>,
}

#[derive(Clone)]
struct SetForm {
    position: u32,
    exercise_id: u32,
    reps: (String, bool, Option<u32>, bool),
    time: (String, bool, Option<u32>, bool),
    weight: (String, bool, Option<f32>, bool),
    rpe: (String, bool, Option<f32>, bool),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    RepsChanged(usize, String),
    TimeChanged(usize, String),
    WeightChanged(usize, String),
    RPEChanged(usize, String),
    NotesChanged(String),

    SaveWorkout,
    DataEvent(data::Event),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::RepsChanged(index, reps) => match reps.parse::<u32>() {
            Ok(parsed_reps) => {
                let valid = parsed_reps > 0 && parsed_reps < 1000;
                model.form.sets[index].reps = (
                    reps,
                    valid,
                    if valid { Some(parsed_reps) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].reps = (reps.clone(), reps.is_empty(), None, true),
        },
        Msg::TimeChanged(index, time) => match time.parse::<u32>() {
            Ok(parsed_time) => {
                let valid = parsed_time > 0 && parsed_time < 1000;
                model.form.sets[index].time = (
                    time,
                    valid,
                    if valid { Some(parsed_time) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].time = (time.clone(), time.is_empty(), None, true),
        },
        Msg::WeightChanged(index, weight) => match weight.parse::<f32>() {
            Ok(parsed_weight) => {
                let valid = parsed_weight > 0.0
                    && parsed_weight < 1000.0
                    && (parsed_weight * 10.0 % 1.0).abs() < f32::EPSILON;
                model.form.sets[index].weight = (
                    weight,
                    valid,
                    if valid { Some(parsed_weight) } else { None },
                    true,
                )
            }
            Err(_) => {
                model.form.sets[index].weight = (weight.clone(), weight.is_empty(), None, true)
            }
        },
        Msg::RPEChanged(index, rpe) => match rpe.parse::<f32>() {
            Ok(parsed_rpe) => {
                let valid =
                    (0.0..=10.0).contains(&parsed_rpe) && (parsed_rpe % 0.5).abs() < f32::EPSILON;
                model.form.sets[index].rpe = (
                    rpe,
                    valid,
                    if valid { Some(parsed_rpe) } else { None },
                    true,
                )
            }
            Err(_) => model.form.sets[index].rpe = (rpe.clone(), rpe.is_empty(), None, true),
        },
        Msg::NotesChanged(notes) => {
            model.form.notes = notes;
        }

        Msg::SaveWorkout => {
            model.loading = true;
            orders.notify(data::Msg::ModifyWorkout(
                model.workout_id,
                Some(model.form.notes.clone()),
                Some(
                    model
                        .form
                        .sets
                        .iter()
                        .map(|s| data::WorkoutSet {
                            position: s.position,
                            exercise_id: s.exercise_id,
                            reps: s.reps.2,
                            time: s.time.2,
                            weight: s.weight.2,
                            rpe: s.rpe.2,
                        })
                        .collect::<Vec<_>>(),
                ),
            ));
        }
        Msg::DataEvent(event) => {
            match event {
                data::Event::WorkoutsReadOk => {
                    let workout = &data_model
                        .workouts
                        .iter()
                        .find(|w| w.id == model.workout_id);
                    model.form = init_form(workout);
                    model.previous_sets = init_previous_sets(workout, data_model);
                }
                data::Event::WorkoutModifiedOk => {
                    model.loading = false;
                }
                _ => {}
            };
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.workouts.iter().any(|w| w.id == model.workout_id) {
        let changed = model
            .form
            .sets
            .iter()
            .any(|s| s.reps.3 || s.time.3 || s.weight.3 || s.rpe.3);
        let valid = model
            .form
            .sets
            .iter()
            .all(|s| s.reps.1 && s.time.1 && s.weight.1 && s.rpe.1);
        let save_disabled = not(changed) || not(valid);
        let mut form: std::vec::Vec<seed::virtual_dom::Node<Msg>> = nodes![];
        for sets in (&model.form.sets[..]).linear_group_by(|a, b| a.exercise_id == b.exercise_id) {
            form.push(div![
                C!["field"],
                label![
                    C!["label"],
                    a![
                        attrs! {
                            At::Href => {
                                crate::Urls::new(&data_model.base_url)
                                    .exercise()
                                    .add_hash_path_part(sets.first().unwrap().exercise_id.to_string())
                            },
                            At::from("tabindex") => -1
                        },
                        &data_model
                            .exercises
                            .iter()
                            .find(|e| e.id == sets.first().unwrap().exercise_id)
                            .map(|e| e.name.clone())
                            .unwrap_or_default()
                    ],
                ],
                sets.iter().enumerate().map(|(j, s)| {
                    let i = usize::try_from(s.position).unwrap() - 1;
                    let (prev_reps, prev_time, prev_weight, prev_rpe) =
                        if let Some(prev_sets) = model.previous_sets.get(&s.exercise_id) {
                            if let Some(prev_set) = prev_sets.get(j) {
                                (prev_set.reps.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.time.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.weight.map(|v| v.to_string()).unwrap_or_default(),
                                prev_set.rpe.map(|v| v.to_string()).unwrap_or_default())
                            } else {
                                (String::new(),String::new(),String::new(),String::new())
                            }
                        } else {
                            (String::new(),String::new(),String::new(),String::new())
                        };
                    div![
                        C!["field"],
                        C!["has-addons"],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::RepsChanged(i, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.reps.1) => "is-danger"]],
                                C![IF![s.reps.3 => "is-info"]],
                                attrs! {
                                    At::Type => "number",
                                    At::Min => 0,
                                    At::Max => 999,
                                    At::Step => 1,
                                    At::Size => 2,
                                    At::Value => s.reps.0,
                                    At::Placeholder => prev_reps,
                                }
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "✕"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::TimeChanged(i, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.time.1) => "is-danger"]],
                                C![IF![s.time.3 => "is-info"]],
                                attrs! {
                                    At::Type => "number",
                                    At::Min => 0,
                                    At::Max => 999,
                                    At::Step => 1,
                                    At::Size => 2,
                                    At::Value => s.time.0,
                                    At::Placeholder => prev_time,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "s"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::WeightChanged(i, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.weight.1) => "is-danger"]],
                                C![IF![s.weight.3 => "is-info"]],
                                attrs! {
                                    At::from("inputmode") => "numeric",
                                    At::Size => 3,
                                    At::Value => s.weight.0,
                                    At::Placeholder => prev_weight,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "kg"],
                        ],
                        div![
                            C!["control"],
                            C!["has-icons-right"],
                            C!["has-text-right"],
                            input_ev(Ev::Input, move |v| Msg::RPEChanged(i, v)),
                            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                                IF!(
                                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                        Msg::SaveWorkout
                                    }
                                )
                            }),
                            input![
                                C!["input"],
                                C!["has-text-right"],
                                C![IF![not(s.rpe.1) => "is-danger"]],
                                C![IF![s.rpe.3 => "is-info"]],
                                attrs! {
                                    At::from("inputmode") => "numeric",
                                    At::Size => 2,
                                    At::Value => s.rpe.0,
                                    At::Placeholder => prev_rpe,
                                },
                            ],
                            span![C!["icon"], C!["is-small"], C!["is-right"], "@"],
                        ],
                    ]
                })
            ]);
        }
        div![
            C!["container"],
            C!["mx-2"],
            form![
                attrs! {
                    At::Action => "javascript:void(0);",
                    At::OnKeyPress => "if (event.which == 13) return false;"
                },
                &form
            ],
            div![
                C!["field"],
                label![C!["label"], "Notes"],
                input_ev(Ev::Input, Msg::NotesChanged),
                textarea![C!["textarea"],]
            ],
            button![
                C!["button"],
                C!["is-fab"],
                C!["is-medium"],
                C!["is-link"],
                C![IF![not(valid) => "is-danger"]],
                C![IF![model.loading => "is-loading"]],
                attrs![
                    At::Disabled => save_disabled.as_at_value(),
                ],
                ev(Ev::Click, |_| Msg::SaveWorkout),
                span![C!["icon"], i![C!["fas fa-save"]]]
            ]
        ]
    } else if data_model.workouts.is_empty() {
        common::view_loading()
    } else {
        common::view_error_not_found("Workout")
    }
}
