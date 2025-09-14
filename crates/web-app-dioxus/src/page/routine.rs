use dioxus::prelude::*;

use valens_domain as domain;

#[component]
pub fn Routine(id: domain::RoutineID) -> Element {
    rsx! {
        div {
            class: "message has-background-white is-warning mx-2",
            div {
                class: "message-body has-text-dark",
                div {
                    class: "title has-text-danger is-size-4",
                    "Not yet implemented"
                }
            }
        }
    }
}
