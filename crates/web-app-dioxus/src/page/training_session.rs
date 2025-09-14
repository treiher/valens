use dioxus::prelude::*;

use valens_domain as domain;

#[component]
pub fn TrainingSession(id: domain::TrainingSessionID) -> Element {
    rsx! {
        div {
            class: "message has-background-white is-warning mx-2",
            div {
                class: "message-body has-text-dark",
                div {
                    class: "title has-text-warning is-size-4",
                    "Not yet implemented"
                }
            }
        }
    }
}
