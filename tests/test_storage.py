import pathlib
import tempfile
from typing import Any

import _pytest.capture

import tests.data
import tests.utils
from valens import storage


def test_initialization(
    monkeypatch: Any, tmp_path: pathlib.Path, capsys: _pytest.capture.CaptureFixture
) -> None:
    monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tmp_path)

    files = [
        storage.USERS_FILE,
        storage.ROUTINES_FILE,
        storage.ROUTINE_SETS_FILE,
        storage.WORKOUTS_FILE,
        storage.SETS_FILE,
        storage.BODYWEIGHT_FILE,
    ]

    assert all(not (tmp_path / f).exists() for f in files)

    storage.initialize()

    assert all((tmp_path / f).is_file() for f in files)
    assert not capsys.readouterr().out

    storage.initialize()

    assert capsys.readouterr().out.count("warning") == len(files)


def test_users(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        users = storage.read_users()
        assert users.equals(tests.data.USERS_DF)

        storage.write_users(users)
        assert storage.read_users().equals(users)


def test_routines(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        routines = storage.read_routine_sets(1)
        assert routines.equals(tests.data.ROUTINE_SETS_DF)

        storage.write_routine_sets(routines, 1)
        assert storage.read_routine_sets(1).equals(routines)


def test_sets(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        sets = storage.read_sets(1)
        assert sets.equals(tests.data.SETS_DF)

        storage.write_sets(sets, 1)
        assert storage.read_sets(1).equals(sets)


def test_bodyweight(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        bodyweight = storage.read_bodyweight(1)
        assert bodyweight.equals(tests.data.BODYWEIGHT_DF)

        storage.write_bodyweight(bodyweight, 1)
        assert storage.read_bodyweight(1).equals(bodyweight)
