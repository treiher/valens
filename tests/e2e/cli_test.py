import os
import re
from pathlib import Path
from queue import Empty, Queue
from subprocess import PIPE, STDOUT, Popen, run
from threading import Thread
from typing import IO

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support import expected_conditions as EC

from valens.config import create_config_file

from .const import HOST, PORT
from .page import wait


def test_version() -> None:
    assert re.match(
        r"\d+\.\d+\..*",
        run("valens --version".split(), capture_output=True, check=True).stdout.decode("utf-8"),
    )


def test_config(tmp_path: Path) -> None:
    p = run(f"valens config -d {tmp_path}".split(), check=False, stdout=PIPE, stderr=STDOUT)
    assert p.stdout.decode("utf-8") == f"Created {tmp_path}/config.py\n"
    assert p.returncode == 0


def test_upgrade(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    p = run(
        "valens upgrade".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    )
    assert p.stdout.decode("utf-8") == ""
    assert p.returncode == 0


def test_run(tmp_path: Path, driver: webdriver.Chrome) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    with Popen(
        f"valens run --port {PORT}".split(),
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
        f"valens demo --port {PORT}".split(),
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


def wait_for_output(out: IO[bytes], expected: str) -> None:
    def enqueue_output(out: IO[bytes], queue: Queue[bytes]) -> None:
        for line in iter(out.readline, b""):
            queue.put(line)
        out.close()

    q: Queue[bytes] = Queue()
    t = Thread(target=enqueue_output, args=(out, q))
    t.daemon = True
    t.start()

    for _ in range(100):
        try:
            line = q.get(timeout=0.1).decode("utf-8")
        except Empty:
            pass
        else:
            print(line)
            if expected in line:
                break
    else:
        raise Exception("expected output not found")
