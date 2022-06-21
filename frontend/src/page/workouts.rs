use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, data_model: &data::Model) -> Model {
    let base_url = url.to_hash_base_url();

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddWorkoutDialog);
    }

    orders.subscribe(Msg::DataEvent);

    Model {
        base_url,
        interval: common::init_interval(
            &data_model
                .workouts
                .iter()
                .map(|w| w.date)
                .collect::<Vec<NaiveDate>>(),
            false,
        ),
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
    dialog: Dialog,
    loading: bool,
}

enum Dialog {
    Hidden,
    AddWorkout(Form),
    DeleteWorkout(u32),
}

struct Form {
    date: (String, Option<NaiveDate>),
    routine_id: (String, Option<u32>),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddWorkoutDialog,
    ShowDeleteWorkoutDialog(u32),
    CloseWorkoutDialog,

    DateChanged(String),
    RoutineChanged(String),

    SaveWorkout,
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
        Msg::ShowAddWorkoutDialog => {
            let local = Local::now().date().naive_local();
            model.dialog = Dialog::AddWorkout(Form {
                date: (local.to_string(), Some(local)),
                routine_id: (
                    String::new(),
                    data_model.routines.first().map(|routine| routine.id),
                ),
            });
        }
        Msg::ShowDeleteWorkoutDialog(id) => {
            model.dialog = Dialog::DeleteWorkout(id);
        }
        Msg::CloseWorkoutDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&model.base_url).workouts());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddWorkout(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        if data_model.workouts.iter().all(|p| p.date != parsed_date) {
                            form.date = (date, Some(parsed_date));
                        } else {
                            form.date = (date, None);
                        }
                    }
                    Err(_) => form.date = (date, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteWorkout(_) => {
                panic!();
            }
        },
        Msg::RoutineChanged(routine_id) => match model.dialog {
            Dialog::AddWorkout(ref mut form) => match routine_id.parse::<u32>() {
                Ok(parsed_routine_id) => {
                    form.routine_id = (
                        routine_id,
                        if parsed_routine_id > 0 {
                            Some(parsed_routine_id)
                        } else {
                            None
                        },
                    )
                }
                Err(_) => form.routine_id = (routine_id, None),
            },
            Dialog::Hidden | Dialog::DeleteWorkout(_) => {
                panic!();
            }
        },

        Msg::SaveWorkout => {
            model.loading = true;
            match model.dialog {
                Dialog::AddWorkout(ref mut form) => {
                    orders.notify(data::Msg::CreateWorkout(
                        form.date.1.unwrap(),
                        form.routine_id.1.unwrap(),
                    ));
                }
                Dialog::Hidden | Dialog::DeleteWorkout(_) => {
                    panic!();
                }
            };
        }
        Msg::DeleteWorkout(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteWorkout(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::WorkoutsReadOk => {
                    model.interval = common::init_interval(
                        &data_model
                            .workouts
                            .iter()
                            .map(|w| w.date)
                            .collect::<Vec<NaiveDate>>(),
                        false,
                    );
                }
                data::Event::WorkoutCreatedOk | data::Event::WorkoutDeletedOk => {
                    orders.skip().send_msg(Msg::CloseWorkoutDialog);
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
    div![
        view_workouts_dialog(&data_model.routines, &model.dialog, model.loading),
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            "workouts",
            &model.interval,
            &data_model
                .workouts
                .iter()
                .map(|w| (w.id, w.date))
                .collect::<Vec<_>>(),
        ),
        view_table(
            &data_model.workouts,
            &data_model.routines,
            &model.interval,
            &model.base_url,
            Msg::ShowDeleteWorkoutDialog
        ),
        common::view_fab(|_| Msg::ShowAddWorkoutDialog),
    ]
}

fn view_workouts_dialog(routines: &[data::Routine], dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddWorkout(ref f) => {
            title = "Add workout";
            form = f;
            date_disabled = false;
        }
        Dialog::DeleteWorkout(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "workout",
                &ev(Ev::Click, move |_| Msg::DeleteWorkout(date)),
                &ev(Ev::Click, |_| Msg::CloseWorkoutDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || form.date.1.is_none() || form.routine_id.1.is_none();
    common::view_dialog(
        "primary",
        title,
        nodes![
            div![
                C!["field"],
                label![C!["label"], "Date"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::DateChanged),
                    input![
                        C!["input"],
                        C![IF![form.date.1.is_none() => "is-danger"]],
                        attrs! {
                            At::Type => "date",
                            At::Value => form.date.0,
                            At::Disabled => date_disabled.as_at_value(),
                        }
                    ],
                ]
            ],
            div![
                C!["field"],
                label![C!["label"], "Routine"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::RoutineChanged),
                    div![
                        C!["select"],
                        select![routines
                            .iter()
                            .map(|r| {
                                option![
                                    &r.name,
                                    attrs![
                                        At::Value => r.id,
                                    ]
                                ]
                            })
                            .collect::<Vec<_>>()],
                    ],
                ],
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
                        ev(Ev::Click, |_| Msg::CloseWorkoutDialog),
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
                        ev(Ev::Click, |_| Msg::SaveWorkout),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseWorkoutDialog),
    )
}

pub fn view_table<Ms: 'static>(
    workouts: &[data::Workout],
    routines: &[data::Routine],
    interval: &common::Interval,
    base_url: &Url,
    delete_workout_message: fn(u32) -> Ms,
) -> Node<Ms> {
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
                th!["Avg. reps"],
                th!["Avg. time (s)"],
                th!["Avg. weight (kg)"],
                th!["Avg. RPE"],
                th!["Avg. reps+RIR"],
                th!["Volume"],
                th!["TUT"],
                th![]
            ]],
            tbody![workouts
                .iter()
                .rev()
                .filter(|w| w.date >= interval.first && w.date <= interval.last)
                .map(|w| {
                    #[allow(clippy::clone_on_copy)]
                    let id = w.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => crate::Urls::new(base_url).workout().add_hash_path_part(w.id.to_string()),
                            },
                            span![style! {St::WhiteSpace => "nowrap" }, w.date.to_string()]
                        ]],
                        td![
                            if let Some(routine_id) = w.routine_id {
                                a![
                                    attrs! {
                                        At::Href => crate::Urls::new(base_url).routine().add_hash_path_part(w.routine_id.unwrap().to_string()),
                                    },
                                    &routines.iter().find(|r| r.id == routine_id).unwrap().name
                                ]
                            } else {
                                plain!["-"]
                            }
                        ],
                        td![common::value_or_dash(w.avg_reps())],
                        td![common::value_or_dash(w.avg_time())],
                        td![common::value_or_dash(w.avg_weight())],
                        td![common::value_or_dash(w.avg_rpe())],
                        td![if let (Some(avg_reps), Some(avg_rpe)) = (w.avg_reps(), w.avg_rpe()) {
                            format!("{:.1}", avg_reps + 10.0 - avg_rpe)
                        } else {
                            "-".into()
                        }],
                        td![&w.volume()],
                        td![&w.tut()],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| delete_workout_message(id)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
