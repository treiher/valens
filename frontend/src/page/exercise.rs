use std::collections::BTreeMap;

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
    let exercise_id = url
        .next_hash_path_part()
        .unwrap_or("")
        .parse::<u32>()
        .unwrap_or(0);

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Exercise");

    Model {
        interval: common::init_interval(
            &data_model
                .workouts
                .values()
                .filter(|w| w.exercises().contains(&exercise_id))
                .map(|w| w.date)
                .collect::<Vec<NaiveDate>>(),
            false,
        ),
        exercise_id,
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: common::Interval,
    exercise_id: u32,
    dialog: Dialog,
    loading: bool,
}

enum Dialog {
    Hidden,
    DeleteWorkout(u32),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowDeleteWorkoutDialog(u32),
    CloseDialog,

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
        Msg::ShowDeleteWorkoutDialog(position) => {
            model.dialog = Dialog::DeleteWorkout(position);
        }
        Msg::CloseDialog => {
            model.dialog = Dialog::Hidden;
            model.loading = false;
            Url::go_and_replace(
                &crate::Urls::new(&data_model.base_url)
                    .routine()
                    .add_hash_path_part(model.exercise_id.to_string()),
            );
        }

        Msg::DeleteWorkout(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteWorkout(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    model.interval = common::init_interval(
                        &data_model
                            .workouts
                            .values()
                            .filter(|w| w.exercises().contains(&model.exercise_id))
                            .map(|w| w.date)
                            .collect::<Vec<NaiveDate>>(),
                        false,
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

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.exercises.is_empty() && data_model.loading_exercises {
        common::view_loading()
    } else if let Some(exercise) = data_model.exercises.get(&model.exercise_id) {
        let workouts = exercise_workouts(model, data_model);
        div![
            common::view_title(&span![&exercise.name], 5),
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            view_charts(&workouts.iter().collect::<Vec<_>>(), &model.interval),
            workouts::view_table(
                &workouts.iter().collect::<Vec<_>>(),
                &data_model.routines,
                &data_model.base_url,
                Msg::ShowDeleteWorkoutDialog
            ),
            view_dialog(&model.dialog, model.loading)
        ]
    } else {
        common::view_error_not_found("Exercise")
    }
}

pub fn view_charts<Ms>(workouts: &[&data::Workout], interval: &common::Interval) -> Vec<Node<Ms>> {
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_rpe: BTreeMap<NaiveDate, (Vec<f32>, Vec<f32>)> = BTreeMap::new();
    let mut weight: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();
    let mut time: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();
    for workout in workouts {
        set_volume
            .entry(workout.date)
            .and_modify(|e| *e += workout.set_volume() as f32)
            .or_insert(workout.set_volume() as f32);
        volume_load
            .entry(workout.date)
            .and_modify(|e| *e += workout.volume_load() as f32)
            .or_insert(workout.volume_load() as f32);
        tut.entry(workout.date)
            .and_modify(|e| *e += workout.tut() as f32)
            .or_insert(workout.tut() as f32);
        if let Some(avg_reps) = workout.avg_reps() {
            reps_rpe
                .entry(workout.date)
                .and_modify(|e| e.0.push(avg_reps))
                .or_insert((vec![avg_reps], vec![]));
        }
        if let Some(avg_rpe) = workout.avg_rpe() {
            reps_rpe
                .entry(workout.date)
                .and_modify(|e| e.1.push(avg_rpe));
        }
        if let Some(avg_weight) = workout.avg_weight() {
            weight
                .entry(workout.date)
                .and_modify(|e| e.push(avg_weight))
                .or_insert(vec![avg_weight]);
        }
        if let Some(avg_time) = workout.avg_time() {
            time.entry(workout.date)
                .and_modify(|e| e.push(avg_time))
                .or_insert(vec![avg_time]);
        }
    }
    nodes![
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
            &[("Volume load", common::COLOR_VOLUME_LOAD)],
            common::plot_line_chart(
                &[(
                    volume_load.into_iter().collect::<Vec<_>>(),
                    common::COLOR_VOLUME_LOAD,
                )],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Time under tension (s)", common::COLOR_TUT)],
            common::plot_line_chart(
                &[(tut.into_iter().collect::<Vec<_>>(), common::COLOR_TUT,)],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[
                ("Repetitions", common::COLOR_REPS),
                ("+ Repetititions in reserve", common::COLOR_REPS_RIR)
            ],
            common::plot_line_chart(
                &[
                    (
                        reps_rpe
                            .iter()
                            .map(|(date, (avg_reps, _))| {
                                (*date, avg_reps.iter().sum::<f32>() / avg_reps.len() as f32)
                            })
                            .collect::<Vec<_>>(),
                        common::COLOR_REPS,
                    ),
                    (
                        reps_rpe
                            .into_iter()
                            .filter_map(|(date, (avg_reps_values, avg_rpe_values))| {
                                let avg_reps = avg_reps_values.iter().sum::<f32>()
                                    / avg_reps_values.len() as f32;
                                let avg_rpe = avg_rpe_values.iter().sum::<f32>()
                                    / avg_rpe_values.len() as f32;
                                if not(avg_rpe_values.is_empty()) {
                                    Some((date, avg_reps + 10.0 - avg_rpe))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                        common::COLOR_REPS_RIR,
                    ),
                ],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Weight (kg)", common::COLOR_WEIGHT)],
            common::plot_line_chart(
                &[(
                    weight
                        .into_iter()
                        .map(|(date, values)| {
                            (date, values.iter().sum::<f32>() / values.len() as f32)
                        })
                        .collect::<Vec<_>>(),
                    common::COLOR_WEIGHT,
                )],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Time (s)", common::COLOR_TIME)],
            common::plot_line_chart(
                &[(
                    time.into_iter()
                        .map(|(date, values)| {
                            (date, values.iter().sum::<f32>() / values.len() as f32)
                        })
                        .collect::<Vec<_>>(),
                    common::COLOR_TIME,
                )],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
    ]
}

fn view_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    match dialog {
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

fn exercise_workouts(model: &Model, data_model: &data::Model) -> Vec<data::Workout> {
    data_model
        .workouts
        .values()
        .filter(|w| {
            w.exercises().contains(&model.exercise_id)
                && w.date >= model.interval.first
                && w.date <= model.interval.last
        })
        .map(|w| data::Workout {
            id: w.id,
            routine_id: w.routine_id,
            date: w.date,
            notes: w.notes.clone(),
            elements: w
                .elements
                .iter()
                .filter(|e| match e {
                    data::WorkoutElement::WorkoutSet { exercise_id, .. } => {
                        *exercise_id == model.exercise_id
                    }
                    _ => false,
                })
                .cloned()
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
}
