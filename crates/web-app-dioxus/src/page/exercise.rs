use std::{borrow::Borrow, collections::BTreeMap};

use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{
    self as domain, ExerciseService, Property, RoutineService, SessionService,
    TrainingSessionService,
};
use valens_web_app::{self as web_app, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, Route, WEB_APP_SERVICE,
    component::element::{
        Block, Calendar, Chart, ChartLabel, ElementWithDescription, Error, ErrorMessage,
        FloatingActionButton, IntervalControl, Loading, LoadingPage, NoConnection, NoData, NoWrap,
        Title,
    },
    eh, ensure_session, page,
};

#[component]
pub fn Exercise(id: domain::ExerciseID) -> Element {
    ensure_session!();

    let exercise = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_exercise(id).await
    });
    let memorized_exercise = use_memo(move || {
        exercise
            .read()
            .as_ref()
            .and_then(|e| e.as_ref().ok())
            .and_then(std::clone::Clone::clone)
    });
    let training_sessions = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        if let Some(e) = memorized_exercise() {
            Some(
                DOMAIN_SERVICE
                    .read()
                    .get_training_sessions_by_exercise_id(e.id)
                    .await,
            )
        } else {
            None
        }
    });
    let routines = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_routines().await
    });
    let dates = use_memo(move || {
        if let Some(Some(Ok(training_session))) = &*training_sessions.read() {
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
    let mut exercise_dialog = use_signal(|| page::exercises::ExerciseDialog::None);
    let training_dialog = use_signal(|| page::training::TrainingDialog::None);

    match (
        &*exercise.read(),
        &*training_sessions.read(),
        &*routines.read(),
        &*settings.read(),
    ) {
        (
            Some(Ok(Some(exercise))),
            Some(training_sessions),
            Some(Ok(routines)),
            Some(Ok(settings)),
        ) => {
            rsx! {
                Title { title: "{exercise.name}", x_padding: 2 },
                Block {
                    {view_muscles(exercise.muscle_stimulus().iter())},
                }
                match training_sessions {
                    Some(Ok(training_sessions)) => {
                        if training_sessions.is_empty() {
                            rsx! {
                                NoData {}
                            }
                        } else {
                            let interval = *current_interval.read();
                            let training_sessions = training_sessions
                                .iter()
                                .filter(|t| t.date >= interval.first && t.date <= interval.last)
                                .cloned()
                                .collect::<Vec<_>>();
                            rsx! {
                                IntervalControl { current_interval, all },
                                if training_sessions.is_empty() {
                                    NoData {}
                                } else {
                                    {view_charts(&training_sessions, interval, *settings)}
                                    {view_calendar(&training_sessions, interval)}
                                    {page::training::view_table(&training_sessions, routines, interval, training_dialog, *settings)}
                                    {view_sets(&training_sessions, routines, *settings)}
                                    {page::training::view_dialog(training_dialog, &training_sessions, routines, None)}
                                }
                            }
                        }
                    }
                    Some(Err(err)) => {
                        rsx! { Error { message: err } }
                    }
                    None => {
                        rsx! {
                            Loading {}
                        }
                    }
                }
                {page::exercises::view_dialog(exercise_dialog, None)}
                FloatingActionButton {
                    icon: "edit".to_string(),
                    onclick: eh!(exercise; {
                        *exercise_dialog.write() = page::exercises::ExerciseDialog::Options(exercise.clone());
                    }),
                }
            }
        }
        (Some(Ok(None)), _, _, _) => {
            rsx! {
                ErrorMessage { message: "Exercise not found" }
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _, _) | (_, _, Some(Err(err)), _) => {
            rsx! { ErrorMessage { message: err } }
        }
        (_, _, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _, _, _) | (_, None, _, _) | (_, _, None, _) | (_, _, _, None) => {
            rsx! { LoadingPage {} }
        }
    }
}

pub fn view_muscles<M, I, S>(muscles: M) -> Element
where
    M: IntoIterator<Item = (I, S)>,
    I: Borrow<domain::MuscleID>,
    S: Borrow<domain::Stimulus>,
{
    let mut muscles = muscles
        .into_iter()
        .map(|(k, v)| (*k.borrow(), *v.borrow()))
        .filter(|(_, stimulus)| **stimulus > *domain::Stimulus::NONE)
        .collect::<Vec<_>>();
    muscles.sort_by(|a, b| b.1.cmp(&a.1));
    rsx! {
        div {
            class: "tags is-centered m-2",
            for (m, stimulus) in muscles {
                ElementWithDescription {
                    description: m.description(),
                    span {
                        class: "tag",
                        class: if *stimulus >= *domain::Stimulus::PRIMARY { "is-dark" } else { "is-link" },
                        {m.name()}
                    }
                }
            }
        }
    }
}

fn view_charts(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
    settings: web_app::Settings,
) -> Element {
    let mut set_volume: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut volume_load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut tut: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    let mut reps_rpe: BTreeMap<NaiveDate, (Vec<f32>, Vec<domain::RPE>)> = BTreeMap::new();
    for training_session in training_sessions {
        #[allow(clippy::cast_precision_loss)]
        set_volume
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.set_volume() as f32)
            .or_insert(training_session.set_volume() as f32);
        #[allow(clippy::cast_precision_loss)]
        volume_load
            .entry(training_session.date)
            .and_modify(|e| *e += training_session.volume_load() as f32)
            .or_insert(training_session.volume_load() as f32);
        #[allow(clippy::cast_precision_loss)]
        tut.entry(training_session.date)
            .and_modify(|e| *e += training_session.tut().unwrap_or_default() as f32)
            .or_insert(training_session.tut().unwrap_or_default() as f32);
        if let Some(avg_reps) = training_session.avg_reps() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.0.push(avg_reps))
                .or_insert((vec![avg_reps], vec![]));
        }
        if let Some(avg_rpe) = training_session.avg_rpe() {
            reps_rpe
                .entry(training_session.date)
                .and_modify(|e| e.1.push(avg_rpe));
        }
    }

    let mut reps_labels = vec![ChartLabel {
        name: "Reps".to_string(),
        color: web_app::chart::COLOR_REPS,
        opacity: web_app::chart::OPACITY_LINE,
    }];
    let reps_rpe_values = reps_rpe
        .iter()
        .map(|(date, (avg_reps, _))| {
            #[allow(clippy::cast_precision_loss)]
            (*date, avg_reps.iter().sum::<f32>() / avg_reps.len() as f32)
        })
        .collect::<Vec<_>>();

    let mut reps_data = vec![];

    if settings.show_rpe {
        let rir_values = reps_rpe
            .into_iter()
            .filter_map(|(date, (avg_reps_values, avg_rpe_values))| {
                #[allow(clippy::cast_precision_loss)]
                let avg_reps = avg_reps_values.iter().sum::<f32>() / avg_reps_values.len() as f32;
                domain::RPE::avg(&avg_rpe_values)
                    .map(|avg_rpe| (date, avg_reps + f32::from(domain::RIR::from(avg_rpe))))
            })
            .collect::<Vec<_>>();
        if !rir_values.is_empty() {
            reps_labels.push(ChartLabel {
                name: "RIR".to_string(),
                color: web_app::chart::COLOR_REPS_RIR,
                opacity: web_app::chart::OPACITY_AREA,
            });
            reps_data.push(web_app::chart::PlotData {
                values_high: rir_values,
                values_low: Some(reps_rpe_values.clone()),
                plots: web_app::chart::plot_area(web_app::chart::COLOR_REPS_RIR),
                params: web_app::chart::PlotParams::primary_range(0., 10.),
            });
        }
    }

    reps_data.push(web_app::chart::PlotData {
        values_high: reps_rpe_values,
        values_low: None,
        plots: web_app::chart::plot_line(web_app::chart::COLOR_REPS),
        params: web_app::chart::PlotParams::primary_range(0., 10.),
    });

    let theme = settings.current_theme();

    rsx! {
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Set volume".to_string(),
                    color: web_app::chart::COLOR_SET_VOLUME,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: set_volume.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(
                            web_app::chart::COLOR_SET_VOLUME,
                            web_app::chart::COLOR_SET_VOLUME,
                        ),
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
                    name: "Volume load".to_string(),
                    color: web_app::chart::COLOR_VOLUME_LOAD,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot(
                &[
                    web_app::chart::PlotData {
                        values_high: volume_load.into_iter().collect::<Vec<_>>(),
                        values_low: None,
                        plots: web_app::chart::plot_area_with_border(
                            web_app::chart::COLOR_VOLUME_LOAD,
                            web_app::chart::COLOR_VOLUME_LOAD,
                        ),
                        params: web_app::chart::PlotParams::primary_range(0., 10.),
                    }
                ],
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        if settings.show_tut {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "Time under tension (s)".to_string(),
                        color: web_app::chart::COLOR_TUT,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot(
                    &[
                        web_app::chart::PlotData {
                            values_high: tut.into_iter().collect::<Vec<_>>(),
                            values_low: None,
                            plots: web_app::chart::plot_area_with_border(
                                web_app::chart::COLOR_TUT,
                                web_app::chart::COLOR_TUT,
                            ),
                            params: web_app::chart::PlotParams::primary_range(0., 10.),
                        }
                    ],
                    interval,
                    theme,
                ).map_err(|err| err.to_string()),
                no_data_label: false,
            }
        }
        Chart {
            labels: reps_labels,
            chart: web_app::chart::plot(
                &reps_data,
                interval,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        Chart {
            labels: vec![
                ChartLabel {
                    name: "Weight (kg)".to_string(),
                    color: web_app::chart::COLOR_WEIGHT,
                    opacity: web_app::chart::OPACITY_AREA,
                },
                ChartLabel {
                    name: "Avg. Weight (kg)".to_string(),
                    color: web_app::chart::COLOR_WEIGHT,
                    opacity: web_app::chart::OPACITY_LINE,
                },
            ],
            chart: web_app::chart::plot_min_avg_max(
                &training_sessions
                    .iter()
                    .flat_map(|s| s
                        .elements
                        .iter()
                        .filter_map(|e| match e {
                            domain::TrainingSessionElement::Set { weight, .. } =>
                                weight.map(|w| (s.date, w)),
                            domain::TrainingSessionElement::Rest { .. } => None,
                        })
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                interval,
                web_app::chart::PlotParams::primary_range(0., 10.),
                web_app::chart::COLOR_WEIGHT,
                theme,
            ).map_err(|err| err.to_string()),
            no_data_label: false,
        }
        if settings.show_tut {
            Chart {
                labels: vec![
                    ChartLabel {
                        name: "Time (s)".to_string(),
                        color: web_app::chart::COLOR_TIME,
                        opacity: web_app::chart::OPACITY_AREA,
                    },
                    ChartLabel {
                        name: "Avg. time (s)".to_string(),
                        color: web_app::chart::COLOR_TIME,
                        opacity: web_app::chart::OPACITY_LINE,
                    },
                ],
                chart: web_app::chart::plot_min_avg_max(
                    &training_sessions
                        .iter()
                        .flat_map(|s| s
                            .elements
                            .iter()
                            .filter_map(|e| match e {
                                #[allow(clippy::cast_precision_loss)]
                                domain::TrainingSessionElement::Set { time, .. } =>
                                    time.map(|v| (s.date, u32::from(v) as f32)),
                                domain::TrainingSessionElement::Rest { .. } => None,
                            })
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                    interval,
                    web_app::chart::PlotParams::primary_range(0., 10.),
                    web_app::chart::COLOR_TIME,
                    theme,
                ).map_err(|err| err.to_string()),
                no_data_label: false,
            }
        }
    }
}

fn view_calendar(
    training_sessions: &[domain::TrainingSession],
    interval: domain::Interval,
) -> Element {
    let mut volume_load: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            volume_load
                .entry(training_session.date)
                .and_modify(|e| *e += training_session.volume_load())
                .or_insert(training_session.volume_load());
        }
    }
    let min = volume_load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let max = volume_load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0);
    let entries = volume_load
        .iter()
        .map(|(date, volume_load)| {
            (
                *date,
                web_app::chart::COLOR_VOLUME_LOAD,
                if max > min {
                    (f64::from(volume_load - min) / f64::from(max - min)) * 0.8 + 0.2
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

fn view_sets(
    training_sessions: &[domain::TrainingSession],
    routines: &[domain::Routine],
    settings: web_app::Settings,
) -> Element {
    let blocks = training_sessions.iter().rev().flat_map(|t| {
        let routine = routines.iter().find(|r| r.id == t.routine_id);
        let routine_id = routine.map(|r| r.id).unwrap_or_default();
        let sets = t.elements.iter().filter_map(|e| {
            if let domain::TrainingSessionElement::Set { .. } = e {
                Some(rsx! {
                    div {
                        NoWrap { {e.to_string(settings.show_tut, settings.show_rpe)} }
                    }
                })
            } else {
                None
            }
        });
        [
            rsx! {
                div {
                    class: "block has-text-centered has-text-weight-bold mb-2",
                    Link {
                        to: Route::TrainingSession { id: t.id },
                        NoWrap { "{t.date}" }
                    }
                    " "
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
                }
            },
            rsx! {
                div {
                    class: "block has-text-centered",
                    for s in sets {
                        {s}
                    }
                }
            },
        ]
    });

    rsx! {
        for b in blocks {
            {b}
        }
    }
}
