use std::collections::HashMap;

use web_sys;

#[derive(serde::Serialize)]
#[serde(tag = "task", content = "content")]
pub enum Message {
    UpdateCache,
    ShowNotification {
        title: String,
        options: HashMap<String, String>,
    },
    CloseNotifications,
}

#[allow(clippy::missing_errors_doc)]
pub fn post(message: &Message) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("failed to get window".to_string());
    };
    let Some(service_worker) = window.navigator().service_worker().controller() else {
        return Err("failed to get service worker".to_string());
    };
    match serde_wasm_bindgen::to_value(message) {
        Ok(json_message) => {
            let Err(err) = service_worker.post_message(&json_message) else {
                return Ok(());
            };
            Err(format!("failed to post message to service worker: {err:?}"))
        }
        Err(err) => Err(format!(
            "failed to prepare message for service worker: {err}"
        )),
    }
}
