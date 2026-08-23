use std::sync::atomic::{AtomicBool, Ordering};

use log::warn;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys;

#[derive(serde::Serialize)]
#[serde(tag = "task", content = "content")]
pub enum OutboundMessage {
    SkipWaiting,
    ShowNotification {
        title: String,
        options: NotificationOptions,
    },
    CloseNotifications,
}

#[derive(serde::Serialize)]
pub struct NotificationOptions {
    pub body: Option<String>,
}

#[allow(clippy::missing_errors_doc)]
pub fn post(message: &OutboundMessage) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("failed to access window".to_string());
    };
    let Some(service_worker) = window.navigator().service_worker().controller() else {
        return Err("failed to access service worker".to_string());
    };
    post_to(&service_worker, message)
}

#[allow(clippy::missing_errors_doc)]
pub fn post_to(
    service_worker: &web_sys::ServiceWorker,
    message: &OutboundMessage,
) -> Result<(), String> {
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

/// Result of a check for a new service worker.
pub enum Update {
    /// The new service worker is installed and waiting for activation.
    Waiting(web_sys::ServiceWorker),
    /// The new service worker is activating without waiting.
    Activating,
}

/// Check for a new service worker and return the result once it left the `installing` state.
///
/// # Errors
///
/// Returns an error if the check fails or if no new service worker is available.
pub async fn request_update() -> Result<Update, String> {
    let Some(window) = web_sys::window() else {
        return Err("failed to access window".to_string());
    };
    let registration = window
        .navigator()
        .service_worker()
        .ready()
        .map_err(|err| format!("failed to access service worker registration: {err:?}"))?;
    let registration: web_sys::ServiceWorkerRegistration = JsFuture::from(registration)
        .await
        .map_err(|err| format!("failed to access service worker registration: {err:?}"))?
        .into();
    JsFuture::from(registration.update().map_err(|err| {
        format!("failed to check for a new version of the service worker: {err:?}")
    })?)
    .await
    .map_err(|err| format!("failed to check for a new version of the service worker: {err:?}"))?;

    let installing = registration.installing();
    if let Some(installing) = &installing {
        await_installation(installing).await;
    }

    let update = if let Some(waiting) = registration.waiting() {
        Update::Waiting(waiting)
    } else if let Some(installing) = &installing {
        if installing.state() == web_sys::ServiceWorkerState::Redundant {
            return Err("failed to install the new version of the service worker".to_string());
        }
        Update::Activating
    } else {
        return Err("no new version of the service worker available".to_string());
    };

    RELOAD_ON_CONTROLLER_CHANGE.store(true, Ordering::Relaxed);

    Ok(update)
}

/// Wait until the service worker leaves the `installing` state.
async fn await_installation(service_worker: &web_sys::ServiceWorker) {
    if service_worker.state() != web_sys::ServiceWorkerState::Installing {
        return;
    }
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let installing = service_worker.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if installing.state() != web_sys::ServiceWorkerState::Installing {
                let _ = resolve.call0(&JsValue::NULL);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        service_worker.set_onstatechange(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    });
    if let Err(err) = JsFuture::from(promise).await {
        warn!("failed to await installation of the service worker: {err:?}");
    }
}

static RELOADING: AtomicBool = AtomicBool::new(false);
static RELOAD_ON_CONTROLLER_CHANGE: AtomicBool = AtomicBool::new(false);

/// Reload the app as soon as another service worker takes control of it.
///
/// Nothing is done if the app is not controlled by a service worker yet and no update was
/// requested, as the first service worker takes control without replacing an already running
/// version of the app.
pub fn listen_for_controller_change() {
    let Some(window) = web_sys::window() else {
        warn!("failed to access window");
        return;
    };
    let service_worker = window.navigator().service_worker();
    if service_worker.controller().is_some() {
        RELOAD_ON_CONTROLLER_CHANGE.store(true, Ordering::Relaxed);
    }
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if !RELOAD_ON_CONTROLLER_CHANGE.load(Ordering::Relaxed) {
            return;
        }
        if RELOADING.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(window) = web_sys::window() {
            if let Err(err) = window.location().reload() {
                warn!("failed to reload app: {err:?}");
            }
        } else {
            warn!("failed to access window");
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    if let Err(err) = service_worker
        .add_event_listener_with_callback("controllerchange", closure.as_ref().unchecked_ref())
    {
        warn!("failed to listen for service worker changes: {err:?}");
    }
    closure.forget();
}

/// Stop reloading the app when another service worker takes control of it.
pub fn cancel_reload_on_controller_change() {
    RELOAD_ON_CONTROLLER_CHANGE.store(false, Ordering::Relaxed);
}
