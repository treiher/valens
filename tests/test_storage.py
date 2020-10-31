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

    assert not (tmp_path / storage.ROUTINES_FILE).exists()
    assert not (tmp_path / storage.ROUTINE_SETS_FILE).exists()
    assert not (tmp_path / storage.WORKOUTS_FILE).exists()
    assert not (tmp_path / storage.SETS_FILE).exists()
    assert not (tmp_path / storage.BODYWEIGHT_FILE).exists()

    storage.initialize()

    assert (tmp_path / storage.ROUTINES_FILE).is_file()
    assert (tmp_path / storage.ROUTINE_SETS_FILE).is_file()
    assert (tmp_path / storage.WORKOUTS_FILE).is_file()
    assert (tmp_path / storage.SETS_FILE).is_file()
    assert (tmp_path / storage.BODYWEIGHT_FILE).is_file()
    assert not capsys.readouterr().out

    storage.initialize()

    assert capsys.readouterr().out.count("warning") == 5


def test_routines(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        routines = storage.read_routine_sets()
        assert routines.equals(tests.data.ROUTINE_SETS_DF)

        storage.write_routine_sets(routines)
        assert storage.read_routine_sets().equals(routines)


def test_sets(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        sets = storage.read_sets()
        assert sets.equals(tests.data.SETS_DF)

        storage.write_sets(sets)
        assert storage.read_sets().equals(sets)


def test_bodyweight(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        bodyweight = storage.read_bodyweight()
        assert bodyweight.equals(tests.data.BODYWEIGHT_DF)

        storage.write_bodyweight(bodyweight)
        assert storage.read_bodyweight().equals(bodyweight)
