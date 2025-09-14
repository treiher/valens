//! App update detection and installation.
//!
//! [`UpdateNotification`] checks for a new server version on startup and shows a dialog
//! when one is available. The update is applied by instructing the service worker to
//! refresh the cache, after which all clients are reloaded automatically.

use dioxus::prelude::*;

use log::{info, warn};

use valens_domain as domain;
use valens_domain::VersionService;
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE, NOTIFICATIONS,
    ui::element::{Color, Dialog, ErrorMessage, Icon, Loading, NoConnection},
};

const APP_VERSION: &str = env!("VALENS_VERSION");

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
    NoConnection,
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
                close_event: |_| UPDATE_STATUS.with_mut(|u| *u = UpdateStatus::Deferred),
                color: Color::Info,
                div {
                    class: "block",
                    p { "An app update is available." },
                    p {
                        class: "my-3",
                        VersionInfo { }
                    }
                    p { "Update to stay compatible with the server and avoid errors." }
                },
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        button {
                            class: "button is-light is-soft",
                            disabled: UPDATE_STATUS() == UpdateStatus::Updating,
                            onclick: move |_| UPDATE_STATUS.with_mut(|u| *u = UpdateStatus::Deferred),
                            "Later"
                        }
                    }
                    div {
                        class: "control",
                        button {
                            class: "button is-info",
                            class: if UPDATE_STATUS() == UpdateStatus::Updating { "is-loading" },
                            disabled: UPDATE_STATUS() == UpdateStatus::Updating,
                            onclick: move |_| {
                                if let ServerVersion::Version(version) = SERVER_VERSION() {
                                    info!("updating app to version {version}");
                                }
                                UPDATE_STATUS.with_mut(|u| *u = UpdateStatus::Updating);
                                match web_app::service_worker::post(&web_app::service_worker::OutboundMessage::UpdateCache) {
                                    Ok(()) => {
                                        spawn(async move {
                                            gloo_timers::future::TimeoutFuture::new(10_000).await;
                                            if UPDATE_STATUS() == UpdateStatus::Updating {
                                                warn!("app update timed out");
                                                UPDATE_STATUS.with_mut(|u| *u = UpdateStatus::Available);
                                                NOTIFICATIONS.write().push("App update timed out. Please try again.".to_string());
                                            }
                                        });
                                    }
                                    Err(err) => {
                                        warn!("app update failed: {err}");
                                        UPDATE_STATUS.with_mut(|u| *u = UpdateStatus::Available);
                                        NOTIFICATIONS.write().push(format!("App update failed: {err}"));
                                    }
                                }
                            },
                            "Update"
                        }
                    }
                }
            }
        }
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
                        Loading { }
                    },
                    ServerVersion::Version(version) => rsx! {
                        {version.clone()}
                    },
                    ServerVersion::NoConnection => {
                        rsx! {
                            NoConnection { }
                        }
                    }
                    ServerVersion::Error(err) => rsx! {
                        ErrorMessage { message: err }
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
    CHECKING_FOR_UPDATES.with_mut(|c| *c = true);

    match &DOMAIN_SERVICE().get_version().await {
        Ok(version) => {
            UPDATE_STATUS.with_mut(|status| {
                *status = if version == APP_VERSION {
                    UpdateStatus::UpToDate
                } else if cfg!(debug_assertions) {
                    UpdateStatus::Deferred
                } else {
                    UpdateStatus::Available
                }
            });
            SERVER_VERSION.with_mut(|v| *v = ServerVersion::Version(version.to_string()));
        }
        Err(domain::ReadError::Storage(domain::StorageError::NoConnection)) => {
            SERVER_VERSION.with_mut(|v| *v = ServerVersion::NoConnection);
        }
        Err(err) => {
            SERVER_VERSION.with_mut(|v| *v = ServerVersion::Error(err.to_string()));
        }
    };

    CHECKING_FOR_UPDATES.with_mut(|c| *c = false);
}
