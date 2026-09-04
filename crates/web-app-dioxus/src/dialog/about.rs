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

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use pretty_assertions::assert_eq;
    use valens_web_app as web_app;

    use crate::test_render::{all_text_of, contains, render, seed_web_app_service, text_of};

    use super::*;

    fn entry(level: log::Level, message: &str) -> web_app::log::Entry {
        web_app::log::Entry {
            time: "2026-01-01 00:00:00".to_string(),
            level,
            message: message.to_string(),
        }
    }

    fn render_log(repository: web_app::tests::FakeRepository) -> String {
        render(move || {
            seed_web_app_service(repository.clone());
            rsx! { Log {} }
        })
    }

    #[test]
    fn test_every_log_entry_is_shown_with_its_severity() {
        let html = render_log(web_app::tests::FakeRepository::default().with_entries(
            VecDeque::from([
                entry(log::Level::Error, "failed"),
                entry(log::Level::Info, "started"),
            ]),
        ));

        assert_eq!(all_text_of(&html, "log-entry").len(), 2);
        assert!(all_text_of(&html, "log-entry")[0].contains("failed"));
        assert!(html.contains("data-severity=\"error\""), "{html}");
    }

    #[test]
    fn test_an_unreadable_log_is_reported() {
        let html = render_log(web_app::tests::FakeRepository::default().failing());

        assert!(!contains(&html, "log-entry"));
        assert_eq!(text_of(&html, "log"), "Storage is unavailable");
    }

    #[test]
    fn test_a_deferred_update_offers_the_download() {
        let html = render(|| {
            *UPDATE_STATUS.write() = UpdateStatus::Deferred;
            rsx! { Version {} }
        });

        assert!(contains(&html, "icon-download"));
    }

    #[test]
    fn log_levels_are_visually_distinct() {
        let levels = [
            log::Level::Error,
            log::Level::Warn,
            log::Level::Info,
            log::Level::Debug,
            log::Level::Trace,
        ];
        let appearances = levels
            .into_iter()
            .map(appearance)
            .map(|(color, name)| (color.to_string(), name))
            .collect::<HashSet<_>>();

        assert_eq!(appearances.len(), levels.len());
    }
}
