use std::collections::BTreeMap;

use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddWorkoutDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Workouts");

    Model {
        interval: common::init_interval(
            &data_model
                .workouts
                .values()
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
            let local = Local::now().date_naive();
            model.dialog = Dialog::AddWorkout(Form {
                date: (local.to_string(), Some(local)),
                routine_id: (String::new(), data_model.routines.keys().max().copied()),
            });
        }
        Msg::ShowDeleteWorkoutDialog(id) => {
            model.dialog = Dialog::DeleteWorkout(id);
        }
        Msg::CloseWorkoutDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).workouts());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddWorkout(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        form.date = (date, Some(parsed_date));
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
                    let routine = data_model
                        .routines
                        .get(&form.routine_id.1.unwrap())
                        .unwrap();
                    let sets = routine
                        .sections
                        .iter()
                        .flat_map(to_workout_elements)
                        .collect::<Vec<data::WorkoutElement>>();
                    orders.notify(data::Msg::CreateWorkout(
                        form.routine_id.1.unwrap(),
                        form.date.1.unwrap(),
                        String::new(),
                        sets,
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
                data::Event::DataChanged => {
                    model.interval = common::init_interval(
                        &data_model
                            .workouts
                            .values()
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
    if data_model.workouts.is_empty() && data_model.loading_workouts {
        common::view_loading()
    } else {
        let weighted_sum_of_load = data_model
            .workout_stats
            .weighted_sum_of_load
            .iter()
            .filter(|(date, _)| *date >= model.interval.first && *date <= model.interval.last)
            .cloned()
            .collect::<Vec<_>>();
        let total_set_volume_per_week = data_model
            .workout_stats
            .total_set_volume_per_week
            .iter()
            .filter(|(date, _)| {
                *date >= model.interval.first
                    && *date <= model.interval.last.week(Weekday::Mon).last_day()
            })
            .cloned()
            .collect::<Vec<_>>();
        let avg_rpe_per_week = data_model
            .workout_stats
            .avg_rpe_per_week
            .iter()
            .filter(|(date, _)| {
                *date >= model.interval.first
                    && *date <= model.interval.last.week(Weekday::Mon).last_day()
            })
            .cloned()
            .collect::<Vec<_>>();
        let workouts = data_model
            .workouts
            .values()
            .filter(|w| w.date >= model.interval.first && w.date <= model.interval.last)
            .collect::<Vec<_>>();
        div![
            view_workouts_dialog(
                &data_model
                    .routines
                    .values()
                    .collect::<Vec<&data::Routine>>(),
                &model.dialog,
                model.loading
            ),
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            view_charts(
                weighted_sum_of_load,
                total_set_volume_per_week,
                avg_rpe_per_week,
                &model.interval
            ),
            view_calendar(&workouts, &model.interval),
            view_table(
                &workouts,
                &data_model.routines,
                &data_model.base_url,
                Msg::ShowDeleteWorkoutDialog
            ),
            common::view_fab("plus", |_| Msg::ShowAddWorkoutDialog),
        ]
    }
}

pub fn view_calendar<Ms>(workouts: &[&data::Workout], interval: &common::Interval) -> Node<Ms> {
    let mut load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for workout in workouts {
        if (interval.first..=interval.last).contains(&workout.date) {
            load.entry(workout.date)
                .and_modify(|e| *e += workout.load())
                .or_insert(workout.load());
        }
    }
    let min = load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);

    common::view_calendar(
        load.iter()
            .map(|(date, load)| {
                (
                    *date,
                    common::COLOR_LOAD,
                    if max > min {
                        ((load - min) as f64 / (max - min) as f64) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
            .collect(),
        interval,
    )
}

fn view_workouts_dialog(routines: &[&data::Routine], dialog: &Dialog, loading: bool) -> Node<Msg> {
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
                    input_ev(Ev::Change, Msg::RoutineChanged),
                    div![
                        C!["select"],
                        select![routines
                            .iter()
                            .rev()
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

pub fn view_charts<Ms>(
    weighted_sum_of_load: Vec<(NaiveDate, f32)>,
    total_set_volume_per_week: Vec<(NaiveDate, f32)>,
    avg_rpe_per_week: Vec<(NaiveDate, f32)>,
    interval: &common::Interval,
) -> Vec<Node<Ms>> {
    nodes![
        common::view_chart(
            &[("Load (weighted sum)", common::COLOR_LOAD)],
            common::plot_line_chart(
                &[(weighted_sum_of_load, common::COLOR_LOAD)],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Set volume (weekly total)", common::COLOR_SET_VOLUME)],
            common::plot_line_chart(
                &[(total_set_volume_per_week, common::COLOR_SET_VOLUME)],
                interval.first,
                interval.last,
                Some(0.),
                None,
            )
        ),
        common::view_chart(
            &[("Intensity (weekly average RPE)", common::COLOR_INTENSITY)],
            common::plot_line_chart(
                &[(avg_rpe_per_week, common::COLOR_INTENSITY)],
                interval.first,
                interval.last,
                Some(5.),
                None,
            )
        ),
    ]
}

pub fn view_table<Ms: 'static>(
    workouts: &[&data::Workout],
    routines: &BTreeMap<u32, data::Routine>,
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
                th!["Load"],
                th!["Set volume"],
                th!["Intensity (RPE)"],
                th!["Volume load"],
                th!["TUT"],
                th!["Reps"],
                th!["Reps+RIR"],
                th!["Weight (kg)"],
                th!["Time (s)"],
                th![]
            ]],
            tbody![workouts
                .iter()
                .rev()
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
                                    match &routines.get(&routine_id) {
                                        Some(routine) => raw![&routine.name],
                                        None => vec![common::view_loading()]
                                    }
                                ]
                            } else {
                                plain!["-"]
                            }
                        ],
                        td![&w.load()],
                        td![&w.set_volume()],
                        td![common::value_or_dash(w.avg_rpe())],
                        td![&w.volume_load()],
                        td![&w.tut()],
                        td![common::value_or_dash(w.avg_reps())],
                        td![if let (Some(avg_reps), Some(avg_rpe)) = (w.avg_reps(), w.avg_rpe()) {
                            format!("{:.1}", avg_reps + 10.0 - avg_rpe)
                        } else {
                            "-".into()
                        }],
                        td![common::value_or_dash(w.avg_weight())],
                        td![common::value_or_dash(w.avg_time())],
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

fn to_workout_elements(part: &data::RoutinePart) -> Vec<data::WorkoutElement> {
    let mut result = vec![];
    match part {
        data::RoutinePart::RoutineSection { rounds, parts, .. } => {
            for _ in 0..*rounds {
                for p in parts {
                    for s in to_workout_elements(p) {
                        result.push(s);
                    }
                }
            }
        }
        data::RoutinePart::RoutineActivity {
            exercise_id,
            reps,
            time,
            weight,
            rpe,
            automatic,
        } => {
            result.push(if let Some(exercise_id) = exercise_id {
                data::WorkoutElement::WorkoutSet {
                    exercise_id: *exercise_id,
                    reps: None,
                    time: None,
                    weight: None,
                    rpe: None,
                    target_reps: if *reps > 0 { Some(*reps) } else { None },
                    target_time: if *time > 0 { Some(*time) } else { None },
                    target_weight: if *weight > 0.0 { Some(*weight) } else { None },
                    target_rpe: if *rpe > 0.0 { Some(*rpe) } else { None },
                    automatic: *automatic,
                }
            } else {
                data::WorkoutElement::WorkoutRest {
                    target_time: if *time > 0 { Some(*time) } else { None },
                    automatic: *automatic,
                }
            });
        }
    }
    result
}
