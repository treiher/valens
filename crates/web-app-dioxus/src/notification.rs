//! App-wide notifications shown in a bar below the navigation bar.
//!
//! Notifications are pushed onto a stack via the [`notify_error`] and [`notify_warning`] helpers,
//! which are callable from any context, including detached async tasks. [`NotificationBar`]
//! renders the topmost notification; dismissing it
//! reveals the next. Every notification auto-dismisses after a per-severity timeout (see
//! [`dismiss_after`]), visualized by a depleting progress bar that pauses while the notification
//! is hovered or focused.

use dioxus::prelude::*;

use crate::ui::element::Color;

static NOTIFICATIONS: GlobalSignal<Vec<Notification>> = Signal::global(Vec::new);
static NEXT_ID: GlobalSignal<usize> = Signal::global(|| 0);

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    Warning,
    Error,
}

impl Severity {
    fn color(self) -> Color {
        match self {
            Severity::Warning => Color::Warning,
            Severity::Error => Color::Danger,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Severity::Warning => "triangle-exclamation",
            Severity::Error => "circle-exclamation",
        }
    }

    fn dismiss_after(self) -> u32 {
        match self {
            Severity::Warning => 6_000,
            Severity::Error => 8_000,
        }
    }
}

#[derive(Clone, PartialEq)]
struct Notification {
    id: usize,
    severity: Severity,
    message: String,
}

pub fn notify_error(message: impl Into<String>) {
    push(Severity::Error, message);
}

#[allow(dead_code)]
pub fn notify_warning(message: impl Into<String>) {
    push(Severity::Warning, message);
}

fn push(severity: Severity, message: impl Into<String>) {
    let id = {
        let mut next = NEXT_ID.write();
        *next += 1;
        *next
    };
    NOTIFICATIONS.write().push(Notification {
        id,
        severity,
        message: message.into(),
    });
}

#[component]
pub fn NotificationBar() -> Element {
    let current = NOTIFICATIONS.read().last().cloned();
    // Notifications stacked below the visible one, counted across all severities
    let hidden = NOTIFICATIONS.read().len().saturating_sub(1);

    rsx! {
        if let Some(notification) = current {
            div {
                class: "notification-bar",
                div {
                    // Cap and center the notification to the same width as the page content
                    class: "container is-max-desktop",
                    // The single-element `for` puts `CurrentNotification` in a keyed list context, so
                    // changing the `id` key rebuilds the subtree, and with it the countdown animation,
                    // whenever the shown notification changes. A reused element would keep its already
                    // finished animation, so a resurfacing notification would never auto-dismiss.
                    for notification in [notification] {
                        CurrentNotification { key: "{notification.id}", notification, hidden }
                    }
                }
            }
        }
    }
}

#[component]
fn CurrentNotification(notification: Notification, hidden: usize) -> Element {
    let id = notification.id;
    rsx! {
        div {
            class: "notification is-{notification.severity.color()} is-flex is-align-items-center is-undecorated",
            role: "alert",
            "data-testid": "notification",
            span {
                class: "icon has-text-light mr-3",
                i { class: "fas fa-{notification.severity.icon()}" }
            }
            div {
                class: "is-flex-grow-1",
                "{notification.message}"
            }
            if hidden > 0 {
                span {
                    class: "tag is-rounded is-light ml-3",
                    "data-testid": "notification-count",
                    "+{hidden}"
                }
            }
            button {
                r#type: "button",
                class: "icon has-text-light ml-3",
                aria_label: "close",
                "data-testid": "notification-close",
                onclick: move |_| { NOTIFICATIONS.write().pop(); },
                i { class: "fas fa-xmark" }
            }
            // Dismissal is driven by the countdown finishing. Pop only if this notification is still
            // on top, in case the stack changed meanwhile.
            div {
                class: "notification-progress",
                style: "--dismiss-ms: {notification.severity.dismiss_after()}ms",
                "data-testid": "notification-progress",
                onanimationend: move |_| {
                    let mut notifications = NOTIFICATIONS.write();
                    if notifications.last().map(|n| n.id) == Some(id) {
                        notifications.pop();
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severities_are_visually_distinct() {
        assert_ne!(
            Severity::Warning.color().to_string(),
            Severity::Error.color().to_string()
        );
        assert_ne!(Severity::Warning.icon(), Severity::Error.icon());
    }

    #[test]
    fn dismiss_after_increases_with_severity() {
        assert!(Severity::Error.dismiss_after() >= Severity::Warning.dismiss_after());
    }
}
