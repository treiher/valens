use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    orders.send_msg(Msg::FetchPeriod);

    if url.next_hash_path_part() == Some("add") {
        orders.send_msg(Msg::ShowAddPeriodDialog);
    }

    let local = Local::now().date().naive_local();

    Model {
        base_url,
        interval: common::Interval {
            first: local - Duration::days(30),
            last: local,
        },
        period: Vec::new(),
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
    period: Vec<Period>,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
}

enum Dialog {
    Hidden,
    AddPeriod(Form),
    EditPeriod(Form),
    DeletePeriod(NaiveDate),
}

struct Form {
    date: (String, Option<NaiveDate>),
    intensity: (String, Option<u8>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Period {
    pub date: NaiveDate,
    pub intensity: u8,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddPeriodDialog,
    ShowEditPeriodDialog(usize),
    ShowDeletePeriodDialog(NaiveDate),
    ClosePeriodDialog,

    DateChanged(String),
    IntensityChanged(String),

    FetchPeriod,
    PeriodFetched(Result<Vec<Period>, String>),

    SavePeriod,
    PeriodSaved(Result<Period, String>),

    DeletePeriod(NaiveDate),
    PeriodDeleted(Result<(), String>),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddPeriodDialog => {
            let local = Local::now().date().naive_local();
            model.dialog = Dialog::AddPeriod(Form {
                date: (
                    local.to_string(),
                    if model.period.iter().all(|p| p.date != local) {
                        Some(local)
                    } else {
                        None
                    },
                ),
                intensity: (String::new(), None),
            });
        }
        Msg::ShowEditPeriodDialog(index) => {
            let date = model.period[index].date;
            let intensity = model.period[index].intensity;
            model.dialog = Dialog::EditPeriod(Form {
                date: (date.to_string(), Some(date)),
                intensity: (intensity.to_string(), Some(intensity)),
            });
        }
        Msg::ShowDeletePeriodDialog(date) => {
            model.dialog = Dialog::DeletePeriod(date);
        }
        Msg::ClosePeriodDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&model.base_url).period());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddPeriod(ref mut form) => match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                Ok(parsed_date) => {
                    if model.period.iter().all(|p| p.date != parsed_date) {
                        form.date = (date, Some(parsed_date));
                    } else {
                        form.date = (date, None);
                    }
                }
                Err(_) => form.date = (date, None),
            },
            Dialog::Hidden | Dialog::EditPeriod(_) | Dialog::DeletePeriod(_) => {
                panic!();
            }
        },
        Msg::IntensityChanged(intensity) => match model.dialog {
            Dialog::AddPeriod(ref mut form) | Dialog::EditPeriod(ref mut form) => {
                match intensity.parse::<u8>() {
                    Ok(parsed_intensity) => {
                        form.intensity = (
                            intensity,
                            if parsed_intensity > 0 {
                                Some(parsed_intensity)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.intensity = (intensity, None),
                }
            }
            Dialog::Hidden | Dialog::DeletePeriod(_) => {
                panic!();
            }
        },

        Msg::FetchPeriod => {
            orders.skip().perform_cmd(async {
                common::fetch("api/period?format=statistics", Msg::PeriodFetched).await
            });
        }
        Msg::PeriodFetched(Ok(period)) => {
            model.period = period;
        }
        Msg::PeriodFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch period: ".to_owned() + &message);
        }

        Msg::SavePeriod => {
            model.loading = true;
            let request = match model.dialog {
                Dialog::AddPeriod(ref mut form) => Request::new("api/period")
                    .method(Method::Post)
                    .json(&Period {
                        date: form.date.1.unwrap(),
                        intensity: form.intensity.1.unwrap(),
                    })
                    .expect("serialization failed"),
                Dialog::EditPeriod(ref mut form) => {
                    Request::new(format!("api/period/{}", form.date.1.unwrap()))
                        .method(Method::Put)
                        .json(&json!({ "intensity": form.intensity.1.unwrap() }))
                        .expect("serialization failed")
                }
                Dialog::Hidden | Dialog::DeletePeriod(_) => {
                    panic!();
                }
            };
            orders.perform_cmd(async move { common::fetch(request, Msg::PeriodSaved).await });
        }
        Msg::PeriodSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchPeriod)
                .send_msg(Msg::ClosePeriodDialog)
                .send_msg(Msg::ChangeInterval(
                    model.interval.first,
                    model.interval.last,
                ));
        }
        Msg::PeriodSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save period: ".to_owned() + &message);
        }

        Msg::DeletePeriod(date) => {
            model.loading = true;
            let request = Request::new(format!("api/period/{}", date)).method(Method::Delete);
            orders.perform_cmd(async move {
                common::fetch_no_content(request, Msg::PeriodDeleted).await
            });
        }
        Msg::PeriodDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchPeriod)
                .send_msg(Msg::ClosePeriodDialog);
        }
        Msg::PeriodDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete period: ".to_owned() + &message);
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
        view_period_dialog(&model.dialog, model.loading),
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_fab(|_| Msg::ShowAddPeriodDialog),
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            "period",
            &model.interval,
            &model
                .period
                .iter()
                .map(|p| (p.date, p.intensity as u32))
                .collect::<Vec<_>>(),
        ),
        view_table(model),
    ]
}

fn view_period_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddPeriod(ref f) => {
            title = "Add period";
            form = f;
            date_disabled = false;
        }
        Dialog::EditPeriod(ref f) => {
            title = "Edit period";
            form = f;
            date_disabled = true;
        }
        Dialog::DeletePeriod(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "period entry",
                &ev(Ev::Click, move |_| Msg::DeletePeriod(date)),
                &ev(Ev::Click, |_| Msg::ClosePeriodDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || form.date.1.is_none() || form.intensity.1.is_none();
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
                label![C!["label"], "Intensity"],
                div![
                    C!["control"],
                    ["1", "2", "3", "4"]
                        .iter()
                        .map(|i| {
                            button![
                                C!["button"],
                                C!["mr-2"],
                                C![IF![&form.intensity.0 == i => "is-link"]],
                                ev(Ev::Click, |_| Msg::IntensityChanged(i.to_string())),
                                i,
                            ]
                        })
                        .collect::<Vec<_>>(),
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
                        ev(Ev::Click, |_| Msg::ClosePeriodDialog),
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
                        ev(Ev::Click, |_| Msg::SavePeriod),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::ClosePeriodDialog),
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
            thead![tr![th!["Date"], th!["Intensity"], th![]]],
            tbody![&model
                .period
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, p)| p.date >= model.interval.first && p.date <= model.interval.last)
                .map(|(i, p)| {
                    #[allow(clippy::clone_on_copy)]
                    let date = p.date.clone();
                    tr![
                        td![span![
                            style! {St::WhiteSpace => "nowrap" },
                            p.date.to_string(),
                        ]],
                        td![format!("{:.1}", p.intensity)],
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditPeriodDialog(i)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeletePeriodDialog(date)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
