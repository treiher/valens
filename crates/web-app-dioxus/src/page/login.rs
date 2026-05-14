use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::SessionService;

use crate::{
    DOMAIN_SERVICE, Route,
    ui::{
        element::{IconText, LoadingPage},
        form::InputField,
    },
};

#[component]
pub fn Login() -> Element {
    let session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
    match &*session.read() {
        None => return rsx! { LoadingPage {} },
        Some(Ok(_)) => {
            navigator().push(Route::Home {});
            return rsx! {};
        }
        Some(Err(_)) => {}
    }

    let mut username = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);

    let submit = move || {
        spawn(async move {
            let name_str = username.read().trim().to_string();
            match domain::Name::new(&name_str) {
                Ok(name) => {
                    is_loading.set(true);
                    error.set(None);
                    let result = DOMAIN_SERVICE().request_session(name).await;
                    is_loading.set(false);
                    match result {
                        Ok(_) => {
                            navigator().push(Route::Home {});
                        }
                        Err(domain::ReadError::NotFound) => {
                            error.set(Some("User not found".to_string()));
                        }
                        Err(domain::ReadError::Storage(
                            domain::StorageError::NoConnection | domain::StorageError::Timeout,
                        )) => {
                            error.set(Some("No connection to server".to_string()));
                        }
                        Err(err) => {
                            error.set(Some(format!("Something went wrong: {err}")));
                        }
                    }
                }
                Err(domain::NameError::Empty) => {
                    error.set(Some("Enter your username".to_string()));
                }
                Err(err) => {
                    error.set(Some(format!("{err}")));
                }
            }
        });
    };

    rsx! {
        section {
            class: "hero is-primary is-fullheight",
            div {
                class: "hero-body",
                div {
                    class: "container has-text-centered",
                    figure {
                        class: "image is-128x128 is-inline-block mb-4",
                        img {
                            src: "/images/android-chrome-512x512.png",
                            alt: "Valens",
                        }
                    }
                    p {
                        class: "title is-1 mb-5",
                        "Valens"
                    }
                    div {
                        class: "box",
                        form {
                            onsubmit: move |e| {
                                e.prevent_default();
                                submit();
                            },
                            InputField {
                                label: Some("Username".to_string()),
                                value: username.read().clone(),
                                error: error.read().clone(),
                                has_changed: false,
                                autofocus: true,
                                "data-testid": "login-username",
                                on_input: move |e: FormEvent| {
                                    username.set(e.value());
                                    error.set(None);
                                },
                            }
                            button {
                                class: "button is-link is-fullwidth mt-2",
                                class: if is_loading() { "is-loading" },
                                "data-testid": "login-button",
                                r#type: "submit",
                                disabled: is_loading(),
                                IconText { icon: "sign-in-alt", text: "Sign in" }
                            }
                        }
                    }
                }
            }
        }
    }
}
