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

#[component]
pub fn SessionProvider() -> Element {
    Cache::provide();
    Synchronization::provide();
    OngoingTrainingSession::provide();
    let session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
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
    use_context_provider(|| Session { user: user.clone() });
    use_effect(move || {
        consume_context::<Cache>().refresh();
        consume_context::<Synchronization>().sync();
        consume_context::<OngoingTrainingSession>().load();
    });
    rsx! { Outlet::<Route> {} }
}
