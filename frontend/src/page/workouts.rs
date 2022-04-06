use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    orders.skip().send_msg(Msg::FetchWorkouts);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddWorkoutDialog);
    }

    let today = Local::today().naive_local();

    Model {
        base_url,
        interval: common::Interval {
            first: today,
            last: today,
        },
        workouts: Vec::new(),
        dialog: Dialog::Hidden,
        loading: false,
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    interval: common::Interval,
    workouts: Vec<WorkoutStats>,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
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

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Workout {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub date: NaiveDate,
    pub notes: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct WorkoutStats {
    pub id: u32,
    pub routine_id: Option<u32>,
    pub routine: String,
    pub date: NaiveDate,
    pub avg_reps: Option<f32>,
    pub avg_time: Option<f32>,
    pub avg_weight: Option<f32>,
    pub avg_rpe: Option<f32>,
    pub volume: u32,
    pub tut: u32,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddWorkoutDialog,
    ShowDeleteWorkoutDialog(u32),
    CloseWorkoutDialog,

    DateChanged(String),
    RoutineChanged(String),

    FetchWorkouts,
    WorkoutsFetched(Result<Vec<WorkoutStats>, String>),

    SaveWorkout,
    WorkoutSaved(Result<Workout, String>),

    DeleteWorkout(u32),
    WorkoutDeleted(Result<(), String>),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddWorkoutDialog => {
            let local = Local::now().date().naive_local();
            model.dialog = Dialog::AddWorkout(Form {
                date: (local.to_string(), Some(local)),
                routine_id: (
                    String::new(),
                    model.routines.first().map(|routine| routine.id),
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
                        if model.workouts.iter().all(|p| p.date != parsed_date) {
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

        Msg::FetchWorkouts => {
            orders.skip().perform_cmd(async {
                common::fetch("api/workouts?format=statistics", Msg::WorkoutsFetched).await
            });
        }
        Msg::WorkoutsFetched(Ok(workouts)) => {
            model.workouts = workouts;

            let today = Local::today().naive_local();

            if model.interval.first == today && model.interval.last == today {
                let dates = || model.workouts.iter().map(|w| w.date);

                model.interval.first = dates().min().unwrap_or(today);
                model.interval.last = dates().max().unwrap_or(today);

                if model.interval.last >= today - Duration::days(30) {
                    model.interval.first = today - Duration::days(30);
                } else {
                    model.interval.last = today;
                };
            }
        }
        Msg::WorkoutsFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch workouts: ".to_owned() + &message);
        }

        Msg::SaveWorkout => {
            model.loading = true;
            let request = match model.dialog {
                Dialog::AddWorkout(ref mut form) => Request::new("api/workouts")
                    .method(Method::Post)
                    .json(&json!({
                        "date": form.date.1.unwrap(),
                        "routine_id": form.routine_id.1.unwrap()
                    }))
                    .expect("serialization failed"),
                Dialog::Hidden | Dialog::DeleteWorkout(_) => {
                    panic!();
                }
            };
            orders.perform_cmd(async move { common::fetch(request, Msg::WorkoutSaved).await });
        }
        Msg::WorkoutSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchWorkouts)
                .send_msg(Msg::CloseWorkoutDialog)
                .send_msg(Msg::ChangeInterval(
                    model.interval.first,
                    model.interval.last,
                ));
        }
        Msg::WorkoutSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save workout: ".to_owned() + &message);
        }

        Msg::DeleteWorkout(date) => {
            model.loading = true;
            let request = Request::new(format!("api/workouts/{}", date)).method(Method::Delete);
            orders.perform_cmd(async move {
                common::fetch_no_content(request, Msg::WorkoutDeleted).await
            });
        }
        Msg::WorkoutDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchWorkouts)
                .send_msg(Msg::CloseWorkoutDialog);
        }
        Msg::WorkoutDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete workout: ".to_owned() + &message);
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

pub fn view(model: &Model) -> Node<Msg> {
    div![
        view_workouts_dialog(&model.dialog, model.loading),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            "workouts",
            &model.interval,
            &model
                .workouts
                .iter()
                .map(|w| (w.id, w.date))
                .collect::<Vec<_>>(),
        ),
        view_table(model),
        common::view_fab(|_| Msg::ShowAddWorkoutDialog),
    ]
}

fn view_workouts_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
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
                        select![option![
                            "TODO",
                            attrs![
                                At::Value => 0,
                            ]
                        ],],
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

fn view_table(model: &Model) -> Node<Msg> {
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
            tbody![&model
                .workouts
                .iter()
                .rev()
                .filter(|w| w.date >= model.interval.first && w.date <= model.interval.last)
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
