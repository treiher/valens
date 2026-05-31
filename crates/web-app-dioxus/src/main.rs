#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use log::error;

use valens_domain as domain;
use valens_storage as storage;
use valens_web_app as web_app;

use notification::NotificationBar;
use page::common::{DropSetCalculatorState, MetronomeService, OneRepMaxCalculatorState};
use routing::Route;
use settings::Settings;
use unsaved_changes::router_config;
use update::UpdateNotification;

mod cache;
mod navbar;
mod notification;
mod ongoing_training_session;
mod page;
mod routing;
mod session;
mod settings;
mod synchronization;
mod ui;
mod unsaved_changes;
mod update;

static DOMAIN_SERVICE: GlobalSignal<
    domain::Service<storage::cached_rest::CachedREST<storage::rest::GlooNetSendRequest>>,
> = Signal::global(|| domain::Service::new(storage::cached_rest::CachedREST::new()));
static WEB_APP_SERVICE: GlobalSignal<web_app::Service<storage::local_storage::LocalStorage>> =
    Signal::global(|| web_app::Service::new(storage::local_storage::LocalStorage));
static NO_CONNECTION: GlobalSignal<bool> = Signal::global(|| false);
static DATA_CHANGED: GlobalSignal<usize> = Signal::global(|| 0);
static METRONOME: GlobalSignal<MetronomeService> = Signal::global(MetronomeService::new);
static ONE_REP_MAX_CALCULATOR: GlobalSignal<OneRepMaxCalculatorState> =
    Signal::global(|| OneRepMaxCalculatorState::new(5, 100.0));
static DROP_SET_CALCULATOR: GlobalSignal<DropSetCalculatorState> =
    Signal::global(|| DropSetCalculatorState::new(100.0, 20.0, 2.0));

fn main() {
    init_logging();
    init_service_worker();
    dioxus::launch(App);
}

fn init_logging() {
    let _ = web_app::log::init(Arc::new(Mutex::new(storage::local_storage::LocalStorage)));
}

fn init_service_worker() {
    web_app::service_worker::listen_for_reload();
}

#[component]
fn App() -> Element {
    std::panic::set_hook(Box::new(|info| {
        error!("panic: {info}");
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("main"))
            .map(|el| {
                el.set_inner_html(&format!("
                    <section class=\"section\">
                        <div class=\"container\">
                            <div class=\"message is-danger\">
                                <div class=\"message-header\">
                                    <p>Something went wrong</p>
                                </div>
                                <div class=\"message-body\">
                                    <div class=\"block\">
                                        An unexpected error occurred and the application cannot continue.
                                    </div>
                                    <div class=\"block\">
                                        <pre>{info}</pre>
                                    </div>
                                    <div class=\"block field is-grouped is-grouped-centered\">
                                        <button class=\"button\" onclick=\"location.reload()\">
                                            <span class=\"icon\">
                                                <i class=\"fa fa-arrow-rotate-right\"></i>
                                            </span>
                                            <span>Reload page</span>
                                        </button>
                                        <a class=\"button\" href=\"https://github.com/treiher/valens/issues\" target=\"_blank\">
                                            <span class=\"icon\">
                                                <i class=\"fa fa-flag\"></i>
                                            </span>
                                            <span>Report issue</span>
                                        </a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </section>
                "));
                Some(())
            });
    }));

    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        el.set_outer_html("");
    }

    Settings::provide();

    rsx! {
        Router::<Route> {
            config: router_config
        }
        UpdateNotification {}
        NotificationBar {}
    }
}

fn signal_changed_data() {
    *DATA_CHANGED.write() += 1;
}

#[macro_export]
macro_rules! eh {
    ($($closure:ident),+; $expr:expr) => {{
        $(let $closure = $closure.clone();)+
            move |_| {
                $(let $closure = $closure.clone();)+
                $expr
            }
    }};
    (mut $($mut_closure:ident),*; $expr:expr) => {{
        $(let $mut_closure = $mut_closure.clone();)+
            move |_| {
                $(let mut $mut_closure = $mut_closure.clone();)*
                $expr
            }
    }};
    (mut $($mut_closure:ident),*; $($closure:ident),+; $expr:expr) => {{
        $(let $mut_closure = $mut_closure.clone();)+
        $(let $closure = $closure.clone();)+
            move |_| {
                $(let mut $mut_closure = $mut_closure.clone();)*
                $(let $closure = $closure.clone();)*
                $expr
            }
    }};
}
