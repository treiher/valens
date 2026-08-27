from __future__ import annotations

from playwright.sync_api import expect

from .base import BasePage


class ExercisesPage(BasePage):
    @property
    def path(self) -> str:
        return "/exercises"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Exercises")

    def add_exercise(self, name: str) -> None:
        self.fab().click()
        self.dialog.set_name(name)
        self.dialog.save()
        self.wait_until_idle()

    def search(self, name: str) -> None:
        self.page.get_by_test_id("search").fill(name)

    def add_catalog_exercise(self, index: int) -> None:
        self.page.get_by_test_id("add-catalog-exercise").nth(index).click()
        self.wait_until_idle()

    def open_exercise(self, name: str) -> None:
        self.page.get_by_test_id("exercise-item").filter(has_text=name).click()
        self.wait_until_idle()

    def copy_exercise(self, index: int, name: str) -> None:
        self._open_item_options(index)
        self.page.get_by_test_id("options-copy").click()
        self.dialog.set_name(name)
        self.dialog.save()
        self.wait_until_idle()
