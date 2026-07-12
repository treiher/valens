//! Pointer-based drag & drop for reordering, moving and removing list elements.
//!
//! A drag starts on a drag handle rendered by [`view_drag_handle`] and is tracked in a
//! [`Drag`] signal. Drop targets are identified by the `data-drop` attribute of the elements
//! under the pointer and represented by a page-specific type implementing [`DropTarget`].

use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;

use crate::ui::element::Icon;

/// State of an ongoing drag, from pressing a drag handle until releasing the pointer.
///
/// `active` stays false until the first pointer movement, so that a mere tap on a handle does not
/// show the drag overlay.
#[derive(Clone, PartialEq)]
pub struct Drag<S, T> {
    pub source: S,
    pub position: (f64, f64),
    pub target: Option<T>,
    pub active: bool,
}

/// A position where a dragged element can be dropped.
pub trait DropTarget: Clone + PartialEq {
    /// Parse the value of a `data-drop` attribute.
    fn parse(value: &str) -> Option<Self>;

    /// Adjust the target based on the hovered `element` and the vertical pointer position `y`.
    fn resolve(self, element: &web_sys::Element, y: f64) -> Self;

    /// Whether hovering this target suspends auto-scrolling.
    fn suspends_auto_scroll(&self) -> bool {
        false
    }
}

/// Render a drag handle that controls `drag`.
///
/// Only targets for which `is_valid_target` returns true are set while dragging. Releasing the
/// pointer over a valid target calls `on_drop` with the source and target.
pub fn view_drag_handle<S, T>(
    mut drag: Signal<Option<Drag<S, T>>>,
    source: S,
    data_testid: &str,
    is_valid_target: impl Fn(S, T) -> bool + Clone + 'static,
    on_drop: impl Fn(S, T) + 'static,
) -> Element
where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    rsx! {
        span {
            class: "ml-2 has-text-grey",
            style: "touch-action: none; cursor: grab; user-select: none; -webkit-user-select: none;",
            "data-testid": "{data_testid}",
            oncontextmenu: |event| event.prevent_default(),
            onpointerdown: {
                let is_valid_target = is_valid_target.clone();
                move |event: PointerEvent| {
                    start_drag(&event, drag, source.clone(), is_valid_target.clone());
                }
            },
            onpointermove: move |event: PointerEvent| update_drag(&event, drag, &is_valid_target),
            onpointerup: move |_| finish_drag(drag, &on_drop),
            onpointercancel: move |_| drag.set(None),
            Icon { name: "grip-vertical", is_small: true }
        }
    }
}

/// Render the remove drop zone and a floating label following the pointer at `position`.
pub fn view_drag_overlay(position: (f64, f64), label: &str, remove_hovered: bool) -> Element {
    let (x, y) = position;
    rsx! {
        div {
            class: "notification is-danger has-text-centered py-4 px-6",
            class: if !remove_hovered { "is-light" },
            style: "position: fixed; bottom: 1rem; left: 50%; transform: translateX(-50%); z-index: 40;",
            "data-testid": "remove-drop-zone",
            "data-drop": "remove",
            Icon { name: "xmark" }
        }
        div {
            class: "box px-4 py-3",
            style: "position: fixed; left: {x}px; top: {y}px; transform: translate(-50%, -125%); pointer-events: none; z-index: 40; opacity: 0.9;",
            "{label}"
        }
    }
}

/// The target hovered during an active drag, or `None` if no drag is in progress.
pub fn hovered_target<S, T>(drag: Signal<Option<Drag<S, T>>>) -> Option<T>
where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    drag()
        .filter(|drag| drag.active)
        .and_then(|drag| drag.target)
}

fn start_drag<S, T>(
    event: &PointerEvent,
    mut drag: Signal<Option<Drag<S, T>>>,
    source: S,
    is_valid_target: impl Fn(S, T) -> bool + 'static,
) where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    event.prevent_default();
    capture_pointer(event);
    drag.set(Some(Drag {
        source,
        position: client_position(event),
        target: None,
        active: false,
    }));
    spawn(auto_scroll(drag, is_valid_target));
}

/// Scroll the page while the pointer is dragged near the top or bottom edge of the viewport.
///
/// Pointer events are only emitted on movement, so a stationary pointer near an edge requires a
/// periodic task to keep scrolling. Scrolling is suspended while a target near an edge, like the
/// remove drop zone, is hovered. The task ends when the drag ends.
async fn auto_scroll<S, T>(
    mut drag: Signal<Option<Drag<S, T>>>,
    is_valid_target: impl Fn(S, T) -> bool,
) where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    loop {
        gloo_timers::future::sleep(std::time::Duration::from_millis(30)).await;
        let Some(current) = drag.peek().clone() else {
            return;
        };
        if !current.active
            || current
                .target
                .as_ref()
                .is_some_and(DropTarget::suspends_auto_scroll)
        {
            continue;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(height) = window
            .inner_height()
            .ok()
            .and_then(|height| height.as_f64())
        else {
            return;
        };
        let (x, y) = current.position;
        let velocity = scroll_velocity(y, height);
        if velocity == 0.0 {
            continue;
        }
        window.scroll_by_with_x_and_y(0.0, velocity);
        let target = drop_target_at(x, y)
            .filter(|target: &T| is_valid_target(current.source.clone(), target.clone()));
        if target != current.target {
            drag.set(Some(Drag { target, ..current }));
        }
    }
}

/// Determine the scroll distance per tick for a pointer at `y` in a viewport of `height`.
///
/// The distance is zero outside the edge zones and increases linearly towards the edge, negative
/// (scrolling up) at the top and positive (scrolling down) at the bottom.
fn scroll_velocity(y: f64, height: f64) -> f64 {
    const ZONE: f64 = 80.0;
    const MAX_VELOCITY: f64 = 24.0;
    if y < ZONE {
        -MAX_VELOCITY * ((ZONE - y) / ZONE).min(1.0)
    } else if y > height - ZONE {
        MAX_VELOCITY * ((y - (height - ZONE)) / ZONE).min(1.0)
    } else {
        0.0
    }
}

fn update_drag<S, T>(
    event: &PointerEvent,
    mut drag: Signal<Option<Drag<S, T>>>,
    is_valid_target: impl Fn(S, T) -> bool,
) where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    let Some(current) = drag.peek().clone() else {
        return;
    };
    let (x, y) = client_position(event);
    let target = drop_target_at(x, y)
        .filter(|target: &T| is_valid_target(current.source.clone(), target.clone()));
    drag.set(Some(Drag {
        position: (x, y),
        target,
        active: true,
        ..current
    }));
}

fn finish_drag<S, T>(mut drag: Signal<Option<Drag<S, T>>>, on_drop: impl FnOnce(S, T))
where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    let Some(current) = drag.take() else {
        return;
    };
    if !current.active {
        return;
    }
    if let Some(target) = current.target {
        on_drop(current.source, target);
    }
}

/// Route all subsequent pointer events to the pressed element until the pointer is released.
///
/// Touch pointers are captured implicitly, but mouse pointers would stop sending events to the
/// element as soon as the cursor leaves it.
fn capture_pointer(event: &PointerEvent) {
    if let Some(raw_event) = event.data().downcast::<web_sys::PointerEvent>()
        && let Some(element) = raw_event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    {
        let _ = element.set_pointer_capture(raw_event.pointer_id());
    }
}

fn client_position(event: &PointerEvent) -> (f64, f64) {
    let coordinates = event.client_coordinates();
    (coordinates.x, coordinates.y)
}

fn drop_target_at<T: DropTarget>(x: f64, y: f64) -> Option<T> {
    #[allow(clippy::cast_possible_truncation)]
    let element = web_sys::window()?
        .document()?
        .element_from_point(x as f32, y as f32)?;
    let target = element.closest("[data-drop]").ok()??;
    let parsed = T::parse(&target.get_attribute("data-drop")?)?;
    Some(parsed.resolve(&target, y))
}

/// Determine the insertion position among the drop elements matching `selector` within `element`
/// from the vertical pointer position `y`, or `None` if no such elements exist.
pub fn insertion_index(element: &web_sys::Element, selector: &str, y: f64) -> Option<usize> {
    let elements = element.query_selector_all(selector).ok()?;
    if elements.length() == 0 {
        return None;
    }
    Some(
        (0..elements.length())
            .filter_map(|index| elements.get(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .filter(|child| in_lower_half(child, y))
            .count(),
    )
}

pub fn in_lower_half(element: &web_sys::Element, y: f64) -> bool {
    let rect = element.get_bounding_client_rect();
    y >= rect.top() + rect.height() / 2.0
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_scroll_velocity() {
        let height = 800.0;
        assert_approx_eq!(scroll_velocity(height / 2.0, height), 0.0);
        assert!(scroll_velocity(40.0, height) < 0.0);
        assert!(scroll_velocity(height - 40.0, height) > 0.0);
        assert!(scroll_velocity(10.0, height) < scroll_velocity(40.0, height));
        assert!(scroll_velocity(height - 10.0, height) > scroll_velocity(height - 40.0, height));
        assert_approx_eq!(
            scroll_velocity(-100.0, height),
            scroll_velocity(0.0, height)
        );
        assert_approx_eq!(
            scroll_velocity(height + 100.0, height),
            scroll_velocity(height, height)
        );
    }
}
