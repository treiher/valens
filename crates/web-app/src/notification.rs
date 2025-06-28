use std::collections::HashMap;

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
    close_notifications();
    let mut options = HashMap::new();
    if let Some(body) = body {
        options.insert(String::from("body"), body);
    }
    if let Err(err) = service_worker::post(&service_worker::OutboundMessage::ShowNotification {
        title: title.to_string(),
        options,
    }) {
        warn!("failed to show notification: {err}");
    }
}

pub fn close_notifications() {
    if let Err(err) = service_worker::post(&service_worker::OutboundMessage::CloseNotifications) {
        warn!("failed to close notifications: {err}");
    }
}
