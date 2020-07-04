import pathlib
from typing import Mapping

import yaml

import tests.data


def initialize_data(tmp_dir: str) -> None:
    tmp_path = pathlib.Path(tmp_dir)
    with open(tmp_path / "workouts.yml", "x") as f:
        f.write(yaml.dump(tests.data.WORKOUTS, sort_keys=False))
    with open(tmp_path / "bodyweight.yml", "x") as f:
        f.write(yaml.dump(tests.data.BODYWEIGHT, sort_keys=False))
    with open(tmp_path / "template.yml", "x") as f:
        f.write(yaml.dump(tests.data.TEMPLATES, sort_keys=False))


def config(tmp_dir: str) -> Mapping[str, pathlib.Path]:
    tmp_path = pathlib.Path(tmp_dir)
    return {
        "workout_file": tmp_path / "workouts.yml",
        "bodyweight_file": tmp_path / "bodyweight.yml",
        "template_file": tmp_path / "template.yml",
    }
