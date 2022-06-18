use std::cmp;

use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, data_model: &data::Model) -> Model {
    let base_url = url.to_hash_base_url();
    let routine_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddExerciseDialog);
    }

    orders.subscribe(Msg::DataEvent);

    let (first, last) = common::initial_interval(
        &data_model
            .workouts
            .iter()
            .filter(|w| w.routine_id == Some(routine_id))
            .map(|w| w.date)
            .collect::<Vec<NaiveDate>>(),
    );

    Model {
        base_url,
        interval: common::Interval { first, last },
        routine_id,
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    interval: common::Interval,
    routine_id: u32,
    dialog: Dialog,
    loading: bool,
}

enum Dialog {
    Hidden,
    AddExercise(Form),
    EditExercise(Form),
    DeleteExercise(u32),
    DeleteWorkout(u32),
}

struct Form {
    position: (String, Option<u32>),
    exercise_id: (String, Option<u32>),
    sets: (String, Option<u32>),
    current_position: u32,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddExerciseDialog,
    ShowEditExerciseDialog(u32),
    ShowDeleteExerciseDialog(u32),
    ShowDeleteWorkoutDialog(u32),
    CloseDialog,

    PositionChanged(String),
    ExerciseChanged(String),
    SetsChanged(String),

    SaveExercise,
    DeleteExercise(u32),
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
        Msg::ShowAddExerciseDialog => {
            let exercise_id = data_model.exercises[0].id;
            if let Some(routine) = &data_model
                .routines
                .iter()
                .find(|r| r.id == model.routine_id)
            {
                let position = routine.exercises.len() + 1;
                model.dialog = Dialog::AddExercise(Form {
                    position: (position.to_string(), Some(position.try_into().unwrap())),
                    exercise_id: (exercise_id.to_string(), Some(exercise_id)),
                    sets: (String::new(), None),
                    current_position: 0,
                });
            }
        }
        Msg::ShowEditExerciseDialog(position) => {
            if let Some(routine) = &data_model
                .routines
                .iter()
                .find(|r| r.id == model.routine_id)
            {
                let exercise = &routine.exercises[usize::try_from(position - 1).unwrap()];
                model.dialog = Dialog::EditExercise(Form {
                    position: (position.to_string(), Some(position)),
                    exercise_id: (exercise.exercise_id.to_string(), Some(exercise.exercise_id)),
                    sets: (exercise.sets.to_string(), Some(exercise.sets)),
                    current_position: position,
                });
            }
        }
        Msg::ShowDeleteExerciseDialog(position) => {
            model.dialog = Dialog::DeleteExercise(position);
        }
        Msg::ShowDeleteWorkoutDialog(position) => {
            model.dialog = Dialog::DeleteWorkout(position);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
            model.loading = false;
            Url::go_and_replace(
                &crate::Urls::new(&model.base_url)
                    .routine()
                    .add_hash_path_part(model.routine_id.to_string()),
            );
        }

        Msg::PositionChanged(position) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                match position.parse::<u32>() {
                    Ok(parsed_position) => {
                        form.position = (
                            position,
                            if parsed_position > 0 {
                                Some(parsed_position)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.position = (position, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) | Dialog::DeleteWorkout(_) => {
                panic!();
            }
        },
        Msg::ExerciseChanged(exercise_id) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                match exercise_id.parse::<u32>() {
                    Ok(parsed_exercise_id) => {
                        form.exercise_id = (
                            exercise_id,
                            if parsed_exercise_id > 0 {
                                Some(parsed_exercise_id)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.exercise_id = (exercise_id, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) | Dialog::DeleteWorkout(_) => {
                panic!();
            }
        },
        Msg::SetsChanged(sets) => match model.dialog {
            Dialog::AddExercise(ref mut form) | Dialog::EditExercise(ref mut form) => {
                match sets.parse::<u32>() {
                    Ok(parsed_sets) => {
                        form.sets = (
                            sets,
                            if parsed_sets > 0 {
                                Some(parsed_sets)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.sets = (sets, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteExercise(_) | Dialog::DeleteWorkout(_) => {
                panic!();
            }
        },

        Msg::SaveExercise => {
            model.loading = true;
            if let Some(routine) = &data_model
                .routines
                .iter()
                .find(|r| r.id == model.routine_id)
            {
                match model.dialog {
                    Dialog::AddExercise(ref mut form) => {
                        let position = form.position.1.unwrap();
                        let mut exercises = vec![];
                        if not(routine.exercises.is_empty()) {
                            if usize::try_from(position).unwrap() > routine.exercises.len() {
                                exercises.extend(
                                    routine.exercises[..position as usize - 1].iter().cloned(),
                                );
                            } else {
                                exercises.extend(routine.exercises.clone());
                            }
                        }
                        exercises.push(data::RoutineExercise {
                            position,
                            exercise_id: form.exercise_id.1.unwrap(),
                            sets: form.sets.1.unwrap(),
                        });
                        if usize::try_from(position).unwrap() <= routine.exercises.len() {
                            exercises.extend(
                                routine.exercises[position as usize - 1..].iter().map(|e| {
                                    data::RoutineExercise {
                                        position: e.position + 1,
                                        exercise_id: e.exercise_id,
                                        sets: e.sets,
                                    }
                                }),
                            );
                        }
                        orders.notify(data::Msg::ModifyRoutine(
                            model.routine_id,
                            None,
                            Some(exercises),
                        ));
                    }
                    Dialog::EditExercise(ref mut form) => {
                        let position = form.position.1.unwrap();
                        let position_idx = usize::try_from(position).unwrap() - 1;
                        let current_position_idx =
                            usize::try_from(form.current_position).unwrap() - 1;
                        let unchanged_until_idx = cmp::min(position_idx, current_position_idx);
                        let unchanged_from_idx = cmp::max(position_idx, current_position_idx) + 1;
                        let mut exercises = vec![];
                        exercises.extend(routine.exercises[..unchanged_until_idx].iter().cloned());
                        if current_position_idx != position_idx {
                            if current_position_idx < position_idx {
                                exercises.extend(
                                    routine.exercises[current_position_idx + 1..position_idx + 1]
                                        .iter()
                                        .map(|e| data::RoutineExercise {
                                            position: e.position - 1,
                                            exercise_id: e.exercise_id,
                                            sets: e.sets,
                                        }),
                                );
                            } else {
                                exercises.extend(
                                    routine.exercises[position_idx..current_position_idx]
                                        .iter()
                                        .map(|e| data::RoutineExercise {
                                            position: e.position + 1,
                                            exercise_id: e.exercise_id,
                                            sets: e.sets,
                                        }),
                                );
                            }
                        }
                        exercises.push(data::RoutineExercise {
                            position,
                            exercise_id: form.exercise_id.1.unwrap(),
                            sets: form.sets.1.unwrap(),
                        });
                        exercises.extend(routine.exercises[unchanged_from_idx..].iter().cloned());
                        orders.notify(data::Msg::ModifyRoutine(
                            model.routine_id,
                            None,
                            Some(exercises),
                        ));
                    }
                    Dialog::Hidden | Dialog::DeleteExercise(_) | Dialog::DeleteWorkout(_) => {
                        panic!();
                    }
                };
            }
        }
        Msg::DeleteExercise(position) => {
            model.loading = true;
            if let Some(routine) = &data_model
                .routines
                .iter()
                .find(|r| r.id == model.routine_id)
            {
                let position_idx = usize::try_from(position).unwrap() - 1;
                let mut exercises = vec![];
                exercises.extend(routine.exercises[..position_idx].iter().cloned());
                exercises.extend(routine.exercises[position_idx + 1..].iter().map(|e| {
                    data::RoutineExercise {
                        position: e.position - 1,
                        exercise_id: e.exercise_id,
                        sets: e.sets,
                    }
                }));
                orders.notify(data::Msg::ModifyRoutine(
                    model.routine_id,
                    None,
                    Some(exercises),
                ));
            }
        }
        Msg::DeleteWorkout(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteWorkout(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::RoutineCreatedOk
                | data::Event::RoutineModifiedOk
                | data::Event::RoutineDeletedOk
                | data::Event::WorkoutDeletedOk => {
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

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if let Some(routine) = &data_model
        .routines
        .iter()
        .find(|r| r.id == model.routine_id)
    {
        div![
            view_exercise_dialog(&data_model.exercises, routine, &model.dialog, model.loading),
            nodes![
                view_routine_exercises(model, data_model, routine),
                view_workouts(model, data_model),
                common::view_fab(|_| Msg::ShowAddExerciseDialog),
            ]
        ]
    } else {
        empty![]
    }
}

fn view_exercise_dialog(
    exercises: &[data::Exercise],
    routine: &data::Routine,
    dialog: &Dialog,
    loading: bool,
) -> Node<Msg> {
    let title;
    let form;
    let max_position;
    match dialog {
        Dialog::AddExercise(ref f) => {
            title = "Add exercise";
            form = f;
            max_position = u32::try_from(routine.exercises.len()).unwrap() + 2;
        }
        Dialog::EditExercise(ref f) => {
            title = "Edit exercise";
            form = f;
            max_position = u32::try_from(routine.exercises.len()).unwrap() + 1;
        }
        Dialog::DeleteExercise(position) => {
            #[allow(clippy::clone_on_copy)]
            let position = position.clone();
            return common::view_delete_confirmation_dialog(
                "exercise",
                &ev(Ev::Click, move |_| Msg::DeleteExercise(position)),
                &ev(Ev::Click, |_| Msg::CloseDialog),
                loading,
            );
        }
        Dialog::DeleteWorkout(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            return common::view_delete_confirmation_dialog(
                "workout",
                &ev(Ev::Click, move |_| Msg::DeleteWorkout(id)),
                &ev(Ev::Click, |_| Msg::CloseDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading
        || form.position.1.is_none()
        || form.exercise_id.1.is_none()
        || form.sets.1.is_none();
    common::view_dialog(
        "primary",
        title,
        nodes![
            div![
                C!["field"],
                label![C!["label"], "Position"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::PositionChanged),
                    div![
                        C!["select"],
                        select![(1..max_position)
                            .map(|p| {
                                option![
                                    &p,
                                    attrs![
                                        At::Value => p,
                                        At::Selected => (p == form.position.1.unwrap_or(0)).as_at_value()
                                    ]
                                ]
                            })
                            .collect::<Vec<_>>()],
                    ],
                ],
            ],
            div![
                C!["field"],
                label![C!["label"], "Exercise"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::ExerciseChanged),
                    div![
                        C!["select"],
                        select![exercises
                            .iter()
                            .map(|e| {
                                option![
                                    &e.name,
                                    attrs![
                                        At::Value => e.id,
                                        At::Selected => (e.id == form.exercise_id.1.unwrap_or(0)).as_at_value()
                                    ]
                                ]
                            })
                            .collect::<Vec<_>>()],
                    ],
                ],
            ],
            div![
                C!["field"],
                label![C!["label"], "Sets"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::SetsChanged),
                    input![
                        C!["input"],
                        C![IF![form.sets.1.is_none() => "is-danger"]],
                        attrs! {
                            At::Type => "text",
                            At::Value => form.sets.0,
                        }
                    ],
                ]
            ],
            div![
                C!["field"],
                C!["is-grouped"],
                C!["is-grouped-centered"],
                C!["mt-5"],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-light"],
                        ev(Ev::Click, |_| Msg::CloseDialog),
                        "Cancel",
                    ]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-primary"],
                        C![IF![loading => "is-loading"]],
                        attrs![
                            At::Disabled => save_disabled.as_at_value(),
                        ],
                        ev(Ev::Click, |_| Msg::SaveExercise),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseDialog),
    )
}

fn view_routine_exercises(
    model: &Model,
    data_model: &data::Model,
    routine: &data::Routine,
) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            C!["has-text-centered"],
            thead![tr![
                th![],
                th!["Exercise"],
                th!["Sets"],
                th![],
            ]],
            tbody![&routine
                .exercises
                .iter()
                .map(|e| {
                    #[allow(clippy::clone_on_copy)]
                    let position = e.position;
                    tr![
                        td![format!("E{}", &e.position)],
                        td![
                                a![
                                    attrs! {
                                        At::Href => crate::Urls::new(&model.base_url).exercise().add_hash_path_part(e.exercise_id.to_string()),
                                    },
                                        &data_model.exercises.iter().find(|x| x.id == e.exercise_id).unwrap().name
                                ]
                        ],
                        td![e.sets],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditExerciseDialog(position)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteExerciseDialog(position)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}

fn view_workouts(model: &Model, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["container"],
        C!["has-text-centered"],
        C!["mt-6"],
        h1![C!["title"], C!["is-5"], "Workouts"],
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            &format!("workouts/{}", model.routine_id),
            &model.interval,
            &0
        ),
        view_workouts_table(model, data_model),
    ]
}

fn view_workouts_table(model: &Model, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            C!["has-text-centered"],
            thead![tr![
                th!["Date"],
                th!["Routine"],
                th!["Reps"],
                th!["Time"],
                th!["Weight"],
                th!["RPE"],
                th!["Reps+RIR"],
                th!["Volume"],
                th!["TUT"],
                th![]
            ]],
            tbody![&data_model
                .workouts
                .iter()
                .rev()
                .filter(|w| w.routine_id == Some(model.routine_id) && w.date >= model.interval.first && w.date <= model.interval.last)
                .map(|w| {
                    #[allow(clippy::clone_on_copy)]
                    let id = w.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => crate::Urls::new(&model.base_url).workout().add_hash_path_part(w.id.to_string()),
                            },
                            span![style! {St::WhiteSpace => "nowrap" }, w.date.to_string()]
                        ]],
                        td![
                            if w.routine_id.is_some() {
                                a![
                                    attrs! {
                                        At::Href => crate::Urls::new(&model.base_url).routine().add_hash_path_part(w.routine_id.unwrap().to_string()),
                                    },
                                    &w.routine
                                ]
                            } else {
                                plain!["-"]
                            }
                        ],
                        td![common::value_or_dash(w.avg_reps)],
                        td![common::value_or_dash(w.avg_time)],
                        td![common::value_or_dash(w.avg_weight)],
                        td![common::value_or_dash(w.avg_rpe)],
                        td![if w.avg_reps.is_some() && w.avg_rpe.is_some() {
                            format!("{:.1}", w.avg_reps.unwrap() + w.avg_rpe.unwrap() - 10.0)
                        } else {
                            "-".into()
                        }],
                        td![&w.volume],
                        td![&w.tut],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteWorkoutDialog(id)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}