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
                                {
                                    let (color, severity) = appearance(entry.level);
                                    rsx! {
                                        Message {
                                            "data-testid": "log-entry",
                                            "data-severity": severity,
                                            color,
                                            p { class: "is-size-7", {entry.time} }
                                            p { "{entry.message}" }
                                        }
                                    }
                                }
                            }
                        },
                        Err(err) => rsx! {
                            Error { message: "{err}" }
                        },
                    }
                }
            }
        }
    }
}

/// The color and `data-severity` of a log entry of `level`.
fn appearance(level: log::Level) -> (Color, &'static str) {
    match level {
        log::Level::Error => (Color::Danger, "error"),
        log::Level::Warn => (Color::Warning, "warning"),
        log::Level::Info => (Color::Primary, "info"),
        log::Level::Debug => (Color::Info, "debug"),
        log::Level::Trace => (Color::Dark, "trace"),
    }
}
