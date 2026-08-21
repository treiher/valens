//! Session context that holds the signed-in user.
//!
//! [`Session`] is a Dioxus context provided at the session root. Components never load the user
//! from the domain service; they read it via [`Session::user`], which subscribes them so they
//! re-render when the user changes. After the user has been modified, it is re-read from the
//! storage via [`SessionRefresh::refresh`].

use dioxus::prelude::*;

use valens_domain::{self as domain, AuthService, SessionService};
use valens_storage as storage;

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, Route,
    cache::Cache,
    diagnostics::log_failure,
    notification::{notify, notify_error},
    ongoing_training_session::OngoingTrainingSession,
    signal_changed_data,
    synchronization::Synchronization,
    ui::element::{IconText, LoadingPage},
};

#[derive(Clone, Copy)]
pub struct Session {
    user: ReadSignal<domain::User>,
}

impl Session {
    /// Read the session user. Reading subscribes the caller, so it re-renders when the
    /// user changes.
    #[must_use]
    pub fn user(&self) -> domain::User {
        (self.user)()
    }
}

/// Handle for re-reading the session user after the user has been modified.
#[derive(Clone, Copy)]
pub struct SessionRefresh(Resource<Result<domain::User, domain::ReadError>>);

impl SessionRefresh {
    pub fn refresh(mut self) {
        self.0.restart();
    }
}

#[component]
pub fn SessionProvider() -> Element {
    Cache::provide();
    Synchronization::provide();
    OngoingTrainingSession::provide();
    let ongoing = consume_context::<OngoingTrainingSession>();
    // An in-progress training session must not outlive the session it belongs to. Discarding it
    // here instead of in the failure branch below keeps the navigation to the login page from
    // cancelling the task before the change is persisted.
    let session = use_resource(move || async move {
        let session = DOMAIN_SERVICE().get_session().await;
        if session.is_err() {
            ongoing.clear().await;
        }
        session
    });
    use_context_provider(|| SessionRefresh(session));
    match &*session.read() {
        Some(Ok(user)) => {
            let user = user.clone();
            rsx! { AuthenticatedSession { user } }
        }
        Some(Err(err)) => {
            log_failure("restore the session", err);
            navigator().push(Route::Login {});
            rsx! {}
        }
        None => rsx! { LoadingPage {} },
    }
}

#[component]
fn AuthenticatedSession(user: domain::User) -> Element {
    // Provide the session user through a signal so memoized descendant pages re-render when
    // it changes (e.g. after a profile edit), which re-providing a plain value would not do.
    let mut user_signal = use_signal(|| user.clone());
    if *user_signal.peek() != user {
        user_signal.set(user.clone());
    }
    use_context_provider(|| Session {
        user: ReadSignal::from(user_signal),
    });
    // Must not subscribe to the cache, because it refreshes the cache itself.
    use_effect(move || {
        consume_context::<Cache>().refresh();
        consume_context::<Synchronization>().sync();
        consume_context::<OngoingTrainingSession>().load();
    });
    // Subscribes to the cache and re-runs once the training sessions are available.
    use_effect(move || {
        consume_context::<OngoingTrainingSession>().discard_if_missing(consume_context::<Cache>());
    });
    let user_id = user.id;
    let auth_methods = use_resource(|| async { DOMAIN_SERVICE().get_auth_methods().await });
    let registration_required = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        // Without username login, a passkey is the only way to log in again, so a user
        // without any passkey is held at the registration view. If a check fails or has
        // not finished yet (e.g. offline), the app stays usable.
        let passkey_only = match &*auth_methods.read() {
            Some(Ok(methods)) => !methods.contains(&domain::AuthMethod::Username),
            None | Some(Err(_)) => false,
        };
        passkey_only && passkey_registration_required(user_id).await
    });
    if matches!(*registration_required.read(), Some(true)) {
        rsx! { PasskeyRegistrationRequired {} }
    } else {
        rsx! { Outlet::<Route> {} }
    }
}

async fn passkey_registration_required(user_id: domain::UserID) -> bool {
    match DOMAIN_SERVICE().get_passkeys(user_id).await {
        Ok(passkeys) => passkeys.is_empty(),
        Err(_) => false,
    }
}

#[component]
fn PasskeyRegistrationRequired() -> Element {
    let mut is_loading = use_signal(|| false);

    rsx! {
        section {
            class: "hero is-primary is-fullheight",
            div {
                class: "hero-body",
                div {
                    class: "container has-text-centered",
                    p {
                        class: "title is-4 mb-5",
                        "Register a passkey"
                    }
                    div {
                        class: "box",
                        p {
                            class: "mb-4 has-text-left",
                            "A passkey is required to sign in to your account. \
                             Register a passkey now to keep access to your account."
                        }
                        button {
                            class: "button is-link is-fullwidth",
                            class: if is_loading() { "is-loading" },
                            "data-testid": "register-passkey-button",
                            disabled: is_loading(),
                            onclick: move |_| async move {
                                is_loading.set(true);
                                match DOMAIN_SERVICE().register_passkey().await {
                                    Ok(_) => signal_changed_data(),
                                    // Cancelling the ceremony is a normal user action, not a failure
                                    Err(domain::CreateError::Other(err))
                                        if storage::webauthn::Error::is_cancellation(
                                            err.as_ref(),
                                        ) => {}
                                    Err(err) => notify("register passkey", &err),
                                }
                                is_loading.set(false);
                            },
                            IconText { icon: "key", "Register passkey" }
                        }
                        button {
                            class: "button is-fullwidth mt-2",
                            "data-testid": "registration-logout-button",
                            disabled: is_loading(),
                            onclick: move |_| async move {
                                if sign_out().await {
                                    navigator().push(Route::Login {});
                                }
                            },
                            IconText { icon: "sign-out-alt", "Sign out" }
                        }
                    }
                }
            }
        }
    }
}

/// Remove the session and report a failure to the user.
///
/// Returns `false` if the session still exists on the server.
pub async fn sign_out() -> bool {
    match DOMAIN_SERVICE().delete_session().await {
        Ok(domain::SignOut::Complete) => true,
        Ok(domain::SignOut::DataRetained) => {
            notify_error("sign out", "data on this device could not be removed");
            true
        }
        Err(err) => {
            notify("sign out", &err);
            false
        }
    }
}
