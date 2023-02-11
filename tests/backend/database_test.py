from datetime import datetime, timedelta
from multiprocessing import Process
from pathlib import Path
from time import sleep
from typing import Generator

import pytest
from alembic import command

from valens import app, database as db


@pytest.fixture(name="test_db")
def fixture_test_db(tmp_path: Path) -> Generator[Path, None, None]:
    db_file = tmp_path / "db"
    app.config["DATABASE"] = f"sqlite:///{db_file}"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.app_context():
        yield db_file


def test_init_implicit(test_db: Path) -> None:
    assert not test_db.exists()
    db.session.commit()
    assert test_db.exists()
    db.session.commit()
    db.remove_session()


def test_init_explicit(test_db: Path) -> None:
    assert not test_db.exists()
    db.init()
    assert test_db.exists()
    db.session.commit()
    db.remove_session()


def test_upgrade(test_db: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert not test_db.exists()
    db.upgrade()
    assert test_db.exists()
    assert capsys.readouterr().out == "Creating database\n"
    command.downgrade(db.alembic_cfg, "4cacd61cb0c5")
    db.upgrade()
    assert capsys.readouterr().out.startswith("Upgrading database from 4cacd61cb0c5 to ")
    db.upgrade()
    assert capsys.readouterr().out == ""


def test_upgrade_in_progress(test_db: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert not test_db.exists()

    db.upgrade()

    assert test_db.exists()
    assert capsys.readouterr().out == "Creating database\n"

    command.downgrade(db.alembic_cfg, "4cacd61cb0c5")
    db.upgrade_lock_file().touch()
    wait = 3

    def wait_and_remove_lock() -> None:
        sleep(wait)
        db.upgrade_lock_file().unlink()

    expected_lock_release = datetime.now() + timedelta(seconds=wait)
    Process(target=wait_and_remove_lock).start()
    db.upgrade()

    assert capsys.readouterr().out == "Waiting for completion of database upgrade\n"
    assert datetime.now() >= expected_lock_release
