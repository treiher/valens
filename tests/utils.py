import pathlib

import tests.data
from valens import storage


def initialize_data(tmp_dir: str) -> pathlib.Path:
    tmp_path = pathlib.Path(tmp_dir)
    tests.data.ROUTINE_SETS_DF.to_feather(tmp_path / storage.ROUTINE_SETS_FILE)
    tests.data.ROUTINES_DF.to_feather(tmp_path / storage.ROUTINES_FILE)
    tests.data.SETS_DF.to_feather(tmp_path / storage.SETS_FILE)
    tests.data.WORKOUTS_DF.to_feather(tmp_path / storage.WORKOUTS_FILE)
    tests.data.BODYWEIGHT_DF.to_feather(tmp_path / storage.BODYWEIGHT_FILE)
    return tmp_path
