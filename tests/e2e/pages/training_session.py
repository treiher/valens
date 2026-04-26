from __future__ import annotations

import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage
from .utils import parse_float, parse_int

if TYPE_CHECKING:
    from playwright.sync_api import Page


class TrainingSessionPage(BasePage):
    def __init__(self, page: Page, session_id: int, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.session_id = session_id

    @property
    def path(self) -> str:
        return f"/training_session/{uuid.UUID(int=self.session_id)}"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Training session")

    def expect_view_mode(self) -> None:
        expect(self.page.locator("textarea").first).to_be_hidden()

    def expect_edit_mode(self) -> None:
        expect(self.page.locator("textarea").first).to_be_visible()

    def edit(self) -> None:
        if self.fab_has_icon("edit"):
            self.fab().click()
        self.expect_edit_mode()

    def view(self) -> None:
        if not self.fab_has_icon("edit"):
            self.fab().click()
        self.expect_view_mode()

    def save(self) -> None:
        self.expect_fab("save")
        self.fab().click()
        self.wait_until_idle()

    def get_sets(self) -> list[tuple[int | None, int | None, float | None, float | None]]:
        self.expect_view_mode()
        return [
            (
                parse_int(tds[1]),
                parse_int(tds[2]),
                parse_float(tds[3]),
                parse_float(tds[4]),
            )
            for row in self.page.locator("table tr").all()
            for tds in [[td.inner_text() for td in row.locator("td").all()]]
            if len(tds) == 5
        ]

    def get_form(self) -> list[tuple[int | None, int | None, float | None, float | None]]:
        self.expect_edit_mode()
        return [
            (
                parse_int(tds[0]),
                parse_int(tds[1]),
                parse_float(tds[2]),
                parse_float(tds[3]),
            )
            for row in self.page.locator("table tr").all()
            for tds in [[td.input_value() for td in row.locator("input").all()]]
            if len(tds) == 4
        ]

    def set_form(
        self, index: int, values: tuple[int | None, int | None, float | None, float | None]
    ) -> None:
        self.expect_edit_mode()
        inputs_in_row = [
            inputs_in_row
            for row in self.page.locator("table tr").all()
            for inputs_in_row in [row.locator("input").all()]
            if len(inputs_in_row) == 4
        ][index]
        for inp, val in zip(inputs_in_row, values, strict=False):
            inp.fill(str(val) if val is not None else "")

    def get_notes(self) -> str:
        self.expect_edit_mode()
        return self.page.locator("textarea").first.input_value()

    def set_notes(self, text: str) -> None:
        self.expect_edit_mode()
        self.page.locator("textarea").first.fill(text)
