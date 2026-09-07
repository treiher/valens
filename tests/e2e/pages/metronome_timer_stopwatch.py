from __future__ import annotations

from typing import TYPE_CHECKING

from playwright.sync_api import expect

from .base import BaseDialog, phase_bar_fills

if TYPE_CHECKING:
    from playwright.sync_api import Locator


class MetronomeTimerStopwatchDialog(BaseDialog):
    def open(self) -> None:
        self.navbar.open_metronome_timer_stopwatch()
        self.dialog.wait_until_open()

    def expect_open(self) -> None:
        expect(self.dialog.root.get_by_text("Metronome", exact=True)).to_be_visible()

    # Metronome

    def start_pause_metronome(self) -> None:
        self.dialog.root.get_by_test_id("metronome-play").click()

    def expect_metronome_active(self, *, active: bool) -> None:
        icon = "pause" if active else "play"
        expect(
            self.dialog.root.get_by_test_id("metronome-play").get_by_test_id(f"icon-{icon}")
        ).to_be_visible()

    def expect_metronome_playable(self, *, playable: bool) -> None:
        play = self.dialog.root.get_by_test_id("metronome-play")
        if playable:
            expect(play).to_be_enabled()
        else:
            expect(play).to_be_disabled()

    def set_metronome_tempo(self, *phases: str) -> None:
        for index, phase in enumerate(phases):
            self.dialog.root.get_by_test_id(f"metronome-tempo-{index}").fill(phase)

    def expect_metronome_tempo(self, *phases: str) -> None:
        for index, phase in enumerate(phases):
            expect(self.dialog.root.get_by_test_id(f"metronome-tempo-{index}")).to_have_value(
                phase
            )

    def get_metronome_bar_fills(self) -> list[float]:
        return phase_bar_fills(self.dialog.root)

    def expect_metronome_bar(self, segments: int) -> None:
        expect(self.dialog.root.get_by_test_id("phase-bar")).to_be_visible()
        expect(self.dialog.root.get_by_test_id("phase-bar").locator("> div")).to_have_count(
            segments
        )

    # Stopwatch

    def start_pause_stopwatch(self) -> None:
        self.dialog.root.get_by_test_id("stopwatch-play").click()

    def reset_stopwatch(self) -> None:
        self.dialog.root.get_by_test_id("stopwatch-reset").click()

    def click_stopwatch_time(self) -> None:
        self.stopwatch_time.click()

    def stopwatch_seconds(self) -> float:
        text = self.stopwatch_time.text_content()
        assert text is not None
        return float(text)

    def expect_stopwatch_seconds(self, seconds: float) -> None:
        expect(self.stopwatch_time).to_have_text(f"{seconds:.1f}")

    @property
    def stopwatch_time(self) -> Locator:
        return self.dialog.root.get_by_test_id("stopwatch-time")

    # Timer

    def set_timer(self, seconds: int) -> None:
        self.timer_time.fill(str(seconds))

    def start_pause_timer(self) -> None:
        self.dialog.root.get_by_test_id("timer-play").click()

    def reset_timer(self) -> None:
        self.dialog.root.get_by_test_id("timer-reset").click()

    def timer_seconds(self) -> int:
        return int(self.timer_time.input_value())

    def expect_timer_seconds(self, seconds: int) -> None:
        expect(self.timer_time).to_have_value(str(seconds))

    @property
    def timer_time(self) -> Locator:
        return self.dialog.root.get_by_test_id("timer-time")
