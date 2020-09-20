import pathlib

import yaml

import tests.data


def initialize_data(tmp_dir: str) -> pathlib.Path:
    tmp_path = pathlib.Path(tmp_dir)
    with open(tmp_path / "workout.yml", "w") as f:
        f.write(yaml.dump(tests.data.WORKOUTS, sort_keys=False))
    with open(tmp_path / "bodyweight.yml", "w") as f:
        f.write(yaml.dump(tests.data.BODYWEIGHT, sort_keys=False))
    with open(tmp_path / "routine.yml", "w") as f:
        f.write(yaml.dump(tests.data.ROUTINES, sort_keys=False))
    return tmp_path
