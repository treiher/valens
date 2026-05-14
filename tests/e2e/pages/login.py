from __future__ import annotations

from playwright.sync_api import Locator, expect

from .base import BasePage


class LoginPage(BasePage):
    @property
    def path(self) -> str:
        return "/login"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("login-username")).to_be_visible()

    def login(self, username: str) -> None:
        self.submit_username(username)
        self.page.wait_for_selector('[data-testid="home-training-sessions"]')

    def login_with_enter(self, username: str) -> None:
        self.type_username(username)
        self.page.get_by_test_id("login-username").press("Enter")
        self.page.wait_for_selector('[data-testid="home-training-sessions"]')

    def submit_username(self, username: str) -> None:
        self.type_username(username)
        self.page.get_by_test_id("login-button").click()

    def type_username(self, username: str) -> None:
        self.page.get_by_test_id("login-username").fill(username)

    @property
    def error_message(self) -> Locator:
        return self.page.locator(".help.is-danger")
