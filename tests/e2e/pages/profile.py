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

    def expect_passkeys(self, labels: list[str]) -> None:
        expect(self.passkey_rows).to_have_count(len(labels))
        for index, label in enumerate(labels):
            expect(self.passkey_rows.nth(index).locator("td").first).to_have_text(label)

    def expect_no_passkey_section(self) -> None:
        expect(self.name_input).to_be_visible()
        expect(self.page.get_by_test_id("add-passkey")).to_have_count(0)

    def add_passkey(self) -> None:
        self.page.get_by_test_id("add-passkey").click()
        self.wait_until_idle()

    def rename_passkey(self, index: int, label: str) -> None:
        self._open_passkey_options(index)
        self.page.get_by_test_id("options-rename-passkey").click()
        rename_dialog = self.page.get_by_test_id("dialog").nth(1)
        rename_dialog.wait_for(state="visible")
        rename_dialog.get_by_test_id("passkey-name").fill(label)
        rename_dialog.get_by_test_id("dialog-save").click()
        rename_dialog.wait_for(state="hidden")
        self.wait_until_idle()

    def delete_passkey(self, index: int) -> None:
        self._open_passkey_options(index)
        self.page.get_by_test_id("options-delete-passkey").click()
        delete_dialog = self.page.get_by_test_id("dialog").nth(1)
        delete_dialog.wait_for(state="visible")
        delete_dialog.get_by_test_id("dialog-delete").click()
        delete_dialog.wait_for(state="hidden")
        self.wait_until_idle()

    def expect_passkey_deletion_blocked(self, index: int) -> None:
        self._open_passkey_options(index)
        expect(self.page.get_by_test_id("options-rename-passkey")).to_be_visible()
        expect(self.page.get_by_test_id("options-delete-passkey")).to_have_count(0)
        self.page.get_by_test_id("options-menu-close").click()
        self.page.get_by_test_id("options-menu").wait_for(state="detached")

    @property
    def name_input(self) -> Locator:
        return self.dialog.root.get_by_test_id("profile-name")

    @property
    def height_input(self) -> Locator:
        return self.dialog.root.get_by_test_id("profile-height")

    @property
    def passkey_rows(self) -> Locator:
        return self.dialog.root.get_by_test_id("table").locator("tbody tr")

    def _open_passkey_options(self, index: int) -> None:
        self.page.get_by_test_id("item-options").nth(index).click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
