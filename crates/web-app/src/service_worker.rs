use std::collections::HashMap;

use log::warn;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys;

#[derive(serde::Serialize)]
#[serde(tag = "task", content = "content")]
pub enum OutboundMessage {
    UpdateCache,
    ShowNotification {
        title: String,
        options: HashMap<String, String>,
    },
    CloseNotifications,
}

#[allow(clippy::missing_errors_doc)]
pub fn post(message: &OutboundMessage) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("failed to access window".to_string());
    };
    let Some(service_worker) = window.navigator().service_worker().controller() else {
        return Err("failed to access service worker".to_string());
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

#[derive(serde::Deserialize)]
#[serde(tag = "task")]
enum InboundMessage {
    Reload,
}

pub fn listen_for_reload() {
    let Some(window) = web_sys::window() else {
        warn!("failed to access window");
        return;
    };
    let sw_container = window.navigator().service_worker();
    let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        match serde_wasm_bindgen::from_value(event.data()) {
            Ok(InboundMessage::Reload) => {
                if let Some(w) = web_sys::window() {
                    if let Err(err) = w.location().reload() {
                        warn!("failed to reload app: {err:?}");
                    }
                } else {
                    warn!("failed to access window");
                }
            }
            Err(err) => {
                warn!("failed to parse message from service worker: {err}");
            }
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    sw_container.set_onmessage(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
}
