from __future__ import annotations

import re
import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import Locator, expect

from .base import BasePage, Dialog
from .utils import MUSCLE_SECTIONS, expect_session_items, get_text

if TYPE_CHECKING:
    from playwright.sync_api import Page


class ExercisePage(BasePage):
    def __init__(self, page: Page, exercise_id: int | None = None) -> None:
        super().__init__(page)

        self.exercise_id = exercise_id
        self.notes_dialog: ExerciseNotesDialog = ExerciseNotesDialog(page)

    @property
    def path(self) -> str:
        assert self.exercise_id is not None
        return f"/exercise/{uuid.UUID(int=self.exercise_id)}"

    def expect_session_items(self) -> None:
        expect_session_items(self.page)

    def get_set_history_sides(self) -> list[list[str]]:
        return [
            [get_text(cell) for cell in row.locator("th, td").all()]
            for row in self.page.get_by_test_id("side-header")
            .or_(self.page.get_by_test_id("set-sides-row"))
            .all()
        ]

    def get_personal_records(self) -> list[list[str]]:
        """Return the cells of the personal records without the dates of the records."""
        return [
            [
                cell.evaluate(
                    """cell => {
                        const copy = cell.cloneNode(true);
                        const dates = copy.querySelectorAll('[data-testid="record-date"]');
                        dates.forEach(d => d.remove());
                        return copy.textContent.trim();
                    }"""
                )
                for cell in row.locator("th, td").all()
            ]
            for row in self.page.get_by_test_id("personal-records").locator("tr").all()
        ]

    def get_personal_record_dates(self) -> list[str]:
        return [
            get_text(date)
            for date in self.page.get_by_test_id("personal-records")
            .get_by_test_id("record-date")
            .all()
        ]

    def click_routine(self, name: str) -> None:
        self.page.get_by_role("link", name=name, exact=True).click()

    def session_exercise_notes(self) -> Locator:
        return self.page.get_by_test_id("session-exercise-notes")

    def session_exercise_measurements(self) -> Locator:
        return self.page.get_by_test_id("session-exercise-measurement")

    def get_notes(self) -> str:
        return get_text(self.page.get_by_test_id("exercise-notes"))

    def click_notes(self) -> None:
        self.page.get_by_test_id("exercise-notes").click()
        self.notes_dialog.wait_until_open()

    def open_notes_dialog(self) -> None:
        self.fab().click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
        self.page.get_by_test_id("options-edit-exercise-notes").click()
        self.notes_dialog.wait_until_open()

    def set_notes(self, notes: str) -> None:
        self.notes_dialog.set_notes(notes)
        self.notes_dialog.save()
        self.wait_until_idle()

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

    def stretch_tag(self, name: str) -> Locator:
        return self.page.get_by_test_id("stretch-tag").filter(has_text=name)

    def expect_stretched_muscle(self, name: str) -> None:
        expect(self.stretch_tag(name)).to_be_visible()

    def expect_no_stretched_muscle(self, name: str) -> None:
        expect(self.stretch_tag(name)).to_have_count(0)

    def cycle_muscle(self, name: str, kind: str) -> None:
        """Cycle the level of a muscle of the kind "worked" or "stretched"."""
        self._open_properties_dialog()
        self.dialog.root.get_by_test_id(f"property-section-{MUSCLE_SECTIONS[kind]}").get_by_test_id(
            "multi-toggle-tag"
        ).get_by_text(name, exact=True).click()
        self.dialog.save()
        self.wait_until_idle()

    def toggle_properties(self, *names: str) -> None:
        self._open_properties_dialog()
        self.toggle_properties_in_dialog(*names)

    def toggle_properties_in_dialog(self, *names: str) -> None:
        for name in names:
            self.dialog.root.get_by_test_id("property-chip").filter(
                has_text=re.compile(f"^{re.escape(name)}$")
            ).click()
        self.dialog.save()
        self.wait_until_idle()

    def click_caption(self, caption: str) -> None:
        self.page.get_by_test_id("caption").get_by_text(caption, exact=True).click()
        self.dialog.wait_until_open()

    def click_no_data(self) -> None:
        self.page.get_by_test_id("no-data").click()
        self.dialog.wait_until_open()

    def expect_no_dialog(self) -> None:
        # A dialog opened by a preceding click is only rendered after the click was handled.
        self.page.evaluate(
            "() => new Promise("
            "resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))"
            ")"
        )
        expect(self.page.get_by_test_id("dialog")).to_have_count(0)

    def open_catalog_update(self) -> None:
        self.fab().click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
        self.page.get_by_test_id("options-update-from-catalog").click()
        self.dialog.wait_until_open()

    def apply_catalog_updates(self) -> None:
        self.dialog.root.get_by_test_id("update-from-catalog").click()

    def _open_properties_dialog(self) -> None:
        self.fab().click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")
        self.page.get_by_test_id("options-properties").click()
        self.dialog.wait_until_open()

    def expect_page(self) -> None:
        expect(self.page).to_have_url(re.compile(r"/exercise/[^/]+(/[a-z]+)?$"))


class ExerciseNotesDialog(Dialog):
    def get_notes(self) -> str:
        return self.root.get_by_test_id("exercise-notes-input").input_value()

    def set_notes(self, notes: str) -> None:
        self.root.get_by_test_id("exercise-notes-input").fill(notes)
