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
                        exercise.stretch,
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{all_text_of, render, text_of};

    use super::*;

    #[test]
    fn test_the_properties_of_a_catalog_exercise_are_shown() {
        let html = render(|| {
            rsx! { Catalog { name: "Back Extension".to_string() } }
        });

        assert_eq!(text_of(&html, "title"), "Back Extension");
        assert!(all_text_of(&html, "property-tag").contains(&"Pull".to_string()));
        assert!(all_text_of(&html, "muscle-tag").contains(&"Erector Spinae".to_string()));
    }

    #[test]
    fn test_the_worked_and_the_stretched_muscles_of_a_hybrid_are_shown_in_rows_of_their_own() {
        let html = render(|| {
            rsx! { Catalog { name: "Jefferson Curl".to_string() } }
        });

        assert_eq!(
            all_text_of(&html, "muscles-caption"),
            vec!["Worked muscles", "Stretched muscles"]
        );
        assert_eq!(
            all_text_of(&html, "muscle-tag"),
            vec!["Erector Spinae", "Hamstrings"]
        );
        assert_eq!(
            all_text_of(&html, "stretch-tag"),
            vec!["Hamstrings", "Erector Spinae"]
        );
    }

    #[test]
    fn test_a_stretch_shows_its_stretched_muscles_alone() {
        let html = render(|| {
            rsx! { Catalog { name: "Couch Stretch".to_string() } }
        });

        assert_eq!(
            all_text_of(&html, "muscles-caption"),
            vec!["Stretched muscles"]
        );
        assert!(all_text_of(&html, "muscle-tag").is_empty());
        assert_eq!(
            all_text_of(&html, "stretch-tag"),
            vec!["Hip Flexors", "Quads"]
        );
    }

    #[test]
    fn test_an_unknown_exercise_is_reported() {
        let html = render(|| {
            rsx! { Catalog { name: "No Such Exercise".to_string() } }
        });

        assert_eq!(text_of(&html, "error-page"), "Exercise not found");
    }

    #[test]
    fn test_an_invalid_name_is_reported() {
        let html = render(|| {
            rsx! { Catalog { name: String::new() } }
        });

        assert_eq!(text_of(&html, "error-page"), "Exercise not found");
    }
}
