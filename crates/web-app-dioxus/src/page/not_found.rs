use dioxus::prelude::*;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div {
            class: "message has-background-white is-danger mx-2",
            div {
                class: "message-body has-text-dark",
                div {
                    "data-testid": "not-found",
                    class: "title has-text-danger is-size-4",
                    "Page not found (attemped to navigate to: {route:?})"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{render, text_of};

    use super::*;

    #[test]
    fn test_the_attempted_route_is_named() {
        let html = render(|| {
            rsx! { NotFound { route: vec!["nowhere".to_string()] } }
        });

        assert_eq!(
            text_of(&html, "not-found"),
            "Page not found (attemped to navigate to: [\"nowhere\"])"
        );
    }
}
