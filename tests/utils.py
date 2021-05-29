import pathlib

import pandas as pd

import tests.data
from valens import storage


def initialize_data(tmp_dir: str) -> pathlib.Path:
    tmp_path = pathlib.Path(tmp_dir)
    tests.data.USERS_DF.to_feather(tmp_path / storage.USERS_FILE)
    add_user_id(tests.data.ROUTINE_SETS_DF).to_feather(tmp_path / storage.ROUTINE_SETS_FILE)
    add_user_id(tests.data.ROUTINES_DF).to_feather(tmp_path / storage.ROUTINES_FILE)
    add_user_id(tests.data.SETS_DF).to_feather(tmp_path / storage.SETS_FILE)
    add_user_id(tests.data.WORKOUTS_DF).to_feather(tmp_path / storage.WORKOUTS_FILE)
    add_user_id(tests.data.BODYWEIGHT_DF).to_feather(tmp_path / storage.BODYWEIGHT_FILE)
    add_user_id(tests.data.BODYFAT_DF).to_feather(tmp_path / storage.BODYFAT_FILE)
    add_user_id(tests.data.PERIOD_DF).to_feather(tmp_path / storage.PERIOD_FILE)
    return tmp_path


def add_user_id(df: pd.DataFrame) -> pd.DataFrame:
    return pd.concat(
        [
            pd.Series(len(df) * [1], index=df.index, name="user_id"),
            df,
        ],
        axis=1,
    ).append(
        pd.concat(
            [
                pd.Series(len(df) * [2], index=df.index, name="user_id"),
                df,
            ],
            axis=1,
        ),
        ignore_index=True,
    )
