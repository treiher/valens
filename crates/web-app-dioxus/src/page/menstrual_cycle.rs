use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService, PeriodService, SessionService};
use valens_web_app::{self as web_app, Settings, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        element::{
            Calendar, Chart, ChartLabel, DataBox, DeleteConfirmationDialog, Dialog, ErrorMessage,
            FloatingActionButton, Icon, IntervalControl, LoadingPage, MenuOption, NoConnection,
            NoWrap, OptionsMenu, Table,
        },
        form::{Field, FieldValue, FieldValueState, InputField},
    },
    ensure_session, signal_changed_data,
};

#[component]
pub fn MenstrualCycle(add: bool) -> Element {
    ensure_session!();

    let period = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_period().await
    });
    let dates = use_memo(move || {
        if let Some(Ok(period)) = &*period.read() {
            period.iter().map(|bw| bw.date).collect::<Vec<_>>()
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
    let mut dialog = use_signal(|| PeriodDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE
            .read()
            .validate_body_weight_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        *dialog.write() = PeriodDialog::Add {
            date,
            intensity: FieldValue::default(),
        };
        navigator().replace(Route::MenstrualCycle { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match &*period.read() {
        Some(Ok(period)) => {
            let cycles = domain::cycles(period);
            rsx! {
                {current_cycle(&cycles)}
                IntervalControl { current_interval, all }
                {calendar(period, *current_interval.read())}
                {chart(period, *current_interval.read(), settings)}
                {cycle_stats(&cycles, *current_interval.read())}
                {table(period, *current_interval.read(), dialog)}
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

fn chart(
    period: &[domain::Period],
    interval: domain::Interval,
    settings: Resource<Result<Settings, String>>,
) -> Element {
    let period = period
        .iter()
        .filter(|p| p.date >= interval.first && p.date <= interval.last)
        .collect::<Vec<_>>();

    rsx! {
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Intensity".to_string(),
                    color: web_app::chart::COLOR_PERIOD_INTENSITY,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
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
                interval,
                if let Some(Ok(settings)) = *settings.read() { settings.current_theme() } else { web_app::Theme::Light },
                ).map_err(|err| err.to_string()),
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
                    a {
                        class: "mx-2",
                        onclick: move |_| { *dialog.write() = PeriodDialog::Options(p.clone()); },
                        Icon { name: "ellipsis-vertical"}
                    }
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

    let mut close_dialog = move || {
        *dialog.write() = PeriodDialog::None;
        navigator().replace(Route::MenstrualCycle { add: false });
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
                PeriodDialog::Add { date, intensity } => {
                    if let (Ok(date), Ok(intensity)) = (date.validated.clone(), intensity.validated.clone()) {
                        match DOMAIN_SERVICE
                            .read()
                            .create_period(domain::Period { date, intensity })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    signal_changed_data();
                                }
                                Err(err) => {
                                    NOTIFICATIONS
                                        .write()
                                        .push(format!("Failed to add period: {err}"));
                                    }
                            }
                    }
                }
                PeriodDialog::Edit { date, intensity } => {
                    if let (Ok(date), Ok(intensity)) = (date.validated.clone(), intensity.validated.clone()) {
                        match DOMAIN_SERVICE
                            .read()
                            .replace_period(domain::Period { date, intensity })
                            .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    signal_changed_data();
                                }
                                Err(err) => {
                                    NOTIFICATIONS
                                        .write()
                                        .push(format!("Failed to edit period: {err}"));
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
                match DOMAIN_SERVICE.read().delete_period(period.date).await {
                    Ok(_) => {
                        deleted = true;
                        signal_changed_data();
                    },
                    Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete period: {err}"))
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
                                onclick: move |_| {
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
                                onclick: move |_| { *dialog.write() = PeriodDialog::Delete(period_delete.clone()); }
                            },
                        },
                        ],
                        close_event: close
                }
            }
        }
        PeriodDialog::Add { date, intensity }
        | PeriodDialog::Edit {
            date, intensity, ..
        } => rsx! {
            Dialog {
                title: rsx! { if let PeriodDialog::Add { .. } = &*dialog.read() { "Add period" } else { "Edit period" } },
                close_event: close,
                InputField {
                    label: "Date".to_string(),
                    r#type: "date".to_string(),
                    max: Local::now().date_naive().to_string(),
                    value: date.input.clone(),
                    error: if let Err(err) = &date.validated { err.clone() },
                    has_changed: date.changed(),
                    is_disabled: if let PeriodDialog::Edit { .. } = *dialog.read() { true },
                    oninput: move |event: FormEvent| {
                        async move {
                            match &mut *dialog.write() {
                                PeriodDialog::Add { date, .. }
                                | PeriodDialog::Edit { date, .. } => {
                                    date.input = event.value();
                                    date.validated = DOMAIN_SERVICE
                                        .read()
                                        .validate_period_date(&date.input)
                                        .await
                                        .map_err(|err| err.to_string());
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
                            disabled: !FieldValue::has_valid_changes(&[date as &dyn FieldValueState, intensity]),
                            "Save"
                        }
                    }
                }
            }
        },
        PeriodDialog::Delete(period) => rsx! {
            DeleteConfirmationDialog {
                element_type: "period".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{period.date}" } } },
                delete_event: delete,
                cancel_event: close,
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
