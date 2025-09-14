use std::collections::BTreeMap;

use dioxus::prelude::*;

use valens_domain::{
    self as domain, ExerciseService, Property, SessionService, TrainingSessionService,
};
use valens_web_app::{self as web_app, Settings, SettingsService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, Route, WEB_APP_SERVICE,
    component::element::{
        Chart, ChartLabel, ErrorMessage, IntervalControl, LoadingPage, NoConnection, Title,
    },
    ensure_session,
};

#[component]
pub fn Muscles() -> Element {
    ensure_session!();

    let training_sessions = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_training_sessions().await
    });
    let exercises = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_exercises().await
    });
    let dates = use_memo(move || {
        if let Some(Ok(training_session)) = &*training_sessions.read() {
            training_session
                .iter()
                .map(|bw| bw.date)
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

    match (
        &*training_sessions.read(),
        &*exercises.read(),
        &*settings.read(),
    ) {
        (Some(Ok(training_sessions)), Some(Ok(exercises)), Some(Ok(settings))) => {
            rsx! {
                IntervalControl { current_interval, all }
                {charts(training_sessions, exercises, *current_interval.read(), *settings)}
            }
        }
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _, _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _, _) | (_, Some(Err(err)), _) => {
            rsx! { ErrorMessage { message: err } }
        }
        (_, _, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _, _) | (_, None, _) | (_, _, None) => rsx! { LoadingPage {} },
    }
}

fn charts(
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let exercises = exercises
        .iter()
        .map(|e| (e.id, e.clone()))
        .collect::<BTreeMap<_, _>>();
    let charts = domain::MuscleID::iter().map(|m| {
        #[allow(clippy::cast_precision_loss)]
        let total_7day_set_volume = domain::centered_moving_total(
            &training_sessions
                .iter()
                .filter_map(|s| {
                    s.stimulus_per_muscle(&exercises)
                        .get(m)
                        .map(|stimulus| (s.date, **stimulus as f32 / 100.))
                })
                .collect::<Vec<_>>(),
            interval,
            3,
        );

        rsx! {
            div {
                Title {
                    title: m.name(),
                    y_margin: 1,
                }
                div {
                    class: "block is-size-7 has-text-centered",
                    {m.description()}
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
                        &[
                            web_app::chart::PlotData {
                                values_high: total_7day_set_volume,
                                values_low: None,
                                plots: web_app::chart::plot_area_with_border(
                                    web_app::chart::COLOR_SET_VOLUME,
                                    web_app::chart::COLOR_SET_VOLUME,
                                ),
                                params: web_app::chart::PlotParams::primary_range(0., 10.),
                            }
                        ],
                        interval,
                        settings.current_theme(),
                    ).map_err(|err| err.to_string()),
                    no_data_label: true,
                }
            }
        }
    });

    rsx! {
        for chart in charts {
            {chart}
        }
    }
}
