import datetime
import tempfile
from typing import Any

import pytest  # type: ignore

from tests import utils
from valens import storage


def test_workouts(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: utils.config(tmp_dir))

        workouts = storage.read_workouts()
        assert len(workouts) == len(
            [
                s
                for exercises in utils.INITIAL_WORKOUTS_DATA.values()
                for sets in exercises.values()
                for s in sets
            ]
        )
        assert workouts.loc[0].date == list(utils.INITIAL_WORKOUTS_DATA)[0]
        assert workouts.loc[0].exercise == list(list(utils.INITIAL_WORKOUTS_DATA.values())[0])[0]
        assert (
            workouts.loc[0].reps
            == list(list(utils.INITIAL_WORKOUTS_DATA.values())[0].values())[0][0]
        )


def test_bodyweight(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: utils.config(tmp_dir))

        bodyweight = storage.read_bodyweight()
        assert bodyweight == utils.INITIAL_BODYWEIGHT_DATA

        storage.write_bodyweight(datetime.date(2002, 2, 24), 82.0)
        bodyweight = storage.read_bodyweight()
        assert bodyweight == {**utils.INITIAL_BODYWEIGHT_DATA, datetime.date(2002, 2, 24): 82.0}


def test_parse_set() -> None:
    assert storage.parse_set("10") == {"reps": "10", "time": None, "weight": None, "rpe": None}
    assert storage.parse_set("60s") == {"reps": None, "time": "60", "weight": None, "rpe": None}
    assert storage.parse_set("20.0kg") == {
        "reps": None,
        "time": None,
        "weight": "20.0",
        "rpe": None,
    }
    assert storage.parse_set("@8") == {"reps": None, "time": None, "weight": None, "rpe": "8"}
    assert storage.parse_set("10x20.0kg") == {
        "reps": "10",
        "time": None,
        "weight": "20.0",
        "rpe": None,
    }
    assert storage.parse_set("10@8") == {"reps": "10", "time": None, "weight": None, "rpe": "8"}
    assert storage.parse_set("10x20.0kg@8") == {
        "reps": "10",
        "time": None,
        "weight": "20.0",
        "rpe": "8",
    }
    assert storage.parse_set("10x60sx20.0kg@8") == {
        "reps": "10",
        "time": "60",
        "weight": "20.0",
        "rpe": "8",
    }
    with pytest.raises(Exception, match=r"unexpected format for set 'invalid'"):
        storage.parse_set("invalid")
