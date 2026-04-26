from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog

if TYPE_CHECKING:
    from playwright.sync_api import Page


class BodyWeightPage(BasePage):
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.dialog: BodyWeightDialog = BodyWeightDialog(page)

    @property
    def path(self) -> str:
        return "/body_weight"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Body weight")


class BodyWeightDialog(Dialog):
    def get_date(self) -> str:
        return self.page.locator("input[type='date']").first.input_value()

    def set_weight(self, weight: str) -> None:
        self.page.locator("input[inputmode='numeric']").first.fill(weight)
