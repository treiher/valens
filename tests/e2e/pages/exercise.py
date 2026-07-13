from __future__ import annotations

import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import Locator, expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Page


class ExercisePage(BasePage):
    def __init__(self, page: Page, exercise_id: int) -> None:
        super().__init__(page)

        self.exercise_id = exercise_id

    @property
    def path(self) -> str:
        return f"/exercise/{uuid.UUID(int=self.exercise_id)}"

    def exercise_note(self) -> Locator:
        return self.page.get_by_test_id("exercise-note")

    def muscle_tag(self, name: str) -> Locator:
        return self.page.get_by_test_id("muscle-tag").filter(has_text=name)

    def expect_muscle(self, name: str) -> None:
        expect(self.muscle_tag(name)).to_be_visible()

    def expect_no_muscle(self, name: str) -> None:
        expect(self.muscle_tag(name)).to_have_count(0)

    def cycle_muscle(self, name: str) -> None:
        self.fab().click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
        self.page.get_by_test_id("options-properties").click()
        self.dialog.wait_until_open()
        self.dialog.root.get_by_test_id("multi-toggle-tag").get_by_text(name, exact=True).click()
        self.dialog.save()
        self.wait_until_idle()

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Exercise")
