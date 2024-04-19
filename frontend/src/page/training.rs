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
        orders.send_msg(Msg::ShowAddTrainingSessionDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Training");

    Model {
        interval: common::init_interval(
            &data_model
                .training_sessions
                .values()
                .map(|t| t.date)
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
    AddTrainingSession(Form),
    DeleteTrainingSession(u32),
}

struct Form {
    date: (String, Option<NaiveDate>),
    routine_id: (String, Option<u32>),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddTrainingSessionDialog,
    ShowDeleteTrainingSessionDialog(u32),
    CloseTrainingSessionDialog,

    DateChanged(String),
    RoutineChanged(String),

    SaveTrainingSession,
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
        Msg::ShowAddTrainingSessionDialog => {
            let local = Local::now().date_naive();
            model.dialog = Dialog::AddTrainingSession(Form {
                date: (local.to_string(), Some(local)),
                routine_id: (String::new(), None),
            });
        }
        Msg::ShowDeleteTrainingSessionDialog(id) => {
            model.dialog = Dialog::DeleteTrainingSession(id);
        }
        Msg::CloseTrainingSessionDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).training());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddTrainingSession(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        form.date = (date, Some(parsed_date));
                    }
                    Err(_) => form.date = (date, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteTrainingSession(_) => {
                panic!();
            }
        },
        Msg::RoutineChanged(routine_id) => match model.dialog {
            Dialog::AddTrainingSession(ref mut form) => match routine_id.parse::<u32>() {
                Ok(parsed_routine_id) => {
                    form.routine_id = (
                        routine_id,
                        if parsed_routine_id > 0 {
                            Some(parsed_routine_id)
                        } else {
                            None
                        },
                    );
                }
                Err(_) => form.routine_id = (routine_id, None),
            },
            Dialog::Hidden | Dialog::DeleteTrainingSession(_) => {
                panic!();
            }
        },

        Msg::SaveTrainingSession => {
            model.loading = true;
            match model.dialog {
                Dialog::AddTrainingSession(ref mut form) => {
                    let Some(date) = form.date.1 else {
                        return;
                    };
                    if let Some(routine_id) = form.routine_id.1 {
                        let Some(routine) = data_model.routines.get(&routine_id) else {
                            return;
                        };
                        let sets = routine
                            .sections
                            .iter()
                            .flat_map(to_training_session_elements)
                            .collect::<Vec<data::TrainingSessionElement>>();
                        orders.notify(data::Msg::CreateTrainingSession(
                            Some(routine_id),
                            date,
                            String::new(),
                            sets,
                        ));
                    } else {
                        orders.notify(data::Msg::CreateTrainingSession(
                            None,
                            date,
                            String::new(),
                            vec![],
                        ));
                    }
                }
                Dialog::Hidden | Dialog::DeleteTrainingSession(_) => {
                    panic!();
                }
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
                    model.interval = common::init_interval(
                        &data_model
                            .training_sessions
                            .values()
                            .map(|t| t.date)
                            .collect::<Vec<NaiveDate>>(),
                        false,
                    );
                }
                data::Event::TrainingSessionCreatedOk => {
                    if let Some((training_session_id, _)) =
                        data_model.training_sessions.last_key_value()
                    {
                        orders.request_url(
                            crate::Urls::new(&data_model.base_url)
                                .training_session()
                                .add_hash_path_part(training_session_id.to_string())
                                .add_hash_path_part("edit"),
                        );
                    }
                }
                data::Event::TrainingSessionDeletedOk => {
                    orders.skip().send_msg(Msg::CloseTrainingSessionDialog);
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
    if data_model.training_sessions.is_empty() && data_model.loading_training_sessions {
        common::view_page_loading()
    } else {
        let short_term_load = data_model
            .training_stats
            .short_term_load
            .iter()
            .filter(|(date, _)| *date >= model.interval.first && *date <= model.interval.last)
            .copied()
            .collect::<Vec<_>>();
        let long_term_load = data_model
            .training_stats
            .long_term_load
            .iter()
            .filter(|(date, _)| *date >= model.interval.first && *date <= model.interval.last)
            .copied()
            .collect::<Vec<_>>();
        let total_set_volume_per_week = data_model
            .training_stats
            .total_set_volume_per_week
            .iter()
            .filter(|(date, _)| {
                *date >= model.interval.first
                    && *date <= model.interval.last.week(Weekday::Mon).last_day()
            })
            .copied()
            .collect::<Vec<_>>();
        let avg_rpe_per_week = data_model
            .training_stats
            .avg_rpe_per_week
            .iter()
            .filter(|(date, _)| {
                *date >= model.interval.first
                    && *date <= model.interval.last.week(Weekday::Mon).last_day()
            })
            .copied()
            .collect::<Vec<_>>();
        let training_sessions = data_model
            .training_sessions
            .values()
            .filter(|t| t.date >= model.interval.first && t.date <= model.interval.last)
            .collect::<Vec<_>>();
        let dates = data_model.training_sessions.values().map(|t| t.date);
        let training_sessions_interval = common::Interval {
            first: dates.clone().min().unwrap_or_default(),
            last: dates.max().unwrap_or_default(),
        };
        div![
            view_training_sessions_dialog(
                &data_model.routines_sorted_by_last_use(),
                &model.dialog,
                model.loading
            ),
            div![
                C!["container"],
                C!["has-text-centered"],
                div![
                    C!["columns"],
                    C!["is-mobile"],
                    C!["is-gapless"],
                    C!["mx-1"],
                    C!["mb-5"],
                    div![
                        C!["column"],
                        a![
                            C!["box"],
                            C!["title"],
                            C!["is-size-5"],
                            C!["has-text-link"],
                            C!["mx-2"],
                            C!["p-3"],
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).routines(),
                            },
                            "Routines",
                        ]
                    ],
                    div![
                        C!["column"],
                        a![
                            C!["box"],
                            C!["title"],
                            C!["is-size-5"],
                            C!["has-text-link"],
                            C!["mx-2"],
                            C!["p-3"],
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).exercises(),
                            },
                            "Exercises",
                        ]
                    ]
                ]
            ],
            common::view_interval_buttons(
                &model.interval,
                &training_sessions_interval,
                Msg::ChangeInterval
            ),
            view_charts(
                short_term_load,
                long_term_load,
                total_set_volume_per_week,
                avg_rpe_per_week,
                &model.interval
            ),
            view_calendar(&training_sessions, &model.interval),
            view_table(
                &training_sessions,
                &data_model.routines,
                &data_model.base_url,
                Msg::ShowDeleteTrainingSessionDialog
            ),
            common::view_fab("plus", |_| Msg::ShowAddTrainingSessionDialog),
        ]
    }
}

pub fn view_calendar<Ms>(
    training_sessions: &[&data::TrainingSession],
    interval: &common::Interval,
) -> Node<Ms> {
    let mut load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            load.entry(training_session.date)
                .and_modify(|e| *e += training_session.load())
                .or_insert(training_session.load());
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
                        (f64::from(load - min) / f64::from(max - min)) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
            .collect(),
        interval,
    )
}

fn view_training_sessions_dialog(
    routines: &[data::Routine],
    dialog: &Dialog,
    loading: bool,
) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddTrainingSession(ref f) => {
            title = "Add training session";
            form = f;
            date_disabled = false;
        }
        Dialog::DeleteTrainingSession(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "training session",
                &ev(Ev::Click, move |_| Msg::DeleteTrainingSession(date)),
                &ev(Ev::Click, |_| Msg::CloseTrainingSessionDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || form.date.1.is_none();
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
                        select![
                            option![
                                "",
                                attrs![
                                    At::Value => 0,
                                ]
                            ],
                            routines
                                .iter()
                                .map(|r| {
                                    option![
                                        &r.name,
                                        attrs![
                                            At::Value => r.id,
                                        ]
                                    ]
                                })
                                .collect::<Vec<_>>()
                        ],
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
                        ev(Ev::Click, |_| Msg::CloseTrainingSessionDialog),
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
                        ev(Ev::Click, |_| Msg::SaveTrainingSession),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseTrainingSessionDialog),
    )
}

pub fn view_charts<Ms>(
    short_term_load: Vec<(NaiveDate, f32)>,
    long_term_load: Vec<(NaiveDate, f32)>,
    total_set_volume_per_week: Vec<(NaiveDate, f32)>,
    avg_rpe_per_week: Vec<(NaiveDate, f32)>,
    interval: &common::Interval,
) -> Vec<Node<Ms>> {
    let long_term_load_high = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * data::TrainingStats::LOAD_RATIO_HIGH))
        .collect::<Vec<_>>();
    let long_term_load_low = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * data::TrainingStats::LOAD_RATIO_LOW))
        .collect::<Vec<_>>();
    nodes![
        common::view_chart(
            &[
                ("Short-term load", common::COLOR_LOAD),
                ("Long-term load", common::COLOR_LONG_TERM_LOAD)
            ],
            common::plot_line_chart(
                &[
                    (long_term_load_low, common::COLOR_LONG_TERM_LOAD_BOUNDS),
                    (long_term_load_high, common::COLOR_LONG_TERM_LOAD_BOUNDS),
                    (long_term_load, common::COLOR_LONG_TERM_LOAD),
                    (short_term_load, common::COLOR_LOAD)
                ],
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
    training_sessions: &[&data::TrainingSession],
    routines: &BTreeMap<u32, data::Routine>,
    base_url: &Url,
    delete_training_session_message: fn(u32) -> Ms,
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
            tbody![training_sessions
                .iter()
                .rev()
                .map(|t| {
                    #[allow(clippy::clone_on_copy)]
                    let id = t.id;
                    tr![
                        td![a![
                            attrs! {
                                At::Href => crate::Urls::new(base_url).training_session().add_hash_path_part(t.id.to_string()),
                            },
                            span![style! {St::WhiteSpace => "nowrap" }, t.date.to_string()]
                        ]],
                        td![
                            if let Some(routine_id) = t.routine_id {
                                a![
                                    attrs! {
                                        At::Href => crate::Urls::new(base_url).routine().add_hash_path_part(t.routine_id.unwrap().to_string()),
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
                        td![&t.load()],
                        td![&t.set_volume()],
                        td![common::value_or_dash(t.avg_rpe())],
                        td![&t.volume_load()],
                        td![&t.tut()],
                        td![common::value_or_dash(t.avg_reps())],
                        td![if let (Some(avg_reps), Some(avg_rpe)) = (t.avg_reps(), t.avg_rpe()) {
                            format!("{:.1}", avg_reps + 10.0 - avg_rpe)
                        } else {
                            "-".into()
                        }],
                        td![common::value_or_dash(t.avg_weight())],
                        td![common::value_or_dash(t.avg_time())],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| delete_training_session_message(id)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}

fn to_training_session_elements(part: &data::RoutinePart) -> Vec<data::TrainingSessionElement> {
    let mut result = vec![];
    match part {
        data::RoutinePart::RoutineSection { rounds, parts, .. } => {
            for _ in 0..*rounds {
                for p in parts {
                    for s in to_training_session_elements(p) {
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
                data::TrainingSessionElement::Set {
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
                data::TrainingSessionElement::Rest {
                    target_time: if *time > 0 { Some(*time) } else { None },
                    automatic: *automatic,
                }
            });
        }
    }
    result
}
