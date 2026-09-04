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

    def choose_theme(self, theme: str) -> None:
        self.dialog.root.get_by_test_id(f"settings-theme-{theme}").click()

    def toggle_metronome(self) -> None:
        self.dialog.root.get_by_test_id("settings-metronome").click()

    def expect_metronome(self, state: str) -> None:
        expect(self.dialog.root.get_by_test_id("settings-metronome")).to_have_text(state)

    def toggle_notifications(self) -> None:
        self.notifications_button.click()

    def expect_notifications(self, state: str) -> None:
        expect(self.notifications_button).to_have_text(state)

    def expect_notifications_explanation(self) -> None:
        expect(
            self.dialog.root.get_by_text("To enable notifications, open the site settings")
        ).to_be_visible()

    @property
    def notifications_button(self) -> Locator:
        return self.dialog.root.get_by_test_id("settings-notifications")

    def toggle_rpe(self) -> None:
        self.rpe_button.click()

    def expect_rpe(self, state: str) -> None:
        expect(self.rpe_button).to_have_text(state)

    @property
    def rpe_button(self) -> Locator:
        return self.dialog.root.get_by_test_id("settings-rpe")
