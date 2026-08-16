use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{AuthService, SessionService, Unreachable};
use valens_storage as storage;

use crate::{
    DOMAIN_SERVICE, LOGIN_LINK_TOKEN, Route,
    diagnostics::log_failure,
    ui::{
        element::{IconText, LoadingPage},
        form::InputField,
    },
};

#[component]
pub fn Login() -> Element {
    let token = use_hook(|| LOGIN_LINK_TOKEN.lock().unwrap().take());
    let mut redemption_failed = use_signal(|| false);
    let redemption = use_resource(move || {
        let token = token.clone();
        async move {
            match token {
                Some(token) => Some(DOMAIN_SERVICE().redeem_login_link(token).await),
                None => None,
            }
        }
    });
    match &*redemption.read() {
        None => return rsx! { LoadingPage {} },
        Some(Some(Ok(_))) => {
            navigator().push(Route::Home {});
            return rsx! {};
        }
        Some(Some(Err(err))) => {
            if !redemption_failed() {
                log_failure("redeem login link", err);
                redemption_failed.set(true);
            }
        }
        Some(None) => {}
    }

    // A failed login link redemption takes precedence over an existing session
    let session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
    if !redemption_failed() {
        match &*session.read() {
            None => return rsx! { LoadingPage {} },
            Some(Ok(_)) => {
                navigator().push(Route::Home {});
                return rsx! {};
            }
            Some(Err(_)) => {}
        }
    }

    let auth_methods = use_resource(|| async { DOMAIN_SERVICE().get_auth_methods().await });
    let (username_login, passkey_login) = match &*auth_methods.read() {
        None => return rsx! { LoadingPage {} },
        Some(Ok(methods)) => (
            methods.contains(&domain::AuthMethod::Username),
            methods.contains(&domain::AuthMethod::Passkey),
        ),
        // If the offered authentication methods cannot be determined, all controls are
        // shown and the server remains the authority on every login attempt
        Some(Err(_)) => (true, true),
    };

    rsx! {
        LoginForm { username_login, passkey_login, redemption_failed: redemption_failed() }
    }
}

#[component]
fn LoginForm(username_login: bool, passkey_login: bool, redemption_failed: bool) -> Element {
    let mut username = use_signal(String::new);
    let mut error = use_signal(|| {
        redemption_failed.then(|| "The login link is invalid or has expired".to_string())
    });
    let mut is_loading = use_signal(|| false);

    let login_with_passkey = move || {
        spawn(async move {
            is_loading.set(true);
            error.set(None);
            let result = DOMAIN_SERVICE().login_with_passkey().await;
            is_loading.set(false);
            match result {
                Ok(_) => {
                    navigator().push(Route::Home {});
                }
                Err(domain::ReadError::NotFound) => {
                    error.set(Some("Passkey is not registered".to_string()));
                }
                Err(err @ domain::ReadError::Unauthorized(_)) => {
                    log_failure("sign in with passkey", &err);
                    error.set(Some("Passkey could not be verified".to_string()));
                }
                Err(err) if err.unreachable() => {
                    error.set(Some("Server unreachable".to_string()));
                }
                // Cancelling the ceremony is a normal user action, not a failure
                Err(domain::ReadError::Other(err))
                    if storage::webauthn::Error::is_cancellation(err.as_ref()) => {}
                Err(err) => {
                    log_failure("sign in with passkey", &err);
                    error.set(Some("Something went wrong".to_string()));
                }
            }
        });
    };

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
                        Err(err) if err.unreachable() => {
                            error.set(Some("Server unreachable".to_string()));
                        }
                        Err(err) => {
                            log_failure("sign in", &err);
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
                        if username_login {
                            form {
                                onsubmit: move |e| {
                                    e.prevent_default();
                                    submit();
                                },
                                InputField {
                                    label: Some("Username".to_string()),
                                    value: username.read().clone(),
                                    error: error.read().clone(),
                                    error_testid: "login-error",
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
                                    IconText { icon: "sign-in-alt", "Sign in" }
                                }
                            }
                        }
                        if !username_login {
                            if let Some(error) = error() {
                                p {
                                    class: "help is-danger has-text-left mb-2",
                                    "data-testid": "login-error",
                                    {error}
                                }
                            }
                        }
                        if passkey_login {
                            button {
                                class: "button is-fullwidth mt-2",
                                class: if is_loading() { "is-loading" },
                                "data-testid": "login-passkey-button",
                                disabled: is_loading(),
                                onclick: move |_| login_with_passkey(),
                                IconText { icon: "key", "Sign in with passkey" }
                            }
                        }
                    }
                }
            }
        }
    }
}
