use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

use chrono::{Local, NaiveDate};
use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    chart::{Calendar, Chart, IntervalControl, view_stretch_charts},
    eh,
    notification::{notify, notify_error},
    routing::NavigatorScrollExt,
    settings::Settings,
    ui::{
        element::{
            DeleteConfirmationDialog, ErrorPage, FloatingActionButton, Icon, LoadingPage, NoWrap,
            SaveDialog, Table, count_to_string, minutes_to_string, value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn TrainingSessions(add: bool) -> Element {
    let cache = consume_context::<Cache>();
    let dates = use_memo(move || {
        if let CacheState::Ready(training_session) = &*cache.training_sessions.read() {
            training_session
                .iter()
                .map(|ts| ts.date)
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    });
    let current_interval =
        use_signal(|| domain::init_interval(&dates.read(), domain::DefaultInterval::_1M));
    let all = *use_memo(move || domain::Interval {
        first: dates.read().iter().min().copied().unwrap_or_default(),
        last: dates.read().iter().max().copied().unwrap_or_default(),
    })
    .read();
    let settings = use_context::<Settings>();
    let mut dialog = use_signal(|| TrainingDialog::None);

    let show_add_dialog = move || async move {
        let mut date = FieldValue::new(Local::now().date_naive());
        date.validated = DOMAIN_SERVICE()
            .validate_training_session_date(&date.input)
            .map_err(|err| err.to_string());
        dialog.set(TrainingDialog::Add {
            date,
            routine_id: FieldValue {
                input: String::new(),
                validated: Ok(domain::RoutineID::nil()),
                orig: String::new(),
            },
        });
        navigator().replace_preserving_scroll(Route::TrainingSessions { add: true });
    };

    use_future(move || async move {
        if add {
            show_add_dialog().await;
        }
    });

    match (
        &*cache.training_sessions.read(),
        &*cache.routines.read(),
        stretch_only_exercises(&cache.exercises.read()),
    ) {
        (CacheState::Ready(training_sessions), CacheState::Ready(routines), Some(stretch_only)) => {
            let interval = *current_interval.read();
            let metric_sessions = without_stretch_only_sets(training_sessions, &stretch_only);
            rsx! {
                IntervalControl { current_interval, all }
                {view_charts(training_sessions, &metric_sessions, &stretch_only, interval, settings)}
                {view_calendar(training_sessions, &stretch_only, interval)}
                {view_table(training_sessions, &stretch_only, StretchMetrics::Exclude(&metric_sessions), settings.show_tut(), routines, interval, dialog, settings)}
                {view_dialog(dialog, training_sessions, routines, Some(Route::TrainingSessions { add: false }))}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    on_click: move |_| { show_add_dialog() },
                }
            }
        }
        (CacheState::Error(err), _, _) | (_, CacheState::Error(err), _) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        _ => {
            rsx! { LoadingPage {} }
        }
    }
}

/// The stretch-only exercises, or `None` while the exercises are loading.
///
/// If the exercises cannot be read, no exercise is considered stretch-only.
pub fn stretch_only_exercises(
    exercises: &CacheState<Vec<domain::Exercise>>,
) -> Option<domain::StretchOnlyExercises> {
    match exercises {
        CacheState::Ready(exercises) => Some(domain::StretchOnlyExercises::new(exercises)),
        CacheState::Error(_) => Some(domain::StretchOnlyExercises::default()),
        CacheState::Loading => None,
    }
}

/// The sessions without the sets of stretch-only exercises, in the same order.
pub fn without_stretch_only_sets<'a>(
    training_sessions: &'a [domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
) -> Vec<Cow<'a, domain::TrainingSession>> {
    training_sessions
        .iter()
        .map(|t| t.without_stretch_only_sets(stretch_only))
        .collect()
}

fn view_charts(
    training_sessions: &[domain::TrainingSession],
    metric_sessions: &[Cow<'_, domain::TrainingSession>],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let training_stats = DOMAIN_SERVICE().get_training_stats(training_sessions, stretch_only);
    let has_stretch_sets = training_sessions.iter().any(|s| {
        (interval.first..=interval.last).contains(&s.date) && s.stretch_sets(stretch_only) > 0.
    });
    let stretch_sets = domain::centered_moving_total(
        &training_sessions
            .iter()
            .map(|s| (s.date, s.stretch_sets(stretch_only)))
            .collect::<Vec<_>>(),
        interval,
        3,
    );
    let stretch_minutes = domain::centered_moving_total(
        &training_sessions
            .iter()
            .map(|s| (s.date, s.stretch_time(stretch_only) / 60.))
            .collect::<Vec<_>>(),
        interval,
        3,
    );
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
    let total_7day_set_volume = domain::centered_moving_total(
        &metric_sessions
            .iter()
            .map(|s| (s.date, s.set_volume()))
            .collect::<Vec<_>>(),
        interval,
        3,
    );
    let rpe_values = metric_sessions
        .iter()
        .flat_map(|s| {
            s.elements
                .iter()
                .filter_map(|e| match e {
                    domain::TrainingSessionElement::Set { rpe, .. } => {
                        rpe.non_zero().map(|v| (s.date, v))
                    }
                    domain::TrainingSessionElement::Rest { .. } => None,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let rpe_params = web_app::chart::PlotParams::primary_range(5., 10.);
    let avg_rpe_segments: Vec<web_app::chart::PlotData> =
        domain::centered_moving_average(&rpe_values, interval, 3)
            .into_iter()
            .map(|seg| web_app::chart::PlotData {
                values_high: seg,
                values_low: None,
                plots: web_app::chart::plot_line(web_app::chart::COLOR_RPE),
                params: rpe_params,
            })
            .collect();
    let min_max_rpe_segments: Vec<web_app::chart::PlotData> =
        domain::centered_moving_min(&rpe_values, interval, 3)
            .into_iter()
            .zip(domain::centered_moving_max(&rpe_values, interval, 3))
            .map(|(min_seg, max_seg)| web_app::chart::PlotData {
                values_high: min_seg,
                values_low: Some(max_seg),
                plots: web_app::chart::plot_area(web_app::chart::COLOR_RPE),
                params: rpe_params,
            })
            .collect();

    let load_params = web_app::chart::PlotParams::primary_range(0., 10.);
    let show_rpe = settings.show_rpe();

    rsx! {
        Chart {
            series: vec![
                web_app::chart::LabeledSeries::new(
                    "Short-term load",
                    web_app::chart::PlotData {
                        values_high: short_term_load,
                        values_low: None,
                        plots: web_app::chart::plot_line(web_app::chart::COLOR_LOAD),
                        params: load_params,
                    },
                ),
                web_app::chart::LabeledSeries::new(
                    "Long-term load",
                    web_app::chart::PlotData {
                        values_high: long_term_load_high,
                        values_low: Some(long_term_load_low),
                        plots: web_app::chart::plot_area(web_app::chart::COLOR_LONG_TERM_LOAD),
                        params: load_params,
                    },
                ),
            ],
            interval,
            no_data_label: false,
        }
        Chart {
            series: vec![web_app::chart::LabeledSeries::new(
                "7-day set volume",
                web_app::chart::PlotData {
                    values_high: total_7day_set_volume,
                    values_low: None,
                    plots: web_app::chart::plot_area_with_border(
                        web_app::chart::COLOR_SET_VOLUME,
                    ),
                    params: load_params,
                },
            )],
            interval,
            no_data_label: false,
        }
        if show_rpe {
            Chart {
                series: vec![
                    web_app::chart::LabeledSeries::new("7-day avg. RPE", avg_rpe_segments),
                    web_app::chart::LabeledSeries::new("7-day min./max. RPE", min_max_rpe_segments),
                ],
                interval,
                no_data_label: false,
            }
        }
        if has_stretch_sets {
            {view_stretch_charts(
                "7-day stretch sets",
                stretch_sets,
                "7-day time under stretch (min)",
                stretch_minutes,
                interval,
                false,
            )}
        }
    }
}

pub fn view_calendar(
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
) -> Element {
    let mut load: BTreeMap<NaiveDate, f32> = BTreeMap::new();
    for training_session in training_sessions {
        if (interval.first..=interval.last).contains(&training_session.date) {
            let session_load = training_session.load(stretch_only);
            load.entry(training_session.date)
                .and_modify(|e| *e += session_load)
                .or_insert(session_load);
        }
    }
    let min = load
        .values()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
    let max = load
        .values()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or_default();
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

/// How the session table treats the sets of stretch-only exercises.
#[derive(Clone, Copy)]
pub enum StretchMetrics<'a> {
    /// The volume metrics are taken from the given sessions, which leave out the sets and
    /// correspond to the shown sessions. The sets are shown in stretch columns instead.
    Exclude(&'a [Cow<'a, domain::TrainingSession>]),
    /// The sets count like any other set.
    Include,
    /// The sets are all stretch-only, so a stretch sets column replaces the volume metrics.
    Only,
}

/// Show a row per training session.
///
/// `show_time` decides whether the TUT and the time columns are shown.
#[allow(clippy::too_many_arguments)]
pub fn view_table(
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    stretch_metrics: StretchMetrics<'_>,
    show_time: bool,
    routines: &[domain::Routine],
    interval: domain::Interval,
    mut dialog: Signal<TrainingDialog>,
    settings: Settings,
) -> Element {
    let borrowed_sessions;
    let metric_sessions = if let StretchMetrics::Exclude(metric_sessions) = stretch_metrics {
        metric_sessions
    } else {
        borrowed_sessions = training_sessions
            .iter()
            .map(Cow::Borrowed)
            .collect::<Vec<_>>();
        &borrowed_sessions
    };
    let shows_volume = !matches!(stretch_metrics, StretchMetrics::Only);
    let shows_stretch_sets = match stretch_metrics {
        StretchMetrics::Exclude(_) => training_sessions.iter().any(|t| {
            (interval.first..=interval.last).contains(&t.date) && t.stretch_sets(stretch_only) > 0.
        }),
        StretchMetrics::Include => false,
        StretchMetrics::Only => true,
    };
    let shows_stretch_time =
        matches!(stretch_metrics, StretchMetrics::Exclude(_)) && shows_stretch_sets;
    let (has_avg_rpe_data, has_tut_data, has_avg_reps_data, has_avg_weight_data, has_avg_time_data) =
        metric_sessions
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
    head.extend_from_slice(&[rsx! {"Date"}, rsx! {"Routine"}, rsx! {"Load"}]);
    if shows_volume {
        head.push(rsx! {"Set volume"});
    }
    if settings.show_rpe() && has_avg_rpe_data {
        head.push(rsx! {"RPE"});
    }
    if shows_volume {
        head.push(rsx! {"Volume load"});
    }
    if show_time && has_tut_data {
        head.push(rsx! {"TUT"});
    }
    if has_avg_reps_data {
        head.push(rsx! {"Reps"});
    }
    if settings.show_rpe() && has_avg_reps_data && has_avg_rpe_data {
        head.push(rsx! {"Reps+RIR"});
    }
    if has_avg_weight_data {
        head.push(rsx! {"Weight (kg)"});
    }
    if show_time && has_avg_time_data {
        head.push(rsx! {"Time (s)"});
    }
    if shows_stretch_sets {
        head.push(rsx! {"Stretch sets"});
    }
    if shows_stretch_time {
        head.push(rsx! {"Stretch (min)"});
    }
    head.push(rsx! {});

    let body = training_sessions
        .iter()
        .zip(metric_sessions)
        .rev()
        .filter(|(p, _)| p.date >= interval.first && p.date <= interval.last)
        .map(|(t, m)| {
            let t = t.clone();
            let routine = routines.iter().find(|r| r.id == t.routine_id);
            let routine_id = routine.map(|r| r.id).unwrap_or_default();
            let mut data = Vec::with_capacity(12);
            data.push(rsx! {
                Link {
                    to: Route::TrainingSession { id: t.id },
                    NoWrap { "{t.date}" }
                }
            });
            data.push(rsx! {
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
            });
            data.push(rsx! { {count_to_string(f64::from(t.load(stretch_only)))} });
            if shows_volume {
                data.push(rsx! { {count_to_string(f64::from(m.set_volume()))} });
            }
            if settings.show_rpe() && has_avg_rpe_data {
                data.push(rsx! { {value_or_dash(m.avg_rpe())} });
            }
            if shows_volume {
                data.push(rsx! { "{m.volume_load()}" });
            }
            if show_time && has_tut_data {
                data.push(rsx! { {value_or_dash(m.tut())} });
            }
            if has_avg_reps_data {
                data.push(rsx! { {value_or_dash(m.avg_reps())} });
            }
            if settings.show_rpe() && has_avg_reps_data && has_avg_rpe_data {
                data.push(rsx! {
                    if let (Some(avg_reps), Some(avg_rpe)) = (m.avg_reps(), m.avg_rpe()) {
                        {format!("{:.1}", avg_reps + f32::from(domain::RIR::from(avg_rpe)))}
                    } else {
                        "-"
                    }
                });
            }
            if has_avg_weight_data {
                data.push(rsx! { {value_or_dash(m.avg_weight())} });
            }
            if show_time && has_avg_time_data {
                data.push(rsx! { {value_or_dash(m.avg_time())} });
            }
            if shows_stretch_sets {
                data.push(rsx! { {count_to_string(f64::from(t.stretch_sets(stretch_only)))} });
            }
            if shows_stretch_time {
                data.push(rsx! { {minutes_to_string(t.stretch_time(stretch_only))} });
            }
            data.push(rsx! {
                a {
                    class: "mx-2",
                    "data-testid": "item-delete",
                    onclick: move |_| { *dialog.write() = TrainingDialog::Delete(t.clone()); },
                    Icon { name: "xmark"}
                }
            });
            data
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

pub fn view_dialog(
    mut dialog: Signal<TrainingDialog>,
    training_sessions: &[domain::TrainingSession],
    routines: &[domain::Routine],
    closed_dialog_route: Option<Route>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let close_dialog = move || {
        dialog.set(TrainingDialog::None);
        if let Some(route) = closed_dialog_route {
            navigator().replace_preserving_scroll(route);
        }
    };

    let save = move |_| async move {
        is_loading! {
            if let TrainingDialog::Add { date, routine_id } = &*dialog.read()
                && let (Ok(date), Ok(routine_id)) = (date.validated.clone(), routine_id.validated.clone()) {
                    match DOMAIN_SERVICE().get_routines().await {
                        Ok(routines) => {
                            let routine = routines.iter().find(|r| r.id == routine_id);
                            if routine.is_none() && !routine_id.is_nil() {
                                notify_error("add training session", "unknown routine");
                            } else {
                                start_training_session(routine, date).await;
                            }
                        }
                        Err(err) => {
                            notify("add training session", &err);
                        }
                    }
                }
        }
    };
    let delete = eh!(close_dialog; {
        async move {
            let mut deleted = false;
            is_loading! {
                if let TrainingDialog::Delete(training_session) = &*dialog.read() {
                    let id = training_session.id;
                    match DOMAIN_SERVICE().delete_training_session(id).await {
                        Ok(()) => {
                            deleted = true;
                            consume_context::<Cache>().refresh_training_sessions();
                        },
                        Err(err) => notify("delete training session", &err)
                    }
                }
            }
            if deleted {
                close_dialog();
            }
        }
    });
    let close = move |_| {
        close_dialog.clone()();
    };

    match &*dialog.read() {
        TrainingDialog::None => rsx! {},
        TrainingDialog::Add { date, routine_id } => {
            rsx! {
                SaveDialog {
                    title: rsx! { "Add training session" },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: !date.valid() || !routine_id.valid(),
                    InputField {
                        label: "Date".to_string(),
                        "data-testid": "date",
                        r#type: "date".to_string(),
                        max: Local::now().date_naive().to_string(),
                        value: date.input.clone(),
                        error: if let Err(err) = &date.validated { err.clone() },
                        has_changed: date.changed(),
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            let validated_date = DOMAIN_SERVICE()
                                .validate_training_session_date(&input)
                                .map_err(|err| err.to_string());
                            if let TrainingDialog::Add { date, .. } = &mut *dialog.write() {
                                date.input = input;
                                date.validated = validated_date;
                            }
                        },
                    }
                    SelectField {
                        label: "Routine".to_string(),
                        "data-testid": "routine",
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
                        on_change: move |event: FormEvent| {
                            if let TrainingDialog::Add { routine_id, .. } = &mut *dialog.write() {
                                routine_id.input = event.value();
                                routine_id.validated = Ok(domain::RoutineID::from_str(&routine_id.input).unwrap_or(domain::RoutineID::nil()));
                            }
                        }
                    }
                }
            }
        }
        TrainingDialog::Delete(training_session) => rsx! {
            DeleteConfirmationDialog {
                element_type: "training session".to_string(),
                element_name: rsx! { span { "of " NoWrap { "{training_session.date}" } } },
                on_delete: delete,
                on_cancel: close,
                is_loading: is_loading(),
            }
        },
    }
}

/// Creates a training session on `date`, prefilled with the elements of `routine`, and navigates
/// to it.
pub async fn start_training_session(routine: Option<&domain::Routine>, date: NaiveDate) {
    let elements = routine
        .map(|routine| {
            routine
                .sections
                .iter()
                .flat_map(domain::RoutinePart::to_training_session_elements)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match DOMAIN_SERVICE()
        .create_training_session(
            routine.map_or_else(domain::RoutineID::nil, |routine| routine.id),
            date,
            String::new(),
            elements,
        )
        .await
    {
        Ok(training_session) => {
            let id = training_session.id;
            consume_context::<Cache>().load_training_sessions().await;
            navigator().push(Route::TrainingSession { id });
        }
        Err(err) => {
            notify("add training session", &err);
        }
    }
}

pub enum TrainingDialog {
    None,
    Add {
        date: FieldValue<NaiveDate>,
        routine_id: FieldValue<domain::RoutineID>,
    },
    Delete(domain::TrainingSession),
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{TestCache, contains, provide_settings, render, rows_of, text_of};

    use super::*;

    fn training_session(id: u128, date: NaiveDate) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date,
            notes: String::new(),
            elements: vec![domain::TrainingSessionElement::Set {
                exercise_id: 1.into(),
                side: domain::Side::Unset,
                reps: domain::Reps::new(10).unwrap(),
                time: domain::Time::new(30).unwrap(),
                weight: domain::Weight::default(),
                rpe: domain::RPE::new(8.0).unwrap(),
                target_reps: domain::Reps::default(),
                target_tempo: domain::Tempo::default(),
                target_weight: domain::Weight::default(),
                target_rpe: domain::RPE::ZERO,
                automatic: false,
            }],
            exercise_notes: BTreeMap::new(),
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    fn routine() -> domain::Routine {
        domain::Routine {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
            notes: String::new(),
            archived: false,
            sections: vec![],
        }
    }

    fn sessions() -> TestCache {
        let today = Local::now().date_naive();
        TestCache::default()
            .with_routines(vec![routine()])
            .with_training_sessions(vec![
                training_session(1, today - chrono::Duration::days(1)),
                training_session(2, today),
            ])
    }

    fn render_training_sessions(
        settings: web_app::Settings,
        cache: impl Fn() -> TestCache + 'static,
    ) -> String {
        render(move || {
            cache().provide();
            provide_settings(settings);
            rsx! { TrainingSessions { add: false } }
        })
    }

    #[test]
    fn test_sessions_are_tabulated_newest_first_with_their_routine() {
        let html = render_training_sessions(web_app::Settings::default(), sessions);
        let today = Local::now().date_naive();

        let rows = rows_of(&html, "table");
        assert_eq!(rows[1][0], today.to_string());
        assert_eq!(rows[1][1], "A");
        assert_eq!(rows[2][0], (today - chrono::Duration::days(1)).to_string());
    }

    #[test]
    fn test_the_rpe_and_tut_columns_follow_the_settings() {
        let with_both = render_training_sessions(web_app::Settings::default(), sessions);
        let without = render_training_sessions(
            web_app::Settings {
                show_rpe: false,
                show_tut: false,
                ..web_app::Settings::default()
            },
            sessions,
        );

        assert!(rows_of(&with_both, "table")[0].contains(&"RPE".to_string()));
        assert!(rows_of(&with_both, "table")[0].contains(&"TUT".to_string()));
        assert!(!rows_of(&without, "table")[0].contains(&"RPE".to_string()));
        assert!(!rows_of(&without, "table")[0].contains(&"TUT".to_string()));
    }

    fn exercise(id: u128, muscles: Vec<domain::ExerciseMuscle>) -> domain::Exercise {
        domain::Exercise {
            id: id.into(),
            name: domain::Name::new(&format!("Exercise {id}")).unwrap(),
            notes: String::new(),
            muscles,
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }

    fn muscle(
        muscle_id: domain::MuscleID,
        stimulus: domain::Stimulus,
        stretch: domain::Stimulus,
    ) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus,
            stretch,
        }
    }

    /// A session of today holding a hybrid exercise 1 and a hold of a stretch-only exercise 2.
    fn sessions_with_stretch() -> TestCache {
        stretch_cache(vec![session_with_stretch(1, Local::now().date_naive())])
    }

    fn session_with_stretch(id: u128, date: NaiveDate) -> domain::TrainingSession {
        let mut session = training_session(id, date);
        session.elements.push(domain::TrainingSessionElement::Set {
            exercise_id: 2.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::default(),
            time: domain::Time::new(90).unwrap(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::new(9.0).unwrap(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        });
        session
    }

    /// The `training_sessions` with a hybrid exercise 1 and a stretch-only exercise 2.
    fn stretch_cache(training_sessions: Vec<domain::TrainingSession>) -> TestCache {
        TestCache::default()
            .with_routines(vec![routine()])
            .with_exercises(vec![
                exercise(
                    1,
                    vec![
                        muscle(
                            domain::MuscleID::ErectorSpinae,
                            domain::Stimulus::PRIMARY,
                            domain::Stimulus::NONE,
                        ),
                        muscle(
                            domain::MuscleID::Hamstrings,
                            domain::Stimulus::NONE,
                            domain::Stimulus::PRIMARY,
                        ),
                    ],
                ),
                exercise(
                    2,
                    vec![muscle(
                        domain::MuscleID::Hamstrings,
                        domain::Stimulus::NONE,
                        domain::Stimulus::PRIMARY,
                    )],
                ),
            ])
            .with_training_sessions(training_sessions)
    }

    #[test]
    fn test_a_stretch_leaves_the_volume_metrics_for_stretch_columns_and_a_chart() {
        let html = render_training_sessions(web_app::Settings::default(), sessions_with_stretch);
        let rows = rows_of(&html, "table");

        assert_eq!(
            rows[0],
            vec![
                "Date",
                "Routine",
                "Load",
                "Set volume",
                "RPE",
                "Volume load",
                "TUT",
                "Reps",
                "Reps+RIR",
                "Time (s)",
                "Stretch sets",
                "Stretch (min)",
                "",
            ]
        );
        assert_eq!(
            rows[1][2..],
            [
                "9", "1", "8", "10", "300", "10.0", "12.0", "30.0", "1", "1.5", ""
            ]
        );
        assert!(html.contains("7-day time under stretch (min)"));
    }

    #[test]
    fn test_a_stretch_outside_the_interval_shows_no_stretch_columns_and_chart() {
        let html = render_training_sessions(web_app::Settings::default(), || {
            let today = Local::now().date_naive();
            stretch_cache(vec![
                session_with_stretch(1, today - chrono::Duration::days(365)),
                training_session(2, today),
            ])
        });

        assert!(!rows_of(&html, "table")[0].contains(&"Stretch sets".to_string()));
        assert!(!html.contains("7-day stretch"));
    }

    #[test]
    fn test_with_unreadable_exercises_stretches_count_like_other_sets() {
        let html = render_training_sessions(web_app::Settings::default(), || {
            sessions_with_stretch().with_unreadable_exercises()
        });
        let rows = rows_of(&html, "table");

        assert!(!rows[0].contains(&"Stretch sets".to_string()));
        assert_eq!(rows[1][3], "2");
    }

    #[test]
    fn test_without_stretch_sets_no_stretch_columns_and_chart_are_shown() {
        let html = render_training_sessions(web_app::Settings::default(), sessions);

        assert!(!rows_of(&html, "table")[0].contains(&"Stretch sets".to_string()));
        assert!(!html.contains("7-day stretch"));
    }

    #[test]
    fn test_without_sessions_no_chart_is_shown() {
        let html = render_training_sessions(web_app::Settings::default(), TestCache::default);

        assert!(!contains(&html, "chart"));
        assert_eq!(rows_of(&html, "table").len(), 1);
    }

    #[test]
    fn test_unread_sessions_are_shown_as_loading() {
        let html = render_training_sessions(web_app::Settings::default(), TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_sessions_are_shown_as_an_error() {
        let html = render_training_sessions(web_app::Settings::default(), TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }
}
