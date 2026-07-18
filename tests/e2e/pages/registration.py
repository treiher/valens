from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import PageElement

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class PasskeyRegistrationView(PageElement):
    """Full-page view shown when passkey registration is required."""

    def expect_view(self) -> None:
        expect(self.register_button).to_be_visible()

    def register_passkey(self) -> None:
        self.register_button.click()

    @property
    def register_button(self) -> Locator:
        return self.page.get_by_test_id("register-passkey-button")
