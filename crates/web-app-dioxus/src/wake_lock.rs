//! Screen wake lock, keeping the display on while a countdown is running.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use log::warn;
use web_sys::{
    js_sys,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

thread_local! {
    static WAKE_LOCK: RefCell<Weak<WakeLock>> = const { RefCell::new(Weak::new()) };
}

static VISIBILITY_LISTENER: std::sync::Once = std::sync::Once::new();

/// Keeps the screen on for as long as any holder of the returned lock is alive.
pub fn hold() -> Rc<WakeLock> {
    WAKE_LOCK.with(|wake_lock| {
        if let Some(held) = wake_lock.borrow().upgrade() {
            return held;
        }
        let held = Rc::new(WakeLock {
            state: Rc::default(),
        });
        held.request();
        *wake_lock.borrow_mut() = Rc::downgrade(&held);
        listen_for_visibility_changes();
        held
    })
}

pub struct WakeLock {
    state: Rc<RefCell<State>>,
}

impl WakeLock {
    /// Asks the browser to keep the screen on.
    ///
    /// A browser without a wake lock, which includes any browser on an insecure connection, is
    /// left alone. A rejected request is not reported either, since the browser rejects while the
    /// page is hidden or the device saves power, neither of which the user can act on.
    fn request(&self) {
        // Leaving the request pending would keep the lock from being asked for again.
        let Some(request) = request_wake_lock() else {
            *self.state.borrow_mut() = State::Idle;
            return;
        };
        let state = Rc::clone(&self.state);
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(sentinel) = wasm_bindgen_futures::JsFuture::from(request).await else {
                *state.borrow_mut() = State::Idle;
                return;
            };
            if matches!(*state.borrow(), State::Idle) {
                release(&sentinel);
            } else {
                *state.borrow_mut() = State::Acquired(sentinel);
            }
        });
    }
}

impl Drop for WakeLock {
    fn drop(&mut self) {
        if let State::Acquired(sentinel) = self.state.replace(State::Idle) {
            release(&sentinel);
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Requested,
    Acquired(JsValue),
    /// No lock is held and none has been asked for, either because the holder is gone or because
    /// the request was rejected.
    Idle,
}

/// Asks for the lock again once the page is visible, since the browser drops it while hidden.
fn listen_for_visibility_changes() {
    VISIBILITY_LISTENER.call_once(|| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            warn!("failed to access document");
            return;
        };
        let listening_document = document.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if listening_document.hidden() {
                return;
            }
            let held = WAKE_LOCK.with(|wake_lock| wake_lock.borrow().upgrade());
            let Some(held) = held else {
                return;
            };
            // A pending request will store its sentinel on its own. Asking again would leave that
            // sentinel unreleased once the second request overwrites it.
            if matches!(*held.state.borrow(), State::Requested) {
                return;
            }
            if let State::Acquired(sentinel) = held.state.replace(State::Requested) {
                release(&sentinel);
            }
            held.request();
        }) as Box<dyn FnMut(web_sys::Event)>);
        if let Err(err) = document
            .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
        {
            warn!("failed to listen for visibility changes: {err:?}");
            return;
        }
        closure.forget();
    });
}

fn request_wake_lock() -> Option<js_sys::Promise> {
    let navigator = web_sys::window()?.navigator();
    let wake_lock = js_sys::Reflect::get(&navigator, &JsValue::from_str("wakeLock")).ok()?;
    let request = js_sys::Reflect::get(&wake_lock, &JsValue::from_str("request"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    request
        .call1(&wake_lock, &JsValue::from_str("screen"))
        .ok()?
        .dyn_into::<js_sys::Promise>()
        .ok()
}

fn release(sentinel: &JsValue) {
    let Ok(release) = js_sys::Reflect::get(sentinel, &JsValue::from_str("release")) else {
        return;
    };
    let Ok(release) = release.dyn_into::<js_sys::Function>() else {
        return;
    };
    let _ = release.call0(sentinel);
}
