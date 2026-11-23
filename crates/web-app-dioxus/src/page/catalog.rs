use dioxus::prelude::*;

use valens_domain as domain;

use crate::{page, ui::element::ErrorPage};

#[component]
pub fn Catalog(name: String) -> Element {
    if let Ok(name) = domain::Name::new(&name) {
        if let Some(exercise) = domain::catalog::exercise(&name) {
            rsx! {
                if !exercise.aliases.is_empty() {
                    {page::exercise::view_block(
                        "Other names",
                        "catalog-other-names",
                        None,
                        rsx! {
                            p {
                                class: "has-text-centered m-2",
                                for (i, alias) in exercise.aliases.iter().enumerate() {
                                    if i > 0 { ", " }
                                    span { "data-testid": "other-name", "{alias}" }
                                }
                            }
                        },
                    )}
                }
                {page::exercise::view_exercise_properties(
                    Some(exercise.force),
                    Some(exercise.mechanic),
                    Some(exercise.laterality),
                    Some(exercise.assistance),
                    exercise.equipment,
                    exercise.muscles,
                    exercise.stretch,
                    Some(exercise.category),
                    None,
                )}
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

        assert!(all_text_of(&html, "property-tag").contains(&"Pull".to_string()));
        assert!(all_text_of(&html, "muscle-tag").contains(&"Erector Spinae".to_string()));
    }

    #[test]
    fn test_the_worked_and_the_stretched_muscles_of_a_hybrid_are_shown_in_rows_of_their_own() {
        let html = render(|| {
            rsx! { Catalog { name: "Jefferson Curl".to_string() } }
        });

        assert_eq!(
            all_text_of(&html, "caption"),
            vec![
                "Worked muscles",
                "Stretched muscles",
                "Properties",
                "Equipment"
            ]
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

        assert!(!all_text_of(&html, "caption").contains(&"Worked muscles".to_string()));
        assert!(all_text_of(&html, "muscle-tag").is_empty());
        assert_eq!(
            all_text_of(&html, "stretch-tag"),
            vec!["Hip Flexors", "Quads"]
        );
    }

    #[test]
    fn test_the_other_names_of_a_catalog_exercise_are_shown_first() {
        let html = render(|| {
            rsx! { Catalog { name: "Barbell Shoulder Press".to_string() } }
        });

        assert_eq!(all_text_of(&html, "caption")[0], "Other names");
        assert_eq!(
            all_text_of(&html, "other-name"),
            vec!["Barbell Overhead Press", "Military Press", "Barbell OHP"]
        );
    }

    #[test]
    fn test_the_other_names_are_left_out_for_an_exercise_without_aliases() {
        let html = render(|| {
            rsx! { Catalog { name: "Couch Stretch".to_string() } }
        });

        assert!(!all_text_of(&html, "caption").contains(&"Other names".to_string()));
        assert!(all_text_of(&html, "other-name").is_empty());
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
