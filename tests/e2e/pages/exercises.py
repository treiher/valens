from __future__ import annotations

import re
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, NestedDialog

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


class ExercisesPage(BasePage):
    def __init__(self, page: Page) -> None:
        super().__init__(page)

        self.nested_dialog = NestedDialog(page)

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
        self.page.get_by_test_id("exercise-item").filter(
            has_text=re.compile(f"^{re.escape(name)}$")
        ).click()
        self.wait_until_idle()

    def open_filter(self) -> None:
        self.page.get_by_test_id("filter-exercises").click()
        self.dialog.wait_until_open()

    def toggle_filter(self, section: str, name: str) -> None:
        self.dialog.root.get_by_test_id(f"filter-section-{section}").get_by_test_id(
            "filter-tag"
        ).filter(has_text=re.compile(f"^{re.escape(name)}$")).click()

    def filter_muscle(self, name: str, level: str) -> None:
        for _ in range(1 if level == "Secondary" else 2):
            self.toggle_filter("muscles", name)

    def apply_filter(self) -> None:
        self.page.get_by_test_id("filter-show").click()
        self.dialog.wait_until_closed()

    def copy_exercise(self, index: int, name: str) -> None:
        self._open_item_options(index)
        self.page.get_by_test_id("options-copy").click()
        self.dialog.set_name(name)
        self.dialog.save()
        self.wait_until_idle()

    def open_catalog_update(self) -> None:
        self.page.get_by_test_id("update-exercises-from-catalog").click()
        self.dialog.wait_until_open()

    def select_catalog_update_mode(self, mode: str) -> None:
        self.dialog.root.get_by_test_id("mode-selection").get_by_text(mode, exact=True).click()

    def catalog_update(self, name: str) -> Locator:
        return self.dialog.root.get_by_test_id("catalog-update").filter(
            has=self.page.get_by_test_id("catalog-update-toggle").filter(
                has_text=re.compile(f"^{re.escape(name)}$")
            )
        )

    def expect_catalog_updates(self, *names: str) -> None:
        expect(self.dialog.root.get_by_test_id("catalog-update")).to_have_count(len(names))
        for name in names:
            expect(self.catalog_update(name)).to_be_visible()

    def expect_catalog_update_selected(self, name: str, *, selected: bool) -> None:
        expect(self.catalog_update(name).get_by_test_id("catalog-update-toggle")).to_have_attribute(
            "data-selected", str(selected).lower()
        )

    def expect_catalog_update_source(self, name: str, source: str) -> None:
        expect(self.catalog_update(name).get_by_test_id("catalog-update-source")).to_have_text(
            f"from {source}"
        )

    def toggle_catalog_update(self, name: str) -> None:
        self.catalog_update(name).get_by_test_id("catalog-update-toggle").click()

    def apply_catalog_updates(self) -> None:
        self.dialog.root.get_by_test_id("update-from-catalog").click()

    def confirm_catalog_updates(self) -> None:
        self.nested_dialog.root.get_by_test_id("dialog-yes").click()

    def expect_catalog_update_dialog_closed(self) -> None:
        expect(self.page.get_by_test_id("dialog")).to_have_count(0)
