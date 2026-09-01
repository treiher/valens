from __future__ import annotations

import uuid
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog
from .exercises import ExerciseListDialog
from .utils import get_text, parse_float, parse_int

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


class TrainingSessionPage(BasePage):
    def __init__(self, page: Page, session_id: int, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.session_id = session_id
        self.replace_exercise_dialog: ExerciseListDialog = ExerciseListDialog(page)
        self.session_exercise_notes_dialog: SessionExerciseNotesDialog = SessionExerciseNotesDialog(
            page
        )
        self.exercise_notes_dialog: ExerciseNotesDialog = ExerciseNotesDialog(page)
        self.one_rep_max_dialog: OneRepMaxCalculatorDialog = OneRepMaxCalculatorDialog(page)
        self.drop_set_dialog: DropSetCalculatorDialog = DropSetCalculatorDialog(page)

    @property
    def path(self) -> str:
        return f"/training_session/{uuid.UUID(int=self.session_id)}"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Training session")

    def expect_view_mode(self) -> None:
        expect(self.page.get_by_test_id("session-notes")).to_be_hidden()

    def expect_edit_mode(self) -> None:
        expect(self.page.get_by_test_id("session-notes")).to_be_visible()

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
            for tds in [[td.inner_text().strip() for td in row.locator("td").all()]]
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

    def get_form_input_modes(self) -> list[str | None]:
        self.expect_edit_mode()
        return [
            inp.get_attribute("inputmode")
            for row in self.page.locator("table tr").all()
            for inputs_in_row in [row.locator("input").all()]
            if len(inputs_in_row) == 4
            for inp in inputs_in_row
        ][:4]

    def set_form(
        self, index: int, values: tuple[int | None, int | None, float | None, float | None]
    ) -> None:
        self.set_form_text(index, tuple(str(v) if v is not None else "" for v in values))

    def get_form_text(self, index: int) -> tuple[str, ...]:
        self.expect_edit_mode()
        return tuple(inp.input_value() for inp in self._form_inputs(index))

    def set_form_text(self, index: int, values: tuple[str, ...]) -> None:
        self.expect_edit_mode()
        for inp, val in zip(self._form_inputs(index), values, strict=False):
            inp.fill(val)

    def _form_inputs(self, index: int) -> list[Locator]:
        return [
            inputs_in_row
            for row in self.page.locator("table tr").all()
            for inputs_in_row in [row.locator("input").all()]
            if len(inputs_in_row) == 4
        ][index]

    def get_set_numbers(self, count: int) -> list[str]:
        expect(self.page.get_by_test_id("set-number").nth(count - 1)).to_be_visible()
        return [
            element.inner_text().strip()
            for element in self.page.get_by_test_id("set-number").all()[:count]
        ]

    def get_notes(self) -> str:
        self.expect_edit_mode()
        return self.page.get_by_test_id("session-notes").input_value()

    def set_notes(self, text: str) -> None:
        self.expect_edit_mode()
        self.page.get_by_test_id("session-notes").fill(text)

    def get_displayed_notes(self) -> str:
        self.expect_view_mode()
        return get_text(self.page.get_by_test_id("session-notes-text"))

    def expect_set_action_button_disabled(self, index: int = 0) -> None:
        self.expect_edit_mode()
        expect(self.page.get_by_test_id("set-action").nth(index)).to_be_disabled()

    def count_form_rows(self) -> int:
        return len(self.get_form())

    def end_training_session(self) -> None:
        self.page.get_by_test_id("activity-bar-end-session").click()
        self.dialog.wait_until_open()
        self.page.get_by_test_id("activity-bar-end-session-confirm").click()
        self.dialog.wait_until_closed()

    def cancel_end_training_session(self) -> None:
        self.page.get_by_test_id("activity-bar-end-session").click()
        self.dialog.wait_until_open()
        self.page.get_by_test_id("activity-bar-end-session-cancel").click()
        self.dialog.wait_until_closed()

    def expect_end_training_session_visible(self) -> None:
        expect(self.page.get_by_test_id("activity-bar-end-session")).to_be_visible()

    def expect_end_training_session_hidden(self) -> None:
        expect(self.page.get_by_test_id("activity-bar-end-session")).to_be_hidden()

    def open_exercise_options(self, exercise_idx: int = 0) -> None:
        self.page.get_by_test_id("item-options").nth(exercise_idx).click()
        self.page.get_by_test_id("options-menu").wait_for(state="visible")

    def show_1rm(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-1rm").click()
        self.one_rep_max_dialog.wait_until_open()

    def show_drop_set(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-drop-set").click()
        self.drop_set_dialog.wait_until_open()

    def open_replace_exercise_dialog(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-replace-exercise").click()
        self.replace_exercise_dialog.wait_until_open()

    def edit_session_exercise_notes(self, note: str, exercise_idx: int = 0) -> None:
        self.open_session_exercise_notes_dialog(exercise_idx)
        self.session_exercise_notes_dialog.set_note(note)
        self.session_exercise_notes_dialog.save()
        self.wait_until_idle()

    def open_session_exercise_notes_dialog(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-show-session-notes").click()
        self.session_exercise_notes_dialog.wait_until_open()

    def get_session_exercise_notes(self, exercise_idx: int = 0) -> str:
        return (
            self.page.get_by_test_id("session-exercise-notes")
            .nth(exercise_idx)
            .inner_text()
            .strip()
        )

    def click_session_exercise_notes(self, exercise_idx: int = 0) -> None:
        self.page.get_by_test_id("session-exercise-notes").nth(exercise_idx).click()
        self.session_exercise_notes_dialog.wait_until_open()

    def get_exercise_notes(self, exercise_idx: int = 0) -> str:
        return get_text(self.page.get_by_test_id("exercise-notes").nth(exercise_idx))

    def expect_no_exercise_notes(self) -> None:
        expect(self.page.get_by_test_id("exercise-notes")).to_have_count(0)

    def click_exercise_notes(self, exercise_idx: int = 0) -> None:
        self.page.get_by_test_id("exercise-notes").nth(exercise_idx).click()
        self.exercise_notes_dialog.wait_until_open()

    def open_exercise_notes_dialog(self, exercise_idx: int = 0) -> None:
        self.open_exercise_options(exercise_idx)
        self.page.get_by_test_id("options-edit-exercise-notes").click()
        self.exercise_notes_dialog.wait_until_open()

    def set_exercise_notes(self, notes: str) -> None:
        self.exercise_notes_dialog.set_notes(notes)
        self.exercise_notes_dialog.save()
        self.wait_until_idle()


class SessionExerciseNotesDialog(Dialog):
    def get_note(self) -> str:
        return self.root.get_by_test_id("session-exercise-notes-input").input_value()

    def set_note(self, note: str) -> None:
        self.root.get_by_test_id("session-exercise-notes-input").fill(note)

    def get_previous_notes(self) -> list[str]:
        return [
            element.inner_text().strip()
            for element in self.root.get_by_test_id("previous-session-exercise-note").all()
        ]

    def reuse_previous_note(self, idx: int = 0) -> None:
        self.root.get_by_test_id("session-exercise-notes-reuse").nth(idx).click()


class ExerciseNotesDialog(Dialog):
    def get_notes(self) -> str:
        return self.root.get_by_test_id("exercise-notes-input").input_value()

    def set_notes(self, notes: str) -> None:
        self.root.get_by_test_id("exercise-notes-input").fill(notes)


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
                return (
                    cells[1].inner_text().strip(),
                    cells[2].inner_text().strip(),
                )
        msg = f"Row for {percentage}% not found"
        raise ValueError(msg)


class DropSetCalculatorDialog(Dialog):
    def get_start_weight(self) -> str:
        return self.root.get_by_test_id("drop-set-start-weight").input_value()

    def get_drop_percentage(self) -> str:
        return self.root.get_by_test_id("drop-set-drop-percentage").input_value()

    def get_increment(self) -> str:
        return self.root.get_by_test_id("drop-set-increment").input_value()

    def set_start_weight(self, weight: float) -> None:
        self.root.get_by_test_id("drop-set-start-weight").fill(str(weight))

    def set_drop_percentage(self, percentage: float) -> None:
        self.root.get_by_test_id("drop-set-drop-percentage").fill(str(percentage))

    def set_increment(self, increment: str) -> None:
        self.root.get_by_test_id("drop-set-increment").select_option(increment)

    def get_rows(self) -> list[tuple[str, str, str]]:
        """Return (nominal_pct, actual_pct, weight) for all rows."""
        rows = self.root.locator("table tbody tr").all()
        result = []
        for row in rows:
            cells = row.locator("td").all()
            result.append(
                (
                    cells[0].inner_text().strip(),
                    cells[1].inner_text().strip(),
                    cells[2].inner_text().strip(),
                )
            )
        return result
