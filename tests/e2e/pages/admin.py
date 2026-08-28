from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BaseDialog, NestedDialog
from .utils import get_focused_selection

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


class AdminDialog(BaseDialog):
    def __init__(self, page: Page) -> None:
        super().__init__(page)
        self.dialog = NestedDialog(page)

    def open(self) -> None:
        self.navbar.open_administration()
        self.page.get_by_test_id("dialog").first.wait_for(state="visible")
        self.wait_until_idle()

    def expect_open(self) -> None:
        expect(self.page.get_by_test_id("add-user")).to_be_attached()

    def expect_not_authorized(self) -> None:
        expect(self.page.get_by_test_id("not-authorized")).to_be_visible()

    def add_user(self, name: str, height: str, role: str | None = None) -> None:
        self.page.get_by_test_id("add-user").click()
        self.dialog.wait_until_open()
        self.dialog.root.get_by_test_id("user-name").fill(name)
        self.dialog.root.get_by_test_id("user-height").fill(height)
        if role is not None:
            self.role_select.select_option(role)
        self.dialog.save()
        self.wait_until_idle()

    def get_user_name_selection(self, name: str) -> str:
        self.open_edit_user_dialog(name)
        return get_focused_selection(self.dialog.root.get_by_test_id("user-name"))

    def edit_user_height(self, name: str, height: str) -> None:
        self.open_edit_user_dialog(name)
        self.dialog.root.get_by_test_id("user-height").fill(height)
        self.dialog.save()
        self.wait_until_idle()

    def edit_user_role(self, name: str, role: str) -> None:
        self.open_edit_user_dialog(name)
        self.select_role(role)
        self.dialog.save()
        self.wait_until_idle()

    def open_edit_user_dialog(self, name: str) -> None:
        self._open_user_options(name)
        self.page.get_by_test_id("options-edit-user").click()
        self.dialog.wait_until_open()

    def open_delete_user_dialog(self, name: str) -> None:
        self._open_user_options(name)
        self.page.get_by_test_id("options-delete-user").click()
        self.dialog.wait_until_open()

    def select_role(self, role: str) -> None:
        self.role_select.select_option(role)

    def user_row(self, name: str) -> Locator:
        return self.page.get_by_role("cell", name=name, exact=True)

    def expect_user_role(self, name: str, role: str) -> None:
        row = self.page.get_by_role("row").filter(has=self.user_row(name))
        expect(row.get_by_role("cell", name=role, exact=True)).to_be_visible()

    def create_login_link(self, name: str) -> str:
        self._open_user_options(name)
        self.page.get_by_test_id("options-create-login-link").click()
        url_input = self.page.get_by_test_id("login-link-url")
        expect(url_input).to_be_visible()
        url = url_input.input_value()
        assert url
        return url

    def expect_passkey_login_unavailable_info(self) -> None:
        expect(self.page.get_by_test_id("passkey-login-unavailable")).to_be_visible()

    def expect_no_passkey_login_unavailable_info(self) -> None:
        expect(self.page.get_by_test_id("passkey-login-unavailable")).to_have_count(0)

    def expect_no_login_link_option(self, name: str) -> None:
        self._open_user_options(name)
        expect(self.page.get_by_test_id("options-edit-user")).to_be_visible()
        expect(self.page.get_by_test_id("options-create-login-link")).to_have_count(0)

    def open_user_passkeys(self, name: str) -> None:
        self._open_user_options(name)
        self.page.get_by_test_id("options-manage-passkeys").click()
        self.wait_until_idle()

    def expect_user_passkeys(self, count: int) -> None:
        if count == 0:
            expect(self.page.get_by_test_id("no-passkeys")).to_be_visible()
        else:
            expect(self.page.get_by_test_id("delete-passkey")).to_have_count(count)

    def delete_user_passkey(self, index: int) -> None:
        self.page.get_by_test_id("delete-passkey").nth(index).click()
        delete_dialog = self.page.get_by_test_id("dialog").nth(2)
        delete_dialog.wait_for(state="visible")
        delete_dialog.get_by_test_id("dialog-delete").click()
        delete_dialog.wait_for(state="hidden")
        self.wait_until_idle()

    @property
    def role_select(self) -> Locator:
        return self.dialog.root.get_by_test_id("user-role")

    def _open_user_options(self, name: str) -> None:
        row = self.page.get_by_role("row").filter(has=self.user_row(name))
        row.get_by_test_id("item-options").click()
