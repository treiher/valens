from __future__ import annotations

import uuid

from playwright.sync_api import expect

from .base import BasePage


class RoutinesPage(BasePage):
    @property
    def path(self) -> str:
        return "/routines"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Routines")

    def get_routine_id(self, name: str) -> int:
        href = self.page.get_by_role("link", name=name, exact=True).get_attribute("href")
        assert href is not None
        return uuid.UUID(href.rsplit("/", 1)[-1]).int
