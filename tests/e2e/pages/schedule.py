from __future__ import annotations

import re
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import FloatRect, Locator


class SchedulePage(BasePage):
    @property
    def path(self) -> str:
        return "/schedule"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Schedule")

    def add_slot(self, weekday: int, option: str) -> None:
        slots = self._day(weekday).get_by_test_id("schedule-slot")
        count = slots.count()
        self._day(weekday).get_by_test_id("add-slot").click()
        self.dialog.wait_until_open()
        self.dialog.root.get_by_test_id("slot-option").filter(has_text=option).click()
        expect(slots).to_have_count(count + 1)

    def drag_slot(
        self,
        source_weekday: int,
        source_index: int,
        target_weekday: int,
        target_index: int | None = None,
    ) -> None:
        self._start_drag(self._day(source_weekday).get_by_test_id("slot-handle").nth(source_index))
        if target_index is None:
            day = self._day(target_weekday)
            day.hover()
            rest_day = day.get_by_test_id("rest-day")
            if rest_day.count() > 0:
                expect(rest_day).to_have_class(re.compile("has-text-primary"))
        else:
            _hover_insertion_position(
                self._day(target_weekday).get_by_test_id("schedule-slot"), target_index
            )
        self.page.mouse.up()

    def drag_slot_by_touch(
        self, source_weekday: int, source_index: int, target_weekday: int
    ) -> None:
        handle_box = (
            self._day(source_weekday).get_by_test_id("slot-handle").nth(source_index).bounding_box()
        )
        target_box = self._day(target_weekday).bounding_box()
        assert handle_box
        assert target_box
        start = _center(handle_box)
        end = _center(target_box)
        cdp = self.page.context.new_cdp_session(self.page)
        cdp.send(
            "Input.dispatchTouchEvent",
            {"type": "touchStart", "touchPoints": [{"x": start[0], "y": start[1]}]},
        )
        cdp.send(
            "Input.dispatchTouchEvent",
            {
                "type": "touchMove",
                "touchPoints": [{"x": (start[0] + end[0]) / 2, "y": (start[1] + end[1]) / 2}],
            },
        )
        cdp.send(
            "Input.dispatchTouchEvent",
            {"type": "touchMove", "touchPoints": [{"x": end[0], "y": end[1]}]},
        )
        cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []})
        cdp.detach()

    def remove_slot(self, weekday: int, index: int) -> None:
        self._start_drag(self._day(weekday).get_by_test_id("slot-handle").nth(index))
        self._drop_on_remove_zone()

    def drag_rotation_routine(
        self,
        source_rotation: int,
        source_index: int,
        target_rotation: int,
        target_index: int | None = None,
    ) -> None:
        self._start_drag(
            self._rotation(source_rotation)
            .get_by_test_id("rotation-routine-handle")
            .nth(source_index)
        )
        if target_index is None:
            self._rotation(target_rotation).hover()
        else:
            _hover_insertion_position(
                self._rotation(target_rotation).get_by_test_id("rotation-routine"), target_index
            )
        self.page.mouse.up()

    def remove_rotation_routine(self, rotation: int, index: int) -> None:
        self._start_drag(
            self._rotation(rotation).get_by_test_id("rotation-routine-handle").nth(index)
        )
        self._drop_on_remove_zone()

    def _start_drag(self, handle: Locator) -> None:
        handle.hover()
        self.page.mouse.down()
        # An initial movement is required to activate the drag and show the drop targets
        self.page.mouse.move(0, 0)

    def _drop_on_remove_zone(self) -> None:
        remove_zone = self.page.get_by_test_id("remove-drop-zone")
        remove_zone.hover()
        expect(remove_zone).not_to_have_class(re.compile("is-light"))
        self.page.mouse.up()

    def expect_slots(self, weekday: int, names: list[str]) -> None:
        expect(self._day(weekday).get_by_test_id("slot-name")).to_have_text(names)

    def add_rotation(self, name: str) -> None:
        self.page.get_by_test_id("add-rotation").click()
        self.dialog.wait_until_open()
        self.dialog.set_name(name)
        self.dialog.save()

    def rename_rotation(self, index: int, name: str) -> None:
        self.rename_item(index)
        self.dialog.wait_until_open()
        self.dialog.set_name(name)
        self.dialog.save()

    def delete_rotation(self, index: int) -> None:
        self.delete_item(index)

    def add_rotation_routine(self, index: int, routine: str) -> None:
        routines = self._rotation(index).get_by_test_id("rotation-routine")
        count = routines.count()
        self._open_item_options(index)
        self.page.get_by_test_id("options-add-routine").click()
        self.dialog.wait_until_open()
        self.dialog.root.get_by_test_id("routine-option").filter(has_text=routine).click()
        expect(routines).to_have_count(count + 1)

    def expect_rotation(self, index: int, name: str, routines: list[str]) -> None:
        rotation = self.page.get_by_test_id("schedule-rotation").nth(index)
        expect(rotation.get_by_test_id("rotation-name")).to_have_text(name)
        expect(rotation.get_by_test_id("rotation-routine-name")).to_have_text(routines)

    def expect_rotations(self, count: int) -> None:
        expect(self.page.get_by_test_id("schedule-rotation")).to_have_count(count)

    def _day(self, weekday: int) -> Locator:
        return self.page.get_by_test_id(f"schedule-day-{weekday}")

    def _rotation(self, index: int) -> Locator:
        return self.page.get_by_test_id("schedule-rotation").nth(index)


def _center(box: FloatRect) -> tuple[float, float]:
    return (box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)


# Hovering the upper half of an element targets the insertion position before it, hovering the
# lower half the one after it. Positions between two elements are targeted by hovering the gap.
def _hover_insertion_position(elements: Locator, index: int) -> None:
    if index == 0:
        target = elements.nth(0)
        _hover_at_height(target, 0.25)
        expect(target).to_have_class(re.compile("is-insert-before"))
    elif index < elements.count():
        _hover_gap(elements, index)
        expect(elements.nth(index)).to_have_class(re.compile("is-insert-before"))
    else:
        target = elements.nth(index - 1)
        _hover_at_height(target, 0.75)
        expect(target).to_have_class(re.compile("is-insert-after"))


def _hover_at_height(locator: Locator, fraction: float) -> None:
    box = locator.bounding_box()
    assert box
    locator.hover(position={"x": box["width"] / 2, "y": box["height"] * fraction})


def _hover_gap(elements: Locator, index: int) -> None:
    above = elements.nth(index - 1).bounding_box()
    below = elements.nth(index).bounding_box()
    assert above
    assert below
    elements.page.mouse.move(
        below["x"] + below["width"] / 2,
        (above["y"] + above["height"] + below["y"]) / 2,
    )
