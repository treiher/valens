//! Presentation of muscle stimulus.

use std::collections::BTreeMap;

use dioxus::prelude::*;

use valens_domain::{self as domain, Property};

use crate::ui::element::{TagsWithAddon, count_to_string};

#[component]
pub fn SetsPerMuscle(stimulus_per_muscle: BTreeMap<domain::MuscleID, domain::Stimulus>) -> Element {
    let mut stimulus_per_muscle = stimulus_per_muscle
        .iter()
        .map(|(muscle_id, stimulus)| (*muscle_id, *stimulus))
        .collect::<Vec<_>>();
    stimulus_per_muscle.sort_by_key(|b| std::cmp::Reverse(b.1));
    let mut groups = [vec![], vec![], vec![], vec![]];
    for (muscle, stimulus) in stimulus_per_muscle {
        let name = muscle.name();
        let description = muscle.description();
        let sets = f64::from(*stimulus) / 100.0;
        let sets_str = count_to_string(sets);
        if sets > 10.0 {
            groups[0].push((name, description, sets_str, vec!["is-dark"]));
        } else if sets >= 3.0 {
            groups[1].push((name, description, sets_str, vec!["is-dark", "is-link"]));
        } else if sets > 0.0 {
            groups[2].push((name, description, sets_str, vec!["is-light", "is-link"]));
        } else {
            groups[3].push((name, description, sets_str, vec![]));
        }
    }
    rsx! {
        for tags in groups {
            if !tags.is_empty() {
                TagsWithAddon { tags }
            }
        }
    }
}
