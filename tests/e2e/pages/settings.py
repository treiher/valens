from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BaseDialog

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class SettingsDialog(BaseDialog):
    def open(self) -> None:
        self.navbar.open_settings()
        self.dialog.wait_until_open()

    def expect_open(self) -> None:
        expect(self.dialog.root.get_by_text("Beep volume")).to_be_visible()

    def expect_notifications_unsupported(self) -> None:
        expect(self.dialog.root.get_by_text("Not supported by this browser")).to_be_visible()

    def toggle_rpe(self) -> None:
        self.rpe_button.click()

    def expect_rpe(self, state: str) -> None:
        expect(self.rpe_button).to_have_text(state)

    @property
    def rpe_button(self) -> Locator:
        return self.dialog.root.get_by_test_id("settings-rpe")
