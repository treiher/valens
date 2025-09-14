use std::collections::HashSet;

use dioxus::{
    prelude::*,
    router::{GenericRouterContext, RouterConfig},
};
use log::warn;
use uuid::Uuid;
use web_sys::{
    self,
    wasm_bindgen::{JsCast, closure::Closure},
};

use crate::{
    routing::Route,
    ui::element::{Color, Dialog},
};

static CURRENT_ROUTE: GlobalSignal<Route> = Signal::global(|| Route::Root {});
static UNSAVED_CHANGES_GUARD: GlobalSignal<UnsavedChangesGuard> =
    Signal::global(UnsavedChangesGuard::default);

#[derive(Clone, Copy, Default)]
struct UnsavedChangesGuard {
    sources: Signal<HashSet<Uuid>>,
    pending: Signal<Option<Route>>,
    show_dialog: Signal<bool>,
}

impl UnsavedChangesGuard {
    fn is_dirty(&self) -> bool {
        !self.sources.read().is_empty()
    }
}

#[component]
pub fn UnsavedChangesDialog() -> Element {
    let route = use_route();
    CURRENT_ROUTE.with_mut(|r| *r = route);

    use_hook(move || {
        // Handle page reloads and closing tabs
        let closure = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
            move |event: web_sys::BeforeUnloadEvent| {
                if UNSAVED_CHANGES_GUARD().is_dirty() {
                    event.prevent_default();
                    event.set_return_value("");
                }
            },
        );
        let Some(window) = web_sys::window() else {
            warn!("failed to access window");
            return;
        };
        if let Err(e) = window
            .add_event_listener_with_callback("beforeunload", closure.as_ref().unchecked_ref())
        {
            warn!("failed to register beforeunload handler: {e:?}");
            return;
        }
        closure.forget();
    });

    let mut guard = UNSAVED_CHANGES_GUARD();

    if !*guard.show_dialog.read() {
        return rsx! {};
    }

    let close = move |_| {
        guard.show_dialog.set(false);
        guard.pending.set(None);
    };

    rsx! {
        Dialog {
            close_event: close,
            color: Color::Danger,
            div {
                class: "block",
                "You have unsaved changes. Leave anyway?"
            }
            div {
                class: "field is-grouped is-grouped-centered",
                div {
                    class: "control",
                    onclick: close,
                    button {
                        class: "button is-light is-soft",
                        "Stay"
                    }
                }
                div {
                    class: "control",
                    onclick: move |_| {
                        guard.show_dialog.set(false);
                        guard.sources.write().clear();
                        if let Some(target) = guard.pending.take() {
                            navigator().push(target);
                        }
                    },
                    button {
                        class: "button is-danger",
                        "Leave"
                    }
                }
            }
        }
    }
}

pub fn use_unsaved_changes() -> Signal<bool> {
    let dirty = use_signal(|| false);
    let mut guard = UNSAVED_CHANGES_GUARD();
    let id = use_hook(Uuid::new_v4);

    use_effect(move || {
        if dirty() {
            guard.sources.write().insert(id);
        } else {
            guard.sources.write().remove(&id);
        }
    });

    use_drop(move || {
        guard.sources.write().remove(&id);
    });

    dirty
}

pub fn router_config() -> RouterConfig<Route> {
    RouterConfig::default().on_update(|state: GenericRouterContext<Route>| {
        let mut guard = UNSAVED_CHANGES_GUARD();
        if guard.is_dirty() {
            guard.pending.set(Some(state.current().clone()));
            guard.show_dialog.set(true);
            return Some(NavigationTarget::Internal(CURRENT_ROUTE()));
        }
        CURRENT_ROUTE.with_mut(|route| *route = state.current());
        None
    })
}
