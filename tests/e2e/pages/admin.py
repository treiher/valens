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

    @property
    def log(self) -> Locator:
        return self.page.get_by_test_id("log")

    def log_entry(self, message: str) -> Locator:
        return self.log.locator(".message").filter(has_text=message)

    def expect_log_warning(self, message: str) -> None:
        expect(self.log_entry(message)).to_have_class(re.compile(r"\bis-warning\b"))
