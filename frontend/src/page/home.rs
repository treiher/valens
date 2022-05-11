use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(url: Url, orders: &mut impl Orders<Msg>, session: crate::Session) -> Model {
    let base_url = url.to_hash_base_url();

    orders
        .send_msg(Msg::FetchBodyWeight)
        .send_msg(Msg::FetchBodyFat)
        .send_msg(Msg::FetchPeriod);

    Model {
        base_url,
        session,
        body_weight: None,
        body_fat: None,
        period: None,
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    session: crate::Session,
    body_weight: Option<crate::page::body_weight::BodyWeight>,
    body_fat: Option<crate::page::body_fat::BodyFatStats>,
    period: Option<crate::page::period::Period>,
    errors: Vec<String>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    FetchBodyWeight,
    BodyWeightFetched(Result<Vec<crate::page::body_weight::BodyWeight>, String>),

    FetchBodyFat,
    BodyFatFetched(Result<Vec<crate::page::body_fat::BodyFatStats>, String>),

    FetchPeriod,
    PeriodFetched(Result<Vec<crate::page::period::Period>, String>),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::FetchBodyWeight => {
            orders.skip().perform_cmd(async {
                common::fetch("api/body_weight", Msg::BodyWeightFetched).await
            });
        }
        Msg::BodyWeightFetched(Ok(body_weight)) => {
            model.body_weight = body_weight.last().cloned();
        }
        Msg::BodyWeightFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch body weight: ".to_owned() + &message);
        }

        Msg::FetchBodyFat => {
            orders.skip().perform_cmd(async {
                common::fetch("api/body_fat?format=statistics", Msg::BodyFatFetched).await
            });
        }
        Msg::BodyFatFetched(Ok(body_fat)) => {
            model.body_fat = body_fat.last().cloned();
        }
        Msg::BodyFatFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch body fat: ".to_owned() + &message);
        }

        Msg::FetchPeriod => {
            orders
                .skip()
                .perform_cmd(async { common::fetch("api/period", Msg::PeriodFetched).await });
        }
        Msg::PeriodFetched(Ok(period)) => {
            model.period = period.last().cloned();
        }
        Msg::PeriodFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch period: ".to_owned() + &message);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    let local: NaiveDate = Local::now().date().naive_local();
    let body_weight_subtitle;
    let body_weight_content;
    let body_fat_subtitle;
    let body_fat_content;

    if let Some(body_weight) = &model.body_weight {
        body_weight_subtitle = format!("{:.1} kg", body_weight.weight);
        body_weight_content = last_update(local - body_weight.date);
    } else {
        body_weight_subtitle = String::new();
        body_weight_content = String::new();
    }

    if let Some(body_fat) = &model.body_fat {
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

    let period_content = if let Some(period) = &model.period {
        last_update(local - period.date)
    } else {
        String::new()
    };

    div![
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        view_tile(
            "Workouts",
            "",
            "",
            crate::Urls::new(&model.base_url).workouts()
        ),
        view_tile(
            "Routines",
            "",
            "",
            crate::Urls::new(&model.base_url).routines()
        ),
        view_tile(
            "Exercises",
            "",
            "",
            crate::Urls::new(&model.base_url).exercises()
        ),
        view_tile(
            "Body weight",
            &body_weight_subtitle,
            &body_weight_content,
            crate::Urls::new(&model.base_url).body_weight()
        ),
        view_tile(
            "Body fat",
            &body_fat_subtitle,
            &body_fat_content,
            crate::Urls::new(&model.base_url).body_fat()
        ),
        IF![
            model.session.sex == 0 => {
                view_tile("Period", "", &period_content, crate::Urls::new(&model.base_url).period())
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
