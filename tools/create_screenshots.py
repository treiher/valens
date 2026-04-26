#!/usr/bin/env python

import os
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen, run
from tempfile import TemporaryDirectory

from playwright.sync_api import sync_playwright

from tests.e2e.const import PORT
from tests.e2e.io import wait_for_output
from tests.e2e.pages import (
    BodyFatPage,
    LoginPage,
    MenstrualCyclePage,
    RoutinePage,
    TrainingPage,
    TrainingSessionPage,
)
from valens import config, demo

TARGET_DIR = Path("doc")


def main() -> None:
    with TemporaryDirectory() as d:
        path = Path(d)
        config_file = config.create_config_file(path, path / "test.db")
        with Popen(
            f"valens demo --port {PORT}".split(),
            stdout=PIPE,
            stderr=STDOUT,
            env={"VALENS_CONFIG": str(config_file), **os.environ},
        ) as p:
            assert p.stdout
            wait_for_output(p.stdout, "Running on")
            take_screenshots()
            p.terminate()


def take_screenshots() -> None:
    username = demo.users()[0].name
    base_url = f"http://127.0.0.1:{PORT}"

    screenshots = []

    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=True)
        context = browser.new_context(viewport={"width": 425, "height": 800})
        page = context.new_page()

        def save_screenshot(name: str) -> None:
            filename = TARGET_DIR / f"{name}.png"
            page.screenshot(path=str(filename))
            screenshots.append(filename)

        login_page = LoginPage(page, base_url)
        login_page.goto()
        login_page.login(username)

        # Prevent the mouse from hovering over an element
        page.mouse.move(0, 100)

        page.goto(base_url)
        page.wait_for_timeout(500)

        save_screenshot("home")

        training_page = TrainingPage(page, base_url)
        training_page.goto()

        save_screenshot("training")

        training_session_page = TrainingSessionPage(page, 104, base_url)
        training_session_page.goto()
        training_session_page.edit()

        save_screenshot("training_session")

        routine_page = RoutinePage(page, 4, base_url)
        routine_page.goto()

        save_screenshot("routine")

        body_fat_page = BodyFatPage(page, base_url)
        body_fat_page.goto()
        page.get_by_text("6M").first.click()

        save_screenshot("body_fat")

        menstrual_cycle_page = MenstrualCyclePage(page, base_url)
        menstrual_cycle_page.goto()
        page.get_by_text("3M").first.click()

        save_screenshot("period")

        browser.close()

    run(
        f"magick {' '.join(str(s) for s in screenshots)}"
        " -background none -splice 10x0+0+0 +append -chop 10x0+0+0 doc/screenshots.png",
        check=True,
        shell=True,
    )

    for s in screenshots:
        s.unlink()


if __name__ == "__main__":
    main()
