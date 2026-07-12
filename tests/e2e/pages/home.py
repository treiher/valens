from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BasePage

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class HomePage(BasePage):
    @property
    def path(self) -> str:
        return "/home"

    @property
    def ffmi(self) -> Locator:
        return self.page.get_by_test_id("home-ffmi")

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("home-training-sessions")).to_be_visible()

    def expect_ffmi_requires_height(self) -> None:
        expect(self.ffmi).to_be_visible()
        expect(self.ffmi).to_contain_text("Set your height in the profile.")

    def expect_ffmi_available(self) -> None:
        expect(self.ffmi).to_be_visible()
        expect(self.ffmi).not_to_contain_text("Set your height in the profile.")

    def go_to_training_sessions(self) -> None:
        self.page.get_by_test_id("home-training-sessions").click()

    def go_to_routines(self) -> None:
        self.page.get_by_test_id("home-routines").click()

    def go_to_exercises(self) -> None:
        self.page.get_by_test_id("home-exercises").click()

    def go_to_schedule(self) -> None:
        self.page.get_by_test_id("home-schedule").click()

    def go_to_muscles(self) -> None:
        self.page.get_by_test_id("home-muscles").click()

    def expect_today_entries(self, names: list[str]) -> None:
        if names:
            expect(self.page.get_by_test_id("home-today-routine")).to_have_text(names)
        else:
            expect(self.page.get_by_test_id("home-today-entry")).to_have_count(0)

    def expect_today_rotations(self, names: list[str]) -> None:
        expect(self.page.get_by_test_id("home-today-rotation")).to_have_text(names)

    def start_today_entry(self, index: int) -> None:
        self.page.get_by_test_id("home-today-start").nth(index).click()

    def go_to_body_weight(self) -> None:
        self.page.get_by_test_id("home-body-weight").click()

    def go_to_body_fat(self) -> None:
        self.page.get_by_test_id("home-body-fat").click()

    def go_to_ffmi(self) -> None:
        self.ffmi.click()

    def go_to_menstrual_cycle(self) -> None:
        self.page.get_by_test_id("home-menstrual-cycle").click()
