use dioxus::prelude::*;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div {
            class: "message has-background-white is-danger mx-2",
            div {
                class: "message-body has-text-dark",
                div {
                    class: "title has-text-danger is-size-4",
                    "Page not found (attemped to navigate to: {route:?})"
                }
            }
        }
    }
}
