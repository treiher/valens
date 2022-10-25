#!/usr/bin/env python

import os
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen, run
from tempfile import TemporaryDirectory
from time import sleep

from selenium import webdriver

from tests.e2e.cli_test import wait_for_output
from tests.e2e.const import PORT
from tests.e2e.page import (
    BodyFatPage,
    HomePage,
    LoginPage,
    PeriodPage,
    RoutinePage,
    WorkoutPage,
    WorkoutsPage,
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

    options = webdriver.ChromeOptions()
    options.add_argument("--headless")
    options.add_argument("--hide-scrollbars")
    driver = webdriver.Chrome(options=options)
    driver.set_window_size(425, 800)

    screenshots = []

    def save_screenshot(name: str) -> None:
        filename = TARGET_DIR / f"{name}.png"
        driver.save_screenshot(str(filename))
        screenshots.append(filename)

    login_page = LoginPage(driver)
    login_page.load()
    login_page.login(username)

    home_page = HomePage(driver, username)
    home_page.load()
    sleep(0.5)

    save_screenshot("home")

    workouts_page = WorkoutsPage(driver)
    workouts_page.load()

    save_screenshot("workouts")

    workout_page = WorkoutPage(driver, 104)
    workout_page.load()

    save_screenshot("workout")

    workout_page.click_hamburger_button()
    workout_page.click_hamburger_menu_item("stopwatch")

    save_screenshot("stopwatch")

    routine_page = RoutinePage(driver, 4)
    routine_page.load()

    save_screenshot("routine")

    body_fat_page = BodyFatPage(driver)
    body_fat_page.load()
    body_fat_page.click_plot_6m()

    save_screenshot("body_fat")

    period_page = PeriodPage(driver)
    period_page.load()
    period_page.click_plot_3m()

    save_screenshot("period")

    run(
        f"convert {' '.join(str(s) for s in screenshots)}"
        " -background none -splice 10x0+0+0 +append -chop 10x0+0+0 doc/screenshots.png",
        check=True,
        shell=True,
    )

    for s in screenshots:
        s.unlink()


if __name__ == "__main__":
    main()
