use dioxus::prelude::*;

use valens_domain::{self as domain, SessionService};

use crate::{
    DOMAIN_SERVICE, Route, cache::Cache, synchronization::Synchronization, ui::element::LoadingPage,
};

#[derive(Clone)]
pub struct Session {
    pub user: domain::User,
}

#[component]
pub fn SessionProvider() -> Element {
    Cache::provide();
    Synchronization::provide();
    let session = use_resource(|| async { DOMAIN_SERVICE().get_session().await });
    match &*session.read() {
        Some(Ok(user)) => {
            let user = user.clone();
            rsx! { AuthenticatedSession { user } }
        }
        Some(Err(_)) => {
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
    });
    rsx! { Outlet::<Route> {} }
}
