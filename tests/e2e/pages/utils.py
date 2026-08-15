from __future__ import annotations

import re
from typing import TYPE_CHECKING

from playwright.sync_api import expect

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


def parse_int(text: str) -> int | None:
    try:
        return int(parse_numeric(text))
    except ValueError:
        return None


def parse_float(text: str) -> float | None:
    try:
        return float(parse_numeric(text))
    except ValueError:
        return None


def parse_numeric(text: str) -> str:
    match = re.search(r"\d+\.?\d*", text)
    return match.group() if match else ""


def start_drag(page: Page, handle: Locator) -> None:
    handle.hover()
    page.mouse.down()
    expect(page.locator("[data-drag-state='pressed']")).to_have_count(1)
    # An initial movement is required to activate the drag and show the drop targets. The center
    # of the viewport is outside of the edge zones in which dragging triggers auto-scrolling.
    viewport = page.viewport_size
    assert viewport
    page.mouse.move(viewport["width"] / 2, viewport["height"] / 2)
    expect(page.locator("[data-drag-state='dragging']")).to_have_count(1)
    expect(page.get_by_test_id("drag-ghost")).to_be_visible()


def drop_on_remove_zone(page: Page) -> None:
    remove_zone = page.get_by_test_id("remove-drop-zone")
    remove_zone.hover()
    expect(remove_zone).to_have_attribute("data-drop-state", "hovered")
    page.mouse.up()


# Hovering the upper half of an element targets the insertion position before it, hovering the
# lower half the one after it. Positions between two elements are targeted by hovering the gap.
# The elements to hover can be overridden by `hover_elements` for elements like sections that are
# targeted by their header while the insertion markers are shown on the whole element.
def hover_insertion_position(
    elements: Locator, index: int, hover_elements: Locator | None = None
) -> None:
    targets = elements if hover_elements is None else hover_elements
    if index == 0:
        hover_at_height(targets.nth(0), 0.25)
        expect(elements.nth(0)).to_have_attribute("data-drop-state", "insert-before")
    elif index < elements.count():
        _hover_gap(elements, index)
        expect(elements.nth(index)).to_have_attribute("data-drop-state", "insert-before")
    else:
        hover_at_height(targets.nth(index - 1), 0.75)
        expect(elements.nth(index - 1)).to_have_attribute("data-drop-state", "insert-after")


# Hovering below the last element targets the insertion position at the end of the containing
# list.
def hover_after_last(elements: Locator) -> None:
    last = elements.nth(elements.count() - 1)
    scroll_to_center(last)
    box = last.bounding_box()
    assert box
    elements.page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] + 4)
    expect(last).to_have_attribute("data-drop-state", "insert-after")


def hover_at_height(locator: Locator, fraction: float) -> None:
    scroll_to_center(locator)
    box = locator.bounding_box()
    assert box
    locator.page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] * fraction)


def _hover_gap(elements: Locator, index: int) -> None:
    scroll_to_center(elements.nth(index))
    above = elements.nth(index - 1).bounding_box()
    below = elements.nth(index).bounding_box()
    assert above
    assert below
    elements.page.mouse.move(
        below["x"] + below["width"] / 2,
        (above["y"] + above["height"] + below["y"]) / 2,
    )


def get_focused_selection(input_field: Locator) -> str:
    expect(input_field).to_be_focused()
    return input_field.evaluate(
        "element => element.value.substring(element.selectionStart, element.selectionEnd)"
    )


# Keep the pointer away from the viewport edges where dragging triggers auto-scrolling
def scroll_to_center(locator: Locator) -> None:
    locator.evaluate("element => element.scrollIntoView({block: 'center', behavior: 'instant'})")
