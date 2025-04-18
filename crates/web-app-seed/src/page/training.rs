use std::collections::BTreeMap;

use chrono::prelude::*;
use seed::{prelude::*, *};
use valens_domain as domain;
use valens_web_app as web_app;

use crate::{common, data};

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
        interval: domain::init_interval(
            &data_model
                .training_sessions
                .values()
                .map(|t| t.date)
                .collect::<Vec<NaiveDate>>(),
            domain::DefaultInterval::_1M,
        ),
        dialog: Dialog::Hidden,
        loading: false,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: domain::Interval,
    dialog: Dialog,
    loading: bool,
}

enum Dialog {
    Hidden,
    AddTrainingSession(Form),
    DeleteTrainingSession(domain::TrainingSessionID),
}

struct Form {
    date: (String, Option<NaiveDate>),
    routine_id: (String, Option<domain::RoutineID>),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddTrainingSessionDialog,
    ShowDeleteTrainingSessionDialog(domain::TrainingSessionID),
    CloseTrainingSessionDialog,

    DateChanged(String),
    RoutineChanged(String),

    SaveTrainingSession,
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
            Dialog::AddTrainingSession(ref mut form) => match routine_id.parse::<u128>() {
                Ok(parsed_routine_id) => {
                    form.routine_id = (
                        routine_id,
                        if parsed_routine_id > 0 {
                            Some(parsed_routine_id.into())
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
                            .collect::<Vec<domain::TrainingSessionElement>>();
                        orders.notify(data::Msg::CreateTrainingSession(
                            routine_id,
                            date,
                            String::new(),
                            sets,
                        ));
                    } else {
                        orders.notify(data::Msg::CreateTrainingSession(
                            domain::RoutineID::nil(),
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
                    model.interval = domain::init_interval(
                        &data_model
                            .training_sessions
                            .values()
                            .map(|t| t.date)
                            .collect::<Vec<NaiveDate>>(),
                        domain::DefaultInterval::_1M,
                    );
                }
                data::Event::TrainingSessionCreatedOk => {
                    if let Some((training_session_id, _)) =
                        data_model.training_sessions.last_key_value()
                    {
                        orders.request_url(
                            crate::Urls::new(&data_model.base_url)
                                .training_session()
                                .add_hash_path_part(training_session_id.as_u128().to_string())
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
    if data_model.training_sessions.is_empty() && data_model.loading_training_sessions > 0 {
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
        #[allow(clippy::cast_precision_loss)]
        let total_7day_set_volume = domain::centered_moving_total(
            &data_model
                .training_sessions
                .values()
                .map(|s| (s.date, s.set_volume() as f32))
                .collect::<Vec<_>>(),
            &model.interval,
            3,
        );

        let average_7day_rpe = domain::centered_moving_average(
            &data_model
                .training_sessions
                .values()
                .filter_map(|s| s.avg_rpe().map(|v| (s.date, v)))
                .collect::<Vec<_>>(),
            &model.interval,
            3,
        );
        let mut training_sessions = data_model
            .training_sessions
            .values()
            .filter(|t| t.date >= model.interval.first && t.date <= model.interval.last)
            .collect::<Vec<_>>();
        training_sessions.sort_by_key(|t| t.date);
        let training_sessions_interval: domain::Interval =
            data_model.training_sessions_date_range().into();
        div![
            view_training_sessions_dialog(&model.dialog, model.loading, data_model),
            div![
                C!["fixed-grid"],
                C!["has-3-cols"],
                C!["has-text-centered"],
                C!["px-3"],
                C!["mb-5"],
                div![
                    C!["grid"],
                    div![
                        C!["cell"],
                        a![
                            C!["box"],
                            C!["title"],
                            C!["is-size-5"],
                            C!["has-text-link"],
                            C!["p-3"],
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).routines(),
                            },
                            "Routines",
                        ]
                    ],
                    div![
                        C!["cell"],
                        a![
                            C!["box"],
                            C!["title"],
                            C!["is-size-5"],
                            C!["has-text-link"],
                            C!["p-3"],
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).exercises(),
                            },
                            "Exercises",
                        ]
                    ],
                    div![
                        C!["cell"],
                        a![
                            C!["box"],
                            C!["title"],
                            C!["is-size-5"],
                            C!["has-text-link"],
                            C!["p-3"],
                            attrs! {
                                At::Href => crate::Urls::new(&data_model.base_url).muscles(),
                            },
                            "Muscles",
                        ]
                    ],
                ]
            ],
            common::view_interval_buttons(
                &model.interval,
                &training_sessions_interval,
                Msg::ChangeInterval
            ),
            view_charts(
                short_term_load,
                &long_term_load,
                total_7day_set_volume,
                &average_7day_rpe,
                &model.interval,
                data_model.theme(),
                data_model.settings.show_rpe,
            ),
            view_calendar(&training_sessions, &model.interval),
            view_table(
                &training_sessions,
                &data_model.routines,
                &data_model.base_url,
                Msg::ShowDeleteTrainingSessionDialog,
                data_model.settings.show_rpe,
                data_model.settings.show_tut,
            ),
            common::view_fab("plus", |_| Msg::ShowAddTrainingSessionDialog),
        ]
    }
}

pub fn view_calendar<Ms>(
    training_sessions: &[&domain::TrainingSession],
    interval: &domain::Interval,
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
                    web_app::chart::COLOR_LOAD,
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
    dialog: &Dialog,
    loading: bool,
    data_model: &data::Model,
) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddTrainingSession(f) => {
            title = "Add training session";
            form = f;
            date_disabled = false;
        }
        Dialog::DeleteTrainingSession(id) => {
            #[allow(clippy::clone_on_copy)]
            let id = id.clone();
            let date = data_model
                .training_sessions
                .get(&id)
                .map(|t| t.date)
                .unwrap_or_default();
            return common::view_delete_confirmation_dialog(
                "training session",
                &span!["of ", common::no_wrap(&date.to_string())],
                &ev(Ev::Click, move |_| Msg::DeleteTrainingSession(id)),
                &ev(Ev::Click, |_| Msg::CloseTrainingSessionDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let today = Local::now().date_naive();
    let date_valid = form.date.1.is_some_and(|d| d <= today);
    let save_disabled = loading || !date_valid;
    common::view_dialog(
        "primary",
        span![title],
        nodes![
            div![
                C!["field"],
                label![C!["label"], "Date"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::DateChanged),
                    input![
                        C!["input"],
                        C![IF![!date_valid => "is-danger"]],
                        attrs! {
                            At::Type => "date",
                            At::Value => form.date.0,
                            At::Disabled => date_disabled.as_at_value(),
                            At::Max => today,
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
                            &data_model
                                .routines_sorted_by_last_use(|r: &domain::Routine| !r.archived)
                                .iter()
                                .map(|r| {
                                    option![
                                        &r.name.as_ref(),
                                        attrs![
                                            At::Value => r.id.as_u128(),
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
                        C!["is-soft"],
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
    long_term_load: &[(NaiveDate, f32)],
    total_7day_set_volume: Vec<(NaiveDate, f32)>,
    average_7day_rpe: &[Vec<(NaiveDate, f32)>],
    interval: &domain::Interval,
    theme: &web_app::Theme,
    show_rpe: bool,
) -> Vec<Node<Ms>> {
    let long_term_load_high = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * domain::TrainingStats::LOAD_RATIO_HIGH))
        .collect::<Vec<_>>();
    let long_term_load_low = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * domain::TrainingStats::LOAD_RATIO_LOW))
        .collect::<Vec<_>>();
    nodes![
        common::view_chart(
            &[
                (
                    "Short-term load",
                    web_app::chart::COLOR_LOAD,
                    web_app::chart::OPACITY_LINE
                ),
                (
                    "Long-term load",
                    web_app::chart::COLOR_LONG_TERM_LOAD,
                    web_app::chart::OPACITY_AREA
                )
            ],
            web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: long_term_load_high,
                        values_low: Some(long_term_load_low),
                        plots: web_app::chart::plot_area(web_app::chart::COLOR_LONG_TERM_LOAD),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    },
                    web_app::chart::PlotData {
                        values_high: short_term_load,
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_LOAD),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }
                ],
                interval,
                theme,
            ),
            false,
        ),
        common::view_chart(
            &[(
                "Set volume (7 day total)",
                web_app::chart::COLOR_SET_VOLUME,
                web_app::chart::OPACITY_LINE
            )],
            web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: total_7day_set_volume,
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
                &[("RPE (7 day average)", web_app::chart::COLOR_RPE, web_app::chart::OPACITY_LINE)],
                web_app::chart::plot(
                    &average_7day_rpe.iter().map(|values| web_app::chart::PlotData{values_high: values.clone(),
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_RPE),
                        params: web_app::chart::PlotParams::primary_range(5., 10.)
                    }).collect::<Vec<_>>(),
                    interval,
                    theme,
                ),
                false,
            )
        ],
    ]
}

pub fn view_table<Ms: 'static>(
    training_sessions: &[&domain::TrainingSession],
    routines: &BTreeMap<domain::RoutineID, domain::Routine>,
    base_url: &Url,
    delete_training_session_message: fn(domain::TrainingSessionID) -> Ms,
    show_rpe: bool,
    show_tut: bool,
) -> Node<Ms> {
    let (has_avg_rpe_data, has_tut_data, has_avg_reps_data, has_avg_weight_data, has_avg_time_data) =
        training_sessions
            .iter()
            .fold((false, false, false, false, false), |r, t| {
                (
                    r.0 || t.avg_rpe().is_some(),
                    r.1 || t.tut().is_some(),
                    r.2 || t.avg_reps().is_some(),
                    r.3 || t.avg_weight().is_some(),
                    r.4 || t.avg_time().is_some(),
                )
            });

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
                IF![show_rpe && has_avg_rpe_data => th!["RPE"]],
                th!["Volume load"],
                IF![show_tut && has_tut_data => th!["TUT"]],
                IF![has_avg_reps_data => th!["Reps"]],
                IF![show_rpe && has_avg_reps_data && has_avg_rpe_data => th!["Reps+RIR"]],
                IF![has_avg_weight_data => th!["Weight (kg)"]],
                IF![show_tut && has_avg_time_data => th!["Time (s)"]],
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
                                At::Href => crate::Urls::new(base_url).training_session().add_hash_path_part(t.id.as_u128().to_string()),
                            },
                            common::no_wrap(&t.date.to_string())
                        ]],
                        td![
                            if t.routine_id.is_nil() {
                                plain!["-"]
                            } else {
                                a![
                                    attrs! {
                                        At::Href => crate::Urls::new(base_url).routine().add_hash_path_part(t.routine_id.as_u128().to_string()),
                                    },
                                    match &routines.get(&t.routine_id) {
                                        Some(routine) => raw![&routine.name.as_ref()],
                                        None => vec![common::view_loading()]
                                    }
                                ]
                            }
                        ],
                        td![&t.load()],
                        td![t.set_volume()],
                        IF![show_rpe && has_avg_rpe_data => td![common::value_or_dash(t.avg_rpe())]],
                        td![&t.volume_load()],
                        IF![show_tut && has_tut_data => td![common::value_or_dash(t.tut())]],
                        IF![has_avg_reps_data => td![common::value_or_dash(t.avg_reps())]],
                        IF![show_rpe && has_avg_reps_data && has_avg_rpe_data =>
                            td![if let (Some(avg_reps), Some(avg_rpe)) = (t.avg_reps(), t.avg_rpe()) {
                                format!("{:.1}", avg_reps + f32::from(domain::RIR::from(avg_rpe)))
                            } else {
                                "-".into()
                            }]],
                        IF![has_avg_weight_data => td![common::value_or_dash(t.avg_weight())]],
                        IF![show_tut && has_avg_time_data => td![common::value_or_dash(t.avg_time())]],
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

fn to_training_session_elements(part: &domain::RoutinePart) -> Vec<domain::TrainingSessionElement> {
    let mut result = vec![];
    match part {
        domain::RoutinePart::RoutineSection { rounds, parts, .. } => {
            for _ in 0..*rounds {
                for p in parts {
                    for s in to_training_session_elements(p) {
                        result.push(s);
                    }
                }
            }
        }
        domain::RoutinePart::RoutineActivity {
            exercise_id,
            reps,
            time,
            weight,
            rpe,
            automatic,
        } => {
            result.push(if exercise_id.is_nil() {
                domain::TrainingSessionElement::Rest {
                    target_time: if *time > domain::Time::default() {
                        Some(*time)
                    } else {
                        None
                    },
                    automatic: *automatic,
                }
            } else {
                domain::TrainingSessionElement::Set {
                    exercise_id: *exercise_id,
                    reps: None,
                    time: None,
                    weight: None,
                    rpe: None,
                    target_reps: if *reps > domain::Reps::default() {
                        Some(*reps)
                    } else {
                        None
                    },
                    target_time: if *time > domain::Time::default() {
                        Some(*time)
                    } else {
                        None
                    },
                    target_weight: if *weight > domain::Weight::default() {
                        Some(*weight)
                    } else {
                        None
                    },
                    target_rpe: if *rpe > domain::RPE::ZERO {
                        Some(*rpe)
                    } else {
                        None
                    },
                    automatic: *automatic,
                }
            });
        }
    }
    result
}
