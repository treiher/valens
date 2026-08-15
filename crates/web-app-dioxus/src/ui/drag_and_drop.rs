//! Pointer-based drag & drop for reordering, moving and removing list elements.
//!
//! A drag starts on a drag handle rendered by [`view_drag_handle`] and is tracked in a
//! [`Drag`] signal. Drop targets are identified by the `data-drop` attribute of the elements
//! under the pointer and represented by a page-specific type implementing [`DropTarget`].
//!
//! Elements reflect their current drop state in a `data-drop-state` attribute, which is both the
//! styling hook for the insertion markers and the handle for end-to-end tests. The element a drag
//! started from is marked by a `data-drag-state` attribute and stays in place as a placeholder
//! while a copy of it follows the pointer.

use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;

use crate::ui::element::Icon;

/// The `data-drag-state` of an element that is not being dragged.
const DRAG_STATE_IDLE: &str = "idle";

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
    ghost: Option<Ghost>,
}

/// A copy of the dragged element, taken when the drag starts.
///
/// `grab` is the position within the element at which it was grabbed.
#[derive(Clone, PartialEq)]
struct Ghost {
    html: String,
    size: (f64, f64),
    grab: (f64, f64),
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
    let pressed = drag_state(drag, &source) != DRAG_STATE_IDLE;

    rsx! {
        span {
            class: "has-text-grey",
            style: handle_style(pressed),
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

/// The style of a drag handle that is currently pressed or not.
///
/// The padding enlarges the touch target beyond the icon and is compensated by the negative
/// margins to keep the layout unchanged.
fn handle_style(pressed: bool) -> String {
    let cursor = if pressed { "grabbing" } else { "grab" };
    format!(
        "touch-action: none; user-select: none; -webkit-user-select: none; \
         padding: 0.75rem 0.75rem 0.75rem 0.5rem; margin: -0.75rem -0.75rem -0.75rem 0; \
         cursor: {cursor};"
    )
}

/// Render the remove drop zone and the ghost of the dragged element while a drag is active.
///
/// Dropping on `remove_target` removes the dragged element.
pub fn view_drag_overlay<S, T>(drag: Signal<Option<Drag<S, T>>>, remove_target: &T) -> Element
where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    let Some(drag) = drag().filter(|drag| drag.active) else {
        return rsx! {};
    };
    let remove_hovered = drag.target.as_ref() == Some(remove_target);
    rsx! {
        div {
            class: "notification is-danger has-text-centered py-4 px-6",
            class: if !remove_hovered { "is-light" },
            style: "position: fixed; bottom: 1rem; left: 50%; transform: translateX(-50%); z-index: 40;",
            "data-testid": "remove-drop-zone",
            "data-drop": "remove",
            "data-drop-state": drop_state(remove_hovered),
            Icon { name: "xmark" }
        }
        if let Some(ghost) = &drag.ghost {
            {view_ghost(ghost, drag.position)}
        }
    }
}

/// Render the ghost of the dragged element for a pointer at `position`.
fn view_ghost(ghost: &Ghost, position: (f64, f64)) -> Element {
    let (x, y) = ghost_position(ghost, position, viewport_size());
    let (width, _) = ghost.size;
    rsx! {
        div {
            class: "drag-ghost",
            style: "position: fixed; left: 0; top: 0; width: {width}px; \
                    transform: translate({x}px, {y}px); pointer-events: none; z-index: 30;",
            "data-testid": "drag-ghost",
            dangerous_inner_html: "{ghost.html}",
        }
    }
}

/// Determine the offset of the ghost from the top left corner of a viewport of size `viewport`.
///
/// The ghost keeps the point at which it was grabbed under the pointer, so that it does not jump
/// when the drag starts and stays where the pointer moves it. Elements grabbed near one of their
/// edges are kept within the viewport.
fn ghost_position(ghost: &Ghost, position: (f64, f64), viewport: (f64, f64)) -> (f64, f64) {
    let (x, y) = position;
    let (grab_x, grab_y) = ghost.grab;
    let (width, height) = ghost.size;
    (
        within(x - grab_x, width, viewport.0),
        within(y - grab_y, height, viewport.1),
    )
}

/// Move an element of size `size` at `position` into an axis of length `length`.
///
/// An element longer than the axis is aligned with its start.
fn within(position: f64, size: f64, length: f64) -> f64 {
    position.min(length - size).max(0.0)
}

/// The size of the viewport, or an unbounded size if it cannot be determined.
///
/// The size excludes scrollbars, matching the area a fixed element is positioned in.
fn viewport_size() -> (f64, f64) {
    let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    else {
        return (f64::INFINITY, f64::INFINITY);
    };
    (
        f64::from(element.client_width()),
        f64::from(element.client_height()),
    )
}

/// The `data-drag-state` of the element `source` is dragged from.
///
/// The attribute is present on all draggable elements, marking where the ghost is taken from.
pub fn drag_state<S, T>(drag: Signal<Option<Drag<S, T>>>, source: &S) -> &'static str
where
    S: Clone + PartialEq + 'static,
    T: DropTarget + 'static,
{
    drag()
        .filter(|drag| drag.source == *source)
        .map_or(DRAG_STATE_IDLE, |drag| {
            if drag.active { "dragging" } else { "pressed" }
        })
}

/// The `data-drop-state` of a drop zone that is hovered or not.
pub fn drop_state(hovered: bool) -> Option<&'static str> {
    hovered.then_some("hovered")
}

/// The `data-drop-state` of an element with an insertion marker above or below it.
pub fn insertion_state(insert_before: bool, insert_after: bool) -> Option<&'static str> {
    if insert_before {
        Some("insert-before")
    } else if insert_after {
        Some("insert-after")
    } else {
        None
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
        ghost: capture_ghost(event),
    }));
    spawn(auto_scroll(drag, is_valid_target));
}

/// Copy the element the drag handle belongs to, together with its geometry.
fn capture_ghost(event: &PointerEvent) -> Option<Ghost> {
    let data = event.data();
    let raw_event = data.downcast::<web_sys::PointerEvent>()?;
    let element = raw_event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())?
        .closest("[data-drag-state]")
        .ok()??;
    let copy = element
        .clone_node_with_deep(true)
        .ok()?
        .dyn_into::<web_sys::Element>()
        .ok()?;
    strip_state_attributes(&copy);
    let rect = element.get_bounding_client_rect();
    let (x, y) = client_position(event);
    Some(Ghost {
        html: copy.outer_html(),
        size: (rect.width(), rect.height()),
        grab: (x - rect.left(), y - rect.top()),
    })
}

/// Remove the attributes that must not be duplicated from `element` and its descendants.
///
/// The copied element would otherwise be picked up by the drag state selectors and by the element
/// lookups of the end-to-end tests.
fn strip_state_attributes(element: &web_sys::Element) {
    const ATTRIBUTES: [&str; 5] = [
        "id",
        "data-testid",
        "data-drop",
        "data-drop-state",
        "data-drag-state",
    ];
    let descendants = element.query_selector_all("*").ok();
    let elements = std::iter::once(element.clone()).chain(
        descendants
            .iter()
            .flat_map(|nodes| (0..nodes.length()).filter_map(|index| nodes.get(index)))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok()),
    );
    for element in elements {
        for attribute in ATTRIBUTES {
            let _ = element.remove_attribute(attribute);
        }
    }
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

    fn ghost() -> Ghost {
        Ghost {
            html: String::new(),
            size: (200.0, 50.0),
            grab: (30.0, 20.0),
        }
    }

    const VIEWPORT: (f64, f64) = (800.0, 600.0);

    #[test]
    fn test_ghost_position_follows_grab_point() {
        let (x, y) = ghost_position(&ghost(), (100.0, 300.0), VIEWPORT);
        assert_approx_eq!(x, 70.0);
        assert_approx_eq!(y, 280.0);
    }

    #[test]
    fn test_ghost_position_within_viewport() {
        let ghost = ghost();
        let (x, y) = ghost_position(&ghost, (10.0, 590.0), VIEWPORT);
        assert_approx_eq!(x, 0.0);
        assert_approx_eq!(y, VIEWPORT.1 - ghost.size.1);
    }

    #[test]
    fn test_ghost_position_of_oversized_ghost() {
        let ghost = Ghost {
            size: (1000.0, 800.0),
            ..ghost()
        };
        assert_approx_eq!(ghost_position(&ghost, (100.0, 300.0), VIEWPORT).0, 0.0);
        assert_approx_eq!(ghost_position(&ghost, (100.0, 300.0), VIEWPORT).1, 0.0);
    }

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
