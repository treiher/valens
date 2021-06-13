from pathlib import Path
from typing import Any

import _pytest.capture

import tests.data
import tests.utils
from valens import config, storage


def test_initialization(
    monkeypatch: Any, tmp_path: Path, capsys: _pytest.capture.CaptureFixture
) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)

    files = [
        storage.USERS_FILE,
        storage.ROUTINES_FILE,
        storage.ROUTINE_SETS_FILE,
        storage.WORKOUTS_FILE,
        storage.SETS_FILE,
        storage.BODYWEIGHT_FILE,
        storage.BODYFAT_FILE,
        storage.PERIOD_FILE,
    ]

    assert all(not (tmp_path / f).exists() for f in files)

    storage.initialize()

    assert all((tmp_path / f).is_file() for f in files)
    assert not capsys.readouterr().out

    storage.initialize()

    assert capsys.readouterr().out.count("warning") == len(files)


def test_users(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    users = storage.read_users()
    assert users.equals(tests.data.USERS_DF)

    storage.write_users(users)
    assert storage.read_users().equals(users)


def test_routines(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    routines = storage.read_routines(1)
    assert routines.equals(tests.data.ROUTINES_DF)

    storage.write_routines(routines, 1)
    assert storage.read_routines(1).set_index("routine").equals(routines.set_index("routine"))


def test_routine_sets(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    routine_sets = storage.read_routine_sets(1)
    assert routine_sets.equals(tests.data.ROUTINE_SETS_DF)

    storage.write_routine_sets(routine_sets, 1)
    assert (
        storage.read_routine_sets(1).set_index("routine").equals(routine_sets.set_index("routine"))
    )


def test_sets(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    sets = storage.read_sets(1)
    assert sets.equals(tests.data.SETS_DF)

    storage.write_sets(sets, 1)
    assert storage.read_sets(1).set_index("date").equals(sets.set_index("date"))


def test_bodyweight(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    bodyweight = storage.read_bodyweight(1)
    assert bodyweight.equals(tests.data.BODYWEIGHT_DF)

    storage.write_bodyweight(bodyweight, 1)
    assert storage.read_bodyweight(1).set_index("date").equals(bodyweight.set_index("date"))


def test_bodyfat(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    bodyfat = storage.read_bodyfat(1)
    assert bodyfat.equals(tests.data.BODYFAT_DF)

    storage.write_bodyfat(bodyfat, 1)
    assert storage.read_bodyfat(1).set_index("date").equals(bodyfat.set_index("date"))


def test_period(monkeypatch: Any, tmp_path: Path) -> None:
    monkeypatch.setattr(config, "DATA_DIRECTORY", tmp_path)
    tests.utils.initialize_data()

    period = storage.read_period(1)
    assert period.equals(tests.data.PERIOD_DF)

    storage.write_period(period, 1)
    assert storage.read_period(1).set_index("date").equals(period.set_index("date"))
