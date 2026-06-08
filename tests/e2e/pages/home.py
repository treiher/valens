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

    def go_to_training_sessions(self) -> None:
        self.page.get_by_test_id("home-training-sessions").click()

    def go_to_routines(self) -> None:
        self.page.get_by_test_id("home-routines").click()

    def go_to_exercises(self) -> None:
        self.page.get_by_test_id("home-exercises").click()

    def go_to_muscles(self) -> None:
        self.page.get_by_test_id("home-muscles").click()

    def go_to_body_weight(self) -> None:
        self.page.get_by_test_id("home-body-weight").click()

    def go_to_body_fat(self) -> None:
        self.page.get_by_test_id("home-body-fat").click()

    def go_to_ffmi(self) -> None:
        self.ffmi.click()

    def go_to_menstrual_cycle(self) -> None:
        self.page.get_by_test_id("home-menstrual-cycle").click()
