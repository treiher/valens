use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{SessionService, UserService};

use crate::{
    DOMAIN_SERVICE, NO_CONNECTION, Route,
    ui::element::{ErrorMessage, LoadingPage, NoConnection},
};

#[component]
pub fn Login() -> Element {
    let users = use_resource(|| async { DOMAIN_SERVICE().get_users().await });
    if let Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) = *users.read()
    {
        *NO_CONNECTION.write() = true;
    }
    rsx! {
        div {
            class: "container has-text-centered",
            match &*users.read() {
                Some(Ok(users)) => rsx! {
                    for user in users {
                        div {
                            class: "column",
                            button {
                                class: "button is-link",
                                onclick: {
                                    let user_id = user.id;
                                    move |_| {
                                        async move {
                                            let result = DOMAIN_SERVICE().request_session(user_id).await;
                                            DOMAIN_SERVICE.write(); // trigger reload of session
                                            if result.is_ok() {
                                                navigator().push(Route::Home {});
                                            }
                                        }
                                    }
                                },
                                "{user.name}"
                            }
                        }
                    }
                },
                Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => rsx! {
                    NoConnection {}
                },
                Some(Err(err)) => rsx! {
                    ErrorMessage { message: "Failed to fetch response: {err}"}
                },
                None => rsx! { LoadingPage {} }
            }
        }
    }
}
