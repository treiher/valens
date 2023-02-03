use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(
    _url: Url,
    _orders: &mut impl Orders<Msg>,
    data_model: &data::Model,
    navbar: &mut crate::Navbar,
) -> Model {
    navbar.title = data_model.session.as_ref().unwrap().name.clone();

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
    let local: NaiveDate = Local::now().date_naive();
    let body_weight_subtitle;
    let body_weight_content;
    let body_fat_subtitle;
    let body_fat_content;

    if let Some(body_weight) = &data_model.body_weight.last() {
        body_weight_subtitle = format!("{:.1} kg", body_weight.weight);
        body_weight_content = last("entry", local - body_weight.date);
    } else {
        body_weight_subtitle = String::new();
        body_weight_content = String::new();
    }

    if let Some(body_fat) = &data_model.body_fat.last() {
        body_fat_subtitle = if let Some(jp3) = body_fat.jp3(sex) {
            format!("{:.1} %", jp3)
        } else {
            String::new()
        };
        body_fat_content = last("entry", local - body_fat.date);
    } else {
        body_fat_subtitle = String::new();
        body_fat_content = String::new();
    }

    let menstrual_cycle_subtitle = if let Some(current_cycle) = &data_model.current_cycle {
        format!(
            "{} (±{}) days left",
            current_cycle.time_left.num_days(),
            current_cycle.time_left_variation.num_days(),
        )
    } else {
        String::new()
    };
    let menstrual_cycle_content = if let Some(period) = &data_model.period.last() {
        last("period", local - period.date)
    } else {
        String::new()
    };

    div![
        view_tile(
            "Workouts",
            "",
            "",
            crate::Urls::new(&data_model.base_url).workouts()
        ),
        view_tile(
            "Routines",
            "",
            "",
            crate::Urls::new(&data_model.base_url).routines()
        ),
        view_tile(
            "Exercises",
            "",
            "",
            crate::Urls::new(&data_model.base_url).exercises()
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
            data_model.session.as_ref().unwrap().sex == 0 => {
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
        C!["tile"],
        C!["is-ancestor"],
        C!["is-vertical"],
        C!["mx-0"],
        div![
            C!["tile"],
            C!["is-parent"],
            div![
                C!["tile"],
                C!["is-child"],
                C!["box"],
                {
                    let target = target.clone();
                    ev(Ev::Click, move |_| Msg::ChangePage(target))
                },
                div![
                    C!["is-flex"],
                    C!["is-justify-content-space-between"],
                    div![a![
                        C!["title"],
                        C!["is-size-4"],
                        C!["has-text-link"],
                        attrs! {
                            At::Href => target,
                        },
                        title,
                    ]],
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
                        p![C!["subtitle"], C!["is-size-5"], subtitle]
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
