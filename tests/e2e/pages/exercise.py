from __future__ import annotations

import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import expect

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

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Exercise")
