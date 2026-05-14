use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, ERRORS, Route,
    cache::{Cache, CacheState},
    page::common::{Calendar, Chart, ChartLabel, IntervalControl},
    routing::NavigatorScrollExt,
    settings::Settings,
    ui::{
        element::{
            DeleteConfirmationDialog, ErrorMessage, FloatingActionButton, ItemOptionsButton,
            LoadingPage, MenuOption, NoConnection, NoWrap, OptionsMenu, SaveDialog, Table,
            value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
};

#[component]
pub fn BodyWeight(add: bool) -> Element {
    let cache = consume_context::<Cache>();
    let dates = use_memo(move || {
        if let CacheState::Ready(body_weight) = &*cache.body_weight.read() {
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
    let settings = use_context::<Settings>();
    let mut dialog = use_signal(|| BodyWeightDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE()
            .validate_body_weight_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        dialog.set(BodyWeightDialog::Add {
            date,
            weight: FieldValue::default(),
        });
        navigator().replace_preserving_scroll(Route::BodyWeight { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match &*cache.body_weight.read() {
        CacheState::Ready(body_weight) => {
            let avg_body_weight = DOMAIN_SERVICE().avg_body_weight(body_weight);
            rsx! {
                IntervalControl { current_interval, all }
                {chart(body_weight, &avg_body_weight, *current_interval.read(), settings)}
                {calendar(body_weight, *current_interval.read())}
                {table(body_weight, &avg_body_weight, *current_interval.read(), dialog)}
                {view_dialog(dialog)}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    on_click: move |_| { show_add_dialog() },
                }
            }
        }
        CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)) => {
            rsx! { NoConnection {} }
        }
        CacheState::Error(err) => rsx! {
            ErrorMessage { message: err }
        },
        CacheState::Loading => rsx! {
            LoadingPage {}
        },
    }
}

fn chart(
    body_weight: &[domain::BodyWeight],
    avg_body_weight: &[domain::BodyWeight],
    interval: domain::Interval,
    settings: Settings,
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
                settings.current_theme(),
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
                    if let Some(value) = DOMAIN_SERVICE().avg_weekly_change(avg_body_weight, avg_bw) {
                        "{value:+.1}"
                    } else {
                        "-"
                    }
                },
                rsx! {
                    ItemOptionsButton { on_click: move |_| { *dialog.write() = BodyWeightDialog::Options(bw.clone()); } }
                }
            ]
        }).collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

fn view_dialog(mut dialog: Signal<BodyWeightDialog>) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let mut close_dialog = move || {
        dialog.set(BodyWeightDialog::None);
        navigator().replace_preserving_scroll(Route::BodyWeight { add: false });
    };

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                BodyWeightDialog::Add { date, weight } => {
                    if let (Ok(date), Ok(weight)) = (date.validated.clone(), weight.validated.clone()) {
                        match DOMAIN_SERVICE()
                            .create_body_weight(domain::BodyWeight { date, weight })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    consume_context::<Cache>().refresh_body_weight();
                                }
                                Err(err) => {
                                    ERRORS
                                        .write()
                                        .push(format!("Failed to add body weight: {err}"));
                                    }
                            }
                    }
                }
                BodyWeightDialog::Edit { date, weight } => {
                    if let (Ok(date), Ok(weight)) = (date.validated.clone(), weight.validated.clone()) {
                        match DOMAIN_SERVICE()
                            .replace_body_weight(domain::BodyWeight { date, weight })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    consume_context::<Cache>().refresh_body_weight();
                                }
                                Err(err) => {
                                    ERRORS
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
                match DOMAIN_SERVICE().delete_body_weight(body_weight.date).await {
                    Ok(()) => {
                        deleted = true;
                        consume_context::<Cache>().refresh_body_weight();
                    },
                    Err(err) => ERRORS.write().push(format!("Failed to delete body weight: {err}"))
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
                                "data-testid": "options-edit",
                                on_click: move |_| {
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
                                "data-testid": "options-delete",
                                on_click: move |_| { *dialog.write() = BodyWeightDialog::Delete(body_weight_delete.clone()); }
                            },
                        },
                        ],
                        on_close: close
                }
            }
        }
        BodyWeightDialog::Add { date, weight } | BodyWeightDialog::Edit { date, weight, .. } => {
            rsx! {
                SaveDialog {
                    title: rsx! { if let BodyWeightDialog::Add { .. } = &*dialog.read() { "Add body weight" } else { "Edit body weight" } },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: !FieldValue::has_valid_changes(&[date as &dyn FieldValueState, weight]),
                    InputField {
                        label: "Date".to_string(),
                        r#type: "date".to_string(),
                        max: Local::now().date_naive().to_string(),
                        value: date.input.clone(),
                        error: if let Err(err) = &date.validated { err.clone() },
                        has_changed: date.changed(),
                        is_disabled: if let BodyWeightDialog::Edit { .. } = *dialog.read() { true },
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            async move {
                                let validated_date = DOMAIN_SERVICE()
                                    .validate_body_weight_date(&input)
                                    .await
                                    .map_err(|err| err.to_string());
                                match &mut *dialog.write() {
                                    BodyWeightDialog::Add { date, .. }
                                    | BodyWeightDialog::Edit { date, .. } => {
                                        date.input = input;
                                        date.validated = validated_date;
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
                        autofocus: true,
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            async move {
                                let validated_weight = DOMAIN_SERVICE()
                                    .validate_body_weight_weight(&input)
                                    .map_err(|err| err.to_string());
                                match &mut *dialog.write() {
                                    BodyWeightDialog::Add { weight, .. }
                                    | BodyWeightDialog::Edit { weight, .. } => {
                                        weight.input = input;
                                        weight.validated = validated_weight;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    },
                }
            }
        }
        BodyWeightDialog::Delete(body_weight) => rsx! {
            DeleteConfirmationDialog {
                element_type: "body weight".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{body_weight.date}" } } },
                on_delete: delete,
                on_cancel: close,
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
