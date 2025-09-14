use std::{collections::BTreeMap, str::FromStr};

use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{
    self as domain, BodyWeightService, RoutineService, SessionService, TrainingSessionService,
};
use valens_web_app::{self as web_app, Settings, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route, WEB_APP_SERVICE,
    component::{
        element::{
            Calendar, Chart, ChartLabel, DeleteConfirmationDialog, Dialog, Error,
            FloatingActionButton, Icon, IntervalControl, LoadingPage, NoConnection, NoWrap, Table,
            value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
    ensure_session, signal_changed_data,
};

#[component]
pub fn Training(add: bool) -> Element {
    ensure_session!();

    let training_sessions = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_training_sessions().await
    });
    let routines = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_routines().await
    });
    let dates = use_memo(move || {
        if let Some(Ok(training_session)) = &*training_sessions.read() {
            training_session
                .iter()
                .map(|ts| ts.date)
                .collect::<Vec<_>>()
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
    let mut dialog = use_signal(|| TrainingDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE
            .read()
            .validate_body_weight_date(&date.input)
            .await
            .map_err(|err| err.to_string());
        *dialog.write() = TrainingDialog::Add {
            date,
            routine_id: FieldValue {
                input: String::new(),
                validated: Ok(domain::RoutineID::nil()),
                orig: String::new(),
            },
        };
        navigator().replace(Route::Training { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match (
        &*training_sessions.read(),
        &*routines.read(),
        &*settings.read(),
    ) {
        (Some(Ok(training_sessions)), Some(Ok(routines)), Some(Ok(settings))) => {
            rsx! {
                IntervalControl { current_interval, all }
                {charts(training_sessions, *current_interval.read(), *settings)}
                {calendar(training_sessions, *current_interval.read())}
                {table(training_sessions, routines, *current_interval.read(), dialog, *settings)}
                {view_dialog(dialog, training_sessions, routines)}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    onclick: move |_| { show_add_dialog() },
                }
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _) | (_, Some(Err(err)), _) => {
            rsx! { Error { message: err } }
        }
        (_, _, Some(Err(err))) => {
            rsx! { Error { message: err } }
        }
        (None, _, _) | (_, None, _) | (_, _, None) => rsx! { LoadingPage {} },
    }
}

fn charts(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let training_stats = DOMAIN_SERVICE.read().get_training_stats(training_sessions);
    let short_term_load = training_stats
        .short_term_load
        .iter()
        .filter(|(date, _)| *date >= interval.first && *date <= interval.last)
        .copied()
        .collect::<Vec<_>>();
    let long_term_load = training_stats
        .long_term_load
        .iter()
        .filter(|(date, _)| *date >= interval.first && *date <= interval.last)
        .copied()
        .collect::<Vec<_>>();
    let long_term_load_high = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * domain::TrainingStats::LOAD_RATIO_HIGH))
        .collect::<Vec<_>>();
    let long_term_load_low = long_term_load
        .iter()
        .copied()
        .map(|(d, l)| (d, l * domain::TrainingStats::LOAD_RATIO_LOW))
        .collect::<Vec<_>>();
    #[allow(clippy::cast_precision_loss)]
    let total_7day_set_volume = domain::centered_moving_total(
        &training_sessions
            .iter()
            .map(|s| (s.date, s.set_volume() as f32))
            .collect::<Vec<_>>(),
        interval,
        3,
    );
    let average_7day_rpe = domain::centered_moving_average(
        &training_sessions
            .iter()
            .filter_map(|s| s.avg_rpe().map(|v| (s.date, v)))
            .collect::<Vec<_>>(),
        interval,
        3,
    );
    let theme = settings.current_theme();
    let show_rpe = settings.show_rpe;

    rsx! {
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Short-term load".to_string(),
                    color: web_app::chart::COLOR_LOAD,
                    opacity: web_app::chart::OPACITY_LINE,
                },
                ChartLabel {
                    name: "Short-term load".to_string(),
                    color: web_app::chart::COLOR_LONG_TERM_LOAD,
                    opacity: web_app::chart::OPACITY_AREA,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: long_term_load_high,
                        values_low: Some(long_term_load_low),
                        plots: web_app::chart::plot_area(web_app::chart::COLOR_LONG_TERM_LOAD),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    },
                    web_app::chart::PlotData {
                        values_high: short_term_load,
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_LOAD),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }
                ],
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Set volume (7 day total)".to_string(),
                    color: web_app::chart::COLOR_SET_VOLUME,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[web_app::chart::PlotData {
                    values_high: total_7day_set_volume,
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_SET_VOLUME,
                        web_app::chart::COLOR_SET_VOLUME,
                    ),
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                }],
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        if show_rpe {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "RPE (7 day average)".to_string(),
                        color: web_app::chart::COLOR_RPE,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot(
                    &average_7day_rpe.iter().map(|values| web_app::chart::PlotData{values_high: values.clone(),
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_RPE),
                        params: web_app::chart::PlotParams::primary_range(5., 10.)
                    }).collect::<Vec<_>>(),
                    interval,
                    theme,
                ).map_err(|err| err.to_string()),
                no_data_label: false,
            }
        }
    }
}

fn calendar(training_sessions: &[domain::TrainingSession], interval: domain::Interval) -> Element {
    let mut load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            load.entry(training_session.date)
                .and_modify(|e| *e += training_session.load())
                .or_insert(training_session.load());
        }
    }
    let min = load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let entries = load
        .iter()
        .map(|(date, load)| {
            (
                *date,
                web_app::chart::COLOR_LOAD,
                if max > min {
                    (f64::from(load - min) / f64::from(max - min)) * 0.8 + 0.2
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
    training_sessions: &[domain::TrainingSession],
    routines: &[domain::Routine],
    interval: domain::Interval,
    mut dialog: Signal<TrainingDialog>,
    settings: Settings,
) -> Element {
    let (has_avg_rpe_data, has_tut_data, has_avg_reps_data, has_avg_weight_data, has_avg_time_data) =
        training_sessions
            .iter()
            .fold((false, false, false, false, false), |r, t| {
                (
                    r.0 || t.avg_rpe().is_some(),
                    r.1 || t.tut().is_some(),
                    r.2 || t.avg_reps().is_some(),
                    r.3 || t.avg_weight().is_some(),
                    r.4 || t.avg_time().is_some(),
                )
            });

    let mut head = Vec::with_capacity(12);
    head.extend_from_slice(&[
        rsx! {"Date"},
        rsx! {"Routine"},
        rsx! {"Load"},
        rsx! {"Set volume"},
    ]);
    if settings.show_rpe && has_avg_rpe_data {
        head.push(rsx! {"RPE"});
    }
    head.push(rsx! {"Volume load"});
    if settings.show_tut && has_tut_data {
        head.push(rsx! {"TUT"});
    }
    if has_avg_reps_data {
        head.push(rsx! {"Reps"});
    }
    if settings.show_rpe && has_avg_reps_data && has_avg_rpe_data {
        head.push(rsx! {"Reps+RIR"});
    }
    if has_avg_weight_data {
        head.push(rsx! {"Weight (kg)"});
    }
    if settings.show_tut && has_avg_time_data {
        head.push(rsx! {"Time (s)"});
    }
    head.push(rsx! {});

    let body = training_sessions
        .iter()
        .rev()
        .filter(|p| p.date >= interval.first && p.date <= interval.last)
        .map(|t| {
            let t = t.clone();
            let date = t.date;
            let routine = routines.iter().find(|r| r.id == t.routine_id);
            let routine_id = routine.map(|r| r.id).unwrap_or_default();
            vec![
                rsx! {
                    Link {
                        to: Route::TrainingSession { id: t.id },
                        NoWrap { element: rsx! { "{date}" } }
                    }
                },
                rsx! {
                    if routine_id.is_nil() {
                        "-"
                    } else {
                        Link {
                            to: Route::Routine { id: routine_id },
                            match routine {
                                Some(routine) => rsx! { {routine.name.to_string()} },
                                None => rsx! { "-" }
                            }
                        }
                    }
                },
                rsx! { "{t.load()}" },
                rsx! { "{t.set_volume()}" },
                rsx! { if settings.show_rpe && has_avg_rpe_data { {value_or_dash(t.avg_rpe())} } },
                rsx! { "{t.volume_load()}" },
                rsx! { if settings.show_tut && has_tut_data { {value_or_dash(t.tut())} } },
                rsx! { if has_avg_reps_data { {value_or_dash(t.avg_reps())} } },
                rsx! {
                    if settings.show_rpe && has_avg_reps_data && has_avg_rpe_data {
                        {
                            if let (Some(avg_reps), Some(avg_rpe)) = (t.avg_reps(), t.avg_rpe()) {
                                format!("{:.1}", avg_reps + f32::from(domain::RIR::from(avg_rpe)))
                            } else {
                                "-".into()
                            }
                        }
                    }
                },
                rsx! { if has_avg_weight_data { {value_or_dash(t.avg_weight())} } },
                rsx! { if settings.show_tut && has_avg_time_data { {value_or_dash(t.avg_time())} } },
                rsx! {
                    a {
                        class: "mx-2",
                        onclick: move |_| { *dialog.write() = TrainingDialog::Delete(t.clone()); },
                        Icon { name: "xmark"}
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

fn view_dialog(
    mut dialog: Signal<TrainingDialog>,
    training_sessions: &[domain::TrainingSession],
    routines: &[domain::Routine],
) -> Element {
    let mut is_loading = use_signal(|| false);

    let mut close_dialog = move || {
        *dialog.write() = TrainingDialog::None;
        navigator().replace(Route::Training { add: false });
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
            if let TrainingDialog::Add { date, routine_id } = &*dialog.read() {
                if let (Ok(date), Ok(routine_id)) = (date.validated.clone(), routine_id.validated.clone()) {
                    match DOMAIN_SERVICE.read().get_routines().await {
                        Ok(routines) => {
                            let elements = routines.iter().find(|r| r.id == routine_id).map(|routine| {
                                routine
                                    .sections
                                    .iter()
                                    .flat_map(domain::RoutinePart::to_training_session_elements)
                                    .collect::<Vec<_>>()
                            }).unwrap_or_default();
                            match DOMAIN_SERVICE
                                .read()
                                .create_training_session(routine_id, date, String::new(), elements)
                                .await
                            {
                                Ok(_) => {
                                    saved = true;
                                    signal_changed_data();
                                }
                                Err(err) => {
                                    NOTIFICATIONS
                                        .write()
                                        .push(format!("Failed to add training session: {err}"));
                                }
                            }
                        }
                        Err(err) => {
                            NOTIFICATIONS
                                .write()
                                .push(format!("Failed to add training session: {err}"));
                        }
                    }
                }
            }
        }
        if saved {
            close_dialog();
        }
    };
    let delete = move |_| async move {
        let mut deleted = false;
        is_loading! {
            if let TrainingDialog::Delete(training_session) = &*dialog.read() {
                match DOMAIN_SERVICE.read().delete_training_session(training_session.id).await {
                    Ok(_) => {
                        deleted = true;
                        signal_changed_data();
                    },
                    Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete training session: {err}"))
                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    match &*dialog.read() {
        TrainingDialog::None => rsx! {},
        TrainingDialog::Add { date, routine_id } => rsx! {
            Dialog {
                title: rsx! { "Add training session" },
                content: rsx! {
                    InputField {
                        label: "Date".to_string(),
                        r#type: "date".to_string(),
                        max: Local::now().date_naive().to_string(),
                        value: date.input.clone(),
                        error: if let Err(err) = &date.validated { err.clone() },
                        has_changed: date.changed(),
                        oninput: move |event: FormEvent| {
                            async move {
                                if let TrainingDialog::Add { date, .. } = &mut *dialog.write() {
                                    date.input = event.value();
                                    date.validated = DOMAIN_SERVICE
                                        .read()
                                        .validate_training_session_date(&date.input)
                                        .await
                                        .map_err(|err| err.to_string());
                                }
                            }
                        },
                    }
                    SelectField {
                        label: "Routine".to_string(),
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: String::new(),
                                    value: String::new(),
                                    selected: routine_id.validated == Ok(domain::RoutineID::nil()),
                                }
                            }
                        ].into_iter().chain(domain::routines_sorted_by_last_use(routines, training_sessions, |r: &domain::Routine| !r.archived).iter().map(|r| {
                            rsx! {
                                SelectOption {
                                    text: r.name.to_string(),
                                    value: r.id.to_string(),
                                    selected: routine_id.validated == Ok(r.id),
                                }
                            }
                        })).collect::<Vec<_>>(),
                        has_changed: routine_id.changed(),
                        onchange: move |event: FormEvent| {
                            if let TrainingDialog::Add { routine_id, .. } = &mut *dialog.write() {
                                routine_id.input = event.value();
                                routine_id.validated = Ok(domain::RoutineID::from_str(&routine_id.input).unwrap_or(domain::RoutineID::nil()));
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
                                disabled: !date.valid() || !routine_id.valid(),
                                "Save"
                            }
                        }
                    }
                },
                close_event: close,
            }
        },
        TrainingDialog::Delete(training_session) => rsx! {
            DeleteConfirmationDialog {
                element_type: "training session".to_string(),
                element_name: rsx! { span { "of " NoWrap { element: rsx! { "{training_session.date}" } } } },
                delete_event: delete,
                cancel_event: close,
                is_loading: is_loading(),
            }
        },
    }
}

enum TrainingDialog {
    None,
    Add {
        date: FieldValue<NaiveDate>,
        routine_id: FieldValue<domain::RoutineID>,
    },
    Delete(domain::TrainingSession),
}
