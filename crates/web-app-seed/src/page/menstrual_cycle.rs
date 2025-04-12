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
        orders.send_msg(Msg::ShowAddPeriodDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Menstrual cycle");

    Model {
        interval: domain::init_interval(
            &data_model
                .period
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
    AddPeriod(Form),
    EditPeriod(Form),
    DeletePeriod(NaiveDate),
}

struct Form {
    date: (String, Option<NaiveDate>),
    intensity: Option<domain::Intensity>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddPeriodDialog,
    ShowEditPeriodDialog(NaiveDate),
    ShowDeletePeriodDialog(NaiveDate),
    ClosePeriodDialog,

    DateChanged(String),
    IntensityChanged(domain::Intensity),

    SavePeriod,
    DeletePeriod(NaiveDate),
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
        Msg::ShowAddPeriodDialog => {
            let local = Local::now().date_naive();
            model.dialog = Dialog::AddPeriod(Form {
                date: (
                    local.to_string(),
                    if data_model.period.keys().all(|date| *date != local) {
                        Some(local)
                    } else {
                        None
                    },
                ),
                intensity: None,
            });
        }
        Msg::ShowEditPeriodDialog(date) => {
            let date = data_model.period[&date].date;
            let intensity = data_model.period[&date].intensity;
            model.dialog = Dialog::EditPeriod(Form {
                date: (date.to_string(), Some(date)),
                intensity: Some(intensity),
            });
        }
        Msg::ShowDeletePeriodDialog(date) => {
            model.dialog = Dialog::DeletePeriod(date);
        }
        Msg::ClosePeriodDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).menstrual_cycle());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddPeriod(ref mut form) => match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                Ok(parsed_date) => {
                    if data_model.period.keys().all(|date| *date != parsed_date) {
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
                form.intensity = Some(intensity);
            }
            Dialog::Hidden | Dialog::DeletePeriod(_) => {
                panic!();
            }
        },

        Msg::SavePeriod => {
            model.loading = true;
            match model.dialog {
                Dialog::AddPeriod(ref mut form) => {
                    orders.notify(data::Msg::CreatePeriod(domain::Period {
                        date: form.date.1.unwrap(),
                        intensity: form.intensity.unwrap(),
                    }));
                }
                Dialog::EditPeriod(ref mut form) => {
                    orders.notify(data::Msg::ReplacePeriod(domain::Period {
                        date: form.date.1.unwrap(),
                        intensity: form.intensity.unwrap(),
                    }));
                }
                Dialog::Hidden | Dialog::DeletePeriod(_) => {
                    panic!();
                }
            };
        }
        Msg::DeletePeriod(date) => {
            model.loading = true;
            orders.notify(data::Msg::DeletePeriod(date));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    model.interval = domain::init_interval(
                        &data_model
                            .period
                            .keys()
                            .copied()
                            .collect::<Vec<NaiveDate>>(),
                        domain::DefaultInterval::_3M,
                    );
                }
                data::Event::PeriodCreatedOk
                | data::Event::PeriodReplacedOk
                | data::Event::PeriodDeletedOk => {
                    orders.skip().send_msg(Msg::ClosePeriodDialog);
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
    if data_model.period.is_empty() && data_model.loading_period > 0 {
        common::view_page_loading()
    } else {
        let dates = data_model.period.values().map(|p| p.date);
        let period_interval = domain::Interval {
            first: dates.clone().min().unwrap_or_default(),
            last: dates.max().unwrap_or_default(),
        };
        div![
            view_period_dialog(&model.dialog, model.loading),
            view_current_cycle(data_model),
            common::view_interval_buttons(&model.interval, &period_interval, Msg::ChangeInterval),
            view_calendar(data_model, &model.interval),
            view_chart(model, data_model),
            view_cycle_stats(model, data_model),
            view_period_table(model, data_model),
            common::view_fab("plus", |_| Msg::ShowAddPeriodDialog),
        ]
    }
}

fn view_period_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddPeriod(f) => {
            title = "Add period";
            form = f;
            date_disabled = false;
        }
        Dialog::EditPeriod(f) => {
            title = "Edit period";
            form = f;
            date_disabled = true;
        }
        Dialog::DeletePeriod(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "period entry",
                &span!["of ", common::no_wrap(&date.to_string())],
                &ev(Ev::Click, move |_| Msg::DeletePeriod(date)),
                &ev(Ev::Click, |_| Msg::ClosePeriodDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let today = Local::now().date_naive();
    let date_valid = form.date.1.is_some_and(|d| d <= today);
    let save_disabled = loading || !date_valid || form.intensity.is_none();
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
                label![C!["label"], "Intensity"],
                div![
                    C!["control"],
                    domain::Intensity::iter()
                        .map(|i| {
                            button![
                                C!["button"],
                                C!["mr-2"],
                                C![IF![form.intensity == Some(*i) => "is-link"]],
                                ev(Ev::Click, |_| Msg::IntensityChanged(*i)),
                                i.to_string(),
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
                        C!["is-soft"],
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

fn view_chart(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let period = data_model
        .period
        .values()
        .filter(|p| p.date >= model.interval.first && p.date <= model.interval.last)
        .collect::<Vec<_>>();

    common::view_chart(
        vec![(
            "Intensity",
            web_app::chart::COLOR_PERIOD_INTENSITY,
            web_app::chart::OPACITY_LINE,
        )]
        .as_slice(),
        web_app::chart::plot(
            &[web_app::chart::PlotData {
                values_high: period
                    .iter()
                    .map(|p| (p.date, f32::from(p.intensity as u8)))
                    .collect::<Vec<_>>(),
                values_low: None,
                plots: vec![web_app::chart::PlotType::Histogram(
                    web_app::chart::COLOR_PERIOD_INTENSITY,
                    web_app::chart::OPACITY_LINE,
                )],
                params: web_app::chart::PlotParams::primary_range(0., 4.),
            }],
            &model.interval,
            data_model.theme(),
        ),
        true,
    )
}

fn view_current_cycle(data_model: &data::Model) -> Node<Msg> {
    let today = Local::now().date_naive();
    if let Some(current_cycle) = &data_model.current_cycle {
        common::view_box(
            "Current cycle",
            &format!(
                "<strong>{}</strong> days, <strong>{} (&#177;{})</strong> days left",
                (today - current_cycle.begin).num_days() + 1,
                current_cycle.time_left.num_days(),
                current_cycle.time_left_variation.num_days(),
            ),
        )
    } else {
        empty![]
    }
}

fn view_cycle_stats(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let cycles = &data_model
        .cycles
        .iter()
        .filter(|c| c.begin >= model.interval.first && c.begin <= model.interval.last)
        .collect::<Vec<_>>();
    let stats = domain::cycle_stats(cycles);
    common::view_box(
        "Avg. cycle length",
        &if not(cycles.is_empty()) {
            format!(
                "<strong>{} (&#177;{})</strong> days",
                stats.length_median.num_days(),
                stats.length_variation.num_days(),
            )
        } else {
            String::from("–")
        },
    )
}

fn view_calendar(data_model: &data::Model, interval: &domain::Interval) -> Node<Msg> {
    common::view_calendar(
        data_model
            .period
            .values()
            .filter(|p| (interval.first..=interval.last).contains(&p.date))
            .map(|p| {
                (
                    p.date,
                    web_app::chart::COLOR_PERIOD_INTENSITY,
                    f64::from(p.intensity as u8) * 0.25,
                )
            })
            .collect(),
        interval,
    )
}

fn view_period_table(model: &Model, data_model: &data::Model) -> Node<Msg> {
    div![
        C!["table-container"],
        C!["mt-4"],
        table![
            C!["table"],
            C!["is-fullwidth"],
            C!["is-hoverable"],
            C!["has-text-centered"],
            thead![tr![th!["Date"], th!["Intensity"], th![]]],
            tbody![
                &data_model
                    .period
                    .values()
                    .rev()
                    .filter(|p| p.date >= model.interval.first && p.date <= model.interval.last)
                    .map(|p| {
                        let date = p.date;
                        tr![
                            td![common::no_wrap(&date.to_string())],
                            td![format!("{}", p.intensity)],
                            td![p![
                                C!["is-flex is-flex-wrap-nowrap"],
                                a![
                                    C!["icon"],
                                    C!["mr-1"],
                                    ev(Ev::Click, move |_| Msg::ShowEditPeriodDialog(date)),
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
                    .collect::<Vec<_>>()
            ],
        ]
    ]
}
