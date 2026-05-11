import os
import re
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen, run

from playwright.sync_api import Page, expect

from valens.config import create_config_file

from .const import BASE_URL, PORT, VALENS
from .io import wait_for_output


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
    assert p.stdout.decode("utf-8") == "Creating database\n"
    assert p.returncode == 0


def test_run(tmp_path: Path, page: Page) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    with Popen(
        f"{VALENS} run --port {PORT}".split(),
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    ) as p:
        assert p.stdout
        wait_for_output(p.stdout, "Running on")
        page.goto(BASE_URL)
        expect(page.get_by_text("Valens")).to_be_visible()
        p.terminate()


def test_demo(tmp_path: Path, page: Page) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    with Popen(
        f"{VALENS} demo --port {PORT}".split(),
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(config), **os.environ},
    ) as p:
        assert p.stdout
        wait_for_output(p.stdout, "Running on")
        page.goto(BASE_URL)
        expect(page.get_by_text("Valens")).to_be_visible()
        p.terminate()


def test_user_list(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    run(
        f"{VALENS} user create Bob male".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(f"{VALENS} user list".split(), check=False, stdout=PIPE, stderr=STDOUT, env=env)
    lines = p.stdout.decode("utf-8").splitlines()
    assert p.returncode == 0
    assert any("Alice" in line and "female" in line for line in lines)
    assert any("Bob" in line and "male" in line for line in lines)


def test_user_create(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    p = run(
        f"{VALENS} user create Alice female".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode == 0
    assert 'Created user "Alice"' in p.stdout.decode("utf-8")
    p = run(f"{VALENS} user list".split(), check=False, stdout=PIPE, stderr=STDOUT, env=env)
    assert "Alice" in p.stdout.decode("utf-8")


def test_user_create_duplicate(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(
        f"{VALENS} user create Alice female".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode != 0
    assert 'User "Alice" already exists' in p.stdout.decode("utf-8")


def test_user_update(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(
        f"{VALENS} user update Alice --name Bob --sex male".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode == 0
    assert 'Updated user "Alice"' in p.stdout.decode("utf-8")
    p = run(f"{VALENS} user list".split(), check=False, stdout=PIPE, stderr=STDOUT, env=env)
    lines = p.stdout.decode("utf-8").splitlines()
    assert any("Bob" in line and "male" in line for line in lines)
    assert not any("Alice" in line for line in lines)


def test_user_update_duplicate(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    run(
        f"{VALENS} user create Bob male".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(
        f"{VALENS} user update Alice --name Bob".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode != 0
    assert 'User "Bob" already exists' in p.stdout.decode("utf-8")


def test_user_update_same_name(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(
        f"{VALENS} user update Alice --name Alice --sex male".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode == 0
    assert 'Updated user "Alice"' in p.stdout.decode("utf-8")
    p = run(f"{VALENS} user list".split(), check=False, stdout=PIPE, stderr=STDOUT, env=env)
    lines = p.stdout.decode("utf-8").splitlines()
    assert any("Alice" in line and "male" in line for line in lines)


def test_user_update_not_found(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    p = run(
        f"{VALENS} user update NonExistent --name Someone".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode != 0
    assert 'User "NonExistent" not found' in p.stdout.decode("utf-8")


def test_user_delete(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    run(
        f"{VALENS} user create Alice female".split(),
        check=True,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    p = run(
        f"{VALENS} user delete Alice".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode == 0
    assert 'Deleted user "Alice"' in p.stdout.decode("utf-8")
    p = run(f"{VALENS} user list".split(), check=False, stdout=PIPE, stderr=STDOUT, env=env)
    assert "Alice" not in p.stdout.decode("utf-8")


def test_user_delete_not_found(tmp_path: Path) -> None:
    config = create_config_file(tmp_path, tmp_path / "test.db")
    env = {"VALENS_CONFIG": str(config), **os.environ}
    p = run(
        f"{VALENS} user delete NonExistent".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env=env,
    )
    assert p.returncode != 0
    assert 'User "NonExistent" not found' in p.stdout.decode("utf-8")
