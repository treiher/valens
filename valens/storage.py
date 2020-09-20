import datetime
from collections import defaultdict
from typing import Dict, List

import pandas as pd
import yaml

from valens import config, utils


def read_routines() -> pd.DataFrame:
    cols: Dict[str, list] = {
        "routine": [],
        "exercise": [],
        "reps": [],
        "time": [],
        "weight": [],
        "rpe": [],
    }

    with open(config.DATA_DIRECTORY / "routine.yml") as f:
        yml = yaml.safe_load(f)
        for routine_name, exercises in yml.items():
            for exercise, sets in exercises.items():
                for s in sets:
                    for k, v in utils.parse_set(s).items():
                        cols[k].append(v if v else float("nan"))
                    cols["exercise"].append(exercise)
                    cols["routine"].append(routine_name)

    return pd.DataFrame(cols)


def write_routines(df: pd.DataFrame) -> None:
    routines: Dict[str, Dict[str, List[str]]] = defaultdict(dict)

    for name, routine in df.groupby("routine", sort=False):
        for exercise, sets in routine.groupby("exercise", sort=False):
            routines[name][exercise] = [""] * sets["exercise"].count()

    with open(config.DATA_DIRECTORY / "routine.yml", "w") as f:
        f.write(yaml.dump(dict(routines), default_flow_style=False, sort_keys=False))


def read_workouts() -> pd.DataFrame:
    cols: Dict[str, list] = {
        "date": [],
        "exercise": [],
        "reps": [],
        "time": [],
        "weight": [],
        "rpe": [],
    }

    with open(config.DATA_DIRECTORY / "workout.yml") as log_file:
        log = yaml.safe_load(log_file)

        for date, exercises in log.items():
            for exercise, sets in exercises.items():
                for s in sets:
                    for k, v in utils.parse_set(str(s)).items():
                        cols[k].append(v if v else float("nan"))
                    cols["date"].append(date)
                    cols["exercise"].append(exercise)

    df = pd.DataFrame(cols)
    df["rir"] = 10 - df["rpe"]

    return df


def write_workouts(df: pd.DataFrame) -> None:
    workouts: Dict[datetime.date, Dict[str, List[str]]] = defaultdict(dict)

    for date, workout in df.groupby("date", sort=False):
        for exercise, sets in workout.groupby("exercise", sort=False):
            workouts[date][exercise] = [
                utils.format_set(set_tuple[1:])
                for set_tuple in sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
            ]

    with open(config.DATA_DIRECTORY / "workout.yml", "w") as f:
        f.write(yaml.dump(dict(workouts), default_flow_style=False, sort_keys=False))


def read_bodyweight() -> Dict[datetime.date, float]:
    with open(config.DATA_DIRECTORY / "bodyweight.yml") as f:
        return yaml.safe_load(f)


def write_bodyweight(date: datetime.date, weight: float) -> None:
    log = read_bodyweight()
    log[date] = weight

    with open(config.DATA_DIRECTORY / "bodyweight.yml", "w") as f:
        f.write(yaml.dump(log, default_flow_style=False))
