from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BaseDialog

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class AboutDialog(BaseDialog):
    def open(self) -> None:
        self.navbar.open_about()
        self.dialog.wait_until_open()

    @property
    def log(self) -> Locator:
        return self.page.get_by_test_id("log")

    def log_entry(self, message: str) -> Locator:
        return self.log.get_by_test_id("log-entry").filter(has_text=message)

    def expect_log_warning(self, message: str) -> None:
        expect(self.log_entry(message)).to_have_attribute("data-severity", "warning")
