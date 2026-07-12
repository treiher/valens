use dioxus::prelude::*;

use valens_web_app::log::Service;

use crate::{
    WEB_APP_SERVICE,
    ui::element::{Block, CenteredBlock, Color, Dialog, Error, Icon, Message},
    update::{UPDATE_STATUS, UpdateStatus, VersionInfo, check_for_updates},
};

#[component]
pub fn AboutDialog(on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        Dialog {
            title: rsx! { "About" },
            on_close,
            Version {}
            Log {}
        }
    }
}

#[component]
fn Version() -> Element {
    use_effect(|| {
        spawn(check_for_updates());
    });
    rsx! {
        div { class: "block",
            label { class: "subtitle", "Version" }
            VersionInfo {}
            if let UpdateStatus::Deferred = UPDATE_STATUS() {
                CenteredBlock {
                    button {
                        class: "button is-link mt-5",
                        onclick: move |_| {
                            *UPDATE_STATUS.write() = UpdateStatus::Available;
                        },
                        Icon { name: "download" }
                    }
                }
            }
        }
    }
}

#[component]
fn Log() -> Element {
    let entries = WEB_APP_SERVICE.read().get_log_entries();
    rsx! {
        div { class: "block",
            label { class: "subtitle", "Log" }
            Block {
                div {
                    "data-testid": "log",
                    match entries {
                        Ok(entries) => rsx! {
                            for entry in entries {
                                Message {
                                    "data-testid": "log-entry",
                                    color: match entry.level {
                                        log::Level::Error => Color::Danger,
                                        log::Level::Warn => Color::Warning,
                                        log::Level::Info => Color::Primary,
                                        log::Level::Debug => Color::Info,
                                        log::Level::Trace => Color::Dark,
                                    },
                                    p { class: "is-size-7", {entry.time} }
                                    p { "{entry.message}" }
                                }
                            }
                        },
                        Err(err) => rsx! {
                            Error { message: err }
                        },
                    }
                }
            }
        }
    }
}
