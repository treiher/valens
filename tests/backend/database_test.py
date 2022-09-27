from pathlib import Path
from typing import Generator

import pytest

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
    db.init_db()
    assert test_db.exists()
    db.session.commit()
    db.remove_session()


def test_upgrade(test_db: Path) -> None:
    assert not test_db.exists()
    db.init_db()
    assert test_db.exists()
    db.upgrade_db()
