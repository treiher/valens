from __future__ import annotations

from typing import TYPE_CHECKING

from .base import Dialog, PageElement

if TYPE_CHECKING:
    from playwright.sync_api import Page


class UpdateDialog(PageElement):
    def __init__(self, page: Page) -> None:
        super().__init__(page)
        self.dialog = Dialog(page)

    def wait_until_open(self) -> None:
        self.dialog.wait_until_open()

    def update(self) -> None:
        self.page.get_by_test_id("update-now").click()

    def defer(self) -> None:
        self.page.get_by_test_id("update-later").click()
        self.dialog.wait_until_closed()
