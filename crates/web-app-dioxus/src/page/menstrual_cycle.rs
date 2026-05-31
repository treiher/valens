use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, PeriodService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    notification::notify_error,
    page::common::{Calendar, Chart, IntervalControl},
    routing::NavigatorScrollExt,
    ui::{
        element::{
            DataBox, DeleteConfirmationDialog, ErrorMessage, FloatingActionButton,
            ItemOptionsButton, LoadingPage, MenuOption, NoConnection, NoWrap, OptionsMenu,
            SaveDialog, Table,
        },
        form::{Field, FieldValue, FieldValueState, InputField},
    },
};

#[component]
pub fn MenstrualCycle(add: bool) -> Element {
    let cache = consume_context::<Cache>();
    let dates = use_memo(move || {
        if let CacheState::Ready(period) = &*cache.period.read() {
            period.iter().map(|p| p.date).collect::<Vec<_>>()
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
    let mut dialog = use_signal(|| PeriodDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE()
            .validate_period_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        dialog.set(PeriodDialog::Add {
            date,
            intensity: FieldValue::default(),
        });
        navigator().replace_preserving_scroll(Route::MenstrualCycle { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match &*cache.period.read() {
        CacheState::Ready(period) => {
            let cycles = domain::cycles(period);
            rsx! {
                {current_cycle(&cycles)}
                IntervalControl { current_interval, all }
                {calendar(period, *current_interval.read())}
                {chart(period, *current_interval.read())}
                {cycle_stats(&cycles, *current_interval.read())}
                {table(period, *current_interval.read(), dialog)}
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

fn current_cycle(cycles: &[domain::Cycle]) -> Element {
    let today = Local::now().date_naive();
    if let Some(current_cycle) = domain::current_cycle(cycles) {
        let days = (today - current_cycle.begin).num_days() + 1;
        let days_left = current_cycle.time_left.num_days();
        let days_left_variation = current_cycle.time_left_variation.num_days();
        rsx! {
            DataBox {
                title: "Current cycle".to_string(),
                strong { "{days}" }
                " days, "
                    strong { "{days_left} (±{days_left_variation})"}
                " days left"
            }
        }
    } else {
        rsx! {}
    }
}

fn chart(period: &[domain::Period], interval: domain::Interval) -> Element {
    let period = period
        .iter()
        .filter(|p| p.date >= interval.first && p.date <= interval.last)
        .collect::<Vec<_>>();

    let intensity_data = web_app::chart::PlotData {
        values_high: period
            .iter()
            .map(|p| (p.date, f32::from(p.intensity as u8)))
            .collect::<Vec<_>>(),
        values_low: None,
        plots: web_app::chart::plot_histogram(web_app::chart::COLOR_PERIOD_INTENSITY),
        params: web_app::chart::PlotParams::primary_range(0., 4.),
    };
    rsx! {
        Chart {
            series: vec![web_app::chart::LabeledSeries::new("Intensity", intensity_data)],
            interval,
            no_data_label: true,
        }
    }
}

fn calendar(period: &[domain::Period], interval: domain::Interval) -> Element {
    let period = period
        .iter()
        .filter(|p| (interval.first..=interval.last).contains(&p.date))
        .collect::<Vec<_>>();
    let entries = period
        .iter()
        .map(|p| {
            (
                p.date,
                web_app::chart::COLOR_PERIOD_INTENSITY,
                f64::from(p.intensity as u8) * 0.25,
            )
        })
        .collect();

    rsx! {
        Calendar { entries, interval }
    }
}

fn cycle_stats(cycles: &[domain::Cycle], interval: domain::Interval) -> Element {
    let cycles = cycles
        .iter()
        .filter(|c| c.begin >= interval.first && c.begin <= interval.last)
        .collect::<Vec<_>>();
    let stats = domain::cycle_stats(&cycles);
    rsx! {
        DataBox {
            title: "Avg. cycle length".to_string(),
            if cycles.is_empty() {
                "–"
            } else {
                strong {
                    "{stats.length_median.num_days()} (±{stats.length_variation.num_days()})"
                }
                " days"
            }
        }
    }
}

fn table(
    period: &[domain::Period],
    interval: domain::Interval,
    mut dialog: Signal<PeriodDialog>,
) -> Element {
    let head = vec![rsx! {"Date"}, rsx! {"Intensity"}, rsx! {}];

    let body = period
        .iter()
        .rev()
        .filter(|p| p.date >= interval.first && p.date <= interval.last)
        .map(|p| {
            let p = p.clone();
            let date = p.date;
            vec![
                rsx! { NoWrap { "{date}" } },
                rsx! { "{p.intensity}" },
                rsx! {
                    ItemOptionsButton { on_click: move |_| { *dialog.write() = PeriodDialog::Options(p.clone()); } }
                },
            ]
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

fn view_dialog(mut dialog: Signal<PeriodDialog>) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let mut close_dialog = move || {
        dialog.set(PeriodDialog::None);
        navigator().replace_preserving_scroll(Route::MenstrualCycle { add: false });
    };

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                PeriodDialog::Add { date, intensity } => {
                    if let (Ok(date), Ok(intensity)) = (date.validated.clone(), intensity.validated.clone()) {
                        match DOMAIN_SERVICE()
                            .create_period(domain::Period { date, intensity })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    consume_context::<Cache>().refresh_period();
                                }
                                Err(err) => {
                                    notify_error(format!("Failed to add period: {err}"));
                                    }
                            }
                    }
                }
                PeriodDialog::Edit { date, intensity } => {
                    if let (Ok(date), Ok(intensity)) = (date.validated.clone(), intensity.validated.clone()) {
                        match DOMAIN_SERVICE()
                            .replace_period(domain::Period { date, intensity })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    consume_context::<Cache>().refresh_period();
                                }
                                Err(err) => {
                                    notify_error(format!("Failed to edit period: {err}"));
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
            if let PeriodDialog::Delete(period) = &*dialog.read() {
                match DOMAIN_SERVICE().delete_period(period.date).await {
                    Ok(()) => {
                        deleted = true;
                        consume_context::<Cache>().refresh_period();
                    },
                    Err(err) => notify_error(format!("Failed to delete period: {err}"))
                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    match &*dialog.read() {
        PeriodDialog::None => rsx! {},
        PeriodDialog::Options(period) => {
            let period_edit = period.clone();
            let period_delete = period.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Edit period".to_string(),
                                "data-testid": "options-edit",
                                on_click: move |_| {
                                    *dialog.write() = PeriodDialog::Edit {
                                        date: FieldValue {
                                            input: period_edit.date.to_string(),
                                            validated: Ok(period_edit.date),
                                            orig: period_edit.date.to_string()
                                        },
                                        intensity: FieldValue {
                                            input: period_edit.intensity.to_string(),
                                            validated: Ok(period_edit.intensity),
                                            orig: period_edit.intensity.to_string()
                                        }
                                    };
                                }
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete period".to_string(),
                                "data-testid": "options-delete",
                                on_click: move |_| { *dialog.write() = PeriodDialog::Delete(period_delete.clone()); }
                            },
                        },
                        ],
                        on_close: close
                }
            }
        }
        PeriodDialog::Add { date, intensity }
        | PeriodDialog::Edit {
            date, intensity, ..
        } => rsx! {
            SaveDialog {
                title: rsx! { if let PeriodDialog::Add { .. } = &*dialog.read() { "Add period" } else { "Edit period" } },
                on_close: close,
                on_save: save,
                is_loading: is_loading(),
                disabled: !FieldValue::has_valid_changes(&[date as &dyn FieldValueState, intensity]),
                InputField {
                    label: "Date".to_string(),
                    r#type: "date".to_string(),
                    max: Local::now().date_naive().to_string(),
                    value: date.input.clone(),
                    error: if let Err(err) = &date.validated { err.clone() },
                    has_changed: date.changed(),
                    is_disabled: matches!(*dialog.read(), PeriodDialog::Edit { .. }),
                    on_input: move |event: FormEvent| {
                        let input = event.value();
                        async move {
                            let validated_date = DOMAIN_SERVICE()
                                .validate_period_date(&input)
                                .await
                                .map_err(|err| err.to_string());
                            match &mut *dialog.write() {
                                PeriodDialog::Add { date, .. }
                                | PeriodDialog::Edit { date, .. } => {
                                    date.input = input;
                                    date.validated = validated_date;
                                }
                                _ => {}
                            }
                        }
                    },
                }
                Field {
                    label: "Intensity",
                    for i in domain::Intensity::iter() {
                        button {
                            class: "button mr-2",
                            class: if intensity.validated == Ok(*i) { "is-link" },
                            onclick: move |_| {
                                match &mut *dialog.write() {
                                    PeriodDialog::Add { intensity, .. }
                                    | PeriodDialog::Edit { intensity, .. } => {
                                        intensity.input = i.to_string();
                                        intensity.validated = Ok(*i);
                                    }
                                    _ => {}
                                }
                            },
                            {i.to_string()}
                        }
                    }
                }
            }
        },
        PeriodDialog::Delete(period) => rsx! {
            DeleteConfirmationDialog {
                element_type: "period".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{period.date}" } } },
                on_delete: delete,
                on_cancel: close,
                is_loading: is_loading(),
            }
        },
    }
}

enum PeriodDialog {
    None,
    Options(domain::Period),
    Add {
        date: FieldValue<NaiveDate>,
        intensity: FieldValue<domain::Intensity>,
    },
    Edit {
        date: FieldValue<NaiveDate>,
        intensity: FieldValue<domain::Intensity>,
    },
    Delete(domain::Period),
}
