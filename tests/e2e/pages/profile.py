from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BaseDialog

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class ProfileDialog(BaseDialog):
    def open(self) -> None:
        self.navbar.open_profile()
        self.dialog.wait_until_open()
        self.wait_until_idle()

    def expect_name(self, name: str) -> None:
        expect(self.name_input).to_have_value(name)

    def expect_height(self, height: str) -> None:
        expect(self.height_input).to_have_value(height)

    def edit_name(self, name: str) -> None:
        self.name_input.fill(name)
        self.dialog.save()
        self.wait_until_idle()

    def edit_height(self, height: str) -> None:
        self.height_input.fill(height)
        self.dialog.save()
        self.wait_until_idle()

    @property
    def name_input(self) -> Locator:
        return self.dialog.root.get_by_test_id("profile-name")

    @property
    def height_input(self) -> Locator:
        return self.dialog.root.get_by_test_id("profile-height")
