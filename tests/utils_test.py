import numpy as np
import pytest

from valens import utils


def test_parse_set() -> None:
    assert utils.parse_set("10") == {"reps": 10.0, "time": np.nan, "weight": np.nan, "rpe": np.nan}
    assert utils.parse_set("60s") == {"reps": np.nan, "time": 60.0, "weight": np.nan, "rpe": np.nan}
    assert utils.parse_set("20.0kg") == {
        "reps": np.nan,
        "time": np.nan,
        "weight": 20.0,
        "rpe": np.nan,
    }
    assert utils.parse_set("@8") == {"reps": np.nan, "time": np.nan, "weight": np.nan, "rpe": 8.0}
    assert utils.parse_set("10x20.0kg") == {
        "reps": 10.0,
        "time": np.nan,
        "weight": 20.0,
        "rpe": np.nan,
    }
    assert utils.parse_set("10@8") == {"reps": 10.0, "time": np.nan, "weight": np.nan, "rpe": 8.0}
    assert utils.parse_set("10x20.0kg@8") == {
        "reps": 10.0,
        "time": np.nan,
        "weight": 20.0,
        "rpe": 8,
    }
    assert utils.parse_set("10x60sx20.0kg@8") == {
        "reps": 10.0,
        "time": 60.0,
        "weight": 20.0,
        "rpe": 8,
    }
    with pytest.raises(Exception, match=r"unexpected format for set 'invalid'"):
        utils.parse_set("invalid")


def test_format_set() -> None:
    assert utils.format_set((10, np.nan, np.nan, np.nan)) == "10"
    assert utils.format_set((np.nan, 60, np.nan, np.nan)) == "60s"
    assert utils.format_set((np.nan, np.nan, 20, np.nan)) == "20.0kg"
    assert utils.format_set((np.nan, np.nan, np.nan, 8)) == "@8"
    assert utils.format_set((10, np.nan, 20, np.nan)) == "10x20.0kg"
    assert utils.format_set((10, np.nan, np.nan, 8)) == "10@8"
    assert utils.format_set((10, np.nan, 20, 8)) == "10x20.0kg@8"
    assert utils.format_set((10, 60, 20, 8)) == "10x60sx20.0kg@8"


def test_format_number() -> None:
    assert utils.format_number(np.nan) == "-"
    assert utils.format_number(1.234) == "1.2"
