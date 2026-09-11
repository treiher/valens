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
    let Some(window) = window() else {
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

/// The browser window, if there is one.
///
/// Off wasm `web_sys::window` panics rather than yielding `None`, so it is not reached there.
fn window() -> Option<web_sys::Window> {
    cfg!(target_arch = "wasm32").then(web_sys::window).flatten()
}

/// Result of a check for a new service worker.
pub enum Update {
    /// The new service worker is installed and waiting for activation.
    Waiting(web_sys::ServiceWorker),
    /// The new service worker is activating without waiting.
    Activating(web_sys::ServiceWorker),
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
    // A previous reload may have been cancelled by the user.
    RELOADING.store(false, Ordering::Relaxed);
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
    } else if let Some(installing) = installing {
        if installing.state() == web_sys::ServiceWorkerState::Redundant {
            return Err("failed to install the new version of the service worker".to_string());
        }
        Update::Activating(installing)
    } else {
        return Err("no new version of the service worker available".to_string());
    };

    Ok(update)
}

/// Wait until the service worker is activated.
///
/// # Errors
///
/// Returns an error if the service worker becomes redundant instead of being activated.
pub async fn await_activation(service_worker: &web_sys::ServiceWorker) -> Result<(), String> {
    await_state(service_worker, |state| {
        matches!(
            state,
            web_sys::ServiceWorkerState::Activated | web_sys::ServiceWorkerState::Redundant
        )
    })
    .await;
    if service_worker.state() == web_sys::ServiceWorkerState::Redundant {
        return Err("failed to activate the new version of the service worker".to_string());
    }
    Ok(())
}

/// Wait until the service worker leaves the `installing` state.
async fn await_installation(service_worker: &web_sys::ServiceWorker) {
    await_state(service_worker, |state| {
        state != web_sys::ServiceWorkerState::Installing
    })
    .await;
}

/// Wait until the state of the service worker fulfills the given condition.
async fn await_state(
    service_worker: &web_sys::ServiceWorker,
    reached: fn(web_sys::ServiceWorkerState) -> bool,
) {
    if reached(service_worker.state()) {
        return;
    }
    let mut resolve = None;
    let promise = js_sys::Promise::new(&mut |resolve_promise, _| resolve = Some(resolve_promise));
    let Some(resolve) = resolve else {
        return;
    };
    let changed = service_worker.clone();
    let listener = StateChangeListener::new(
        service_worker,
        Closure::new(move |_: web_sys::Event| {
            if reached(changed.state()) {
                let _ = resolve.call0(&JsValue::NULL);
            }
        }),
    );
    if listener.is_err() {
        warn!("failed to listen for state changes of the service worker");
        return;
    }
    if let Err(err) = JsFuture::from(promise).await {
        warn!("failed to await state change of the service worker: {err:?}");
    }
}

/// Listener for state changes of a service worker, which is removed when it is dropped.
///
/// Each wait registers its own listener, so that concurrent waits on the same service worker do
/// not replace each other.
struct StateChangeListener {
    service_worker: web_sys::ServiceWorker,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl StateChangeListener {
    fn new(
        service_worker: &web_sys::ServiceWorker,
        closure: Closure<dyn FnMut(web_sys::Event)>,
    ) -> Result<Self, JsValue> {
        service_worker
            .add_event_listener_with_callback("statechange", closure.as_ref().unchecked_ref())?;
        Ok(Self {
            service_worker: service_worker.clone(),
            closure,
        })
    }
}

impl Drop for StateChangeListener {
    fn drop(&mut self) {
        let _ = self.service_worker.remove_event_listener_with_callback(
            "statechange",
            self.closure.as_ref().unchecked_ref(),
        );
    }
}

static RELOADING: AtomicBool = AtomicBool::new(false);

/// Reload the app once another service worker has taken control of it and is activated.
///
/// Nothing is done if the app is not controlled by a service worker yet, as the first service
/// worker takes control without replacing an already running version of the app.
pub fn listen_for_controller_change() {
    let Some(window) = web_sys::window() else {
        warn!("failed to access window");
        return;
    };
    let service_worker = window.navigator().service_worker();
    if service_worker.controller().is_none() {
        return;
    }
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        // Reloading before the new service worker is activated can leave requests unanswered.
        let Some(controller) =
            web_sys::window().and_then(|w| w.navigator().service_worker().controller())
        else {
            reload_app();
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(err) = await_activation(&controller).await {
                warn!("{err}");
            }
            reload_app();
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    if let Err(err) = service_worker
        .add_event_listener_with_callback("controllerchange", closure.as_ref().unchecked_ref())
    {
        warn!("failed to listen for service worker changes: {err:?}");
    }
    closure.forget();
}

/// Reload the app, unless it is already being reloaded.
pub fn reload_app() {
    if RELOADING.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(window) = web_sys::window() else {
        warn!("failed to access window");
        return;
    };
    if let Err(err) = window.location().reload() {
        warn!("failed to reload app: {err:?}");
    }
}
