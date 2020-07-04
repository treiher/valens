import datetime
import tempfile
from typing import Any

import tests.data
import tests.utils
from valens import storage


def test_templates(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        templates = storage.read_templates()
        for (actual_index, actual_df), (expected_index, expected_df) in zip(
            templates.items(), tests.data.TEMPLATES_DF.items()
        ):
            assert actual_index == expected_index
            assert actual_df.equals(expected_df)


def test_workouts(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        workouts = storage.read_workouts()
        assert workouts.equals(tests.data.WORKOUTS_DF)

        storage.write_workouts(workouts)
        assert storage.read_workouts().equals(workouts)


def test_bodyweight(monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        bodyweight = storage.read_bodyweight()
        assert bodyweight == tests.data.BODYWEIGHT

        storage.write_bodyweight(datetime.date(2002, 2, 24), 82.0)
        bodyweight = storage.read_bodyweight()
        assert bodyweight == {**tests.data.BODYWEIGHT, datetime.date(2002, 2, 24): 82.0}
