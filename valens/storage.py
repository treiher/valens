import datetime
import re
from typing import Dict

import pandas as pd
import yaml

from valens import utils


def read_workouts() -> pd.DataFrame:
    config = utils.parse_config()
    cols: Dict[str, list] = {
        "date": [],
        "exercise": [],
        "reps": [],
        "time": [],
        "weight": [],
        "rpe": [],
    }

    with open(config["workout_file"]) as log_file:
        log = yaml.safe_load(log_file)

        for date, exercises in log.items():
            for exercise, sets in exercises.items():
                for s in sets:
                    for k, v in parse_set(str(s)).items():
                        if k in ["weight", "rpe"]:
                            cols[k].append(float(v) if v else None)
                        else:
                            cols[k].append(int(v) if v else None)
                    cols["date"].append(date)
                    cols["exercise"].append(exercise)

    df = pd.DataFrame(cols)
    df["rir"] = 10 - df["rpe"]

    return df


def read_bodyweight() -> Dict[datetime.date, float]:
    config = utils.parse_config()

    with open(config["bodyweight_file"]) as f:
        return yaml.safe_load(f)


def write_bodyweight(date: datetime.date, weight: float) -> None:
    config = utils.parse_config()
    log = read_bodyweight()
    log[date] = weight

    with open(config["bodyweight_file"], "w") as f:
        f.write(yaml.dump(log, default_flow_style=False))


def parse_set(set_string: str) -> Dict[str, str]:
    m = re.match(
        r"^(?P<reps>\d+)?"
        r"(?:(?:^|x)(?P<time>\d+)s)?"
        r"(?:(?:^|x)(?P<weight>\d+(?:\.\d+)?)kg)?"
        r"(?:@(?P<rpe>\d+(?:\.\d+)?))?$",
        set_string,
    )
    if not m:
        raise Exception(f"unexpected format for set '{set_string}'")
    return m.groupdict()
