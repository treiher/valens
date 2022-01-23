use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    orders.send_msg(Msg::FetchBodyWeight);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddBodyWeightDialog);
    }

    let local = Local::now().date().naive_local();

    Model {
        base_url,
        interval: common::Interval {
            first: local - Duration::days(30),
            last: local,
        },
        body_weight: Vec::new(),
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
    body_weight: Vec<BodyWeightStats>,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BodyWeightStats {
    pub date: NaiveDate,
    pub weight: f32,
    pub avg_weight: Option<f32>,
    pub avg_weight_change: Option<f32>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddBodyWeightDialog,
    ShowEditBodyWeightDialog(usize),
    ShowDeleteBodyWeightDialog(NaiveDate),
    CloseBodyWeightDialog,

    DateChanged(String),
    WeightChanged(String),

    FetchBodyWeight,
    BodyWeightFetched(Result<Vec<BodyWeightStats>, String>),

    SaveBodyWeight,
    BodyWeightSaved(Result<BodyWeight, String>),

    DeleteBodyWeight(NaiveDate),
    BodyWeightDeleted(Result<(), String>),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddBodyWeightDialog => {
            let local = Local::now().date().naive_local();
            model.dialog = Dialog::AddBodyWeight(Form {
                date: (
                    local.to_string(),
                    if model.body_weight.iter().all(|bw| bw.date != local) {
                        Some(local)
                    } else {
                        None
                    },
                ),
                weight: (String::new(), None),
            });
        }
        Msg::ShowEditBodyWeightDialog(index) => {
            let date = model.body_weight[index].date;
            let weight = model.body_weight[index].weight;
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
            Url::go_and_replace(&crate::Urls::new(&model.base_url).body_weight());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddBodyWeight(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        if model.body_weight.iter().all(|bw| bw.date != parsed_date) {
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

        Msg::FetchBodyWeight => {
            orders.skip().perform_cmd(async {
                common::fetch("api/body_weight?format=statistics", Msg::BodyWeightFetched).await
            });
        }
        Msg::BodyWeightFetched(Ok(body_weight)) => {
            model.body_weight = body_weight;
        }
        Msg::BodyWeightFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch body weight: ".to_owned() + &message);
        }

        Msg::SaveBodyWeight => {
            model.loading = true;
            let request;
            match model.dialog {
                Dialog::AddBodyWeight(ref mut form) => {
                    request = Request::new("api/body_weight")
                        .method(Method::Post)
                        .json(&BodyWeight {
                            date: form.date.1.unwrap(),
                            weight: form.weight.1.unwrap(),
                        })
                        .expect("serialization failed");
                }
                Dialog::EditBodyWeight(ref mut form) => {
                    request = Request::new(format!("api/body_weight/{}", form.date.1.unwrap()))
                        .method(Method::Put)
                        .json(&json!({ "weight": form.weight.1.unwrap() }))
                        .expect("serialization failed");
                }
                Dialog::Hidden | Dialog::DeleteBodyWeight(_) => {
                    panic!();
                }
            }
            orders.perform_cmd(async move { common::fetch(request, Msg::BodyWeightSaved).await });
        }
        Msg::BodyWeightSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchBodyWeight)
                .send_msg(Msg::CloseBodyWeightDialog)
                .send_msg(Msg::ChangeInterval(
                    model.interval.first,
                    model.interval.last,
                ));
        }
        Msg::BodyWeightSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save body weight: ".to_owned() + &message);
        }

        Msg::DeleteBodyWeight(date) => {
            model.loading = true;
            let request = Request::new(format!("api/body_weight/{}", date)).method(Method::Delete);
            orders.perform_cmd(async move {
                common::fetch_no_content(request, Msg::BodyWeightDeleted).await
            });
        }
        Msg::BodyWeightDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchBodyWeight)
                .send_msg(Msg::CloseBodyWeightDialog);
        }
        Msg::BodyWeightDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete body weight: ".to_owned() + &message);
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
        view_body_weight_dialog(&model.dialog, model.loading),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_fab(|_| Msg::ShowAddBodyWeightDialog),
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            "bodyweight",
            &model.interval,
            &model
                .body_weight
                .iter()
                .map(|bw| (bw.date, bw.weight as u32))
                .collect::<Vec<_>>(),
        ),
        view_table(model),
    ]
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
                            !save_disabled && keyboard_event.key_code() == common::ENTER_KEY => {
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
                th!["Weight (kg)"],
                th!["Avg. weight (kg)"],
                th!["Avg. change (%)"],
                th![]
            ]],
            tbody![&model
                .body_weight
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, bw)| bw.date >= model.interval.first && bw.date <= model.interval.last)
                .map(|(i, bw)| {
                    #[allow(clippy::clone_on_copy)]
                    let date = bw.date.clone();
                    tr![
                        td![span![
                            style! {St::WhiteSpace => "nowrap" },
                            bw.date.to_string(),
                        ]],
                        td![format!("{:.1}", bw.weight)],
                        td![common::value_or_dash(bw.avg_weight)],
                        td![common::value_or_dash(bw.avg_weight_change)],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditBodyWeightDialog(i)),
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
