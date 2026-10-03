use dioxus::prelude::*;

use valens_domain::{self as domain, Property, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, Route,
    chart::{Chart, view_stretch_charts},
    page::training_sessions::without_stretch_only_sets,
    routing::{NavigatorScrollExt, TrainingSessionsTab},
    settings::Settings,
    ui::element::Error,
};

/// Show the muscle selector and the charts of all sessions or of the selected muscle.
pub fn view_charts(
    training_sessions: &[domain::TrainingSession],
    exercises: Result<&[domain::Exercise], &domain::ReadError>,
    stretch_only: &domain::StretchOnlyExercises,
    muscle: Option<domain::MuscleID>,
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let charts = match (muscle, exercises) {
        (None, _) => view_session_charts(training_sessions, stretch_only, interval, settings),
        (Some(muscle), Ok(exercises)) => {
            view_muscle_charts(training_sessions, exercises, stretch_only, muscle, interval)
        }
        (Some(_), Err(err)) => rsx! { Error { message: "{err}" } },
    };
    rsx! {
        {view_muscle_selector(muscle)}
        {charts}
    }
}

fn view_muscle_selector(muscle: Option<domain::MuscleID>) -> Element {
    let options = [(None, "All muscles")]
        .into_iter()
        .chain(domain::MuscleID::iter().map(|m| (Some(*m), m.name())));
    rsx! {
        div {
            class: "field is-flex is-justify-content-center",
            div {
                class: "control",
                div {
                    class: "select is-small",
                    select {
                        "data-testid": "muscle",
                        onchange: move |event: FormEvent| {
                            navigator().replace_preserving_scroll(Route::TrainingSessions {
                                tab: TrainingSessionsTab::Charts,
                                muscle: event.value().parse().ok(),
                                add: false,
                            });
                        },
                        for (value, name) in options {
                            option {
                                "data-testid": "muscle-option",
                                value: value.map(|m| (m as u8).to_string()).unwrap_or_default(),
                                selected: value == muscle,
                                "{name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn view_session_charts(
    training_sessions: &[domain::TrainingSession],
    stretch_only: &domain::StretchOnlyExercises,
    interval: domain::Interval,
    settings: Settings,
) -> Element {
    let metric_sessions = without_stretch_only_sets(training_sessions, stretch_only);
    let training_stats = DOMAIN_SERVICE().get_training_stats(training_sessions, stretch_only);
    let stretch_sets_per_session = training_sessions
        .iter()
        .map(|s| (s.date, s.stretch_sets(stretch_only)))
        .collect::<Vec<_>>();
    let has_stretch_sets = stretch_sets_per_session
        .iter()
        .any(|(date, sets)| (interval.first..=interval.last).contains(date) && *sets > 0.);
    let stretch_sets = domain::centered_moving_total(&stretch_sets_per_session, interval, 3);
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

/// Show the set volume of `muscle` and, if a stretch-only exercise stretches it in the interval,
/// its stretch, which counts the stretch of all exercises.
fn view_muscle_charts(
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
    stretch_only: &domain::StretchOnlyExercises,
    muscle: domain::MuscleID,
    interval: domain::Interval,
) -> Element {
    #[allow(clippy::cast_precision_loss)]
    let total_7day_set_volume = domain::centered_moving_total(
        &training_sessions
            .iter()
            .filter_map(|s| {
                s.stimulus_per_muscle(exercises)
                    .get(&muscle)
                    .map(|stimulus| (s.date, **stimulus as f32 / 100.))
            })
            .collect::<Vec<_>>(),
        interval,
        3,
    );
    let stretching_muscle = stretch_only.stretching(muscle, exercises);
    let has_stretch_sets = training_sessions.iter().any(|s| {
        (interval.first..=interval.last).contains(&s.date)
            && s.stretch_sets(&stretching_muscle) > 0.
    });
    let stretch_per_session = training_sessions
        .iter()
        .filter_map(|s| {
            s.stretch_per_muscle(exercises)
                .get(&muscle)
                .map(|stretch| (s.date, *stretch))
        })
        .collect::<Vec<_>>();
    let stretch_series = |value: fn(&domain::Stretch) -> f32| {
        domain::centered_moving_total(
            &stretch_per_session
                .iter()
                .map(|(date, stretch)| (*date, value(stretch)))
                .collect::<Vec<_>>(),
            interval,
            3,
        )
    };
    let description = muscle.description();

    rsx! {
        if !description.is_empty() {
            div {
                class: "block is-size-7 has-text-centered",
                "data-testid": "muscle-description",
                {description}
            }
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
                    params: web_app::chart::PlotParams::primary_range(0., 10.),
                },
            )],
            interval,
            no_data_label: true,
        }
        if has_stretch_sets {
            {view_stretch_charts(
                "7-day stretch sets",
                stretch_series(|s| s.sets),
                "7-day time under stretch (min)",
                stretch_series(|s| s.seconds / 60.),
                interval,
                true,
            )}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{Local, NaiveDate};
    use pretty_assertions::assert_eq;

    use crate::{
        page::training_sessions::TrainingSessions,
        test_render::{
            TestCache, all_attributes_of, all_text_of, contains, provide_settings, render, text_of,
        },
    };

    use super::*;

    fn render_charts(muscle: Option<u8>, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_settings(web_app::Settings::default());
            rsx! { TrainingSessions { tab: TrainingSessionsTab::Charts, muscle, add: false } }
        })
    }

    fn set(exercise_id: u128, reps: u32, time: u32) -> domain::TrainingSessionElement {
        domain::TrainingSessionElement::Set {
            exercise_id: exercise_id.into(),
            side: domain::Side::Unset,
            reps: domain::Reps::new(reps).unwrap(),
            time: domain::Time::new(time).unwrap(),
            weight: domain::Weight::default(),
            rpe: domain::RPE::new(8.0).unwrap(),
            target_reps: domain::Reps::default(),
            target_tempo: domain::Tempo::default(),
            target_weight: domain::Weight::default(),
            target_rpe: domain::RPE::ZERO,
            automatic: false,
        }
    }

    fn training_session(
        id: u128,
        date: NaiveDate,
        elements: Vec<domain::TrainingSessionElement>,
    ) -> domain::TrainingSession {
        domain::TrainingSession {
            id: id.into(),
            routine_id: 1.into(),
            date,
            notes: String::new(),
            elements,
            exercise_notes: BTreeMap::new(),
            exercise_measurements: BTreeMap::new(),
        }
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

    fn worked(muscle_id: domain::MuscleID) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus: domain::Stimulus::PRIMARY,
            stretch: domain::Stimulus::NONE,
        }
    }

    fn stretched(muscle_id: domain::MuscleID) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus: domain::Stimulus::NONE,
            stretch: domain::Stimulus::PRIMARY,
        }
    }

    /// A hybrid exercise 1 working the erector spinae and working and stretching the hamstrings,
    /// and a stretch-only exercise 2 stretching the hamstrings.
    fn exercises() -> Vec<domain::Exercise> {
        vec![
            exercise(
                1,
                vec![
                    worked(domain::MuscleID::ErectorSpinae),
                    domain::ExerciseMuscle {
                        muscle_id: domain::MuscleID::Hamstrings,
                        stimulus: domain::Stimulus::PRIMARY,
                        stretch: domain::Stimulus::PRIMARY,
                    },
                ],
            ),
            exercise(2, vec![stretched(domain::MuscleID::Hamstrings)]),
        ]
    }

    fn cache(training_sessions: Vec<domain::TrainingSession>) -> TestCache {
        TestCache::default()
            .with_exercises(exercises())
            .with_training_sessions(training_sessions)
    }

    /// Sessions of today and yesterday with a set of the hybrid and a hold of the stretch-only
    /// exercise.
    fn sessions_with_stretch() -> TestCache {
        let today = Local::now().date_naive();
        cache(vec![
            training_session(
                1,
                today - chrono::Duration::days(1),
                vec![set(1, 10, 30), set(2, 0, 90)],
            ),
            training_session(2, today, vec![set(1, 10, 30), set(2, 0, 90)]),
        ])
    }

    const HAMSTRINGS: u8 = domain::MuscleID::Hamstrings as u8;

    #[test]
    fn test_all_muscles_show_the_charts_of_all_sessions() {
        let html = render_charts(None, sessions_with_stretch);

        assert!(html.contains("Short-term load"));
        assert!(html.contains("7-day set volume"));
        assert!(html.contains("7-day stretch sets"));
        assert!(html.contains("7-day avg. RPE"));
        assert!(!contains(&html, "muscle-description"));
    }

    #[test]
    fn test_the_selector_lists_all_muscles_followed_by_every_muscle() {
        let html = render_charts(None, sessions_with_stretch);

        assert_eq!(
            all_text_of(&html, "muscle-option"),
            ["All muscles"]
                .into_iter()
                .chain(domain::MuscleID::iter().map(|m| m.name()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_a_selected_muscle_shows_its_description_and_charts() {
        let html = render_charts(Some(HAMSTRINGS), sessions_with_stretch);

        assert_eq!(
            text_of(&html, "muscle-description"),
            domain::MuscleID::Hamstrings.description()
        );
        assert!(html.contains("7-day set volume"));
        assert!(html.contains("7-day stretch sets"));
        assert!(html.contains("7-day time under stretch (min)"));
        assert!(!html.contains("Short-term load"));
        assert!(!html.contains("RPE"));
        let selected = all_attributes_of(&html, "muscle-option", "selected");
        assert_eq!(
            selected.iter().position(|s| s == "true"),
            domain::MuscleID::iter()
                .position(|m| *m == domain::MuscleID::Hamstrings)
                .map(|i| i + 1)
        );
    }

    #[test]
    fn test_a_selected_muscle_without_stretch_sets_in_the_interval_shows_no_stretch_charts() {
        let html = render_charts(Some(HAMSTRINGS), || {
            let today = Local::now().date_naive();
            cache(vec![
                training_session(1, today - chrono::Duration::days(365), vec![set(2, 0, 90)]),
                training_session(2, today, vec![set(1, 10, 30)]),
            ])
        });

        assert!(html.contains("7-day set volume"));
        assert!(!html.contains("7-day stretch"));
    }

    #[test]
    fn test_a_selected_muscle_stretched_only_by_a_hybrid_shows_no_stretch_charts() {
        let html = render_charts(Some(HAMSTRINGS), || {
            cache(vec![training_session(
                1,
                Local::now().date_naive(),
                vec![set(1, 10, 30)],
            )])
        });

        assert!(html.contains("7-day set volume"));
        assert!(!html.contains("7-day stretch"));
    }

    #[test]
    fn test_an_unknown_muscle_shows_the_charts_of_all_sessions() {
        let html = render_charts(Some(u8::MAX), sessions_with_stretch);

        assert!(html.contains("Short-term load"));
        assert_eq!(
            all_attributes_of(&html, "muscle-option", "selected")[0],
            "true"
        );
    }

    #[test]
    fn test_a_selected_muscle_with_unreadable_exercises_shows_the_error() {
        let html = render_charts(Some(HAMSTRINGS), || {
            sessions_with_stretch().with_unreadable_exercises()
        });

        assert!(html.contains("No connection"));
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_all_muscles_with_unreadable_exercises_show_the_charts_of_all_sessions() {
        let html = render_charts(None, || sessions_with_stretch().with_unreadable_exercises());

        assert!(html.contains("Short-term load"));
        assert!(!html.contains("7-day stretch"));
    }
}
