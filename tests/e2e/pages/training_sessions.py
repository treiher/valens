from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog

if TYPE_CHECKING:
    from playwright.sync_api import Page


class TrainingSessionsPage(BasePage):
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.dialog: TrainingDialog = TrainingDialog(page)

    @property
    def path(self) -> str:
        return "/training_sessions"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Training sessions")

    def add_training_session(self, name: str, date: str | None = None) -> None:
        self.fab().click()
        if date is not None:
            self.dialog.set_date(date)
        self.dialog.set_routine(name)
        self.dialog.save()

    def delete_training_session(self, index: int) -> None:
        self.delete_item(index)
        self.dialog.delete()


class TrainingDialog(Dialog):
    def get_date(self) -> str:
        return self.page.get_by_test_id("date").first.input_value()

    def set_date(self, date: str) -> None:
        self.page.get_by_test_id("date").first.fill(date)

    def set_routine(self, name: str) -> None:
        self.page.get_by_test_id("routine").first.select_option(label=name)
