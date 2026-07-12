//! Session context that holds the signed-in user.
//!
//! [`Session`] is a Dioxus context provided at the session root. Components never load the user
//! from the domain service; they read it via [`Session::user`], which subscribes them so they
//! re-render when the user changes. After the user has been modified, it is re-read from the
//! storage via [`SessionRefresh::refresh`].

use dioxus::prelude::*;

use valens_domain::{self as domain, SessionService};

use crate::{
    DOMAIN_SERVICE, Route, cache::Cache, diagnostics::log_failure,
    ongoing_training_session::OngoingTrainingSession, synchronization::Synchronization,
    ui::element::LoadingPage,
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
    let session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
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
    use_effect(move || {
        consume_context::<Cache>().refresh();
        consume_context::<Synchronization>().sync();
        consume_context::<OngoingTrainingSession>().load();
    });
    rsx! { Outlet::<Route> {} }
}
