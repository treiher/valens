use dioxus::prelude::*;

use crate::{Route, cache::Cache, synchronization::Synchronization};

#[component]
pub fn SessionProvider() -> Element {
    use_effect(|| {
        consume_context::<Cache>().refresh();
        consume_context::<Synchronization>().sync();
    });
    rsx! { Outlet::<Route> {} }
}
