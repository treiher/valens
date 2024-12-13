use std::collections::BTreeMap;

use chrono::prelude::*;
use chrono::Duration;
use seed::{prelude::*, *};

use crate::{
    domain,
    ui::{self, common, data},
};

// ------ ------
//     Init
// ------ ------

pub fn init(
    mut url: Url,
    orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut ui::Navbar,
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
            Url::go_and_replace(&ui::Urls::new(&data_model.base_url).body_weight());
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
    if data_model.body_weight.is_empty() && data_model.loading_body_weight {
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
                &model.interval,
                &body_weight_interval,
                Msg::ChangeInterval
            ),
            view_chart(model, data_model),
            view_calendar(data_model, &model.interval),
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
    let today = Local::now().date_naive();
    let date_valid = form.date.1.map_or(false, |d| d <= today);
    let save_disabled = loading || !date_valid || form.weight.1.is_none();
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
    common::view_chart(
        vec![
            ("Weight (kg)", common::COLOR_BODY_WEIGHT),
            ("Avg. weight (kg)", common::COLOR_AVG_BODY_WEIGHT),
        ]
        .as_slice(),
        common::plot_chart(
            &[
                common::PlotData {
                    values: data_model
                        .body_weight
                        .values()
                        .filter(|bw| {
                            bw.date >= model.interval.first && bw.date <= model.interval.last
                        })
                        .map(|bw| (bw.date, bw.weight))
                        .collect::<Vec<_>>(),
                    plots: common::plot_line_with_dots(common::COLOR_BODY_WEIGHT),
                    params: common::PlotParams::default(),
                },
                common::PlotData {
                    values: data_model
                        .avg_body_weight
                        .values()
                        .filter(|bw| {
                            bw.date >= model.interval.first && bw.date <= model.interval.last
                        })
                        .map(|bw| (bw.date, bw.weight))
                        .collect::<Vec<_>>(),
                    plots: common::plot_line_with_dots(common::COLOR_AVG_BODY_WEIGHT),
                    params: common::PlotParams::default(),
                },
            ],
            model.interval.first,
            model.interval.last,
            data_model.theme(),
        ),
        true,
    )
}

fn view_calendar(data_model: &data::Model, interval: &domain::Interval) -> Node<Msg> {
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
                    common::COLOR_BODY_WEIGHT,
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
            tbody![data_model
                .body_weight
                .values()
                .rev()
                .filter(|bw| bw.date >= model.interval.first && bw.date <= model.interval.last)
                .map(|bw| {
                    let date = bw.date;
                    let avg_bw = data_model.avg_body_weight.get(&bw.date);
                    tr![
                        td![span![
                            style! {St::WhiteSpace => "nowrap" },
                            date.to_string(),
                        ]],
                        td![format!("{:.1}", bw.weight)],
                        td![common::value_or_dash(avg_bw.map(|bw| bw.weight))],
                        td![if let Some(value) =
                            avg_weekly_change(&data_model.avg_body_weight, avg_bw)
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
                })],
        ]
    ]
}

fn avg_weekly_change(
    avg_body_weight: &BTreeMap<NaiveDate, domain::BodyWeight>,
    current: Option<&domain::BodyWeight>,
) -> Option<f32> {
    let prev_date = current?.date - Duration::days(7);
    let prev_avg_bw = if let Some(avg_bw) = avg_body_weight.get(&prev_date) {
        avg_bw.clone()
    } else {
        let n = neighbors(avg_body_weight, prev_date);
        interpolate_avg_body_weight(n?.0, n?.1, prev_date)
    };
    Some((current?.weight - prev_avg_bw.weight) / prev_avg_bw.weight * 100.)
}

fn neighbors(
    body_weight: &BTreeMap<NaiveDate, domain::BodyWeight>,
    date: NaiveDate,
) -> Option<(&domain::BodyWeight, &domain::BodyWeight)> {
    use std::ops::Bound::{Excluded, Unbounded};

    let mut before = body_weight.range((Unbounded, Excluded(date)));
    let mut after = body_weight.range((Excluded(date), Unbounded));

    Some((
        before.next_back().map(|(_, v)| v)?,
        after.next().map(|(_, v)| v)?,
    ))
}

fn interpolate_avg_body_weight(
    a: &domain::BodyWeight,
    b: &domain::BodyWeight,
    date: NaiveDate,
) -> domain::BodyWeight {
    #[allow(clippy::cast_precision_loss)]
    domain::BodyWeight {
        date,
        weight: a.weight
            + (b.weight - a.weight)
                * ((date - a.date).num_days() as f32 / (b.date - a.date).num_days() as f32),
    }
}

// ------ ------
//     Tests
// ------ ------

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }

    #[test]
    fn test_avg_weekly_change() {
        assert_eq!(
            avg_weekly_change(
                &BTreeMap::new(),
                Some(&domain::BodyWeight {
                    date: from_num_days(1),
                    weight: 70.0
                })
            ),
            None
        );
        assert_eq!(
            avg_weekly_change(
                &BTreeMap::from([(
                    from_num_days(0),
                    domain::BodyWeight {
                        date: from_num_days(0),
                        weight: 70.0
                    }
                )]),
                Some(&domain::BodyWeight {
                    date: from_num_days(7),
                    weight: 70.0
                })
            ),
            Some(0.0)
        );
        assert_approx_eq!(
            avg_weekly_change(
                &BTreeMap::from([(
                    from_num_days(0),
                    domain::BodyWeight {
                        date: from_num_days(0),
                        weight: 70.0
                    }
                )]),
                Some(&domain::BodyWeight {
                    date: from_num_days(7),
                    weight: 70.7
                })
            )
            .unwrap(),
            1.0,
            0.001
        );
        assert_approx_eq!(
            avg_weekly_change(
                &BTreeMap::from([
                    (
                        from_num_days(0),
                        domain::BodyWeight {
                            date: from_num_days(0),
                            weight: 69.0
                        }
                    ),
                    (
                        from_num_days(2),
                        domain::BodyWeight {
                            date: from_num_days(2),
                            weight: 71.0
                        }
                    )
                ]),
                Some(&domain::BodyWeight {
                    date: from_num_days(8),
                    weight: 69.44
                })
            )
            .unwrap(),
            -0.8,
            0.001
        );
    }
}
