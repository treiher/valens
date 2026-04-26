from __future__ import annotations

from playwright.sync_api import expect

from .base import BasePage


class LoginPage(BasePage):
    @property
    def path(self) -> str:
        return "/login"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Valens")

    def users(self) -> list[str]:
        buttons = self.page.locator('[data-testid^="login-"]').all()
        return [b.inner_text() for b in buttons]

    def login(self, username: str) -> None:
        self.page.get_by_test_id(f"login-{username}").click()
        self.page.wait_for_selector('[data-testid="home-training"]')
