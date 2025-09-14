use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService, SessionService};
use valens_web_app::{self as web_app, Settings, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        element::{
            Calendar, Chart, ChartLabel, DeleteConfirmationDialog, Dialog, ErrorMessage,
            FloatingActionButton, Icon, IntervalControl, LoadingPage, MenuOption, NoConnection,
            NoWrap, OptionsMenu, Table, value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
    ensure_session, signal_changed_data,
};

#[component]
pub fn BodyWeight(add: bool) -> Element {
    ensure_session!();

    let body_weight = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_body_weight().await
    });
    let dates = use_memo(move || {
        if let Some(Ok(body_weight)) = &*body_weight.read() {
            body_weight.iter().map(|bw| bw.date).collect::<Vec<_>>()
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
    let mut dialog = use_signal(|| BodyWeightDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE
            .read()
            .validate_body_weight_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        *dialog.write() = BodyWeightDialog::Add {
            date,
            weight: FieldValue::default(),
        };
        navigator().replace(Route::BodyWeight { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match &*body_weight.read() {
        Some(Ok(body_weight)) => {
            let avg_body_weight = DOMAIN_SERVICE.read().avg_body_weight(body_weight);
            rsx! {
                IntervalControl { current_interval, all }
                {chart(body_weight, &avg_body_weight, *current_interval.read(), settings)}
                {calendar(body_weight, *current_interval.read())}
                {table(body_weight, &avg_body_weight, *current_interval.read(), dialog)}
                {view_dialog(dialog)}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    onclick: move |_| { show_add_dialog() },
                }
            }
        }
        Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
            rsx! {
                NoConnection {}
                {}
            }
        }
        Some(Err(err)) => rsx! {
            ErrorMessage { message: err }
        },
        None => rsx! {
            LoadingPage {}
        },
    }
}

fn chart(
    body_weight: &[domain::BodyWeight],
    avg_body_weight: &[domain::BodyWeight],
    interval: domain::Interval,
    settings: Resource<Result<Settings, String>>,
) -> Element {
    let avg_body_weight_chart = avg_body_weight
        .iter()
        .filter(|bw| bw.date >= interval.first && bw.date <= interval.last)
        .map(|bw| (bw.date, bw.weight))
        .collect::<Vec<_>>();
    rsx! {
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Weight (kg)".to_string(),
                    color: web_app::chart::COLOR_BODY_WEIGHT,
                    opacity: web_app::chart::OPACITY_AREA,
                },
                ChartLabel {
                    name: "Avg. weight (kg)".to_string(),
                    color: web_app::chart::COLOR_AVG_BODY_WEIGHT,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: body_weight
                            .iter()
                            .filter(|bw| {
                                bw.date >= interval.first && bw.date <= interval.last
                            })
                        .map(|bw| (bw.date, bw.weight))
                            .collect::<Vec<_>>(),
                            values_low: Some(avg_body_weight_chart.clone()),
                            plots: web_app::chart::plot_area(web_app::chart::COLOR_BODY_WEIGHT),
                            params: web_app::chart::PlotParams::default(),
                    },
                    web_app::chart::PlotData {
                        values_high: avg_body_weight_chart,
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_AVG_BODY_WEIGHT),
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

fn calendar(body_weight: &[domain::BodyWeight], interval: domain::Interval) -> Element {
    let body_weight = body_weight
        .iter()
        .filter(|bw| (interval.first..=interval.last).contains(&bw.date))
        .collect::<Vec<_>>();
    let body_weight_values = body_weight.iter().map(|bw| bw.weight).collect::<Vec<_>>();
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
    let entries = body_weight
        .iter()
        .map(|bw| {
            (
                bw.date,
                web_app::chart::COLOR_BODY_WEIGHT,
                if max > min {
                    f64::from((bw.weight - min) / (max - min)) * 0.8 + 0.2
                } else {
                    1.0
                },
            )
        })
        .collect();

    rsx! {
        Calendar { entries, interval }
    }
}

fn table(
    body_weight: &[domain::BodyWeight],
    avg_body_weight: &[domain::BodyWeight],
    interval: domain::Interval,
    mut dialog: Signal<BodyWeightDialog>,
) -> Element {
    let head = vec![
        rsx! {"Date"},
        rsx! {"Weight (kg)"},
        rsx! {"Avg. weight (kg)"},
        rsx! {"Avg. weekly change (%)"},
        rsx! {},
    ];

    let body = body_weight
        .iter()
        .rev()
        .filter(|bw| bw.date >= interval.first && bw.date <= interval.last)
        .map(|bw| {
            let bw = bw.clone();
            let date = bw.date;
            let avg_bw = avg_body_weight.iter().find(|avg_bw| avg_bw.date == bw.date);
            vec![
                rsx! { NoWrap { "{date}" } },
                rsx! { "{bw.weight:.1}" },
                rsx! { {value_or_dash(avg_bw.map(|bw| bw.weight))} },
                rsx! {
                    if let Some(value) = DOMAIN_SERVICE.read().avg_weekly_change(avg_body_weight, avg_bw) {
                        "{value:+.1}"
                    } else {
                        "-"
                    }
                },
                rsx! {
                    a {
                        class: "mx-2",
                        onclick: move |_| { *dialog.write() = BodyWeightDialog::Options(bw.clone()); },
                        Icon { name: "ellipsis-vertical"}
                    }
                }
            ]
        }).collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

fn view_dialog(mut dialog: Signal<BodyWeightDialog>) -> Element {
    let mut is_loading = use_signal(|| false);

    let mut close_dialog = move || {
        *dialog.write() = BodyWeightDialog::None;
        navigator().replace(Route::BodyWeight { add: false });
    };

    macro_rules! is_loading {
        ($block:expr) => {
            *is_loading.write() = true;
            $block;
            *is_loading.write() = false;
        };
    }

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                BodyWeightDialog::Add { date, weight } => {
                    if let (Ok(date), Ok(weight)) = (date.validated.clone(), weight.validated.clone()) {
                        match DOMAIN_SERVICE
                            .read()
                            .create_body_weight(domain::BodyWeight { date, weight })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    signal_changed_data();
                                }
                                Err(err) => {
                                    NOTIFICATIONS
                                        .write()
                                        .push(format!("Failed to add body weight: {err}"));
                                    }
                            }
                    }
                }
                BodyWeightDialog::Edit { date, weight } => {
                    if let (Ok(date), Ok(weight)) = (date.validated.clone(), weight.validated.clone()) {
                        match DOMAIN_SERVICE
                            .read()
                            .replace_body_weight(domain::BodyWeight { date, weight })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    signal_changed_data();
                                }
                                Err(err) => {
                                    NOTIFICATIONS
                                        .write()
                                        .push(format!("Failed to edit body weight: {err}"));
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
            if let BodyWeightDialog::Delete(body_weight) = &*dialog.read() {
                match DOMAIN_SERVICE.read().delete_body_weight(body_weight.date).await {
                    Ok(_) => {
                        deleted = true;
                        signal_changed_data();
                    },
                    Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete body weight: {err}"))
                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    match &*dialog.read() {
        BodyWeightDialog::None => rsx! {},
        BodyWeightDialog::Options(body_weight) => {
            let body_weight_edit = body_weight.clone();
            let body_weight_delete = body_weight.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Edit body weight".to_string(),
                                onclick: move |_| {
                                    *dialog.write() = BodyWeightDialog::Edit {
                                        date: FieldValue {
                                            input: body_weight_edit.date.to_string(),
                                            validated: Ok(body_weight_edit.date),
                                            orig: body_weight_edit.date.to_string()
                                        },
                                        weight: FieldValue {
                                            input: body_weight_edit.weight.to_string(),
                                            validated: Ok(body_weight_edit.weight),
                                            orig: body_weight_edit.weight.to_string()
                                        }
                                    };
                                }
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete body weight".to_string(),
                                onclick: move |_| { *dialog.write() = BodyWeightDialog::Delete(body_weight_delete.clone()); }
                            },
                        },
                        ],
                        close_event: close
                }
            }
        }
        BodyWeightDialog::Add { date, weight } | BodyWeightDialog::Edit { date, weight, .. } => {
            rsx! {
                Dialog {
                    title: rsx! { if let BodyWeightDialog::Add { .. } = &*dialog.read() { "Add body weight" } else { "Edit body weight" } },
                    close_event: close,
                    InputField {
                        label: "Date".to_string(),
                        r#type: "date".to_string(),
                        max: Local::now().date_naive().to_string(),
                        value: date.input.clone(),
                        error: if let Err(err) = &date.validated { err.clone() },
                        has_changed: date.changed(),
                        is_disabled: if let BodyWeightDialog::Edit { .. } = *dialog.read() { true },
                        oninput: move |event: FormEvent| {
                            async move {
                                match &mut *dialog.write() {
                                    BodyWeightDialog::Add { date, .. }
                                    | BodyWeightDialog::Edit { date, .. } => {
                                        date.input = event.value();
                                        date.validated = DOMAIN_SERVICE
                                            .read()
                                            .validate_body_weight_date(&date.input)
                                            .await
                                            .map_err(|err| err.to_string());
                                    }
                                    _ => {}
                                }
                            }
                        },
                    }
                    InputField {
                        label: "Weight".to_string(),
                        right_icon: rsx! { "kg" },
                        inputmode: "numeric".to_string(),
                        value: weight.input.clone(),
                        error: if let Err(err) = &weight.validated { err.clone() },
                        has_changed: weight.changed(),
                        oninput: move |event: FormEvent| {
                            async move {
                                match &mut *dialog.write() {
                                    BodyWeightDialog::Add { weight, .. }
                                    | BodyWeightDialog::Edit { weight, .. } => {
                                        weight.input = event.value();
                                        weight.validated = DOMAIN_SERVICE
                                            .read()
                                            .validate_body_weight_weight(&weight.input)
                                            .map_err(|err| err.to_string());
                                    }
                                    _ => {}
                                }
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
                                disabled: !FieldValue::has_valid_changes(&[date as &dyn FieldValueState, weight]),
                                "Save"
                            }
                        }
                    }
                }
            }
        }
        BodyWeightDialog::Delete(body_weight) => rsx! {
            DeleteConfirmationDialog {
                element_type: "body weight".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{body_weight.date}" } } },
                delete_event: delete,
                cancel_event: close,
                is_loading: is_loading(),
            }
        },
    }
}

enum BodyWeightDialog {
    None,
    Options(domain::BodyWeight),
    Add {
        date: FieldValue<NaiveDate>,
        weight: FieldValue<f32>,
    },
    Edit {
        date: FieldValue<NaiveDate>,
        weight: FieldValue<f32>,
    },
    Delete(domain::BodyWeight),
}
