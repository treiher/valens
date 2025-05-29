use chrono::{Local, NaiveDate};
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
        orders.send_msg(Msg::ShowAddBodyWeightDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Body weight");

    Model {
        interval: domain::init_interval(
            &data_model
                .body_weight
                .keys()
                .copied()
                .collect::<Vec<NaiveDate>>(),
            domain::DefaultInterval::_3M,
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
    AddBodyWeight(Form),
    EditBodyWeight(Form),
    DeleteBodyWeight(NaiveDate),
}

struct Form {
    date: (String, Option<NaiveDate>),
    weight: (String, Option<f32>),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddBodyWeightDialog,
    ShowEditBodyWeightDialog(NaiveDate),
    ShowDeleteBodyWeightDialog(NaiveDate),
    CloseBodyWeightDialog,

    DateChanged(String),
    WeightChanged(String),

    SaveBodyWeight,
    DeleteBodyWeight(NaiveDate),
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
        Msg::ShowAddBodyWeightDialog => {
            let local = Local::now().date_naive();
            model.dialog = Dialog::AddBodyWeight(Form {
                date: (
                    local.to_string(),
                    if data_model.body_weight.keys().all(|date| *date != local) {
                        Some(local)
                    } else {
                        None
                    },
                ),
                weight: (String::new(), None),
            });
        }
        Msg::ShowEditBodyWeightDialog(date) => {
            let date = data_model.body_weight[&date].date;
            let weight = data_model.body_weight[&date].weight;
            model.dialog = Dialog::EditBodyWeight(Form {
                date: (date.to_string(), Some(date)),
                weight: (weight.to_string(), Some(weight)),
            });
        }
        Msg::ShowDeleteBodyWeightDialog(date) => {
            model.dialog = Dialog::DeleteBodyWeight(date);
        }
        Msg::CloseBodyWeightDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).body_weight());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddBodyWeight(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        if data_model
                            .body_weight
                            .keys()
                            .all(|date| *date != parsed_date)
                        {
                            form.date = (date, Some(parsed_date));
                        } else {
                            form.date = (date, None);
                        }
                    }
                    Err(_) => form.date = (date, None),
                }
            }
            Dialog::Hidden | Dialog::EditBodyWeight(_) | Dialog::DeleteBodyWeight(_) => {
                panic!();
            }
        },
        Msg::WeightChanged(weight) => match model.dialog {
            Dialog::AddBodyWeight(ref mut form) | Dialog::EditBodyWeight(ref mut form) => {
                match weight.parse::<f32>() {
                    Ok(parsed_weight) => {
                        form.weight = (
                            weight,
                            if parsed_weight > 0.0 {
                                Some(parsed_weight)
                            } else {
                                None
                            },
                        );
                    }
                    Err(_) => form.weight = (weight, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyWeight(_) => {
                panic!();
            }
        },

        Msg::SaveBodyWeight => {
            model.loading = true;
            match model.dialog {
                Dialog::AddBodyWeight(ref mut form) => {
                    orders.notify(data::Msg::CreateBodyWeight(domain::BodyWeight {
                        date: form.date.1.unwrap(),
                        weight: form.weight.1.unwrap(),
                    }));
                }
                Dialog::EditBodyWeight(ref mut form) => {
                    orders.notify(data::Msg::ReplaceBodyWeight(domain::BodyWeight {
                        date: form.date.1.unwrap(),
                        weight: form.weight.1.unwrap(),
                    }));
                }
                Dialog::Hidden | Dialog::DeleteBodyWeight(_) => {
                    panic!();
                }
            };
        }
        Msg::DeleteBodyWeight(date) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteBodyWeight(date));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    model.interval = domain::init_interval(
                        &data_model
                            .body_weight
                            .keys()
                            .copied()
                            .collect::<Vec<NaiveDate>>(),
                        domain::DefaultInterval::_3M,
                    );
                }
                data::Event::BodyWeightCreatedOk
                | data::Event::BodyWeightReplacedOk
                | data::Event::BodyWeightDeletedOk => {
                    orders.skip().send_msg(Msg::CloseBodyWeightDialog);
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
    if data_model.body_weight.is_empty() && data_model.loading_body_weight > 0 {
        common::view_page_loading()
    } else {
        let dates = data_model.body_weight.values().map(|bw| bw.date);
        let body_weight_interval = domain::Interval {
            first: dates.clone().min().unwrap_or_default(),
            last: dates.max().unwrap_or_default(),
        };
        div![
            view_body_weight_dialog(&model.dialog, model.loading),
            common::view_interval_buttons(
                model.interval,
                body_weight_interval,
                Msg::ChangeInterval
            ),
            view_chart(model, data_model),
            view_calendar(data_model, model.interval),
            view_table(model, data_model),
            common::view_fab("plus", |_| Msg::ShowAddBodyWeightDialog),
        ]
    }
}

fn view_body_weight_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddBodyWeight(f) => {
            title = "Add body weight";
            form = f;
            date_disabled = false;
        }
        Dialog::EditBodyWeight(f) => {
            title = "Edit body weight";
            form = f;
            date_disabled = true;
        }
        Dialog::DeleteBodyWeight(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "body weight entry",
                &span!["of ", common::no_wrap(&date.to_string())],
                &ev(Ev::Click, move |_| Msg::DeleteBodyWeight(date)),
                &ev(Ev::Click, |_| Msg::CloseBodyWeightDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let today = Local::now().date_naive();
    let date_valid = form.date.1.is_some_and(|d| d <= today);
    let save_disabled = loading || !date_valid || form.weight.1.is_none();
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
                label![C!["label"], "Weight"],
                div![
                    C!["control"],
                    C!["has-icons-right"],
                    input_ev(Ev::Input, Msg::WeightChanged),
                    keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                        IF!(
                            not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                                Msg::SaveBodyWeight
                            }
                        )
                    }),
                    input![
                        C!["input"],
                        C![IF![form.weight.1.is_none() => "is-danger"]],
                        attrs! {
                            At::from("inputmode") => "numeric",
                            At::Value => form.weight.0,
                        }
                    ],
                    span![C!["icon"], C!["is-small"], C!["is-right"], "kg"],
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
                        ev(Ev::Click, |_| Msg::CloseBodyWeightDialog),
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
                        ev(Ev::Click, |_| Msg::SaveBodyWeight),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseBodyWeightDialog),
    )
}

fn view_chart(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let avg_body_weight = data_model
        .avg_body_weight
        .values()
        .filter(|bw| bw.date >= model.interval.first && bw.date <= model.interval.last)
        .map(|bw| (bw.date, bw.weight))
        .collect::<Vec<_>>();

    common::view_chart(
        vec![
            (
                "Weight (kg)",
                web_app::chart::COLOR_BODY_WEIGHT,
                web_app::chart::OPACITY_AREA,
            ),
            (
                "Avg. weight (kg)",
                web_app::chart::COLOR_AVG_BODY_WEIGHT,
                web_app::chart::OPACITY_LINE,
            ),
        ]
        .as_slice(),
        web_app::chart::plot(
            &[
                web_app::chart::PlotData {
                    values_high: data_model
                        .body_weight
                        .values()
                        .filter(|bw| {
                            bw.date >= model.interval.first && bw.date <= model.interval.last
                        })
                        .map(|bw| (bw.date, bw.weight))
                        .collect::<Vec<_>>(),
                    values_low: Some(avg_body_weight.clone()),
                    plots: web_app::chart::plot_area(web_app::chart::COLOR_BODY_WEIGHT),
                    params: web_app::chart::PlotParams::default(),
                },
                web_app::chart::PlotData {
                    values_high: avg_body_weight,
                    values_low: None,
                    plots: web_app::chart::plot_line(web_app::chart::COLOR_AVG_BODY_WEIGHT),
                    params: web_app::chart::PlotParams::default(),
                },
            ],
            model.interval,
            data_model.theme(),
        ),
        true,
    )
}

fn view_calendar(data_model: &data::Model, interval: domain::Interval) -> Node<Msg> {
    let body_weight_values = data_model
        .body_weight
        .values()
        .filter(|bw| (interval.first..=interval.last).contains(&bw.date))
        .map(|bw| bw.weight)
        .collect::<Vec<_>>();
    let min = body_weight_values
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);
    let max = body_weight_values
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);

    common::view_calendar(
        data_model
            .body_weight
            .values()
            .filter(|bw| (interval.first..=interval.last).contains(&bw.date))
            .map(|bw| {
                (
                    bw.date,
                    web_app::chart::COLOR_BODY_WEIGHT,
                    if max > min {
                        f64::from((bw.weight - min) / (max - min)) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
            .collect(),
        interval,
    )
}

fn view_table(model: &Model, data_model: &data::Model) -> Node<Msg> {
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
                th!["Weight (kg)"],
                th!["Avg. weight (kg)"],
                th!["Avg. weekly change (%)"],
                th![]
            ]],
            tbody![
                data_model
                    .body_weight
                    .values()
                    .rev()
                    .filter(|bw| bw.date >= model.interval.first && bw.date <= model.interval.last)
                    .map(|bw| {
                        let date = bw.date;
                        let avg_bw = data_model.avg_body_weight.get(&bw.date);
                        tr![
                            td![common::no_wrap(&date.to_string())],
                            td![format!("{:.1}", bw.weight)],
                            td![common::value_or_dash(avg_bw.map(|bw| bw.weight))],
                            td![if let Some(value) =
                                domain::avg_weekly_change(&data_model.avg_body_weight, avg_bw)
                            {
                                format!("{value:+.1}")
                            } else {
                                "-".into()
                            }],
                            td![p![
                                C!["is-flex is-flex-wrap-nowrap"],
                                a![
                                    C!["icon"],
                                    C!["mr-1"],
                                    ev(Ev::Click, move |_| Msg::ShowEditBodyWeightDialog(date)),
                                    i![C!["fas fa-edit"]]
                                ],
                                a![
                                    C!["icon"],
                                    C!["ml-1"],
                                    ev(Ev::Click, move |_| Msg::ShowDeleteBodyWeightDialog(date)),
                                    i![C!["fas fa-times"]]
                                ]
                            ]]
                        ]
                    })
            ],
        ]
    ]
}
