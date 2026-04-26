from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog

if TYPE_CHECKING:
    from playwright.sync_api import Page


class TrainingPage(BasePage):
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.dialog: TrainingDialog = TrainingDialog(page)

    @property
    def path(self) -> str:
        return "/training"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Training sessions")

    def add_training_session(self, name: str) -> None:
        self.fab().click()
        self.dialog.set_routine(name)
        self.dialog.save()


class TrainingDialog(Dialog):
    def get_date(self) -> str:
        return self.page.locator("input[type='date']").first.input_value()

    def set_routine(self, name: str) -> None:
        self.page.locator("select").first.select_option(label=name)
