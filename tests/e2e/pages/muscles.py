from __future__ import annotations

from playwright.sync_api import expect

from .base import BasePage


class MusclesPage(BasePage):
    @property
    def path(self) -> str:
        return "/muscles"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Muscles")
