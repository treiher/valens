#!/usr/bin/env python

"""Create the screenshots used in the documentation."""

import argparse
import datetime
import os
import sys
from pathlib import Path
from shutil import which
from subprocess import PIPE, STDOUT, Popen, run
from tempfile import TemporaryDirectory

from playwright.sync_api import sync_playwright

from tests.e2e.const import PORT
from tests.e2e.io import wait_for_output
from tests.e2e.pages import (
    BodyFatPage,
    HomePage,
    LoginPage,
    MenstrualCyclePage,
    RoutinePage,
    TrainingSessionPage,
    TrainingSessionsPage,
)
from valens import config, demo

SCREENSHOTS = Path("doc/screenshots.png")
SEED = 0
TIMEZONE = "UTC"
# Rendering differences between environments are tolerated up to this fraction of all pixels.
MAX_DIFFERENCE = 0.0001


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="check that the created screenshots are up to date instead of updating them",
    )
    parser.add_argument(
        "--valens",
        metavar="PATH",
        type=Path,
        default=Path("build/venv/bin/valens"),
        help="path to the valens executable used to serve the app",
    )
    args = parser.parse_args()

    if which("magick") is None:
        print(  # noqa: T201
            "Error: magick could not be found, install ImageMagick",
            file=sys.stderr,
        )
        return 1

    if args.check:
        if not SCREENSHOTS.exists():
            print(  # noqa: T201
                f"Error: {SCREENSHOTS} does not exist, run `make screenshots`",
                file=sys.stderr,
            )
            return 1

        today = read_reference_date(SCREENSHOTS)
        if today is None:
            print(  # noqa: T201
                f"Error: {SCREENSHOTS} contains no reference date, run `make screenshots`",
                file=sys.stderr,
            )
            return 1

        with TemporaryDirectory() as d:
            screenshots = Path(d) / SCREENSHOTS.name
            create_screenshots(args.valens, screenshots, today)
            return compare(SCREENSHOTS, screenshots)

    create_screenshots(args.valens, SCREENSHOTS, datetime.date.today())

    return 0


def create_screenshots(valens: Path, screenshots: Path, today: datetime.date) -> None:
    with TemporaryDirectory() as d:
        path = Path(d)
        config_file = config.create_config_file(path, path / "test.db")
        with Popen(
            [
                str(valens),
                "demo",
                "--port",
                str(PORT),
                "--date",
                today.isoformat(),
                "--seed",
                str(SEED),
            ],
            stdout=PIPE,
            stderr=STDOUT,
            env={**os.environ, "VALENS_CONFIG": str(config_file)},
        ) as p:
            assert p.stdout
            wait_for_output(p.stdout, "Running on")
            try:
                take_screenshots(screenshots, path, today)
            finally:
                p.terminate()


def take_screenshots(screenshots: Path, target_dir: Path, today: datetime.date) -> None:
    username = demo.users(today, SEED)[0].name
    base_url = f"http://127.0.0.1:{PORT}"

    pages = []

    with sync_playwright() as pw:
        browser = pw.chromium.launch(channel="chromium", headless=True)
        context = browser.new_context(
            viewport={"width": 425, "height": 800},
            timezone_id=TIMEZONE,
        )
        page = context.new_page()
        page.set_default_timeout(5000)
        page.set_default_navigation_timeout(5000)
        # The app renders relative to the current date, which must match the date of the demo data.
        page.clock.set_fixed_time(
            datetime.datetime.combine(today, datetime.time(12), tzinfo=datetime.timezone.utc),
        )

        def save_screenshot(name: str) -> None:
            filename = target_dir / f"{name}.png"
            page.screenshot(path=str(filename))
            pages.append(filename)

        login_page = LoginPage(page, base_url)
        login_page.goto()
        login_page.login(username)

        # Prevent the mouse from hovering over an element
        page.mouse.move(0, 100)

        home_page = HomePage(page)
        home_page.expect_page()
        home_page.navbar.expect_synchronization_to_be_finished()
        home_page.expect_loading_to_be_finished()
        home_page.expect_ffmi_available()

        save_screenshot("home")

        training_page = TrainingSessionsPage(page, base_url)
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
        page.wait_for_timeout(400)

        save_screenshot("body_fat")

        menstrual_cycle_page = MenstrualCyclePage(page, base_url)
        menstrual_cycle_page.goto()
        page.get_by_text("3M").first.click()

        save_screenshot("period")

        browser.close()

    run(
        [
            "magick",
            *[str(p) for p in pages],
            "-background",
            "none",
            "-splice",
            "10x0+0+0",
            "+append",
            "-chop",
            "10x0+0+0",
            "-set",
            "comment",
            today.isoformat(),
            str(screenshots),
        ],
        check=True,
    )


def read_reference_date(screenshots: Path) -> datetime.date | None:
    """Return the date the screenshots were created with, which is stored as image comment."""
    # `identify` only warns if the image has no comment, leaving the output empty. It fails and
    # writes no output if the image cannot be read.
    comment = run(
        ["magick", "identify", "-format", "%[comment]", str(screenshots)],
        capture_output=True,
        check=False,
        text=True,
    ).stdout.strip()

    try:
        return datetime.date.fromisoformat(comment)
    except ValueError:
        return None


def compare(reference: Path, current: Path) -> int:
    """Return 1 if the images differ in size or in more than `MAX_DIFFERENCE` of all pixels."""
    reference_size = size(reference)
    current_size = size(current)

    # `compare` only considers the common area of images of different size.
    if reference_size != current_size:
        print(  # noqa: T201
            f"Error: {reference} is not up to date"
            f" (size {dimensions(reference_size)} instead of {dimensions(current_size)}),"
            " run `make screenshots`",
            file=sys.stderr,
        )
        return 1

    # The absolute error is followed by the normalized error in parentheses.
    result = run(
        [
            "magick",
            "compare",
            "-metric",
            "AE",
            "-fuzz",
            "2%",
            str(reference),
            str(current),
            "null:",
        ],
        capture_output=True,
        check=False,
        text=True,
    )
    # Warnings, e.g. about color profiles, precede the metric on separate lines.
    *warnings, metric = result.stderr.strip().splitlines() or [""]
    difference = next(iter(metric.split()), "")

    for warning in warnings:
        print(f"Warning: {warning}", file=sys.stderr)  # noqa: T201

    try:
        # The absolute error is formatted using `%g`, which switches to scientific notation for
        # large numbers.
        differing_pixels = int(float(difference))
    except ValueError:
        print(  # noqa: T201
            f"Error: {reference} could not be compared: {result.stderr.strip() or 'no output'}",
            file=sys.stderr,
        )
        return 1

    width, height = reference_size

    if differing_pixels > width * height * MAX_DIFFERENCE:
        print(  # noqa: T201
            f"Error: {reference} is not up to date"
            f" ({differing_pixels} of {width * height} pixels differ), run `make screenshots`",
            file=sys.stderr,
        )
        return 1

    return 0


def dimensions(image_size: tuple[int, int]) -> str:
    width, height = image_size

    return f"{width}x{height}"


def size(image: Path) -> tuple[int, int]:
    width, height = run(
        ["magick", "identify", "-format", "%w %h", str(image)],
        capture_output=True,
        check=True,
        text=True,
    ).stdout.split()

    return (int(width), int(height))


if __name__ == "__main__":
    sys.exit(main())
