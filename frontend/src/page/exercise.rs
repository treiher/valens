use std::collections::BTreeMap;

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
                .training_sessions
                .values()
                .filter(|t| t.exercises().contains(&exercise_id))
                .map(|t| t.date)
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
    DeleteTrainingSession(u32),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowDeleteTrainingSessionDialog(u32),
    CloseDialog,

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
        Msg::ShowDeleteTrainingSessionDialog(position) => {
            model.dialog = Dialog::DeleteTrainingSession(position);
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

        Msg::DeleteTrainingSession(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteTrainingSession(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    model.interval = common::init_interval(
                        &data_model
                            .training_sessions
                            .values()
                            .filter(|t| t.exercises().contains(&model.exercise_id))
                            .map(|t| t.date)
                            .collect::<Vec<NaiveDate>>(),
                        false,
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

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.exercises.is_empty() && data_model.loading_exercises {
        common::view_page_loading()
    } else if let Some(exercise) = data_model.exercises.get(&model.exercise_id) {
        let exercise_training_sessions = exercise_training_sessions(model, data_model);
        let dates = exercise_training_sessions.iter().map(|t| t.date);
        let exercise_interval = common::Interval {
            first: dates.clone().min().unwrap_or_default(),
            last: dates.max().unwrap_or_default(),
        };
        let training_sessions = exercise_training_sessions
            .iter()
            .filter(|t| t.date >= model.interval.first && t.date <= model.interval.last)
            .collect::<Vec<_>>();
        div![
            common::view_title(&span![&exercise.name], 5),
            common::view_interval_buttons(&model.interval, &exercise_interval, Msg::ChangeInterval),
            view_charts(&training_sessions, &model.interval),
            view_calendar(&training_sessions, &model.interval),
            training::view_table(
                &training_sessions,
                &data_model.routines,
                &data_model.base_url,
                Msg::ShowDeleteTrainingSessionDialog
            ),
            view_dialog(&model.dialog, model.loading)
        ]
    } else {
        common::view_error_not_found("Exercise")
    }
}

pub fn view_charts<Ms>(
    training_sessions: &[&data::TrainingSession],
    interval: &common::Interval,
) -> Vec<Node<Ms>> {
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_rpe: BTreeMap<NaiveDate, (Vec<f32>, Vec<f32>)> = BTreeMap::new();
    let mut weight: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();
    let mut time: BTreeMap<NaiveDate, Vec<f32>> = BTreeMap::new();
    for training_session in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume() as f32)
            .or_insert(training_session.set_volume() as f32);
        #[allow(clippy::cast_precision_loss)]
        volume_load
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.volume_load() as f32)
            .or_insert(training_session.volume_load() as f32);
        #[allow(clippy::cast_precision_loss)]
        tut.entry(training_session.date)
            .and_modify(|e| *e += training_session.tut() as f32)
            .or_insert(training_session.tut() as f32);
        if let Some(avg_reps) = training_session.avg_reps() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.0.push(avg_reps))
                .or_insert((vec![avg_reps], vec![]));
        }
        if let Some(avg_rpe) = training_session.avg_rpe() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.1.push(avg_rpe));
        }
        if let Some(avg_weight) = training_session.avg_weight() {
            weight
                .entry(training_session.date)
                .and_modify(|e| e.push(avg_weight))
                .or_insert(vec![avg_weight]);
        }
        if let Some(avg_time) = training_session.avg_time() {
            time.entry(training_session.date)
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
                                #[allow(clippy::cast_precision_loss)]
                                (*date, avg_reps.iter().sum::<f32>() / avg_reps.len() as f32)
                            })
                            .collect::<Vec<_>>(),
                        common::COLOR_REPS,
                    ),
                    (
                        reps_rpe
                            .into_iter()
                            .filter_map(|(date, (avg_reps_values, avg_rpe_values))| {
                                #[allow(clippy::cast_precision_loss)]
                                let avg_reps = avg_reps_values.iter().sum::<f32>()
                                    / avg_reps_values.len() as f32;
                                #[allow(clippy::cast_precision_loss)]
                                let avg_rpe = avg_rpe_values.iter().sum::<f32>()
                                    / avg_rpe_values.len() as f32;
                                if avg_rpe_values.is_empty() {
                                    None
                                } else {
                                    Some((date, avg_reps + 10.0 - avg_rpe))
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
                            #[allow(clippy::cast_precision_loss)]
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
                            #[allow(clippy::cast_precision_loss)]
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

fn view_calendar(
    training_sessions: &[&data::TrainingSession],
    interval: &common::Interval,
) -> Node<Msg> {
    let mut volume_load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            volume_load
                .entry(training_session.date)
                .and_modify(|e| *e += training_session.volume_load())
                .or_insert(training_session.volume_load());
        }
    }
    let min = volume_load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = volume_load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);

    common::view_calendar(
        volume_load
            .iter()
            .map(|(date, volume_load)| {
                (
                    *date,
                    common::COLOR_VOLUME_LOAD,
                    if max > min {
                        (f64::from(volume_load - min) / f64::from(max - min)) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
            .collect(),
        interval,
    )
}

fn view_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    match dialog {
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

fn exercise_training_sessions(
    model: &Model,
    data_model: &data::Model,
) -> Vec<data::TrainingSession> {
    data_model
        .training_sessions
        .values()
        .filter(|t| t.exercises().contains(&model.exercise_id))
        .map(|t| data::TrainingSession {
            id: t.id,
            routine_id: t.routine_id,
            date: t.date,
            notes: t.notes.clone(),
            elements: t
                .elements
                .iter()
                .filter(|e| match e {
                    data::TrainingSessionElement::Set { exercise_id, .. } => {
                        *exercise_id == model.exercise_id
                    }
                    _ => false,
                })
                .cloned()
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
}
