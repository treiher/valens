#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use std::sync::{Arc, Mutex};

use dioxus::prelude::*;

use valens_domain::{self as domain, VersionService};
use valens_storage as storage;
use valens_web_app as web_app;

use component::{
    element::{Color, Dialog},
    navbar::Navbar,
};
use page::{
    admin::Admin, body_fat::BodyFat, body_weight::BodyWeight, catalog::Catalog, exercise::Exercise,
    exercises::Exercises, home::Home, login::Login, menstrual_cycle::MenstrualCycle,
    muscles::Muscles, not_found::NotFound, root::Root, routine::Routine, routines::Routines,
    training::Training, training_session::TrainingSession,
};

mod component;
mod page;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Root {},
    #[route("/login")]
    Login {},
    #[route("/home")]
    Home {},
    #[route("/admin")]
    Admin {},
    #[route("/training?:add")]
    Training { add: bool },
    #[route("/training_session/:id")]
    TrainingSession { id: domain::TrainingSessionID },
    #[route("/routines?:add&:search")]
    Routines { add: bool, search: String },
    #[route("/routine/:id")]
    Routine { id: domain::RoutineID },
    #[route("/exercises?:add&:filter")]
    Exercises { add: bool, filter: String },
    #[route("/exercise/:id")]
    Exercise { id: domain::ExerciseID },
    #[route("/catalog/:name")]
    Catalog { name: String },
    #[route("/muscles")]
    Muscles { },
    #[route("/body_weight?:add")]
    BodyWeight { add: bool },
    #[route("/body_fat?:add")]
    BodyFat { add: bool },
    #[route("/menstrual_cycle?:add")]
    MenstrualCycle { add: bool },
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

static DOMAIN_SERVICE: GlobalSignal<
    domain::Service<storage::cached_rest::CachedREST<storage::rest::GlooNetSendRequest>>,
> = Signal::global(|| domain::Service::new(storage::cached_rest::CachedREST::new()));
static WEB_APP_SERVICE: GlobalSignal<web_app::Service<storage::local_storage::LocalStorage>> =
    Signal::global(|| web_app::Service::new(storage::local_storage::LocalStorage));
static NOTIFICATIONS: GlobalSignal<Vec<String>> = Signal::global(Vec::new);
static NO_CONNECTION: GlobalSignal<bool> = Signal::global(|| false);
static SYNC_TRIGGER: GlobalSignal<usize> = Signal::global(|| 0);
static DATA_CHANGED: GlobalSignal<usize> = Signal::global(|| 0);

fn main() {
    init_logging();
    dioxus::launch(App);
}

fn init_logging() {
    let _ = web_app::log::init(Arc::new(Mutex::new(storage::local_storage::LocalStorage)));
}

#[component]
fn App() -> Element {
    if let Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) =
        *use_resource(|| async { DOMAIN_SERVICE.read().get_version().await }).read()
    {
        *NO_CONNECTION.write() = true;
    }

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        div {
            class: "container is-max-desktop py-4",
            Router::<Route> {},
            Notification {}
        }
    }
}

#[component]
fn Notification() -> Element {
    let notification = NOTIFICATIONS.read().last().cloned();

    rsx! {
        if let Some(message) = notification {
            Dialog {
                color: Color::Danger,
                title: rsx! { "Error" },
                content: rsx! {
                    div {
                        class: "block",
                        "{message}"
                    },
                    div {
                        class: "field is-grouped is-grouped-centered",
                        div {
                            class: "control",
                            button {
                                class: "button is-danger",
                                onclick: move |_| { let _ = NOTIFICATIONS.write().pop(); },
                                "Close"
                            }
                        }
                    }
                },
                close_event: move |_| { let _ = NOTIFICATIONS.write().pop(); }
            }
        }
    }
}

#[macro_export]
macro_rules! ensure_session {
    () => {{
        let session = use_resource(|| async { DOMAIN_SERVICE.read().get_session().await });
        if let Some(Err(_)) = *session.read() {
            navigator().push(Route::Login {});
        }
        session
    }};
}

fn trigger_sync() {
    *SYNC_TRIGGER.write() += 1;
}

fn signal_changed_data() {
    *DATA_CHANGED.write() += 1;
}
