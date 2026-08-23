//! App update detection and installation.
//!
//! [`UpdateNotification`] checks for a new server version on startup and shows a dialog
//! when one is available. The update is applied by activating the service worker of the new
//! version, after which the app is reloaded automatically.

use dioxus::prelude::*;

use futures_util::future::{Either, select};
use log::info;

use valens_domain::{Unreachable, VersionService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE,
    diagnostics::log_failure,
    notification::notify_warning,
    ui::element::{Color, Dialog, Error, Icon, Loading, ServerUnreachable},
};

const APP_VERSION: &str = env!("VALENS_VERSION");
const UPDATE_TIMEOUT: u32 = 10_000;

pub static UPDATE_STATUS: GlobalSignal<UpdateStatus> = Signal::global(|| UpdateStatus::UpToDate);
pub static SERVER_VERSION: GlobalSignal<ServerVersion> = Signal::global(|| ServerVersion::Loading);

#[derive(Clone, Copy, PartialEq)]
pub enum UpdateStatus {
    UpToDate,
    Available,
    Deferred,
    Updating,
}

#[derive(Clone, PartialEq)]
pub enum ServerVersion {
    Loading,
    Version(String),
    Unreachable,
    Error(String),
}

#[component]
pub fn UpdateNotification() -> Element {
    use_effect(|| {
        spawn(check_for_updates());
    });
    rsx! {
        if let UpdateStatus::Available | UpdateStatus::Updating = UPDATE_STATUS() {
            Dialog {
                title: rsx! { "Update available" },
                on_close: |_| *UPDATE_STATUS.write() = UpdateStatus::Deferred,
                color: Color::Info,
                div {
                    class: "block",
                    p { "An app update is available." },
                    p {
                        class: "my-3",
                        VersionInfo {}
                    }
                    p { "Update to stay compatible with the server and avoid errors." }
                },
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        button {
                            class: "button is-light is-soft",
                            "data-testid": "update-later",
                            disabled: UPDATE_STATUS() == UpdateStatus::Updating,
                            onclick: move |_| *UPDATE_STATUS.write() = UpdateStatus::Deferred,
                            "Later"
                        }
                    }
                    div {
                        class: "control",
                        button {
                            class: "button is-info",
                            class: if UPDATE_STATUS() == UpdateStatus::Updating { "is-loading" },
                            "data-testid": "update-now",
                            disabled: UPDATE_STATUS() == UpdateStatus::Updating,
                            onclick: move |_| {
                                if let ServerVersion::Version(version) = SERVER_VERSION() {
                                    info!("updating app to version {version}");
                                }
                                *UPDATE_STATUS.write() = UpdateStatus::Updating;
                                spawn(update_app());
                            },
                            "Update"
                        }
                    }
                }
            }
        }
    }
}

/// Activate the service worker of the new version.
///
/// The app is reloaded by the listener of the `controllerchange` event once the new service
/// worker has taken control. An update that is not completed by a reload within the timeout is
/// reported as failed.
async fn update_app() {
    let timeout = gloo_timers::future::TimeoutFuture::new(UPDATE_TIMEOUT);
    let reason = match select(Box::pin(activate_update()), timeout).await {
        Either::Left((Ok(()), timeout)) => {
            timeout.await;
            if UPDATE_STATUS() != UpdateStatus::Updating {
                return;
            }
            "timeout".to_string()
        }
        Either::Left((Err(err), _)) => err,
        Either::Right(((), _)) => "timeout".to_string(),
    };
    web_app::service_worker::cancel_reload_on_controller_change();
    *UPDATE_STATUS.write() = UpdateStatus::Available;
    notify_warning("update app", reason);
}

/// Trigger the activation of the service worker of the new version.
async fn activate_update() -> Result<(), String> {
    match web_app::service_worker::request_update().await? {
        web_app::service_worker::Update::Waiting(service_worker) => {
            web_app::service_worker::post_to(
                &service_worker,
                &web_app::service_worker::OutboundMessage::SkipWaiting,
            )
        }
        web_app::service_worker::Update::Activating => Ok(()),
    }
}

#[component]
pub fn VersionInfo() -> Element {
    rsx! {
        p {
            span {
                class: "icon-text",
                Icon { name: "mobile-screen" }
                {APP_VERSION}
            }
        }
        p {
            span {
                class: "icon-text",
                Icon { name: "server" }
                match &*SERVER_VERSION.read() {
                    ServerVersion::Loading => rsx! {
                        Loading {}
                    },
                    ServerVersion::Version(version) => rsx! {
                        {version.clone()}
                    },
                    ServerVersion::Unreachable => {
                        rsx! {
                            ServerUnreachable {}
                        }
                    }
                    ServerVersion::Error(err) => rsx! {
                        Error { message: "{err}" }
                    },
                }
            }
        }
    }
}

static CHECKING_FOR_UPDATES: GlobalSignal<bool> = Signal::global(|| false);

pub async fn check_for_updates() {
    if CHECKING_FOR_UPDATES() {
        return;
    }
    *CHECKING_FOR_UPDATES.write() = true;

    match &DOMAIN_SERVICE().get_version().await {
        Ok(version) => {
            *UPDATE_STATUS.write() = if version == APP_VERSION {
                UpdateStatus::UpToDate
            } else if cfg!(debug_assertions) {
                UpdateStatus::Deferred
            } else {
                UpdateStatus::Available
            };
            *SERVER_VERSION.write() = ServerVersion::Version(version.clone());
        }
        Err(err) if err.unreachable() => {
            *SERVER_VERSION.write() = ServerVersion::Unreachable;
        }
        Err(err) => {
            log_failure("fetch the server version", err);
            *SERVER_VERSION.write() = ServerVersion::Error(err.to_string());
        }
    }

    *CHECKING_FOR_UPDATES.write() = false;
}
