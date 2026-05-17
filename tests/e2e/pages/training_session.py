from __future__ import annotations

import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog
from .utils import parse_float, parse_int

if TYPE_CHECKING:
    from playwright.sync_api import Page


class TrainingSessionPage(BasePage):
    def __init__(self, page: Page, session_id: int, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.session_id = session_id
        self.exercise_note_dialog: ExerciseNoteDialog = ExerciseNoteDialog(page)
        self.one_rep_max_dialog: OneRepMaxCalculatorDialog = OneRepMaxCalculatorDialog(page)

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

    def open_exercise_options(self, exercise_idx: int = 0) -> None:
        self.page.get_by_test_id("item-options").nth(exercise_idx).click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")

    def show_1rm(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-1rm").click()
        self.one_rep_max_dialog.wait_until_open()

    def edit_exercise_note(self, note: str, exercise_idx: int = 0) -> None:
        self.open_exercise_note_dialog(exercise_idx)
        self.exercise_note_dialog.set_note(note)
        self.exercise_note_dialog.save()
        self.wait_until_idle()

    def open_exercise_note_dialog(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-show-exercise-notes").click()
        self.exercise_note_dialog.wait_until_open()

    def get_exercise_note(self, exercise_idx: int = 0) -> str:
        return self.page.get_by_test_id("exercise-note").nth(exercise_idx).inner_text().strip()

    def click_exercise_note(self, exercise_idx: int = 0) -> None:
        self.page.get_by_test_id("exercise-note").nth(exercise_idx).click()
        self.exercise_note_dialog.wait_until_open()


class ExerciseNoteDialog(Dialog):
    def get_note(self) -> str:
        return self.root.locator("textarea").input_value()

    def set_note(self, note: str) -> None:
        self.root.locator("textarea").fill(note)

    def get_previous_notes(self) -> list[str]:
        return [
            element.inner_text().strip()
            for element in self.root.get_by_test_id("previous-exercise-note").all()
        ]

    def reuse_previous_note(self, idx: int = 0) -> None:
        self.root.get_by_test_id("exercise-note-reuse").nth(idx).click()


class OneRepMaxCalculatorDialog(Dialog):
    def get_weight(self) -> str:
        return self.root.get_by_test_id("1rm-weight").input_value()

    def get_reps(self) -> str:
        return self.root.get_by_test_id("1rm-reps").input_value()

    def set_weight(self, weight: float) -> None:
        self.root.get_by_test_id("1rm-weight").fill(str(weight))

    def set_reps(self, reps: int) -> None:
        self.root.get_by_test_id("1rm-reps").fill(str(reps))

    def get_table_row(self, percentage: int) -> tuple[str, str]:
        """Return (reps, weight) for a given percentage row."""
        rows = self.root.locator("table tbody tr").all()
        for row in rows:
            cells = row.locator("td").all()
            if cells[0].inner_text().strip() == str(percentage):
                return (cells[1].inner_text().strip(), cells[2].inner_text().strip())
        msg = f"Row for {percentage}% not found"
        raise ValueError(msg)
