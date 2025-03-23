use chrono::prelude::*;
use seed::{prelude::*, *};
use valens_domain as domain;

use crate::{common, data};

// ------ ------
//     Init
// ------ ------

#[allow(clippy::needless_pass_by_value)]
pub fn init(
    _url: Url,
    _orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    navbar
        .title
        .clone_from(&data_model.session.as_ref().unwrap().name.to_string());

    Model {}
}

// ------ ------
//     Model
// ------ ------

pub struct Model {}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ChangePage(Url),
}

pub fn update(msg: Msg, _model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::ChangePage(url) => {
            orders.request_url(url);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(_model: &Model, data_model: &data::Model) -> Node<Msg> {
    let sex = data_model.session.as_ref().unwrap().sex;
    let today: NaiveDate = Local::now().date_naive();
    let body_weight_subtitle;
    let body_weight_content;
    let body_fat_subtitle;
    let body_fat_content;

    let training_subtitle =
        if data_model.training_sessions.is_empty() && data_model.loading_training_sessions {
            common::view_loading::<Msg>().to_string()
        } else if let Some(load_ratio) = &data_model.training_stats.load_ratio() {
            String::from(if *load_ratio > domain::TrainingStats::LOAD_RATIO_HIGH {
                "high load"
            } else if *load_ratio < domain::TrainingStats::LOAD_RATIO_LOW {
                "low load"
            } else {
                "optimal load"
            })
        } else {
            String::new()
        };
    let training_content = if let Some(date) = data_model
        .training_sessions
        .values()
        .map(|ts| ts.date)
        .max()
    {
        last("session", today - date)
    } else {
        String::new()
    };

    if data_model.body_weight.is_empty() && data_model.loading_body_weight {
        body_weight_subtitle = common::view_loading::<Msg>().to_string();
        body_weight_content = String::new();
    } else if let Some(body_weight) = data_model
        .body_weight
        .values()
        .max_by(|a, b| a.date.cmp(&b.date))
    {
        body_weight_subtitle = format!("{:.1} kg", body_weight.weight);
        body_weight_content = last("entry", today - body_weight.date);
    } else {
        body_weight_subtitle = String::new();
        body_weight_content = String::new();
    }

    if data_model.body_fat.is_empty() && data_model.loading_body_fat {
        body_fat_subtitle = common::view_loading::<Msg>().to_string();
        body_fat_content = String::new();
    } else if let Some(body_fat) = data_model
        .body_fat
        .values()
        .max_by(|a, b| a.date.cmp(&b.date))
    {
        body_fat_subtitle = if let Some(jp3) = body_fat.jp3(sex) {
            format!("{jp3:.1} %")
        } else {
            String::new()
        };
        body_fat_content = last("entry", today - body_fat.date);
    } else {
        body_fat_subtitle = String::new();
        body_fat_content = String::new();
    }

    let menstrual_cycle_subtitle = if data_model.period.is_empty() && data_model.loading_period {
        common::view_loading::<Msg>().to_string()
    } else if let Some(current_cycle) = &data_model.current_cycle {
        format!(
            "{} (±{}) days left",
            current_cycle.time_left.num_days(),
            current_cycle.time_left_variation.num_days(),
        )
    } else {
        String::new()
    };
    let menstrual_cycle_content = if let Some(current_cycle) = &data_model.current_cycle {
        last("cycle started", today - current_cycle.begin)
    } else {
        String::new()
    };

    div![
        view_tile(
            "Training",
            &training_subtitle,
            &training_content,
            crate::Urls::new(&data_model.base_url).training()
        ),
        view_tile(
            "Body weight",
            &body_weight_subtitle,
            &body_weight_content,
            crate::Urls::new(&data_model.base_url).body_weight()
        ),
        view_tile(
            "Body fat",
            &body_fat_subtitle,
            &body_fat_content,
            crate::Urls::new(&data_model.base_url).body_fat()
        ),
        IF![
            data_model.session.as_ref().unwrap().sex == domain::Sex::FEMALE => {
                view_tile(
                    "Menstrual cycle",
                    &menstrual_cycle_subtitle,
                    &menstrual_cycle_content,
                    crate::Urls::new(&data_model.base_url).menstrual_cycle())
            }
        ],
    ]
}

fn view_tile(title: &str, subtitle: &str, content: &str, target: Url) -> Node<Msg> {
    div![
        C!["grid"],
        C!["mx-3"],
        div![
            C!["cell"],
            a![
                C!["box"],
                {
                    let target = target.clone();
                    ev(Ev::Click, move |_| Msg::ChangePage(target))
                },
                div![
                    C!["is-flex"],
                    C!["is-justify-content-space-between"],
                    div![a![C!["title"], C!["is-size-4"], C!["has-text-link"], title]],
                    div![a![
                        C!["title"],
                        C!["is-size-4"],
                        C!["has-text-link"],
                        attrs! {
                            At::Href => target.add_hash_path_part("add"),
                        },
                        span![C!["icon"], i![C!["fas fa-plus-circle"]]]
                    ]],
                ],
                IF![
                    !subtitle.is_empty() => {
                        p![C!["subtitle"], C!["is-size-5"], raw![subtitle]]
                    }
                ],
                IF![
                    !content.is_empty() => {
                        p![C!["content"], C![IF![subtitle.is_empty() => "mt-5"]], raw![content]]
                    }
                ]
            ],
        ],
    ]
}

fn last(text: &str, duration: chrono::Duration) -> String {
    if duration.num_days() == 0 {
        return format!("Last {text} <strong>today</strong>.");
    }

    if duration.num_days() == 1 {
        return format!("Last {text} <strong>yesterday</strong>.");
    }

    format!(
        "Last {text} <strong>{} days</strong> ago.",
        duration.num_days()
    )
}
