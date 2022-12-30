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
                .iter()
                .filter(|w| w.sets.iter().any(|s| s.exercise_id == exercise_id))
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
                            .iter()
                            .filter(|w| w.sets.iter().any(|s| s.exercise_id == model.exercise_id))
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
    if let Some(exercise) = data_model
        .exercises
        .iter()
        .find(|e| e.id == model.exercise_id)
    {
        let workouts = exercise_workouts(model, data_model);
        div![
            common::view_title(&span![&exercise.name], 5),
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            workouts::view_charts(
                workouts.iter().collect::<Vec<_>>().as_slice(),
                &model.interval,
            ),
            workouts::view_table(
                exercise_workouts(model, data_model).as_slice(),
                &data_model.routines,
                &model.interval,
                &data_model.base_url,
                Msg::ShowDeleteWorkoutDialog
            ),
            view_dialog(&model.dialog, model.loading)
        ]
    } else {
        empty![]
    }
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
        .iter()
        .filter(|w| {
            w.sets.iter().any(|s| s.exercise_id == model.exercise_id)
                && w.date >= model.interval.first
                && w.date <= model.interval.last
        })
        .map(|w| data::Workout {
            id: w.id,
            routine_id: w.routine_id,
            date: w.date,
            notes: w.notes.clone(),
            sets: w
                .sets
                .iter()
                .filter(|s| s.exercise_id == model.exercise_id)
                .cloned()
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
}
