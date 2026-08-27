use dioxus::prelude::*;

use valens_domain as domain;

use crate::{
    page,
    ui::element::{Block, ErrorPage, Title},
};

#[component]
pub fn Catalog(name: String) -> Element {
    let exercises = domain::ExerciseFilter::default().catalog();
    if let Ok(name) = domain::Name::new(&name) {
        if let Some(exercise) = exercises.get(&name) {
            rsx! {
                Title { "{exercise.name}" }
                Block {
                    {page::exercise::view_exercise_properties(
                        Some(exercise.force),
                        Some(exercise.mechanic),
                        Some(exercise.laterality),
                        Some(exercise.assistance),
                        exercise.equipment,
                        exercise.muscles,
                        Some(exercise.category),
                    )}
                }
            }
        } else {
            rsx! { ErrorPage { message: "Exercise not found" } }
        }
    } else {
        rsx! { ErrorPage { message: "Exercise not found" } }
    }
}
