import datetime
import pathlib
from typing import Mapping

import yaml

INITIAL_WORKOUTS_DATA = {
    datetime.date(2002, 2, 20): {"Push Up": [12, 11, 10], "Row": [11, 10, 9]},
    datetime.date(2002, 2, 22): {"Dip": [8, 7, 6], "Pull Up": [5, 4, 3]},
}
INITIAL_BODYWEIGHT_DATA = {datetime.date(2002, 2, 20): 81.2, datetime.date(2002, 2, 22): 82.4}


def initialize_data(tmp_dir: str) -> None:
    tmp_path = pathlib.Path(tmp_dir)
    with open(tmp_path / "workouts.yml", "x") as f:
        f.write(yaml.dump(INITIAL_WORKOUTS_DATA))
    with open(tmp_path / "bodyweight.yml", "x") as f:
        f.write(yaml.dump(INITIAL_BODYWEIGHT_DATA))


def config(tmp_dir: str) -> Mapping[str, pathlib.Path]:
    tmp_path = pathlib.Path(tmp_dir)
    return {
        "workout_file": tmp_path / "workouts.yml",
        "bodyweight_file": tmp_path / "bodyweight.yml",
    }
