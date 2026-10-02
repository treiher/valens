from __future__ import annotations

from playwright.sync_api import expect

from .base import BasePage


class MusclesPage(BasePage):
    @property
    def path(self) -> str:
        return "/muscles"

    def expect_page(self) -> None:
        expect(self.page.get_by_test_id("page-title")).to_have_text("Muscles")

    def select_kind(self, kind: str) -> None:
        """Select the charts of the "worked" or the "stretched" muscles."""
        self.page.get_by_test_id(f"muscle-kind-{kind}").click()
        expect(self.page.get_by_test_id(f"muscle-kind-{kind}")).to_have_attribute(
            "data-selected", "true"
        )

    def expect_chart_label(self, label: str) -> None:
        expect(self.page.get_by_text(label, exact=True).first).to_be_visible()
