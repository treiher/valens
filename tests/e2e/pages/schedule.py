from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage
from .utils import drop_on_remove_zone, hover_insertion_position, start_drag

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
        start_drag(
            self.page, self._day(source_weekday).get_by_test_id("slot-handle").nth(source_index)
        )
        if target_index is None:
            day = self._day(target_weekday)
            day.hover()
            rest_day = day.get_by_test_id("rest-day")
            if rest_day.count() > 0:
                expect(rest_day).to_have_attribute("data-drop-state", "hovered")
        else:
            hover_insertion_position(
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
        start_drag(self.page, self._day(weekday).get_by_test_id("slot-handle").nth(index))
        drop_on_remove_zone(self.page)

    def drag_rotation_routine(
        self,
        source_rotation: int,
        source_index: int,
        target_rotation: int,
        target_index: int | None = None,
    ) -> None:
        start_drag(
            self.page,
            self._rotation(source_rotation)
            .get_by_test_id("rotation-routine-handle")
            .nth(source_index),
        )
        if target_index is None:
            self._rotation(target_rotation).hover()
        else:
            hover_insertion_position(
                self._rotation(target_rotation).get_by_test_id("rotation-routine"), target_index
            )
        self.page.mouse.up()

    def remove_rotation_routine(self, rotation: int, index: int) -> None:
        start_drag(
            self.page, self._rotation(rotation).get_by_test_id("rotation-routine-handle").nth(index)
        )
        drop_on_remove_zone(self.page)

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
