from __future__ import annotations

import re
import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import Locator, expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Page


class ExercisePage(BasePage):
    def __init__(self, page: Page, exercise_id: int | None = None) -> None:
        super().__init__(page)

        self.exercise_id = exercise_id

    @property
    def path(self) -> str:
        assert self.exercise_id is not None
        return f"/exercise/{uuid.UUID(int=self.exercise_id)}"

    def exercise_note(self) -> Locator:
        return self.page.get_by_test_id("exercise-note")

    def get_properties(self) -> list[str]:
        return self.page.get_by_test_id("property-tag").all_inner_texts()

    def get_muscles(self) -> list[str]:
        return self.page.get_by_test_id("muscle-tag").all_inner_texts()

    def muscle_tag(self, name: str) -> Locator:
        return self.page.get_by_test_id("muscle-tag").filter(has_text=name)

    def expect_muscle(self, name: str) -> None:
        expect(self.muscle_tag(name)).to_be_visible()

    def expect_no_muscle(self, name: str) -> None:
        expect(self.muscle_tag(name)).to_have_count(0)

    def cycle_muscle(self, name: str) -> None:
        self._open_properties_dialog()
        self.dialog.root.get_by_test_id("multi-toggle-tag").get_by_text(name, exact=True).click()
        self.dialog.save()
        self.wait_until_idle()

    def toggle_properties(self, *names: str) -> None:
        self._open_properties_dialog()
        for name in names:
            self.dialog.root.get_by_test_id("property-chip").filter(
                has_text=re.compile(f"^{re.escape(name)}$")
            ).click()
        self.dialog.save()
        self.wait_until_idle()

    def _open_properties_dialog(self) -> None:
        self.fab().click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
        self.page.get_by_test_id("options-properties").click()
        self.dialog.wait_until_open()

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Exercise")
