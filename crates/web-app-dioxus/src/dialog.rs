//! Application-level dialogs.

pub mod about;
pub mod admin;
pub mod drop_set;
pub mod one_rep_max;
pub mod profile;
pub mod settings;

use dioxus::prelude::*;

use valens_domain as domain;

use crate::ui::element::{Table, value_or_dash};

#[component]
fn PasskeyTable(
    passkeys: Vec<domain::Passkey>,
    action: Callback<domain::Passkey, Element>,
) -> Element {
    rsx! {
        Table {
            head: vec![rsx! { "Name" }, rsx! { "Created" }, rsx! { "Last used" }, rsx! {}],
            body: passkeys.iter().map(|passkey| {
                vec![
                    rsx! { "{passkey.label}" },
                    rsx! { "{passkey.created}" },
                    rsx! { {value_or_dash(passkey.last_used)} },
                    action.call(passkey.clone()),
                ]
            }).collect::<Vec<_>>()
        }
    }
}
