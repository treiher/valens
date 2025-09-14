use dioxus::prelude::*;

use valens_domain::{self as domain, Property};

use crate::{
    component::element::{Block, ErrorMessage, Title},
    page,
};

#[component]
pub fn Catalog(name: String) -> Element {
    let exercises = domain::ExerciseFilter::default().catalog();
    if let Ok(name) = domain::Name::new(&name) {
        if let Some(exercise) = exercises.get(&name) {
            rsx! {
                Title { title: "{exercise.name}", x_padding: 2 }
                Block {
                    {view_exercise_properties(
                        exercise.force,
                        exercise.mechanic,
                        exercise.laterality,
                        exercise.assistance,
                        exercise.equipment,
                        exercise.muscles,
                        exercise.category,
                    )}
                }
            }
        } else {
            rsx! { ErrorMessage { message: "Exercise not found" } }
        }
    } else {
        rsx! { ErrorMessage { message: "Exercise not found" } }
    }
}

fn view_exercise_properties(
    force: domain::Force,
    mechanic: domain::Mechanic,
    laterality: domain::Laterality,
    assistance: domain::Assistance,
    equipment: &[domain::Equipment],
    muscles: &[(domain::MuscleID, domain::Stimulus)],
    category: domain::Category,
) -> Element {
    rsx! {
        div {
            class: "tags is-centered m-2",
            for p in [force.name(), mechanic.name(), laterality.name(), assistance.name(), category.name()] {
                span {
                    class: "tag",
                    {p}
                }
            }
        }
        {page::exercise::view_muscles(muscles.iter().map(|(k, v)| (k, v)))}
        div {
            class: "tags is-centered m-2",
            for e in equipment {
                span {
                    class: "tag",
                    {e.name()}
                }
            }
        },
    }
}
