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

pub enum Msg {}

pub fn update(_msg: Msg, _model: &mut Model, _orders: &mut impl Orders<Msg>) {}

// ------ ------
//     View
// ------ ------

pub fn view(_model: &Model, data_model: &data::Model) -> Node<Msg> {
    let local: NaiveDate = Local::now().date().naive_local();
    let body_weight_subtitle;
    let body_weight_content;
    let body_fat_subtitle;
    let body_fat_content;

    if let Some(body_weight) = &data_model.body_weight.last() {
        body_weight_subtitle = format!("{:.1} kg", body_weight.weight);
        body_weight_content = last_update(local - body_weight.date);
    } else {
        body_weight_subtitle = String::new();
        body_weight_content = String::new();
    }

    if let Some(body_fat) = &data_model.body_fat.last() {
        body_fat_subtitle = if let Some(jp3) = body_fat.jp3 {
            format!("{:.1} %", jp3)
        } else {
            String::new()
        };
        body_fat_content = last_update(local - body_fat.date);
    } else {
        body_fat_subtitle = String::new();
        body_fat_content = String::new();
    }

    let period_content = if let Some(period) = &data_model.period.last() {
        last_update(local - period.date)
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
                view_tile("Period", "", &period_content, crate::Urls::new(&data_model.base_url).period())
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

fn last_update(duration: chrono::Duration) -> String {
    if duration.num_days() == 0 {
        return String::from("Last update <strong>today</strong>.");
    }

    if duration.num_days() == 1 {
        return String::from("Last update <strong>yesterday</strong>.");
    }

    format!(
        "Last update <strong>{} days</strong> ago.",
        duration.num_days()
    )
}
