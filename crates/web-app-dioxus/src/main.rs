#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use log::{error, warn};
use web_sys::wasm_bindgen::{JsCast, closure::Closure};

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
mod current_date;
mod diagnostics;
mod dialog;
mod loading;
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
/// Counter incremented after every mutation. Components that must reflect fresh data read it
/// first in the closure of a `use_resource`, so the resource re-runs on every data change.
static DATA_CHANGED: GlobalSignal<usize> = Signal::global(|| 0);
// Captured from the URL fragment before the router strips it (see `init_login_link_token`).
// A plain `Mutex` is used instead of a signal because it is written in `main` before the
// Dioxus runtime, which backs global signals, exists.
static LOGIN_LINK_TOKEN: Mutex<Option<String>> = Mutex::new(None);
static METRONOME: GlobalSignal<MetronomeService> = Signal::global(MetronomeService::new);
static ONE_REP_MAX_CALCULATOR: GlobalSignal<OneRepMaxCalculatorState> =
    Signal::global(|| OneRepMaxCalculatorState::new(5, 100.0));
static DROP_SET_CALCULATOR: GlobalSignal<DropSetCalculatorState> =
    Signal::global(|| DropSetCalculatorState::new(100.0, 20.0, 2.0));

fn main() {
    init_logging();
    init_service_worker();
    init_login_link_token();
    dioxus::launch(App);
}

/// Capture the login link token from the URL fragment (`#recover=<token>`).
///
/// This runs before the router initializes, which strips the fragment. The token is passed
/// in the fragment as fragments never reach the server or proxy logs.
fn init_login_link_token() {
    if let Some(token) = login_link_token() {
        *LOGIN_LINK_TOKEN.lock().unwrap() = Some(token);
        // `replaceState` removes the fragment without leaving a dangling `#` or keeping a
        // history entry containing the consumed token
        if let Some(window) = web_sys::window()
            && let Ok(history) = window.history()
            && let Ok(pathname) = window.location().pathname()
        {
            let _ = history.replace_state_with_url(
                &web_sys::wasm_bindgen::JsValue::NULL,
                "",
                Some(&pathname),
            );
        }
    }
    listen_for_login_link();
}

fn login_link_token() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .and_then(|hash| hash.strip_prefix("#recover=").map(str::to_string))
        .filter(|token| !token.is_empty())
}

/// Reload the app when a login link is opened while it is already running.
///
/// Opening a login link on the route it points to changes only the URL fragment. The document
/// is not loaded again in that case, so the token is picked up by reloading it explicitly.
fn listen_for_login_link() {
    let Some(window) = web_sys::window() else {
        warn!("failed to access window");
        return;
    };
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if login_link_token().is_none() {
            return;
        }
        if let Some(window) = web_sys::window()
            && let Err(err) = window.location().reload()
        {
            warn!("failed to reload app: {err:?}");
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    if let Err(err) =
        window.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref())
    {
        warn!("failed to listen for login links: {err:?}");
    }
    closure.forget();
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

    use_future(current_date::update_at_midnight);

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
