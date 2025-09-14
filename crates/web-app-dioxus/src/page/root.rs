use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::SessionService;

use crate::{DOMAIN_SERVICE, NO_CONNECTION, Route, component::element::LoadingPage};

#[component]
pub fn Root() -> Element {
    let session = use_resource(|| async { DOMAIN_SERVICE.read().get_session().await });
    if let Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) =
        *session.read()
    {
        *NO_CONNECTION.write() = true;
    }
    let navigator = use_navigator();

    match *session.read() {
        Some(Ok(_)) => {
            navigator.push(Route::Home {});
            rsx! {}
        }
        Some(Err(_)) => {
            navigator.push(Route::Login {});
            rsx! {}
        }
        None => rsx! {
            LoadingPage {}
        },
    }
}
