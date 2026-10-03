use dioxus::prelude::*;

use valens_domain::{self as domain, ExerciseService, Property, TrainingSessionService};
use valens_web_app as web_app;

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE,
    chart::{Chart, IntervalControl, view_stretch_charts},
    ui::element::{ErrorPage, LoadingPage, Title},
};

#[component]
pub fn Muscles() -> Element {
    let training_sessions = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE().get_training_sessions().await
    });
    let exercises = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE().get_exercises().await
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
    let stretched = use_signal(|| false);
    match (&*training_sessions.read(), &*exercises.read()) {
        (Some(Ok(training_sessions)), Some(Ok(exercises))) => {
            let stretch_only = domain::StretchOnlyExercises::new(exercises);
            let has_stretch_sets = training_sessions
                .iter()
                .any(|s| s.stretch_sets(&stretch_only) > 0.);
            let interval = *current_interval.read();
            rsx! {
                IntervalControl { current_interval, all }
                if has_stretch_sets {
                    {view_kind_control(stretched)}
                }
                if has_stretch_sets && stretched() {
                    {stretch_charts(training_sessions, exercises, interval)}
                } else {
                    {charts(training_sessions, exercises, interval)}
                }
            }
        }
        (Some(Err(err)), _) | (_, Some(Err(err))) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        (None, _) | (_, None) => rsx! { LoadingPage {} },
    }
}

/// Switch between the charts of the worked and the stretched muscles.
fn view_kind_control(mut stretched: Signal<bool>) -> Element {
    let kinds = [("Worked", false), ("Stretched", true)];
    rsx! {
        div {
            class: "field has-addons has-addons-centered",
            for (name, value) in kinds {
                p {
                    class: "control",
                    a {
                        class: "button is-small",
                        class: if stretched() == value { "is-link" },
                        "data-testid": "muscle-kind-{name.to_lowercase()}",
                        "data-selected": "{stretched() == value}",
                        onclick: move |_| stretched.set(value),
                        "{name}"
                    }
                }
            }
        }
    }
}

fn stretch_charts(
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
    interval: domain::Interval,
) -> Element {
    let stretch_per_session = training_sessions
        .iter()
        .map(|s| (s.date, s.stretch_per_muscle(exercises)))
        .collect::<Vec<_>>();
    let charts = domain::MuscleID::iter().map(|m| {
        let series = |value: fn(&domain::Stretch) -> f32| {
            domain::centered_moving_total(
                &stretch_per_session
                    .iter()
                    .filter_map(|(date, stretch)| stretch.get(m).map(|s| (*date, value(s))))
                    .collect::<Vec<_>>(),
                interval,
                3,
            )
        };
        rsx! {
            div {
                Title {
                    {m.name()}
                }
                div {
                    class: "block is-size-7 has-text-centered",
                    {m.description()}
                }
                {view_stretch_charts(
                    "7-day stretch sets",
                    series(|s| s.sets),
                    "7-day time under stretch (min)",
                    series(|s| s.seconds / 60.),
                    interval,
                    true,
                )}
            }
        }
    });

    rsx! {
        for chart in charts {
            {chart}
        }
    }
}

fn charts(
    training_sessions: &[domain::TrainingSession],
    exercises: &[domain::Exercise],
    interval: domain::Interval,
) -> Element {
    let charts = domain::MuscleID::iter().map(|m| {
        #[allow(clippy::cast_precision_loss)]
        let total_7day_set_volume = domain::centered_moving_total(
            &training_sessions
                .iter()
                .filter_map(|s| {
                    s.stimulus_per_muscle(exercises)
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
                    {m.name()}
                }
                div {
                    class: "block is-size-7 has-text-centered",
                    {m.description()}
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
            }
        }
    });

    rsx! {
        for chart in charts {
            {chart}
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        all_text_of, attribute_of, contains, provide_settings, render_settled, seed_domain_service,
        text_of,
    };

    use super::*;

    fn render_muscles(repository: domain::tests::FakeRepository) -> String {
        render_settled(move || {
            seed_domain_service(repository.clone());
            provide_settings(web_app::Settings::default());
            rsx! { Muscles {} }
        })
    }

    #[test]
    fn test_every_muscle_is_shown_with_its_description() {
        let html = render_muscles(domain::tests::FakeRepository::default());

        assert_eq!(
            all_text_of(&html, "title"),
            domain::MuscleID::iter()
                .map(|muscle| muscle.name().to_string())
                .collect::<Vec<_>>()
        );
    }

    fn exercise(muscles: Vec<domain::ExerciseMuscle>) -> domain::Exercise {
        domain::Exercise {
            id: 1.into(),
            name: domain::Name::new("A").unwrap(),
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

    fn stretched(muscle_id: domain::MuscleID) -> domain::ExerciseMuscle {
        domain::ExerciseMuscle {
            muscle_id,
            stimulus: domain::Stimulus::NONE,
            stretch: domain::Stimulus::PRIMARY,
        }
    }

    fn session_with_hold() -> domain::TrainingSession {
        domain::TrainingSession {
            id: 1.into(),
            routine_id: 1.into(),
            date: chrono::Local::now().date_naive(),
            notes: String::new(),
            elements: vec![domain::TrainingSessionElement::Set {
                exercise_id: 1.into(),
                side: domain::Side::Unset,
                reps: domain::Reps::default(),
                time: domain::Time::new(60).unwrap(),
                weight: domain::Weight::default(),
                rpe: domain::RPE::ZERO,
                target_reps: domain::Reps::default(),
                target_tempo: domain::Tempo::default(),
                target_weight: domain::Weight::default(),
                target_rpe: domain::RPE::ZERO,
                automatic: false,
            }],
            exercise_notes: std::collections::BTreeMap::new(),
            exercise_measurements: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn test_with_stretch_sets_the_worked_charts_are_shown_with_a_switch() {
        let html = render_muscles(
            domain::tests::FakeRepository::default()
                .with_exercises(vec![exercise(vec![stretched(
                    domain::MuscleID::Hamstrings,
                )])])
                .with_training_sessions(vec![session_with_hold()]),
        );

        assert_eq!(
            attribute_of(&html, "muscle-kind-worked", "data-selected"),
            "true"
        );
        assert_eq!(
            attribute_of(&html, "muscle-kind-stretched", "data-selected"),
            "false"
        );
    }

    #[test]
    fn test_stretched_muscles_of_a_hybrid_alone_show_no_switch() {
        let html = render_muscles(
            domain::tests::FakeRepository::default()
                .with_exercises(vec![exercise(vec![
                    domain::ExerciseMuscle {
                        muscle_id: domain::MuscleID::ErectorSpinae,
                        stimulus: domain::Stimulus::PRIMARY,
                        stretch: domain::Stimulus::NONE,
                    },
                    stretched(domain::MuscleID::Hamstrings),
                ])])
                .with_training_sessions(vec![session_with_hold()]),
        );

        assert!(!contains(&html, "muscle-kind-worked"));
        assert!(contains(&html, "title"));
    }

    #[component]
    fn StretchedCharts() -> Element {
        let is_stretched = use_signal(|| true);
        let exercises = vec![exercise(vec![stretched(domain::MuscleID::Hamstrings)])];
        let today = chrono::Local::now().date_naive();
        rsx! {
            {view_kind_control(is_stretched)}
            {stretch_charts(&[session_with_hold()], &exercises, domain::Interval { first: today, last: today })}
        }
    }

    #[test]
    fn test_the_stretched_charts_show_a_chart_per_muscle() {
        let html = render_settled(|| {
            provide_settings(web_app::Settings::default());
            rsx! { StretchedCharts {} }
        });

        assert_eq!(
            attribute_of(&html, "muscle-kind-stretched", "data-selected"),
            "true"
        );
        assert_eq!(
            all_text_of(&html, "title"),
            domain::MuscleID::iter()
                .map(|muscle| muscle.name().to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unreadable_training_sessions_are_shown_as_an_error() {
        let html = render_muscles(
            domain::tests::FakeRepository::default()
                .failing(domain::tests::Call::ReadTrainingSessions),
        );

        assert_eq!(text_of(&html, "error-page"), "Not found");
        assert!(!contains(&html, "title"));
    }
}
