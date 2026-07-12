from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class FfmiPage(BasePage):
    @property
    def path(self) -> str:
        return "/ffmi"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("FFMI")

    def expect_height_missing_message(self) -> None:
        expect(self.page.get_by_test_id("ffmi-height-missing")).to_be_visible()

    def expect_height_provided(self) -> None:
        expect(self.page.get_by_test_id("ffmi-height-missing")).to_be_hidden()

    @property
    def chart(self) -> Locator:
        return self.page.locator("svg").first

    @property
    def chart_tooltip(self) -> Locator:
        return self.page.get_by_test_id("chart-tooltip")

    def hover_chart_center(self) -> None:
        self.page.get_by_test_id("chart-overlay").first.hover()

    def touch_chart_center(self) -> None:
        box = self.page.get_by_test_id("chart-overlay").first.bounding_box()
        assert box
        self._cdp = self.page.context.new_cdp_session(self.page)
        self._cdp.send(
            "Input.dispatchTouchEvent",
            {
                "type": "touchStart",
                "touchPoints": [
                    {"x": box["x"] + box["width"] / 2, "y": box["y"] + box["height"] / 2}
                ],
            },
        )

    def release_touch(self) -> None:
        self._cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []})
        self._cdp.detach()

    def interval_button(self, label: str) -> Locator:
        return self.page.get_by_test_id(f"interval-{label}")
