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
        orders.send_msg(Msg::ShowAddBodyFatDialog);
    }

    orders.subscribe(Msg::DataEvent);

    navbar.title = String::from("Body fat");

    Model {
        interval: common::init_interval(
            &data_model
                .body_fat
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
    AddBodyFat(Form),
    EditBodyFat(Form),
    DeleteBodyFat(NaiveDate),
}

struct Form {
    date: (String, Option<NaiveDate>),
    chest: (String, Option<u8>),
    abdominal: (String, Option<u8>),
    tigh: (String, Option<u8>),
    tricep: (String, Option<u8>),
    subscapular: (String, Option<u8>),
    suprailiac: (String, Option<u8>),
    midaxillary: (String, Option<u8>),
}

impl Form {
    fn is_valid(&self) -> bool {
        self.date.1.is_some()
            && (self.chest.1.is_some()
                || self.abdominal.1.is_some()
                || self.tigh.1.is_some()
                || self.tricep.1.is_some()
                || self.subscapular.1.is_some()
                || self.suprailiac.1.is_some()
                || self.midaxillary.1.is_some())
            && (self.chest.1.is_some() || self.chest.0.is_empty())
            && (self.abdominal.1.is_some() || self.abdominal.0.is_empty())
            && (self.tigh.1.is_some() || self.tigh.0.is_empty())
            && (self.tricep.1.is_some() || self.tricep.0.is_empty())
            && (self.subscapular.1.is_some() || self.subscapular.0.is_empty())
            && (self.suprailiac.1.is_some() || self.suprailiac.0.is_empty())
            && (self.midaxillary.1.is_some() || self.midaxillary.0.is_empty())
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddBodyFatDialog,
    ShowEditBodyFatDialog(NaiveDate),
    ShowDeleteBodyFatDialog(NaiveDate),
    CloseBodyFatDialog,

    DateChanged(String),
    ChestChanged(String),
    AbdominalChanged(String),
    TighChanged(String),
    TricepChanged(String),
    SubscapularChanged(String),
    SuprailiacChanged(String),
    MidaxillaryChanged(String),

    SaveBodyFat,
    DeleteBodyFat(NaiveDate),
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
        Msg::ShowAddBodyFatDialog => {
            let local = Local::now().date_naive();
            model.dialog = Dialog::AddBodyFat(Form {
                date: (
                    local.to_string(),
                    if data_model.body_fat.keys().all(|date| *date != local) {
                        Some(local)
                    } else {
                        None
                    },
                ),
                chest: (String::new(), None),
                abdominal: (String::new(), None),
                tigh: (String::new(), None),
                tricep: (String::new(), None),
                subscapular: (String::new(), None),
                suprailiac: (String::new(), None),
                midaxillary: (String::new(), None),
            });
        }
        Msg::ShowEditBodyFatDialog(date) => {
            let date = data_model.body_fat[&date].date;
            let chest = data_model.body_fat[&date].chest;
            let abdominal = data_model.body_fat[&date].abdominal;
            let tigh = data_model.body_fat[&date].tigh;
            let tricep = data_model.body_fat[&date].tricep;
            let subscapular = data_model.body_fat[&date].subscapular;
            let suprailiac = data_model.body_fat[&date].suprailiac;
            let midaxillary = data_model.body_fat[&date].midaxillary;
            model.dialog = Dialog::EditBodyFat(Form {
                date: (date.to_string(), Some(date)),
                chest: (
                    if let Some(chest) = chest {
                        chest.to_string()
                    } else {
                        String::new()
                    },
                    chest,
                ),
                abdominal: (
                    if let Some(abdominal) = abdominal {
                        abdominal.to_string()
                    } else {
                        String::new()
                    },
                    abdominal,
                ),
                tigh: (
                    if let Some(tigh) = tigh {
                        tigh.to_string()
                    } else {
                        String::new()
                    },
                    tigh,
                ),
                tricep: (
                    if let Some(tricep) = tricep {
                        tricep.to_string()
                    } else {
                        String::new()
                    },
                    tricep,
                ),
                subscapular: (
                    if let Some(subscapular) = subscapular {
                        subscapular.to_string()
                    } else {
                        String::new()
                    },
                    subscapular,
                ),
                suprailiac: (
                    if let Some(suprailiac) = suprailiac {
                        suprailiac.to_string()
                    } else {
                        String::new()
                    },
                    suprailiac,
                ),
                midaxillary: (
                    if let Some(midaxillary) = midaxillary {
                        midaxillary.to_string()
                    } else {
                        String::new()
                    },
                    midaxillary,
                ),
            });
        }
        Msg::ShowDeleteBodyFatDialog(date) => {
            model.dialog = Dialog::DeleteBodyFat(date);
        }
        Msg::CloseBodyFatDialog => {
            model.dialog = Dialog::Hidden;
            Url::go_and_replace(&crate::Urls::new(&data_model.base_url).body_fat());
        }

        Msg::DateChanged(date) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) => {
                match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    Ok(parsed_date) => {
                        if data_model.body_fat.keys().all(|date| *date != parsed_date) {
                            form.date = (date, Some(parsed_date));
                        } else {
                            form.date = (date, None);
                        }
                    }
                    Err(_) => form.date = (date, None),
                }
            }
            Dialog::Hidden | Dialog::EditBodyFat(_) | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::ChestChanged(chest) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match chest.parse::<u8>() {
                    Ok(parsed_chest) => {
                        form.chest = (
                            chest,
                            if parsed_chest > 0 {
                                Some(parsed_chest)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.chest = (chest, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::AbdominalChanged(abdominal) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match abdominal.parse::<u8>() {
                    Ok(parsed_abdominal) => {
                        form.abdominal = (
                            abdominal,
                            if parsed_abdominal > 0 {
                                Some(parsed_abdominal)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.abdominal = (abdominal, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::TighChanged(tigh) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match tigh.parse::<u8>() {
                    Ok(parsed_tigh) => {
                        form.tigh = (
                            tigh,
                            if parsed_tigh > 0 {
                                Some(parsed_tigh)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.tigh = (tigh, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::TricepChanged(tricep) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match tricep.parse::<u8>() {
                    Ok(parsed_tricep) => {
                        form.tricep = (
                            tricep,
                            if parsed_tricep > 0 {
                                Some(parsed_tricep)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.tricep = (tricep, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::SubscapularChanged(subscapular) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match subscapular.parse::<u8>() {
                    Ok(parsed_subscapular) => {
                        form.subscapular = (
                            subscapular,
                            if parsed_subscapular > 0 {
                                Some(parsed_subscapular)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.subscapular = (subscapular, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::SuprailiacChanged(suprailiac) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match suprailiac.parse::<u8>() {
                    Ok(parsed_suprailiac) => {
                        form.suprailiac = (
                            suprailiac,
                            if parsed_suprailiac > 0 {
                                Some(parsed_suprailiac)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.suprailiac = (suprailiac, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },
        Msg::MidaxillaryChanged(midaxillary) => match model.dialog {
            Dialog::AddBodyFat(ref mut form) | Dialog::EditBodyFat(ref mut form) => {
                match midaxillary.parse::<u8>() {
                    Ok(parsed_midaxillary) => {
                        form.midaxillary = (
                            midaxillary,
                            if parsed_midaxillary > 0 {
                                Some(parsed_midaxillary)
                            } else {
                                None
                            },
                        )
                    }
                    Err(_) => form.midaxillary = (midaxillary, None),
                }
            }
            Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                panic!();
            }
        },

        Msg::SaveBodyFat => {
            model.loading = true;
            match model.dialog {
                Dialog::AddBodyFat(ref mut form) => {
                    orders.notify(data::Msg::CreateBodyFat(data::BodyFat {
                        date: form.date.1.unwrap(),
                        chest: form.chest.1,
                        abdominal: form.abdominal.1,
                        tigh: form.tigh.1,
                        tricep: form.tricep.1,
                        subscapular: form.subscapular.1,
                        suprailiac: form.suprailiac.1,
                        midaxillary: form.midaxillary.1,
                    }));
                }
                Dialog::EditBodyFat(ref mut form) => {
                    orders.notify(data::Msg::ReplaceBodyFat(data::BodyFat {
                        date: form.date.1.unwrap(),
                        chest: form.chest.1,
                        abdominal: form.abdominal.1,
                        tigh: form.tigh.1,
                        tricep: form.tricep.1,
                        subscapular: form.subscapular.1,
                        suprailiac: form.suprailiac.1,
                        midaxillary: form.midaxillary.1,
                    }));
                }
                Dialog::Hidden | Dialog::DeleteBodyFat(_) => {
                    panic!();
                }
            };
        }
        Msg::DeleteBodyFat(date) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteBodyFat(date));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::DataChanged => {
                    model.interval = common::init_interval(
                        &data_model
                            .body_fat
                            .keys()
                            .cloned()
                            .collect::<Vec<NaiveDate>>(),
                        false,
                    );
                }
                data::Event::BodyFatCreatedOk
                | data::Event::BodyFatReplacedOk
                | data::Event::BodyFatDeletedOk => {
                    orders.skip().send_msg(Msg::CloseBodyFatDialog);
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
    if data_model.body_fat.is_empty() && data_model.loading_body_fat {
        common::view_loading()
    } else {
        div![
            view_body_fat_dialog(
                &model.dialog,
                model.loading,
                data_model.session.as_ref().unwrap().sex
            ),
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            view_chart(model, data_model),
            view_calendar(data_model, &model.interval),
            view_table(model, data_model),
            common::view_fab("plus", |_| Msg::ShowAddBodyFatDialog),
        ]
    }
}

fn view_body_fat_dialog(dialog: &Dialog, loading: bool, sex: u8) -> Node<Msg> {
    let title;
    let form;
    let date_disabled;
    match dialog {
        Dialog::AddBodyFat(ref f) => {
            title = "Add body fat";
            form = f;
            date_disabled = false;
        }
        Dialog::EditBodyFat(ref f) => {
            title = "Edit body fat";
            form = f;
            date_disabled = true;
        }
        Dialog::DeleteBodyFat(date) => {
            #[allow(clippy::clone_on_copy)]
            let date = date.clone();
            return common::view_delete_confirmation_dialog(
                "body fat entry",
                &ev(Ev::Click, move |_| Msg::DeleteBodyFat(date)),
                &ev(Ev::Click, |_| Msg::CloseBodyFatDialog),
                loading,
            );
        }
        Dialog::Hidden => {
            return empty![];
        }
    }
    let save_disabled = loading || !form.is_valid();
    common::view_dialog(
        "primary",
        title,
        nodes![form![
            attrs! {
                At::Action => "javascript:void(0);",
                At::OnKeyPress => "if (event.which == 13) return false;"
            },
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
            fieldset![
                C!["fieldset"],
                C!["mb-4"],
                legend![C!["has-text-centered"], "Jackson-Pollock 3"],
                if sex == 0 {
                    nodes![
                        view_body_fat_form_field(
                            "Tricep",
                            &form.tricep,
                            Msg::TricepChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Suprailiac",
                            &form.suprailiac,
                            Msg::SuprailiacChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Tigh",
                            &form.tigh,
                            Msg::TighChanged,
                            save_disabled
                        ),
                    ]
                } else {
                    nodes![
                        view_body_fat_form_field(
                            "Chest",
                            &form.chest,
                            Msg::ChestChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Abdominal",
                            &form.abdominal,
                            Msg::AbdominalChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Tigh",
                            &form.tigh,
                            Msg::TighChanged,
                            save_disabled
                        ),
                    ]
                }
            ],
            fieldset![
                C!["fieldset"],
                C!["mb-4"],
                legend![
                    C!["has-text-centered"],
                    "Additionally for Jackson-Pollock 7"
                ],
                if sex == 0 {
                    nodes![
                        view_body_fat_form_field(
                            "Chest",
                            &form.chest,
                            Msg::ChestChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Abdominal",
                            &form.abdominal,
                            Msg::AbdominalChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Subscapular",
                            &form.subscapular,
                            Msg::SubscapularChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Midaxillary",
                            &form.midaxillary,
                            Msg::MidaxillaryChanged,
                            save_disabled
                        ),
                    ]
                } else {
                    nodes![
                        view_body_fat_form_field(
                            "Tricep",
                            &form.tricep,
                            Msg::TricepChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Subscapular",
                            &form.subscapular,
                            Msg::SubscapularChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Suprailiac",
                            &form.suprailiac,
                            Msg::SuprailiacChanged,
                            save_disabled
                        ),
                        view_body_fat_form_field(
                            "Midaxillary",
                            &form.midaxillary,
                            Msg::MidaxillaryChanged,
                            save_disabled
                        ),
                    ]
                }
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
                        ev(Ev::Click, |_| Msg::CloseBodyFatDialog),
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
                        ev(Ev::Click, |_| Msg::SaveBodyFat),
                        "Save",
                    ]
                ],
            ],
        ]],
        &ev(Ev::Click, |_| Msg::CloseBodyFatDialog),
    )
}

fn view_body_fat_form_field(
    label: &str,
    field: &(String, Option<u8>),
    message: impl FnOnce(std::string::String) -> Msg + 'static + Clone,
    save_disabled: bool,
) -> Node<Msg> {
    div![
        C!["field"],
        label![C!["label"], format!("{} (mm)", label)],
        div![
            C!["control"],
            input_ev(Ev::Input, message),
            keyboard_ev(Ev::KeyDown, move |keyboard_event| {
                IF!(
                    not(save_disabled) && keyboard_event.key_code() == common::ENTER_KEY => {
                        Msg::SaveBodyFat
                    }
                )
            }),
            input![
                C!["input"],
                C![IF![field.1.is_none() && !field.0.is_empty() => "is-danger"]],
                attrs! {
                    At::from("inputmode") => "numeric",
                    At::Value => field.0,
                }
            ],
        ],
    ]
}

fn view_chart(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let body_fat = data_model
        .body_fat
        .values()
        .filter(|bf| bf.date >= model.interval.first && bf.date <= model.interval.last)
        .collect::<Vec<_>>();
    let body_weight = data_model
        .body_weight
        .values()
        .filter(|bw| bw.date >= model.interval.first && bw.date <= model.interval.last)
        .collect::<Vec<_>>();
    let sex = data_model.session.as_ref().unwrap().sex;

    common::view_chart(
        vec![
            ("JP3 (%)", common::COLOR_BODY_FAT_JP3),
            ("JP7 (%)", common::COLOR_BODY_FAT_JP7),
            ("Weight (kg)", common::COLOR_BODY_WEIGHT),
        ]
        .as_slice(),
        common::plot_dual_line_chart(
            &[
                (
                    body_fat
                        .iter()
                        .filter_map(|bf| bf.jp3(sex).map(|jp3| (bf.date, jp3)))
                        .collect::<Vec<_>>(),
                    common::COLOR_BODY_FAT_JP3,
                ),
                (
                    body_fat
                        .iter()
                        .filter_map(|bf| bf.jp7(sex).map(|jp7| (bf.date, jp7)))
                        .collect::<Vec<_>>(),
                    common::COLOR_BODY_FAT_JP7,
                ),
            ],
            &[(
                body_weight
                    .iter()
                    .map(|bw| (bw.date, bw.weight))
                    .collect::<Vec<_>>(),
                common::COLOR_BODY_WEIGHT,
            )],
            model.interval.first,
            model.interval.last,
        ),
    )
}

fn view_calendar(data_model: &data::Model, interval: &common::Interval) -> Node<Msg> {
    let sex = data_model.session.as_ref().unwrap().sex;
    let body_fat_values = data_model
        .body_fat
        .values()
        .filter(|bf| (interval.first..=interval.last).contains(&bf.date))
        .filter_map(|bf| bf.jp3(sex))
        .collect::<Vec<_>>();
    let min = body_fat_values
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);
    let max = body_fat_values
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);

    common::view_calendar(
        data_model
            .body_fat
            .values()
            .filter(|bf| (interval.first..=interval.last).contains(&bf.date))
            .filter_map(|bf| {
                bf.jp3(sex).map(|jp3| {
                    (
                        bf.date,
                        common::COLOR_BODY_FAT_JP3,
                        if max > min {
                            ((jp3 - min) / (max - min)) as f64 * 0.8 + 0.2
                        } else {
                            1.0
                        },
                    )
                })
            })
            .collect(),
        interval,
    )
}

fn view_table(model: &Model, data_model: &data::Model) -> Node<Msg> {
    let sex = data_model.session.as_ref().unwrap().sex;
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
                th!["JP3 (%)"],
                th!["JP7 (%)"],
                if sex == 0 {
                    nodes![
                        th!["Tricep (mm)"],
                        th!["Suprailiac (mm)"],
                        th!["Tigh (mm)"],
                        th!["Chest (mm)"],
                        th!["Abdominal (mm)"],
                        th!["Subscapular (mm)"],
                        th!["Midaxillary (mm)"],
                    ]
                } else {
                    nodes![
                        th!["Chest (mm)"],
                        th!["Abdominal (mm)"],
                        th!["Tigh (mm)"],
                        th!["Tricep (mm)"],
                        th!["Subscapular (mm)"],
                        th!["Suprailiac (mm)"],
                        th!["Midaxillary (mm)"],
                    ]
                },
                th![]
            ]],
            tbody![&data_model
                .body_fat
                .values()
                .rev()
                .filter(|bf| bf.date >= model.interval.first && bf.date <= model.interval.last)
                .map(|bf| {
                    let date = bf.date;
                    tr![
                        td![span![
                            style! {St::WhiteSpace => "nowrap" },
                            bf.date.to_string(),
                        ]],
                        td![common::value_or_dash(bf.jp3(sex))],
                        td![common::value_or_dash(bf.jp7(sex))],
                        if sex == 0 {
                            nodes![
                                td![common::value_or_dash(bf.tricep)],
                                td![common::value_or_dash(bf.suprailiac)],
                                td![common::value_or_dash(bf.tigh)],
                                td![common::value_or_dash(bf.chest)],
                                td![common::value_or_dash(bf.abdominal)],
                                td![common::value_or_dash(bf.subscapular)],
                                td![common::value_or_dash(bf.midaxillary)],
                            ]
                        } else {
                            nodes![
                                td![common::value_or_dash(bf.chest)],
                                td![common::value_or_dash(bf.abdominal)],
                                td![common::value_or_dash(bf.tigh)],
                                td![common::value_or_dash(bf.tricep)],
                                td![common::value_or_dash(bf.subscapular)],
                                td![common::value_or_dash(bf.suprailiac)],
                                td![common::value_or_dash(bf.midaxillary)],
                            ]
                        },
                        td![p![
                            C!["is-flex is-flex-wrap-nowrap"],
                            a![
                                C!["icon"],
                                C!["mr-1"],
                                ev(Ev::Click, move |_| Msg::ShowEditBodyFatDialog(date)),
                                i![C!["fas fa-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-1"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteBodyFatDialog(date)),
                                i![C!["fas fa-times"]]
                            ]
                        ]]
                    ]
                })
                .collect::<Vec<_>>()],
        ]
    ]
}
