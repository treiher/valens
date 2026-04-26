from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from tests.e2e.const import BASE_URL

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


class BasePage:
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        self.page = page
        self.base_url = BASE_URL if base_url is None else base_url

        self.dialog = Dialog(page)
        self.table = Table(page)

    @property
    @abstractmethod
    def path(self) -> str:
        raise NotImplementedError

    @abstractmethod
    def expect_page(self) -> None:
        raise NotImplementedError

    def goto(self) -> None:
        self.page.goto(f"{self.base_url}{self.path}")
        self.page.wait_for_load_state("networkidle")
        self.expect_page()

    def go_back(self) -> None:
        self.page.get_by_test_id("navbar-back").click()

    def reload(self) -> None:
        self.page.reload()
        self.page.wait_for_load_state("networkidle")

    def logout(self) -> None:
        self.page.get_by_test_id("navbar-logout").click()

    def wait_until_idle(self) -> None:
        self.page.get_by_test_id("loading").wait_for(state="detached")
        self.page.locator(".is-loading").wait_for(state="detached")

    def delete_item(self, index: int) -> None:
        if self.page.get_by_test_id("item-delete").nth(index).is_visible(timeout=1000):
            self.page.get_by_test_id("item-delete").nth(index).click()
        else:
            self._open_item_options(index)
            self.page.get_by_test_id("options-delete").click()
        self.wait_until_idle()

    def edit_item(self, index: int) -> None:
        self._open_item_options(index)
        self.page.get_by_test_id("options-edit").click()
        self.wait_until_idle()

    def rename_item(self, index: int) -> None:
        self._open_item_options(index)
        self.page.get_by_test_id("options-rename").click()
        self.wait_until_idle()

    def fab(self) -> Locator:
        return self.page.get_by_test_id("fab")

    def fab_has_icon(self, icon: str) -> bool:
        return self.fab().locator(f"i.fa-{icon}").first.is_visible()

    def expect_fab(self, icon: str) -> None:
        expect(self.fab().locator(f"i.fa-{icon}")).to_be_visible()

    def _open_item_options(self, index: int) -> None:
        self.page.get_by_test_id("item-options").nth(index).click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")


class PageElement:
    def __init__(self, page: Page) -> None:
        self.page = page


class Dialog(PageElement):
    @property
    def root(self) -> Locator:
        return self.page.get_by_test_id("dialog")

    def wait_until_open(self) -> None:
        self.root.wait_for(state="visible")

    def wait_until_closed(self) -> None:
        self.root.wait_for(state="hidden")

    def cancel(self) -> None:
        self.root.get_by_test_id("dialog-cancel").click()
        self.wait_until_closed()

    def save(self) -> None:
        self.root.get_by_test_id("dialog-save").click()
        self.wait_until_closed()

    def delete(self) -> None:
        self.root.get_by_test_id("dialog-delete").click()
        self.wait_until_closed()

    def no(self) -> None:
        self.root.get_by_test_id("dialog-no").click()
        self.wait_until_closed()


class Table(PageElement):
    def root(self, table_idx: int) -> Locator:
        return self.page.get_by_test_id("table").nth(table_idx - 1)

    def get_value(self, table_idx: int, row: int, col: int) -> str:
        return (
            self.root(table_idx)
            .locator("tbody tr")
            .nth(row - 1)
            .locator("td")
            .nth(col - 1)
            .inner_text()
            .strip()
        )

    def get_headers(self, table_idx: int = 1) -> dict[str, int]:
        headers = self.root(table_idx).locator("thead th").all()
        return {th.inner_text(): idx for idx, th in enumerate(headers, start=1)}

    def get_body(self, table_idx: int = 1) -> list[list[str]]:
        rows = self.root(table_idx).locator("tbody tr").all()
        return [[cell.inner_text() for cell in row.locator("td").all()] for row in rows]

    def expect_value(self, table_idx: int, row: int, col: int, text: str) -> None:
        expect(
            self.root(table_idx).locator("tbody tr").nth(row - 1).locator("td").nth(col - 1)
        ).to_contain_text(text)
