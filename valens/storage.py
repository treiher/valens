import pandas as pd

from valens import config

ROUTINES_FILE = "routines.feather"
ROUTINE_SETS_FILE = "routine_sets.feather"
WORKOUTS_FILE = "workouts.feather"
SETS_FILE = "sets.feather"
BODYWEIGHT_FILE = "bodyweight.feather"

ROUTINES_COLS = ["routine", "notes"]
ROUTINE_SETS_COLS = ["routine", "exercise", "reps", "time", "weight", "rpe"]
WORKOUTS_COLS = ["date", "notes"]
SETS_COLS = ["date", "exercise", "reps", "time", "weight", "rpe"]
BODYWEIGHT_COLS = ["date", "weight"]


def initialize() -> None:
    for f, c in [
        (ROUTINES_FILE, ROUTINES_COLS),
        (ROUTINE_SETS_FILE, ROUTINE_SETS_COLS),
        (WORKOUTS_FILE, WORKOUTS_COLS),
        (SETS_FILE, SETS_COLS),
        (BODYWEIGHT_FILE, BODYWEIGHT_COLS),
    ]:
        if not (config.DATA_DIRECTORY / f).exists():
            pd.DataFrame({k: [] for k in c}).to_feather(config.DATA_DIRECTORY / f)
        else:
            print(f"warning: file already exists: {config.DATA_DIRECTORY / f}")


def read_routines() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / ROUTINES_FILE)
    return df


def write_routines(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ROUTINES_COLS]
    df.to_feather(config.DATA_DIRECTORY / ROUTINES_FILE)


def read_routine_sets() -> pd.DataFrame:
    return pd.read_feather(config.DATA_DIRECTORY / ROUTINE_SETS_FILE)


def write_routine_sets(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, ROUTINE_SETS_COLS]
    df.to_feather(config.DATA_DIRECTORY / ROUTINE_SETS_FILE)


def read_workouts() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / WORKOUTS_FILE)
    return df


def write_workouts(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, WORKOUTS_COLS]
    df.to_feather(config.DATA_DIRECTORY / WORKOUTS_FILE)


def read_sets() -> pd.DataFrame:
    df = pd.read_feather(config.DATA_DIRECTORY / SETS_FILE)
    df["rir"] = 10 - df["rpe"]
    return df


def write_sets(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, SETS_COLS]
    df.to_feather(config.DATA_DIRECTORY / SETS_FILE)


def read_bodyweight() -> pd.DataFrame:
    return pd.read_feather(config.DATA_DIRECTORY / BODYWEIGHT_FILE)


def write_bodyweight(df: pd.DataFrame) -> None:
    df = df.reset_index()
    df = df.loc[:, BODYWEIGHT_COLS]
    df.to_feather(config.DATA_DIRECTORY / BODYWEIGHT_FILE)
