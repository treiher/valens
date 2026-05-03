//! Notification lifecycle and permissions.
//!
//! This module uses a Service Worker to display and manage notifications. This
//! approach centralizes notification lifecycle management and avoids
//! limitations of handling notifications directly in the app.
//!
//! ## Considered alternative implementations
//!
//! ### 1. Direct notification handling
//!
//! Show notifications directly via the Notification API.
//!
//! Limitation: There is no reliable way to close all existing notifications.
//! Closing requires keeping a handle to each notification. An attempt was made
//! to store the handle in a Dioxus `Signal` and close it via `use_drop`, but
//! the `Signal` was reset before `use_drop` executed, making the handle
//! unavailable.
//!
//! ### 2. Notification replacement in Service Worker
//!
//! Let the service worker replace notifications by closing existing ones before
//! showing a new one.
//!
//! Limitation: In practice, existing notifications were not consistently
//! closed, likely due to timing or browser-specific behavior.
//!
//! ### 3. Using notification `tag` to replace notifications
//!
//! Use the `tag` property to let the browser automatically replace
//! notifications with the same tag.
//!
//! Limitation: On Android devices and connected smartwatches, notifications
//! were sometimes silent or not shown at all. This occurred especially (but not
//! only) when replacements happened rapidly (e.g., within 10 seconds), leading
//! to inconsistent behavior and poor user experience.
//!
//! Due to these limitations, the service worker–based approach was chosen as
//! the most reliable option across platforms.

use log::warn;

use crate::service_worker;

/// # Errors
///
/// Returns an error if the Notifications API is unavailable or if the permission request fails.
pub async fn request_notification_permission() -> Result<web_sys::NotificationPermission, String> {
    let promise = web_sys::Notification::request_permission()
        .map_err(|e| format!("failed to request notification permission: {e:?}"))?;
    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("notification permission request failed: {e:?}"))?;
    Ok(match result.as_string().as_deref() {
        Some("granted") => web_sys::NotificationPermission::Granted,
        Some("denied") => web_sys::NotificationPermission::Denied,
        _ => web_sys::NotificationPermission::Default,
    })
}

pub fn show_notification(title: &str, body: Option<String>) {
    let title = title.to_string();
    let options = service_worker::NotificationOptions { body };
    if let Err(err) =
        service_worker::post(&service_worker::OutboundMessage::ShowNotification { title, options })
    {
        warn!("failed to show notification: {err}");
    }
}

pub fn replace_notifications(title: &str, body: Option<String>) {
    close_notifications();
    show_notification(title, body);
}

pub fn close_notifications() {
    if let Err(err) = service_worker::post(&service_worker::OutboundMessage::CloseNotifications) {
        warn!("failed to close notifications: {err}");
    }
}
