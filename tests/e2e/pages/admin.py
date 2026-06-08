from __future__ import annotations

import re
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class AdminPage(BasePage):
    @property
    def path(self) -> str:
        return "/admin"

    def expect_page(self) -> None:
        expect(self.log).to_be_attached()

    def add_user(self, name: str, height: str) -> None:
        self.page.get_by_test_id("add-user").click()
        self.dialog.wait_until_open()
        self.dialog.root.locator("input").first.fill(name)
        self.dialog.root.locator("input[inputmode='numeric']").fill(height)
        self.dialog.save()
        self.wait_until_idle()

    def edit_user_height(self, name: str, height: str) -> None:
        row = self.page.get_by_role("row").filter(has=self.user_row(name))
        row.get_by_test_id("item-options").click()
        self.page.get_by_test_id("options-edit-user").click()
        self.dialog.wait_until_open()
        self.dialog.root.locator("input[inputmode='numeric']").fill(height)
        self.dialog.save()
        self.wait_until_idle()

    def user_row(self, name: str) -> Locator:
        return self.page.get_by_role("cell", name=name, exact=True)

    @property
    def log(self) -> Locator:
        return self.page.get_by_test_id("log")

    def log_entry(self, message: str) -> Locator:
        return self.log.locator(".message").filter(has_text=message)

    def expect_log_warning(self, message: str) -> None:
        expect(self.log_entry(message)).to_have_class(re.compile(r"\bis-warning\b"))
