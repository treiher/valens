from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Page


class CatalogPage(BasePage):
    def __init__(self, page: Page, name: str) -> None:
        super().__init__(page)

        self.name = name

    @property
    def path(self) -> str:
        return f"/catalog/{self.name}"

    def get_properties(self) -> list[str]:
        return self.page.get_by_test_id("property-tag").all_inner_texts()

    def get_muscles(self) -> list[str]:
        return self.page.get_by_test_id("muscle-tag").all_inner_texts()

    def expect_page(self) -> None:
        expect(self.page_title).to_have_text("Catalog exercise")
