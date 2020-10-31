import pandas as pd

from valens import config

ROUTINES_FILE = "routines.feather"
ROUTINE_SETS_FILE = "routine_sets.feather"
WORKOUTS_FILE = "workouts.feather"
SETS_FILE = "sets.feather"
BODYWEIGHT_FILE = "bodyweight.feather"


def read_routines() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / ROUTINES_FILE)
    return df


def write_routines(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ["routine", "notes"]]
    df.to_feather(config.DATA_DIRECTORY / ROUTINES_FILE)


def read_routine_sets() -> pd.DataFrame:
    return pd.read_feather(config.DATA_DIRECTORY / ROUTINE_SETS_FILE)


def write_routine_sets(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ["routine", "exercise", "reps", "time", "weight", "rpe"]]
    df.to_feather(config.DATA_DIRECTORY / ROUTINE_SETS_FILE)


def read_workouts() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / WORKOUTS_FILE)
    return df


def write_workouts(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ["date", "notes"]]
    df.to_feather(config.DATA_DIRECTORY / WORKOUTS_FILE)


def read_sets() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / SETS_FILE)
    df["rir"] = 10 - df["rpe"]
    return df


def write_sets(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ["date", "exercise", "reps", "time", "weight", "rpe"]]
    df.to_feather(config.DATA_DIRECTORY / SETS_FILE)


def read_bodyweight() -> pd.DataFrame:
    return pd.read_feather(config.DATA_DIRECTORY / BODYWEIGHT_FILE)


def write_bodyweight(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ["date", "weight"]]
    df.to_feather(config.DATA_DIRECTORY / BODYWEIGHT_FILE)
