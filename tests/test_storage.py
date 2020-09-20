import datetime
import tempfile
from typing import Any

import tests.data
import tests.utils
from valens import config, storage


def test_routines(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        routines = storage.read_routines()
        assert routines.equals(tests.data.ROUTINES_DF)

        storage.write_routines(routines)
        assert storage.read_routines().equals(routines)


def test_workouts(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        workouts = storage.read_workouts()
        assert workouts.equals(tests.data.WORKOUTS_DF)

        storage.write_workouts(workouts)
        assert storage.read_workouts().equals(workouts)


def test_bodyweight(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        bodyweight = storage.read_bodyweight()
        assert bodyweight == tests.data.BODYWEIGHT

        storage.write_bodyweight(datetime.date(2002, 2, 24), 82.0)
        bodyweight = storage.read_bodyweight()
        assert bodyweight == {**tests.data.BODYWEIGHT, datetime.date(2002, 2, 24): 82.0}
