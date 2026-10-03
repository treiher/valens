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

    def get_column(self, header: str) -> list[str]:
        """Return the values of a column of the table, newest session first."""
        return self.table.get_column(header)

    def delete_training_session(self, index: int) -> None:
        self.delete_item(index)
        self.dialog.delete()

    def select_muscle(self, name: str) -> None:
        self.page.get_by_test_id("muscle").select_option(label=name)
        self.expect_muscle(name)

    def expect_muscle(self, name: str) -> None:
        expect(self.page.get_by_test_id("muscle").locator("option:checked")).to_have_text(name)

    def expect_muscle_description(self, description: str) -> None:
        expect(self.page.get_by_test_id("muscle-description")).to_have_text(description)

    def expect_no_chart_label(self, label: str) -> None:
        expect(self.page.get_by_text(label, exact=True)).to_have_count(0)


class TrainingDialog(Dialog):
    def get_date(self) -> str:
        return self.page.get_by_test_id("date").first.input_value()

    def set_date(self, date: str) -> None:
        self.page.get_by_test_id("date").first.fill(date)

    def set_routine(self, name: str) -> None:
        self.page.get_by_test_id("routine").first.select_option(label=name)
