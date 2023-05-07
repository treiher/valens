use std::iter::zip;

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
        orders.send_msg(Msg::ShowAddBodyWeightDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Body weight");

    Model {
        interval: common::init_interval(
            &data_model
                .body_weight
                .keys()
                .cloned()
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
                        )
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
                    orders.notify(data::Msg::CreateBodyWeight(data::BodyWeight {
                        date: form.date.1.unwrap(),
                        weight: form.weight.1.unwrap(),
                    }));
                }
                Dialog::EditBodyWeight(ref mut form) => {
                    orders.notify(data::Msg::ReplaceBodyWeight(data::BodyWeight {
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
                    model.interval = common::init_interval(
                        &data_model
                            .body_weight
                            .keys()
                            .cloned()
                            .collect::<Vec<NaiveDate>>(),
                        false,
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
    if data_model.body_weight.is_empty() && data_model.loading_body_weight {
        common::view_loading()
    } else {
        div![
            view_body_weight_dialog(&model.dialog, model.loading),
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            view_chart(model, data_model),
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
        Dialog::AddBodyWeight(ref f) => {
            title = "Add body weight";
            form = f;
            date_disabled = false;
        }
        Dialog::EditBodyWeight(ref f) => {
            title = "Edit body weight";
            form = f;
            date_disabled = true;
        }
        Dialog::DeleteBodyWeight(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "body weight entry",
                &ev(Ev::Click, move |_| Msg::DeleteBodyWeight(date)),
                &ev(Ev::Click, |_| Msg::CloseBodyWeightDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || form.date.1.is_none() || form.weight.1.is_none();
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
                label![C!["label"], "Weight (kg)"],
                div![
                    C!["control"],
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
    common::view_chart(
        vec![
            ("Weight (kg)", common::COLOR_BODY_WEIGHT),
            ("Avg. weight (kg)", common::COLOR_AVG_BODY_WEIGHT),
        ]
        .as_slice(),
        common::plot_line_chart(
            &[
                (
                    data_model
                        .body_weight
                        .values()
                        .filter(|bw| {
                            bw.date >= model.interval.first && bw.date <= model.interval.last
                        })
                        .map(|bw| (bw.date, bw.weight))
                        .collect::<Vec<_>>(),
                    common::COLOR_BODY_WEIGHT,
                ),
                (
                    data_model
                        .body_weight_stats
                        .values()
                        .filter(|bws| {
                            bws.date >= model.interval.first && bws.date <= model.interval.last
                        })
                        .filter_map(|bws| bws.avg_weight.map(|avg_weight| (bws.date, avg_weight)))
                        .collect::<Vec<_>>(),
                    common::COLOR_AVG_BODY_WEIGHT,
                ),
            ],
            model.interval.first,
            model.interval.last,
            None,
            None,
        ),
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
                th![]
            ]],
            tbody![zip(
                data_model.body_weight.values(),
                data_model.body_weight_stats.values()
            )
            .rev()
            .filter(|(bw, _)| bw.date >= model.interval.first && bw.date <= model.interval.last)
            .map(|(bw, bws)| {
                let date = bw.date;
                tr![
                    td![span![
                        style! {St::WhiteSpace => "nowrap" },
                        bw.date.to_string(),
                    ]],
                    td![format!("{:.1}", bw.weight)],
                    td![common::value_or_dash(bws.avg_weight)],
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
            .collect::<Vec<_>>()],
        ]
    ]
}
