use dioxus::prelude::*;

use valens_domain::{self as domain, SessionService};

use crate::{
    DOMAIN_SERVICE, Route, cache::Cache, diagnostics::log_failure,
    ongoing_training_session::OngoingTrainingSession, synchronization::Synchronization,
    ui::element::LoadingPage,
};

#[derive(Clone)]
pub struct Session {
    pub user: domain::User,
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
    // Re-provide the context on every render to keep it in sync with the re-read session user
    provide_context(Session { user: user.clone() });
    use_effect(move || {
        consume_context::<Cache>().refresh();
        consume_context::<Synchronization>().sync();
        consume_context::<OngoingTrainingSession>().load();
    });
    rsx! { Outlet::<Route> {} }
}
