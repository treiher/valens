use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, BodyFatService, BodyWeightService, SessionService};
use valens_web_app::{self as web_app, Settings, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        element::{
            Block, Calendar, Chart, ChartLabel, DeleteConfirmationDialog, Dialog, ErrorMessage,
            FloatingActionButton, Icon, IntervalControl, LoadingPage, MenuOption, NoConnection,
            NoWrap, OptionsMenu, Table, value_or_dash,
        },
        form::{FieldSet, FieldValue, FieldValueState, InputField},
    },
    ensure_session, signal_changed_data,
};

#[component]
pub fn BodyFat(add: bool) -> Element {
    let session = ensure_session!();

    let body_weight = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_body_weight().await
    });
    let body_fat = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_body_fat().await
    });
    let dates = use_memo(move || {
        if let Some(Ok(body_fat)) = &*body_fat.read() {
            body_fat.iter().map(|bw| bw.date).collect::<Vec<_>>()
        } else {
            vec![]
        }
    });
    let current_interval =
        use_signal(|| domain::init_interval(&dates.read(), domain::DefaultInterval::_3M));
    let all = *use_memo(move || domain::Interval {
        first: dates.read().iter().min().copied().unwrap_or_default(),
        last: dates.read().iter().max().copied().unwrap_or_default(),
    })
    .read();
    let settings = use_resource(|| async { WEB_APP_SERVICE.read().get_settings().await });
    let mut dialog = use_signal(|| BodyFatDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE
            .read()
            .validate_body_fat_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        *dialog.write() = BodyFatDialog::Add {
            date,
            chest: FieldValue::from_option(None),
            abdominal: FieldValue::from_option(None),
            thigh: FieldValue::from_option(None),
            tricep: FieldValue::from_option(None),
            subscapular: FieldValue::from_option(None),
            suprailiac: FieldValue::from_option(None),
            midaxillary: FieldValue::from_option(None),
        };
        navigator().replace(Route::BodyFat { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match (&*session.read(), &*body_fat.read(), &*body_weight.read()) {
        (Some(Ok(user)), Some(Ok(body_fat)), Some(Ok(body_weight))) => {
            let avg_body_weight = DOMAIN_SERVICE.read().avg_body_weight(body_weight);
            rsx! {
                IntervalControl { current_interval, all },
                {chart(body_fat, body_weight, &avg_body_weight, user.sex, *current_interval.read(), settings)},
                {calendar(body_fat, user.sex, *current_interval.read())},
                {table(body_fat, user.sex, *current_interval.read(), dialog)},
                {view_dialog(dialog, user.sex)},
                FloatingActionButton {
                    icon: "plus".to_string(),
                    onclick: move |_| { show_add_dialog() },
                }
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _) | (_, Some(Err(err)), _) | (_, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _, _) | (_, None, _) | (_, _, None) => rsx! { LoadingPage {} },
    }
}

fn chart(
    body_fat: &[domain::BodyFat],
    body_weight: &[domain::BodyWeight],
    avg_body_weight: &[domain::BodyWeight],
    sex: domain::Sex,
    interval: domain::Interval,
    settings: Resource<Result<Settings, String>>,
) -> Element {
    let body_fat = body_fat
        .iter()
        .filter(|bf| bf.date >= interval.first && bf.date <= interval.last)
        .collect::<Vec<_>>();

    let avg_body_weight = avg_body_weight
        .iter()
        .filter(|bw| bw.date >= interval.first && bw.date <= interval.last)
        .map(|bw| (bw.date, bw.weight))
        .collect::<Vec<_>>();

    let body_weight_plot_data = web_app::chart::PlotData {
        values_high: body_weight
            .iter()
            .filter(|bw| bw.date >= interval.first && bw.date <= interval.last)
            .map(|bw| (bw.date, bw.weight))
            .collect::<Vec<_>>(),
        values_low: Some(avg_body_weight.clone()),
        plots: web_app::chart::plot_area(web_app::chart::COLOR_BODY_WEIGHT),
        params: web_app::chart::PlotParams::SECONDARY,
    };

    let avg_body_weight_plot_data = web_app::chart::PlotData {
        values_high: avg_body_weight,
        values_low: None,
        plots: web_app::chart::plot_line(web_app::chart::COLOR_AVG_BODY_WEIGHT),
        params: web_app::chart::PlotParams::SECONDARY,
    };

    let body_fat_jp7 = body_fat
        .iter()
        .filter_map(|bf| bf.jp7(sex).map(|jp7| (bf.date, jp7)))
        .collect::<Vec<_>>();

    let body_fat_jp3 = body_fat
        .iter()
        .filter_map(|bf| bf.jp3(sex).map(|jp3| (bf.date, jp3)))
        .collect::<Vec<_>>();

    rsx! {
        if !body_fat_jp3.is_empty() {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "JP3 (%)".to_string(),
                        color: web_app::chart::COLOR_BODY_FAT_JP3,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                    ChartLabel {
                        name: "Weight (kg)".to_string(),
                        color: web_app::chart::COLOR_BODY_WEIGHT,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot(
                    &[
                        body_weight_plot_data.clone(),
                        avg_body_weight_plot_data.clone(),
                        web_app::chart::PlotData {
                            values_high: body_fat_jp3,
                            values_low: None,
                            plots: web_app::chart::plot_line(web_app::chart::COLOR_BODY_FAT_JP3),
                            params: web_app::chart::PlotParams::default(),
                        },
                    ],
                    interval,
                    if let Some(Ok(settings)) = *settings.read() { settings.current_theme() } else { web_app::Theme::Light },
                ).map_err(|err| err.to_string()),
                no_data_label: true,
            }
        }
        if !body_fat_jp7.is_empty() {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "JP7 (%)".to_string(),
                        color: web_app::chart::COLOR_BODY_FAT_JP7,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                    ChartLabel {
                        name: "Weight (kg)".to_string(),
                        color: web_app::chart::COLOR_BODY_WEIGHT,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot(
                    &[
                        body_weight_plot_data.clone(),
                        avg_body_weight_plot_data.clone(),
                        web_app::chart::PlotData {
                            values_high: body_fat_jp7,
                            values_low: None,
                            plots: web_app::chart::plot_line(web_app::chart::COLOR_BODY_FAT_JP7),
                            params: web_app::chart::PlotParams::default(),
                        },
                    ],
                    interval,
                    if let Some(Ok(settings)) = *settings.read() { settings.current_theme() } else { web_app::Theme::Light },
                ).map_err(|err| err.to_string()),
                no_data_label: true,
            }
        }
    }
}

fn calendar(body_fat: &[domain::BodyFat], sex: domain::Sex, interval: domain::Interval) -> Element {
    let body_fat = body_fat
        .iter()
        .filter(|bw| (interval.first..=interval.last).contains(&bw.date))
        .collect::<Vec<_>>();
    let body_fat_values = body_fat
        .iter()
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
    let entries = body_fat
        .iter()
        .filter_map(|bf| {
            bf.jp3(sex).map(|jp3| {
                (
                    bf.date,
                    web_app::chart::COLOR_BODY_FAT_JP3,
                    if max > min {
                        f64::from((jp3 - min) / (max - min)) * 0.8 + 0.2
                    } else {
                        1.0
                    },
                )
            })
        })
        .collect();

    rsx! {
        Calendar { entries, interval }
    }
}

fn table(
    body_fat: &[domain::BodyFat],
    sex: domain::Sex,
    interval: domain::Interval,
    mut dialog: Signal<BodyFatDialog>,
) -> Element {
    let mut head = Vec::with_capacity(11);
    head.extend_from_slice(&[rsx! { "Date" }, rsx! { "JP3 (%)" }, rsx! { "JP7 (%)" }]);
    for s in match sex {
        domain::Sex::FEMALE => [
            "Tricep (mm)",
            "Suprailiac (mm)",
            "Thigh (mm)",
            "Chest (mm)",
            "Abdominal (mm)",
            "Subscapular (mm)",
            "Midaxillary (mm)",
        ],
        domain::Sex::MALE => [
            "Chest (mm)",
            "Abdominal (mm)",
            "Thigh (mm)",
            "Tricep (mm)",
            "Subscapular (mm)",
            "Suprailiac (mm)",
            "Midaxillary (mm)",
        ],
    } {
        head.push(rsx! { {s} });
    }
    head.push(rsx! {});

    let body = body_fat
        .iter()
        .rev()
        .filter(|bf| bf.date >= interval.first && bf.date <= interval.last)
        .map(|bf| {
            let bf = bf.clone();
            let date = bf.date;
            let mut row = vec![
                rsx! { NoWrap { "{date}" } },
                rsx! { {value_or_dash(bf.jp3(sex))} },
                rsx! { {value_or_dash(bf.jp7(sex))} },
            ];
            row.append(&mut match sex {
                domain::Sex::FEMALE => vec![
                    rsx! { {value_or_dash(bf.tricep)} },
                    rsx! { {value_or_dash(bf.suprailiac)} },
                    rsx! { {value_or_dash(bf.thigh)} },
                    rsx! { {value_or_dash(bf.chest)} },
                    rsx! { {value_or_dash(bf.abdominal)} },
                    rsx! { {value_or_dash(bf.subscapular)} },
                    rsx! { {value_or_dash(bf.midaxillary)} },
                ],
                domain::Sex::MALE => vec![
                    rsx! { {value_or_dash(bf.chest)} },
                    rsx! { {value_or_dash(bf.abdominal)} },
                    rsx! { {value_or_dash(bf.thigh)} },
                    rsx! { {value_or_dash(bf.tricep)} },
                    rsx! { {value_or_dash(bf.subscapular)} },
                    rsx! { {value_or_dash(bf.suprailiac)} },
                    rsx! { {value_or_dash(bf.midaxillary)} },
                ],
            });
            row.push(rsx! {
                a {
                    class: "mx-2",
                    onclick: move |_| { *dialog.write() = BodyFatDialog::Options(bf.clone()); },
                    Icon { name: "ellipsis-vertical"}
                }
            });
            row
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

fn view_dialog(mut dialog: Signal<BodyFatDialog>, sex: domain::Sex) -> Element {
    let mut is_loading = use_signal(|| false);

    let mut close_dialog = move || {
        *dialog.write() = BodyFatDialog::None;
        navigator().replace(Route::BodyFat { add: false });
    };

    macro_rules! is_loading {
        ($block:expr) => {
            *is_loading.write() = true;
            $block;
            *is_loading.write() = false;
        };
    }

    macro_rules! skinfold_name {
        (chest) => {
            "Chest"
        };
        (abdominal) => {
            "Abdominal"
        };
        (thigh) => {
            "Thigh"
        };
        (tricep) => {
            "Tricep"
        };
        (subscapular) => {
            "Subscapular"
        };
        (suprailiac) => {
            "Suprailiac"
        };
        (midaxillary) => {
            "Midaxillary"
        };
    }

    macro_rules! skinfold_description {
        (chest) => {
            "Diagonal fold midway between upper armpit and nipple"
        };
        (abdominal) => {
            "Vertical fold two centimeters to the right of belly button"
        };
        (thigh) => {
            "Vertical fold midway between knee cap and top of thigh"
        };
        (tricep) => {
            "Vertical fold midway between shoulder and elbow"
        };
        (subscapular) => {
            "Diagonal fold below shoulder blade"
        };
        (suprailiac) => {
            "Diagonal fold above crest of hipbone"
        };
        (midaxillary) => {
            "Horizontal fold below armpit"
        };
    }

    macro_rules! skinfold_input_field {
        ($name:ident, $dialog:expr) => {
            rsx! {
                InputField {
                    label: skinfold_name!($name).to_string(),
                    help: skinfold_description!($name).to_string(),
                    right_icon: rsx! { "mm" },
                    inputmode: "numeric".to_string(),
                    value: $name.input.clone(),
                    error: if let Err(err) = &$name.validated {
                        err.clone()
                    },
                    has_changed: $name.changed(),
                    oninput: move |event: FormEvent| async move {
                        match &mut *$dialog.write() {
                            BodyFatDialog::Add { $name, .. } | BodyFatDialog::Edit { $name, .. } => {
                                $name.input = event.value();
                                $name.validated = DOMAIN_SERVICE
                                    .read()
                                    .validate_body_fat_skinfold(&$name.input)
                                    .map_err(|err| err.to_string());
                                }
                            _ => {}
                        }
                    },
                }
            }
        };
    }

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                BodyFatDialog::Add {
                    date,
                    chest,
                    abdominal,
                    thigh,
                    tricep,
                    subscapular,
                    suprailiac,
                    midaxillary,
                } => {
                    if let (
                        Ok(date),
                        Ok(chest),
                        Ok(abdominal),
                        Ok(thigh),
                        Ok(tricep),
                        Ok(subscapular),
                        Ok(suprailiac),
                        Ok(midaxillary),
                    ) = (
                    date.validated.clone(),
                    chest.validated.clone(),
                    abdominal.validated.clone(),
                    thigh.validated.clone(),
                    tricep.validated.clone(),
                    subscapular.validated.clone(),
                    suprailiac.validated.clone(),
                    midaxillary.validated.clone(),
                    ) {
                        match DOMAIN_SERVICE
                            .read()
                            .create_body_fat(domain::BodyFat {
                                date,
                                chest,
                                abdominal,
                                thigh,
                                tricep,
                                subscapular,
                                suprailiac,
                                midaxillary,
                            })
                        .await
                        {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            }
                            Err(err) => {
                                NOTIFICATIONS
                                    .write()
                                    .push(format!("Failed to add body fat: {err}"));
                                }
                        }
                    }
                }
                BodyFatDialog::Edit {
                    date,
                    chest,
                    abdominal,
                    thigh,
                    tricep,
                    subscapular,
                    suprailiac,
                    midaxillary,
                } => {
                    if let (
                        Ok(date),
                        Ok(chest),
                        Ok(abdominal),
                        Ok(thigh),
                        Ok(tricep),
                        Ok(subscapular),
                        Ok(suprailiac),
                        Ok(midaxillary),
                    ) = (
                    date.validated.clone(),
                    chest.validated.clone(),
                    abdominal.validated.clone(),
                    thigh.validated.clone(),
                    tricep.validated.clone(),
                    subscapular.validated.clone(),
                    suprailiac.validated.clone(),
                    midaxillary.validated.clone(),
                    ) {
                        match DOMAIN_SERVICE
                            .read()
                            .replace_body_fat(domain::BodyFat {
                                date,
                                chest,
                                abdominal,
                                thigh,
                                tricep,
                                subscapular,
                                suprailiac,
                                midaxillary,
                            })
                        .await
                        {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            }
                            Err(err) => {
                                NOTIFICATIONS
                                    .write()
                                    .push(format!("Failed to edit body fat: {err}"));
                                }
                        }
                    }
                }
                _ => {}
            }
        }
        if saved {
            close_dialog();
        }
    };
    let delete = move |_| async move {
        let mut deleted = false;
        is_loading! {
            if let BodyFatDialog::Delete(body_fat) = &*dialog.read() {
                match DOMAIN_SERVICE.read().delete_body_fat(body_fat.date).await {
                    Ok(_) => {
                        deleted = true;
                        signal_changed_data();
                    }
                    Err(err) => NOTIFICATIONS
                        .write()
                        .push(format!("Failed to delete body fat: {err}")),
                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    match &*dialog.read() {
        BodyFatDialog::None => rsx! {},
        BodyFatDialog::Options(body_fat) => {
            let body_fat_edit = body_fat.clone();
            let body_fat_delete = body_fat.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Edit body fat".to_string(),
                                onclick: move |_| {
                                    *dialog.write() = BodyFatDialog::Edit {
                                        date: FieldValue::new(body_fat_edit.date),
                                        chest: FieldValue::from_option(body_fat_edit.chest),
                                        abdominal: FieldValue::from_option(body_fat_edit.abdominal),
                                        thigh: FieldValue::from_option(body_fat_edit.thigh),
                                        tricep: FieldValue::from_option(body_fat_edit.tricep),
                                        subscapular: FieldValue::from_option(body_fat_edit.subscapular),
                                        suprailiac: FieldValue::from_option(body_fat_edit.suprailiac),
                                        midaxillary: FieldValue::from_option(body_fat_edit.midaxillary),
                                    };
                                }
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete body fat".to_string(),
                                onclick: move |_| { *dialog.write() = BodyFatDialog::Delete(body_fat_delete.clone()); }
                            },
                        },
                    ],
                    close_event: close
                }
            }
        }
        BodyFatDialog::Add {
            date,
            chest,
            abdominal,
            thigh,
            tricep,
            subscapular,
            suprailiac,
            midaxillary,
        }
        | BodyFatDialog::Edit {
            date,
            chest,
            abdominal,
            thigh,
            tricep,
            subscapular,
            suprailiac,
            midaxillary,
            ..
        } => rsx! {
            Dialog {
                title: rsx! { if let BodyFatDialog::Add { .. } = &*dialog.read() { "Add body fat" } else { "Edit body fat" } },
                close_event: close,
                Block {
                    "Measure your body fat using a skinfold caliper."
                },
                InputField {
                    label: "Date".to_string(),
                    r#type: "date".to_string(),
                    max: Local::now().date_naive().to_string(),
                    value: date.input.clone(),
                    error: if let Err(err) = &date.validated { err.clone() },
                    has_changed: date.changed(),
                    is_disabled: if let BodyFatDialog::Edit { .. } = *dialog.read() { true },
                    oninput: move |event: FormEvent| {
                        async move {
                            match &mut *dialog.write() {
                                BodyFatDialog::Add { date, .. } | BodyFatDialog::Edit { date, .. } => {
                                    date.input = event.value();
                                    date.validated = DOMAIN_SERVICE.read().validate_body_fat_date(&date.input).await.map_err(|err| err.to_string());
                                },
                                _ => {}
                            }
                        }
                    }
                }
                FieldSet {
                    legend: "Jackson-Pollock 3".to_string(),
                    match sex {
                        domain::Sex::FEMALE => rsx! {
                            {skinfold_input_field!(tricep, dialog)},
                            {skinfold_input_field!(suprailiac, dialog)},
                            {skinfold_input_field!(thigh, dialog)},
                        },
                        domain::Sex::MALE => rsx! {
                            {skinfold_input_field!(chest, dialog)},
                            {skinfold_input_field!(abdominal, dialog)},
                            {skinfold_input_field!(thigh, dialog)},
                        }
                    }
                },
                FieldSet {
                    legend: "Additionally for Jackson-Pollock 7".to_string(),
                    match sex {
                        domain::Sex::FEMALE => rsx! {
                            {skinfold_input_field!(chest, dialog)},
                            {skinfold_input_field!(abdominal, dialog)},
                            {skinfold_input_field!(subscapular, dialog)},
                            {skinfold_input_field!(midaxillary, dialog)},
                        },
                        domain::Sex::MALE => rsx! {
                            {skinfold_input_field!(tricep, dialog)},
                            {skinfold_input_field!(subscapular, dialog)},
                            {skinfold_input_field!(suprailiac, dialog)},
                            {skinfold_input_field!(midaxillary, dialog)},
                        }
                    }
                },
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        onclick: close,
                        button { class: "button is-light is-soft", "Cancel" }
                    }
                    div {
                        class: "control",
                        onclick: save,
                        button {
                            class: "button is-primary",
                            class: if is_loading() { "is-loading" },
                            disabled: !date.valid()
                                || !FieldValue::has_valid_changes(&[chest, abdominal, thigh, tricep, subscapular, suprailiac, midaxillary])
                                || [chest, abdominal, thigh, tricep, subscapular, suprailiac, midaxillary].iter().all(|i| i.input.is_empty()),
                            "Save"
                        }
                    }
                }
            }
        },
        BodyFatDialog::Delete(body_fat) => rsx! {
            DeleteConfirmationDialog {
                element_type: "body fat".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{body_fat.date}" } } },
                delete_event: delete,
                cancel_event: close,
                is_loading: is_loading(),
            }
        },
    }
}

enum BodyFatDialog {
    None,
    Options(domain::BodyFat),
    Add {
        date: FieldValue<NaiveDate>,
        chest: FieldValue<Option<u8>>,
        abdominal: FieldValue<Option<u8>>,
        thigh: FieldValue<Option<u8>>,
        tricep: FieldValue<Option<u8>>,
        subscapular: FieldValue<Option<u8>>,
        suprailiac: FieldValue<Option<u8>>,
        midaxillary: FieldValue<Option<u8>>,
    },
    Edit {
        date: FieldValue<NaiveDate>,
        chest: FieldValue<Option<u8>>,
        abdominal: FieldValue<Option<u8>>,
        thigh: FieldValue<Option<u8>>,
        tricep: FieldValue<Option<u8>>,
        subscapular: FieldValue<Option<u8>>,
        suprailiac: FieldValue<Option<u8>>,
        midaxillary: FieldValue<Option<u8>>,
    },
    Delete(domain::BodyFat),
}
