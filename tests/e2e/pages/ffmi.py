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

    @property
    def chart(self) -> Locator:
        return self.page.locator("svg").first

    def interval_button(self, label: str) -> Locator:
        return self.page.get_by_test_id(f"interval-{label}")
