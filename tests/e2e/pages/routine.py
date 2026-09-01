from __future__ import annotations

import uuid
from dataclasses import dataclass
from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage, Dialog
from .exercises import ExerciseListDialog
from .utils import (
    drop_on_remove_zone,
    get_focused_selection,
    get_text,
    hover_after_last,
    hover_at_height,
    hover_insertion_position,
    parse_float,
    parse_int,
    scroll_to_center,
    start_drag,
)

if TYPE_CHECKING:
    from playwright.sync_api import Locator, Page


class RoutinePage(BasePage):
    def __init__(self, page: Page, routine_id: int, base_url: str | None = None) -> None:
        super().__init__(page, base_url)

        self.routine_id = routine_id
        self.notes_dialog: RoutineNotesDialog = RoutineNotesDialog(page)
        self.replace_exercise_dialog: ExerciseListDialog = ExerciseListDialog(page)

    @property
    def path(self) -> str:
        return f"/routine/{uuid.UUID(int=self.routine_id)}"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Routine")

    def get_title(self) -> str:
        return self.page.get_by_test_id("page-title").inner_text().strip()

    def get_sections(self) -> list[RoutineSection]:
        sections_data = []

        all_elements = self.page.locator('[data-testid="routine-section"]').all()

        top_level_sections = []
        for section in all_elements:
            ancestor = section.locator('xpath=ancestor::*[@data-testid="routine-section"][1]')
            if ancestor.count() == 0:
                top_level_sections.append(section)

        for section in top_level_sections:
            routine_section = RoutineSection.from_element(section)
            sections_data.append(routine_section)

        return sections_data

    def wait_for_link(self, text: str) -> None:
        self.page.get_by_text(text).first.wait_for(state="visible")

    def wait_for_link_not_present(self, text: str) -> None:
        self.page.get_by_text(text).first.wait_for(state="hidden")

    def add_exercise(self, section_idx: int, name: str) -> None:
        self.page.get_by_test_id("section-options").nth(section_idx).click()
        self.page.get_by_test_id("options-add-exercise").click()
        self._select_exercise(name)

    def add_rest(self, section_idx: int) -> None:
        self.page.get_by_test_id("section-options").nth(section_idx).click()
        self.page.get_by_test_id("options-add-rest").click()
        self.wait_until_idle()

    def add_section(self, section_idx: int) -> None:
        section_count = self.page.get_by_test_id("section-options").count()
        if section_idx < section_count:
            self.page.get_by_test_id("section-options").nth(section_idx).click()
            self.page.get_by_test_id("options-add-section").click()
        else:
            self.page.get_by_test_id("add-section").click()
        self.wait_until_idle()

    def set_rounds(self, section_idx: int, rounds: int) -> None:
        dialog = self._open_edit_dialog(section_idx)
        inp = dialog.get_by_test_id("rounds").first
        current_rounds = inp.input_value()
        if current_rounds == str(rounds):
            self.dialog.cancel()
            return
        inp.fill(str(rounds))
        self.dialog.save()
        self.wait_until_idle()

    def get_rounds_selection(self, section_idx: int) -> str:
        dialog = self._open_edit_dialog(section_idx)
        return get_focused_selection(dialog.get_by_test_id("rounds").first)

    def get_reps_selection(self, section_idx: int, activity_idx: int) -> str:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        return get_focused_selection(dialog.get_by_test_id("input-reps"))

    def get_rest_time_selection(self, section_idx: int, activity_idx: int) -> str:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        return get_focused_selection(dialog.get_by_test_id("input-time"))

    def move_up(self, section_idx: int, activity_idx: int | None = None) -> None:
        self._open_options_menu(section_idx, activity_idx)
        self.page.get_by_test_id("options-move-up").click()
        self.wait_until_idle()

    def move_down(self, section_idx: int, activity_idx: int | None = None) -> None:
        self._open_options_menu(section_idx, activity_idx)
        self.page.get_by_test_id("options-move-down").click()
        self.wait_until_idle()

    def remove(self, section_idx: int, activity_idx: int | None = None) -> None:
        self._open_options_menu(section_idx, activity_idx)
        self.page.get_by_test_id("options-remove").click()
        self.wait_until_idle()

    def drag_section(self, source_idx: int, target_idx: int) -> None:
        start_drag(self.page, self._top_level_section_handle(source_idx))
        parts = self._top_level_parts()
        if target_idx < parts.count():
            hover_insertion_position(parts, target_idx, self._top_level_section_headers())
        else:
            hover_after_last(parts)
        self.page.mouse.up()
        self.wait_until_idle()

    def drag_activity(
        self,
        source_section_idx: int,
        source_activity_idx: int,
        target_section_idx: int,
        target_idx: int | None = None,
    ) -> None:
        start_drag(self.page, self._activity_handle(source_section_idx, source_activity_idx))
        if target_idx is None:
            drop_zone = self._section(target_section_idx).get_by_test_id("empty-section-drop-zone")
            scroll_to_center(drop_zone)
            drop_zone.hover()
            expect(drop_zone).to_have_attribute("data-drop-state", "hovered")
        else:
            hover_insertion_position(
                self._section(target_section_idx).locator('> [data-testid="routine-part"]'),
                target_idx,
            )
        self.page.mouse.up()
        self.wait_until_idle()

    def drag_activity_to_section_start(
        self, source_section_idx: int, source_activity_idx: int, target_section_idx: int
    ) -> None:
        start_drag(self.page, self._activity_handle(source_section_idx, source_activity_idx))
        hover_at_height(
            self._section(target_section_idx).get_by_test_id("section-header").first, 0.75
        )
        expect(
            self._section(target_section_idx).locator('> [data-testid="routine-part"]').first
        ).to_have_attribute("data-drop-state", "insert-before")
        self.page.mouse.up()
        self.wait_until_idle()

    def start_section_drag(self, section_idx: int) -> None:
        start_drag(self.page, self._top_level_section_handle(section_idx))

    def end_drag(self) -> None:
        self.page.mouse.up()

    def top_level_part(self, index: int) -> Locator:
        return self._top_level_parts().nth(index)

    def remove_by_drag(self, section_idx: int, activity_idx: int | None = None) -> None:
        handle = (
            self._section_handle(section_idx)
            if activity_idx is None
            else self._activity_handle(section_idx, activity_idx)
        )
        start_drag(self.page, handle)
        drop_on_remove_zone(self.page)
        self.wait_until_idle()

    def _section(self, section_idx: int) -> Locator:
        return self.page.get_by_test_id("routine-section").nth(section_idx)

    def _section_handle(self, section_idx: int) -> Locator:
        return self._section(section_idx).get_by_test_id("section-handle").first

    def _activity_handle(self, section_idx: int, activity_idx: int) -> Locator:
        return self._section(section_idx).get_by_test_id("activity-handle").nth(activity_idx)

    def _top_level_parts(self) -> Locator:
        return self.page.get_by_test_id("routine-parts").locator('> [data-testid="routine-part"]')

    def _top_level_section_headers(self) -> Locator:
        return self._top_level_parts().locator(
            '> [data-testid="routine-section"] > [data-testid="section-header"]'
        )

    def _top_level_section_handle(self, section_idx: int) -> Locator:
        return self._top_level_section_headers().nth(section_idx).get_by_test_id("section-handle")

    def open_replace_exercise_dialog(self, section_idx: int, activity_idx: int) -> None:
        self._open_replace_dialog(section_idx, activity_idx)

    def replace_exercise(self, section_idx: int, activity_idx: int, name: str) -> None:
        self._open_replace_dialog(section_idx, activity_idx)
        self.replace_exercise_dialog.clear_filter()
        self._select_exercise(name)

    def replace_with_new_exercise(self, section_idx: int, activity_idx: int, name: str) -> None:
        dialog = self._open_replace_dialog(section_idx, activity_idx)
        self.replace_exercise_dialog.clear_filter()
        dialog.get_by_test_id("search").fill(name)
        dialog.get_by_test_id("create-exercise").click()
        dialog.get_by_test_id("dialog-save").click()
        self.wait_until_idle()
        exercises = dialog.get_by_test_id("exercise-item").all()
        for exercise in exercises:
            if name in exercise.inner_text().strip():
                exercise.click()
                break
        self.wait_until_idle()

    def set_reps(self, section_idx: int, activity_idx: int, reps: str) -> None:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        dialog.get_by_test_id("input-reps").fill(reps)
        self.dialog.save()

    def set_time(self, section_idx: int, activity_idx: int, time: str) -> None:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        dialog.get_by_test_id("input-time").fill(time)
        self.dialog.save()

    def set_weight(self, section_idx: int, activity_idx: int, weight: str) -> None:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        dialog.get_by_test_id("input-weight").fill(weight)
        self.dialog.save()

    def set_rpe(self, section_idx: int, activity_idx: int, rpe: str) -> None:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        dialog.get_by_test_id("input-rpe").fill(rpe)
        self.dialog.save()

    def set_automatic(self, section_idx: int, activity_idx: int) -> None:
        dialog = self._open_edit_dialog(section_idx, activity_idx)
        dialog.get_by_test_id("button-select-automatic").get_by_text(
            "Automatic", exact=True
        ).click()
        self.dialog.save()

    def _open_options_menu(self, section_idx: int, activity_idx: int | None = None) -> None:
        if activity_idx is None:
            self.page.get_by_test_id("section-options").nth(section_idx).click()
        else:
            section = self.page.get_by_test_id("routine-section").nth(section_idx)
            section.get_by_test_id("activity-options").nth(activity_idx).click()

    def _open_replace_dialog(self, section_idx: int, activity_idx: int) -> Locator:
        section = self.page.get_by_test_id("routine-section").nth(section_idx)
        section.get_by_test_id("activity-options").nth(activity_idx).click()
        self.page.get_by_test_id("options-replace-exercise").click()
        self.dialog.wait_until_open()
        return self.page.get_by_test_id("dialog")

    def _open_edit_dialog(self, section_idx: int, activity_idx: int | None = None) -> Locator:
        self._open_options_menu(section_idx, activity_idx)
        self.page.get_by_test_id("options-edit").click()
        self.dialog.wait_until_open()
        return self.page.get_by_test_id("dialog")

    def _select_exercise(self, name: str) -> None:
        exercises = self.page.get_by_test_id("dialog").get_by_test_id("exercise-item").all()
        for exercise in exercises:
            if name in exercise.inner_text().strip():
                exercise.click()
                break
        self.wait_until_idle()

    def get_notes(self) -> str:
        return get_text(self.page.get_by_test_id("routine-notes"))

    def expect_no_notes(self) -> None:
        expect(self.page.get_by_test_id("routine-notes")).to_have_count(0)

    def click_notes(self) -> None:
        self.page.get_by_test_id("routine-notes").click()
        self.notes_dialog.wait_until_open()

    def open_notes_dialog(self) -> None:
        self.page.get_by_test_id("fab").click()
        self.page.get_by_test_id("options-edit-notes").click()
        self.notes_dialog.wait_until_open()

    def set_notes(self, notes: str) -> None:
        self.notes_dialog.set_notes(notes)
        self.notes_dialog.save()
        self.wait_until_idle()

    def show_as_text(self) -> str:
        self.page.get_by_test_id("fab").click()
        self.page.get_by_test_id("options-show-as-text").click()
        self.dialog.wait_until_open()
        return self.page.get_by_test_id("show-text-content").inner_text().strip()


class RoutineNotesDialog(Dialog):
    def get_notes(self) -> str:
        return self.root.get_by_test_id("routine-notes-input").input_value()

    def set_notes(self, notes: str) -> None:
        self.root.get_by_test_id("routine-notes-input").fill(notes)


@dataclass
class RoutinePart:
    pass


@dataclass
class RoutineSet(RoutinePart):
    exercise_name: str
    reps: int | None = None
    time: float | None = None
    weight: float | None = None
    rpe: float | None = None

    @classmethod
    def from_element(cls, element: Locator) -> RoutineSet:
        exercise_elem = element.locator('[data-testid="set-exercise"]').first
        exercise_name = exercise_elem.inner_text().strip() if exercise_elem.is_visible() else ""

        reps_elem = element.locator('[data-testid="set-reps"]').first
        reps_text = reps_elem.inner_text().strip() if reps_elem.is_visible() else ""
        reps = int(reps_text) if reps_text else None

        time_elem = element.locator('[data-testid="set-time"]').first
        time_text = time_elem.inner_text().strip() if time_elem.is_visible() else ""
        time_val = parse_float(time_text)

        weight_elem = element.locator('[data-testid="set-weight"]').first
        weight_text = weight_elem.inner_text().strip() if weight_elem.is_visible() else ""
        weight = parse_float(weight_text)

        rpe_elem = element.locator('[data-testid="set-rpe"]').first
        rpe_text = rpe_elem.inner_text().strip() if rpe_elem.is_visible() else ""
        rpe = parse_float(rpe_text)

        return cls(
            exercise_name=exercise_name,
            reps=reps,
            time=time_val,
            weight=weight,
            rpe=rpe,
        )


@dataclass
class RoutineRest(RoutinePart):
    time: int | None

    @classmethod
    def from_element(cls, element: Locator) -> RoutineRest:
        rest_time_elem = element.locator('[data-testid="rest-time"]').first
        rest_time = (
            parse_int(rest_time_elem.inner_text().strip()) if rest_time_elem.is_visible() else None
        )
        return cls(time=rest_time)


@dataclass
class RoutineSection(RoutinePart):
    rounds: int
    parts: list[RoutinePart]

    @classmethod
    def from_element(cls, element: Locator) -> RoutineSection:
        rounds = cls._parse_rounds(element)
        parts = cls._parse_parts(element)

        return cls(rounds=rounds, parts=parts)

    def get_section_at(self, index: int) -> RoutineSection:
        assert index < len(self.parts), f"index {index} out of range: {self.parts}"
        part = self.parts[index]
        assert isinstance(part, RoutineSection), f"no section at {index}: {self.parts}"
        return part

    def get_set_at(self, index: int) -> RoutineSet:
        assert index < len(self.parts), f"index {index} out of range: {self.parts}"
        part = self.parts[index]
        assert isinstance(part, RoutineSet), f"no set at {index}: {self.parts}"
        return part

    def get_rest_at(self, index: int) -> RoutineRest:
        assert index < len(self.parts), f"index {index} out of range: {self.parts}"
        part = self.parts[index]
        assert isinstance(part, RoutineRest), f"no rest at {index}: {self.parts}"
        return part

    def is_rest_at(self, index: int) -> bool:
        assert index < len(self.parts), f"index {index} out of range: {self.parts}"
        return isinstance(self.parts[index], RoutineRest)

    def has_part_at(self, index: int) -> bool:
        return index < len(self.parts)

    @staticmethod
    def _parse_rounds(element: Locator) -> int:
        rounds_elem = element.locator('[data-testid="section-rounds"]').first
        rounds = 1
        if rounds_elem.count() > 0:
            content = rounds_elem.inner_text().strip()
            parts = content.split()
            if parts:
                try:
                    rounds = int(parts[-1])
                except ValueError:
                    rounds = 1
        return rounds

    @staticmethod
    def _parse_parts(element: Locator) -> list[RoutinePart]:
        parts: list[RoutinePart] = []

        containers = element.locator('> [data-testid="routine-part"]').all()

        for container in containers:
            nested_element = container.locator('[data-testid="routine-section"]').first
            if nested_element.count() > 0:
                parts.append(RoutineSection.from_element(nested_element))
            else:
                message_body = container.locator('> [data-testid="routine-part-body"]').first

                if message_body.count() == 0:
                    continue

                rest_label = message_body.locator('[data-testid="rest-label"]').first
                if rest_label.count() > 0:
                    parts.append(RoutineRest.from_element(message_body))
                else:
                    parts.append(RoutineSet.from_element(message_body))

        return parts
