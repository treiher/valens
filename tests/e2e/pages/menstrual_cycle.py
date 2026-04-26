from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog

if TYPE_CHECKING:
    from playwright.sync_api import Page


class MenstrualCyclePage(BasePage):
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.dialog: MenstrualCycleDialog = MenstrualCycleDialog(page)

    @property
    def path(self) -> str:
        return "/menstrual_cycle"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Menstrual cycle")


class MenstrualCycleDialog(Dialog):
    def get_date(self) -> str:
        return self.page.locator("input[type='date']").first.input_value()

    def set_intensity(self, value: str) -> None:
        buttons = self.page.locator('[data-testid="dialog"] button').all()
        for button in buttons:
            if button.inner_text() == value:
                button.click()
                return
