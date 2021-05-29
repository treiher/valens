from pathlib import Path
from typing import Sequence

import pandas as pd

from valens import config

USERS_FILE = "users.feather"
ROUTINES_FILE = "routines.feather"
ROUTINE_SETS_FILE = "routine_sets.feather"
WORKOUTS_FILE = "workouts.feather"
SETS_FILE = "sets.feather"
BODYWEIGHT_FILE = "bodyweight.feather"
PERIOD_FILE = "period.feather"

USERS_COLS = ["user_id", "name", "sex"]
ROUTINES_COLS = ["user_id", "routine", "notes"]
ROUTINE_SETS_COLS = ["user_id", "routine", "exercise", "reps", "time", "weight", "rpe"]
WORKOUTS_COLS = ["user_id", "date", "notes"]
SETS_COLS = ["user_id", "date", "exercise", "reps", "time", "weight", "rpe"]
BODYWEIGHT_COLS = ["user_id", "date", "weight"]
PERIOD_COLS = ["user_id", "date", "intensity"]


def initialize() -> None:
    for f, c in [
        (USERS_FILE, USERS_COLS),
        (ROUTINES_FILE, ROUTINES_COLS),
        (ROUTINE_SETS_FILE, ROUTINE_SETS_COLS),
        (WORKOUTS_FILE, WORKOUTS_COLS),
        (SETS_FILE, SETS_COLS),
        (BODYWEIGHT_FILE, BODYWEIGHT_COLS),
        (PERIOD_FILE, PERIOD_COLS),
    ]:
        if not (config.DATA_DIRECTORY / f).exists():
            pd.DataFrame({k: [] for k in c}).to_feather(config.DATA_DIRECTORY / f)
        else:
            print(f"warning: file already exists: {config.DATA_DIRECTORY / f}")


def read_users() -> pd.DataFrame:
    return pd.read_feather(config.DATA_DIRECTORY / USERS_FILE)


def write_users(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, USERS_COLS]
    df.to_feather(config.DATA_DIRECTORY / USERS_FILE)


def read_routines(user_id: int) -> pd.DataFrame:
    return read_user_part(config.DATA_DIRECTORY / ROUTINES_FILE, ROUTINES_COLS, user_id)


def write_routines(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, ROUTINES_COLS]
    write_user_part(config.DATA_DIRECTORY / ROUTINES_FILE, df, user_id)


def read_routine_sets(user_id: int) -> pd.DataFrame:
    return read_user_part(config.DATA_DIRECTORY / ROUTINE_SETS_FILE, ROUTINE_SETS_COLS, user_id)


def write_routine_sets(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, ROUTINE_SETS_COLS]
    write_user_part(config.DATA_DIRECTORY / ROUTINE_SETS_FILE, df, user_id)


def read_workouts(user_id: int) -> pd.DataFrame:
    return read_user_part(config.DATA_DIRECTORY / WORKOUTS_FILE, WORKOUTS_COLS, user_id)


def write_workouts(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, WORKOUTS_COLS]
    write_user_part(config.DATA_DIRECTORY / WORKOUTS_FILE, df, user_id)


def read_sets(user_id: int) -> pd.DataFrame:
    df = read_user_part(config.DATA_DIRECTORY / SETS_FILE, SETS_COLS, user_id)
    df["rir"] = 10 - df["rpe"]
    return df


def write_sets(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, SETS_COLS]
    write_user_part(config.DATA_DIRECTORY / SETS_FILE, df, user_id)


def read_bodyweight(user_id: int) -> pd.DataFrame:
    return read_user_part(config.DATA_DIRECTORY / BODYWEIGHT_FILE, BODYWEIGHT_COLS, user_id)


def write_bodyweight(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, BODYWEIGHT_COLS]
    write_user_part(config.DATA_DIRECTORY / BODYWEIGHT_FILE, df, user_id)


def read_period(user_id: int) -> pd.DataFrame:
    return read_user_part(config.DATA_DIRECTORY / PERIOD_FILE, PERIOD_COLS, user_id)


def write_period(df: pd.DataFrame, user_id: int) -> None:
    df = df.reset_index()
    df.insert(0, "user_id", len(df) * [user_id])
    df = df.loc[:, PERIOD_COLS]
    write_user_part(config.DATA_DIRECTORY / PERIOD_FILE, df, user_id)


def read_user_part(storage_file: Path, columns: Sequence[str], user_id: int) -> pd.DataFrame:
    return pd.read_feather(storage_file).loc[lambda x: x["user_id"] == user_id].loc[:, columns[1:]]


def write_user_part(storage_file: Path, df: pd.DataFrame, user_id: int) -> None:
    df_file = pd.read_feather(storage_file).loc[lambda x: x["user_id"] != user_id]
    df_file = pd.concat([df_file, df], ignore_index=True)
    df_file.to_feather(storage_file)
