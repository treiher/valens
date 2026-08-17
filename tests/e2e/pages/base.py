from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from tests.e2e.const import BASE_URL

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


def wait_until_idle(page: Page) -> None:
    page.get_by_test_id("loading").wait_for(state="detached")
    page.locator(".is-loading").wait_for(state="detached")


class BasePage:
    def __init__(self, page: Page, base_url: str | None = None) -> None:
        self.page = page
        self.base_url = BASE_URL if base_url is None else base_url

        self.navbar = Navbar(page)
        self.dialog = Dialog(page)
        self.table = Table(page)
        self.activity_bar = ActivityBar(page)
        self.notification = Notification(page)
        self.drag_ghost = DragGhost(page)

    @property
    @abstractmethod
    def path(self) -> str:
        raise NotImplementedError

    @abstractmethod
    def expect_page(self) -> None:
        raise NotImplementedError

    def goto(self, *, expect_page: bool = True) -> None:
        self.page.goto(f"{self.base_url}{self.path}")
        self.page.wait_for_load_state("networkidle")
        if expect_page:
            self.expect_page()

    def reload(self) -> None:
        self.page.reload()
        self.page.wait_for_load_state("networkidle")

    def wait_until_idle(self) -> None:
        wait_until_idle(self.page)

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

    @property
    def page_title(self) -> Locator:
        return self.page.get_by_test_id("page-title")

    def fab(self) -> Locator:
        return self.page.get_by_test_id("fab")

    def fab_has_icon(self, icon: str) -> bool:
        return self.fab().get_by_test_id(f"icon-{icon}").is_visible()

    def expect_loading_to_be_finished(self) -> None:
        expect(self.page.get_by_test_id("loading")).to_have_count(0)
        expect(self.page.locator(".is-loading")).to_have_count(0)

    def expect_fab(self, icon: str) -> None:
        expect(self.fab().get_by_test_id(f"icon-{icon}")).to_be_visible()

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

    def set_name(self, name: str) -> None:
        self.page.get_by_test_id("dialog-name").first.fill(name)

    def cancel(self) -> None:
        self.root.get_by_test_id("dialog-cancel").click()
        self.wait_until_closed()

    def save(self) -> None:
        self.click_save()
        self.wait_until_closed()

    def click_save(self) -> None:
        self.root.get_by_test_id("dialog-save").click()

    def delete(self) -> None:
        self.click_delete()
        self.wait_until_closed()

    def click_delete(self) -> None:
        self.root.get_by_test_id("dialog-delete").click()

    def no(self) -> None:
        self.root.get_by_test_id("dialog-no").click()
        self.wait_until_closed()

    def close(self) -> None:
        self.page.get_by_test_id("dialog-close").click()
        self.wait_until_closed()


class DragGhost(PageElement):
    @property
    def root(self) -> Locator:
        return self.page.get_by_test_id("drag-ghost")

    def expect_text(self, text: str) -> None:
        expect(self.root).to_have_text(text)

    def expect_contains_text(self, text: str) -> None:
        expect(self.root).to_contain_text(text)

    def expect_width_of(self, element: Locator) -> None:
        ghost_box = self.root.bounding_box()
        element_box = element.bounding_box()
        assert ghost_box
        assert element_box
        assert ghost_box["width"] == element_box["width"], (ghost_box, element_box)


class ActivityBar(PageElement):
    @property
    def root(self) -> Locator:
        return self.page.get_by_test_id("activity-bar")

    def expect_visible(self) -> None:
        expect(self.root).to_be_visible()

    def expect_hidden(self) -> None:
        expect(self.root).to_be_hidden()

    def resume(self) -> None:
        self.root.click()


class Notification(PageElement):
    @property
    def root(self) -> Locator:
        return self.page.get_by_test_id("notification")

    @property
    def progress(self) -> Locator:
        return self.root.get_by_test_id("notification-progress")

    @property
    def count(self) -> Locator:
        return self.root.get_by_test_id("notification-count")

    @property
    def reason(self) -> Locator:
        return self.root.get_by_test_id("notification-reason")

    @property
    def action(self) -> Locator:
        return self.root.get_by_test_id("notification-action")

    def expect_visible(self) -> None:
        expect(self.root).to_be_visible()

    def expect_hidden(self) -> None:
        expect(self.root).to_be_hidden()

    def expect_reason(self, reason: str, timeout: float | None = None) -> None:
        expect(self.reason).to_have_text(reason, timeout=timeout)

    def expect_action(self, action: str) -> None:
        expect(self.action).to_have_text(action)

    def expect_warning(self) -> None:
        expect(self.root).to_have_attribute("data-severity", "warning")

    def expect_error(self) -> None:
        expect(self.root).to_have_attribute("data-severity", "error")

    def expect_stacked(self, hidden: int) -> None:
        expect(self.count).to_have_text(f"+{hidden}")

    def expect_not_stacked(self) -> None:
        expect(self.count).to_have_count(0)

    def expect_auto_dismissed(self, timeout: float = 12_000) -> None:
        # Allow for the longest per-severity timeout plus margin
        expect(self.root).to_be_hidden(timeout=timeout)

    def expect_paused_while_hovered(self) -> None:
        self.root.hover()
        play_state = self.progress.evaluate(
            "el => getComputedStyle(el).animationPlayState",
        )
        assert play_state == "paused", play_state

    def dismiss(self) -> None:
        self.root.get_by_test_id("notification-close").click()
        self.expect_hidden()


class Navbar(PageElement):
    def go_back(self) -> None:
        self.page.get_by_test_id("navbar-back").click()

    def logout(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-logout").click()

    def open_profile(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-profile").click()

    def open_settings(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-settings").click()

    def open_administration(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-administration").click()

    def open_about(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-about").click()

    def expect_no_administration(self) -> None:
        self._open_menu()
        expect(self.page.get_by_test_id("navbar-administration")).to_have_count(0)

    def refresh_data(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-refresh").click()

    def open_1rm_calculator(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-1rm-calculator").click()

    def open_drop_set_calculator(self) -> None:
        self._open_menu()
        self.page.get_by_test_id("navbar-drop-set-calculator").click()

    def _open_menu(self) -> None:
        self.page.get_by_test_id("navbar-menu").click()

    def expect_synchronization_in_progress(self) -> None:
        expect(self.page.get_by_test_id("navbar-sync-indicator")).to_be_visible()

    def expect_synchronization_to_be_finished(self) -> None:
        expect(self.page.get_by_test_id("navbar-sync-indicator")).to_have_count(0)


class BaseDialog(PageElement):
    """Dialog opened from the navbar menu."""

    def __init__(self, page: Page) -> None:
        super().__init__(page)
        self.navbar = Navbar(page)
        self.dialog = Dialog(page)
        self.notification = Notification(page)

    @abstractmethod
    def open(self) -> None:
        raise NotImplementedError

    def close(self) -> None:
        self.page.get_by_test_id("dialog-close").first.click()
        self.page.get_by_test_id("dialog").first.wait_for(state="hidden")

    def wait_until_idle(self) -> None:
        wait_until_idle(self.page)

    def expect_server_unreachable(self) -> None:
        expect(self.page.get_by_test_id("server-unreachable")).to_be_visible()


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
        return {th.inner_text().strip(): idx for idx, th in enumerate(headers, start=1)}

    def get_body(self, table_idx: int = 1) -> list[list[str]]:
        rows = self.root(table_idx).locator("tbody tr").all()
        return [[cell.inner_text().strip() for cell in row.locator("td").all()] for row in rows]

    def expect_value(self, table_idx: int, row: int, col: int, text: str) -> None:
        expect(
            self.root(table_idx).locator("tbody tr").nth(row - 1).locator("td").nth(col - 1)
        ).to_contain_text(text)
