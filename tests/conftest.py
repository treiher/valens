from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory

import _pytest
import pytest
import selenium
import selenium.webdriver

from valens import app, database as db


@pytest.fixture()
def alembic_config() -> dict[str, str]:
    return {"script_location": "valens:migrations"}


@pytest.fixture()
def alembic_engine() -> object:
    with TemporaryDirectory() as tmp_dir:
        tmp_file = Path(tmp_dir) / "test.db"
        tmp_file.touch()
        app.config["DATABASE"] = f"sqlite:///{tmp_file}"
        with app.app_context():
            db.init()
            yield db.get_engine()


def pytest_addoption(parser: _pytest.config.argparsing.Parser) -> None:
    group = parser.getgroup("selenium", "selenium")
    group._addoption(  # noqa: SLF001
        "--headless", action="store_true", help="enable headless mode for supported browsers"
    )


@pytest.fixture()
def chrome_options(
    chrome_options: selenium.webdriver.chrome.options.Options,
    pytestconfig: _pytest.config.Config,
) -> selenium.webdriver.chrome.options.Options:
    if pytestconfig.getoption("headless"):
        chrome_options.add_argument("--headless")
    return chrome_options


@pytest.fixture()
def firefox_options(
    firefox_options: selenium.webdriver.firefox.options.Options,
    pytestconfig: _pytest.config.Config,
) -> selenium.webdriver.firefox.options.Options:
    if pytestconfig.getoption("headless"):
        firefox_options.add_argument("-headless")
    return firefox_options
