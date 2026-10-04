//! Presentation of muscle stimulus and stretch.

use std::collections::BTreeMap;

use dioxus::prelude::*;

use valens_domain::{self as domain, Property};

use crate::ui::element::{
    Caption, CenteredBlock, TagsWithAddon, Title, count_to_string, minutes_to_string,
};

/// Show the sets per worked and per stretched muscle in one block, and the time under stretch per
/// stretched muscle in another. Muscles without sets are left out.
#[component]
pub fn SetsPerMuscle(
    stimulus_per_muscle: BTreeMap<domain::MuscleID, domain::Stimulus>,
    stretch_per_muscle: BTreeMap<domain::MuscleID, domain::Stretch>,
    worked_caption: &'static str,
) -> Element {
    let worked = worked_tags(&stimulus_per_muscle);
    let stretch_sets = stretch_tags(
        &stretch_per_muscle,
        |s| s.sets,
        |sets| count_to_string(f64::from(sets)),
    );
    let stretch_time = stretch_tags(
        &stretch_per_muscle,
        |s| s.seconds,
        |seconds| format!("{} min", minutes_to_string(seconds)),
    );
    rsx! {
        if !worked.is_empty() || !stretch_sets.is_empty() {
            CenteredBlock {
                Title { "Sets per muscle" }
                if !worked.is_empty() {
                    Caption { {worked_caption} }
                    for tags in worked {
                        TagsWithAddon { tags }
                    }
                }
                if !stretch_sets.is_empty() {
                    Caption { "Stretched" }
                    div {
                        "data-testid": "stretch-sets-per-muscle",
                        TagsWithAddon { tags: stretch_sets }
                    }
                }
            }
        }
        if !stretch_time.is_empty() {
            CenteredBlock {
                Title { "Time under stretch per muscle" }
                div {
                    "data-testid": "stretch-time-per-muscle",
                    TagsWithAddon { tags: stretch_time }
                }
            }
        }
    }
}

type Tag = (&'static str, &'static str, String, Vec<&'static str>);

/// Group the worked muscles by the thresholds of their sets, each group ordered by its sets.
fn worked_tags(
    stimulus_per_muscle: &BTreeMap<domain::MuscleID, domain::Stimulus>,
) -> Vec<Vec<Tag>> {
    let mut stimulus_per_muscle = stimulus_per_muscle
        .iter()
        .map(|(muscle_id, stimulus)| (*muscle_id, *stimulus))
        .filter(|(_, stimulus)| *stimulus > domain::Stimulus::NONE)
        .collect::<Vec<_>>();
    stimulus_per_muscle.sort_by_key(|b| std::cmp::Reverse(b.1));
    let mut groups = [vec![], vec![], vec![]];
    for (muscle, stimulus) in stimulus_per_muscle {
        let name = muscle.name();
        let description = muscle.description();
        let sets = f64::from(*stimulus) / 100.0;
        let sets_str = count_to_string(sets);
        if sets > 10.0 {
            groups[0].push((name, description, sets_str, vec!["is-dark"]));
        } else if sets >= 3.0 {
            groups[1].push((name, description, sets_str, vec!["is-dark", "is-link"]));
        } else {
            groups[2].push((name, description, sets_str, vec!["is-light", "is-link"]));
        }
    }
    groups.into_iter().filter(|tags| !tags.is_empty()).collect()
}

/// The tags share one style, as no thresholds for a dose of stretching exist.
fn stretch_tags(
    stretch_per_muscle: &BTreeMap<domain::MuscleID, domain::Stretch>,
    value: fn(&domain::Stretch) -> f32,
    format: fn(f32) -> String,
) -> Vec<Tag> {
    let mut stretch_per_muscle = stretch_per_muscle
        .iter()
        .map(|(muscle_id, stretch)| (*muscle_id, value(stretch)))
        .filter(|(_, value)| *value > 0.)
        .collect::<Vec<_>>();
    stretch_per_muscle.sort_by(|a, b| b.1.total_cmp(&a.1));
    stretch_per_muscle
        .into_iter()
        .map(|(muscle, value)| {
            (
                muscle.name(),
                muscle.description(),
                format(value),
                vec!["is-link", "is-light"],
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{all_text_of, contains, render, text_of};

    use super::*;

    #[test]
    fn test_each_stretch_block_is_ordered_by_its_value() {
        let html = render(|| {
            rsx! {
                SetsPerMuscle {
                    stimulus_per_muscle: BTreeMap::new(),
                    stretch_per_muscle: BTreeMap::from([
                        (
                            domain::MuscleID::Hamstrings,
                            domain::Stretch { sets: 3., seconds: 90. },
                        ),
                        (
                            domain::MuscleID::Calves,
                            domain::Stretch { sets: 1.5, seconds: 270. },
                        ),
                    ]),
                    worked_caption: "Worked",
                }
            }
        });

        assert_eq!(
            text_of(&html, "stretch-sets-per-muscle"),
            "Hamstrings3Back of thighsCalves1.5Back of lower legs"
        );
        assert_eq!(
            text_of(&html, "stretch-time-per-muscle"),
            "Calves4.5 minBack of lower legsHamstrings1.5 minBack of thighs"
        );
    }

    #[test]
    fn test_the_worked_and_the_stretched_sets_are_captioned() {
        let html = render(|| {
            rsx! {
                SetsPerMuscle {
                    stimulus_per_muscle: BTreeMap::from([
                        (domain::MuscleID::Pecs, domain::Stimulus::PRIMARY),
                    ]),
                    stretch_per_muscle: BTreeMap::from([
                        (
                            domain::MuscleID::Hamstrings,
                            domain::Stretch { sets: 1., seconds: 30. },
                        ),
                    ]),
                    worked_caption: "Worked (hard sets)",
                }
            }
        });

        assert_eq!(
            all_text_of(&html, "caption"),
            vec!["Worked (hard sets)", "Stretched"]
        );
    }

    #[test]
    fn test_the_worked_sets_are_captioned_without_stretch() {
        let html = render(|| {
            rsx! {
                SetsPerMuscle {
                    stimulus_per_muscle: BTreeMap::from([
                        (domain::MuscleID::Pecs, domain::Stimulus::PRIMARY),
                    ]),
                    stretch_per_muscle: BTreeMap::new(),
                    worked_caption: "Worked",
                }
            }
        });

        assert_eq!(all_text_of(&html, "caption"), vec!["Worked"]);
        assert!(!contains(&html, "stretch-time-per-muscle"));
    }

    #[test]
    fn test_muscles_without_sets_are_left_out() {
        let html = render(|| {
            rsx! {
                SetsPerMuscle {
                    stimulus_per_muscle: BTreeMap::from([
                        (domain::MuscleID::Pecs, domain::Stimulus::NONE),
                        (domain::MuscleID::Lats, domain::Stimulus::SECONDARY),
                    ]),
                    stretch_per_muscle: BTreeMap::from([
                        (
                            domain::MuscleID::Hamstrings,
                            domain::Stretch { sets: 1., seconds: 0. },
                        ),
                    ]),
                    worked_caption: "Worked",
                }
            }
        });

        assert!(!html.contains("Pecs"));
        assert!(html.contains("Lats"));
        assert!(!contains(&html, "stretch-time-per-muscle"));
    }

    #[test]
    fn test_nothing_is_shown_without_sets() {
        let html = render(|| {
            rsx! {
                div {
                    SetsPerMuscle {
                        stimulus_per_muscle: BTreeMap::from([
                            (domain::MuscleID::Pecs, domain::Stimulus::NONE),
                        ]),
                        stretch_per_muscle: BTreeMap::new(),
                        worked_caption: "Worked",
                    }
                }
            }
        });

        assert!(!contains(&html, "title"));
    }
}
