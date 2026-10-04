from __future__ import annotations

from playwright.sync_api import expect

from .base import BaseDialog


class SettingsDialog(BaseDialog):
    def open(self) -> None:
        self.navbar.open_settings()
        self.dialog.wait_until_open()

    def expect_open(self) -> None:
        expect(self.dialog.root.get_by_text("Volume", exact=True)).to_be_visible()

    def choose_theme(self, theme: str) -> None:
        self.dialog.root.get_by_test_id("settings-theme").get_by_title(theme, exact=True).click()

    def enable_notifications(self) -> None:
        self.dialog.root.get_by_test_id("settings-notifications").click()

    def expect_notifications(self, *, enabled: bool) -> None:
        expect(self.dialog.root.get_by_test_id("settings-notifications")).to_be_checked(
            checked=enabled
        )

    def expect_notifications_not_allowed(self) -> None:
        expect(self.dialog.root.get_by_test_id("settings-notifications")).to_be_disabled()

    def expect_notifications_explanation(self) -> None:
        expect(
            self.dialog.root.get_by_text("To enable notifications, open the site settings")
        ).to_be_visible()

    def disable_rpe(self) -> None:
        self.dialog.root.get_by_test_id("settings-rpe").click()

    def expect_rpe(self, *, enabled: bool) -> None:
        expect(self.dialog.root.get_by_test_id("settings-rpe")).to_be_checked(checked=enabled)
