from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog

if TYPE_CHECKING:
    from playwright.sync_api import Page


class RoutinesPage(BasePage):
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.dialog: RoutinesDialog = RoutinesDialog(page)

    @property
    def path(self) -> str:
        return "/routines"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Routines")


class RoutinesDialog(Dialog):
    def set_name(self, text: str) -> None:
        self.page.locator('[data-testid="dialog"] input[type="text"]').first.fill(text)
