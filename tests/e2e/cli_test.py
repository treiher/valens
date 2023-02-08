import os
import re
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen, run

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support import expected_conditions as EC

from valens.config import create_config_file

from .const import HOST, PORT, VALENS
from .io import wait_for_output
from .page import wait


def test_version() -> None:
    assert re.match(
        r"\d+\.\d+\..*",
        run(f"{VALENS} --version".split(), capture_output=True, check=True).stdout.decode("utf-8"),
    )


def test_config(tmp_path: Path) -> None:
    p = run(f"{VALENS} config -d {tmp_path}".split(), check=False, stdout=PIPE, stderr=STDOUT)
    assert p.stdout.decode("utf-8") == f"Created {tmp_path}/config.py\n"
    assert p.returncode == 0


def test_upgrade(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    p = run(
        f"{VALENS} upgrade".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    )
    assert p.stdout.decode("utf-8") == "Creating database\nNo upgrade necessary\n"
    assert p.returncode == 0


def test_run(tmp_path: Path, driver: webdriver.Chrome) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    with Popen(
        f"{VALENS} run --port {PORT}".split(),
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    ) as p:
        assert p.stdout
        wait_for_output(p.stdout, "Running on")
        driver.get(f"http://{HOST}:{PORT}/")
        wait(driver).until(
            EC.text_to_be_present_in_element(
                (By.XPATH, "//div[contains(@class, 'navbar-item')]"), "Valens"
            )
        )
        p.terminate()


def test_demo(tmp_path: Path, driver: webdriver.Chrome) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    with Popen(
        f"{VALENS} demo --port {PORT}".split(),
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    ) as p:
        assert p.stdout
        wait_for_output(p.stdout, "Running on")
        driver.get(f"http://{HOST}:{PORT}/")
        wait(driver).until(
            EC.text_to_be_present_in_element(
                (By.XPATH, "//div[contains(@class, 'navbar-item')]"), "Valens"
            )
        )
        p.terminate()
