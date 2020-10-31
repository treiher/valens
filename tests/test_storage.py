import tempfile
from typing import Any

import tests.data
import tests.utils
from valens import storage


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
