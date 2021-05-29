import re
from enum import IntEnum
from typing import Dict, Optional, Tuple

import numpy as np
import pandas as pd


class Sex(IntEnum):
    FEMALE = 0
    MALE = 1


def parse_set(set_string: Optional[str]) -> Dict[str, Optional[float]]:
    m = re.match(
        r"^(?P<reps>\d+)?"
        r"(?:(?:^|x)(?P<time>\d+)s)?"
        r"(?:(?:^|x)(?P<weight>\d+(?:\.\d+)?)kg)?"
        r"(?:@(?P<rpe>\d+(?:\.\d+)?))?$",
        set_string or "",
    )
    if not m:
        raise ValueError(f"unexpected format for set '{set_string}'")
    return {k: float(v) if v else np.nan for k, v in m.groupdict().items()}


def format_set(set_tuple: Tuple[float, float, float, float]) -> str:
    reps, time, weight, rpe = set_tuple
    result = ""
    if not pd.isna(reps):
        result += f"{reps:.0f}"
    if not pd.isna(time):
        if result:
            result += "x"
        result += f"{time:.0f}s"
    if not pd.isna(weight):
        if result:
            result += "x"
        result += f"{weight:.1f}kg"
    if not pd.isna(rpe):
        result += f"@{rpe}"
    return result


def format_number(number: float) -> str:
    if pd.isna(number):
        return "-"
    return f"{number:.1f}"


def jackson_pollock(s: pd.Series, k0: float, k1: float, k2: float, ka: float) -> pd.Series:
    # pylint: disable = invalid-name
    a = 30
    return (495 / (k0 - (k1 * s) + (k2 * s ** 2) - (ka * a))) - 450


def jackson_pollock_3_female(df: pd.DataFrame) -> pd.Series:
    return jackson_pollock(
        df["tricep"] + df["suprailiac"] + df["tigh"],
        1.0994921,
        0.0009929,
        0.0000023,
        0.0001392,
    )


def jackson_pollock_3_male(df: pd.DataFrame) -> pd.Series:
    return jackson_pollock(
        df["chest"] + df["abdominal"] + df["tigh"],
        1.10938,
        0.0008267,
        0.0000016,
        0.0002574,
    )


def jackson_pollock_7_female(df: pd.DataFrame) -> pd.Series:
    return jackson_pollock(
        df["chest"]
        + df["abdominal"]
        + df["tigh"]
        + df["tricep"]
        + df["subscapular"]
        + df["suprailiac"]
        + df["midaxillary"],
        1.097,
        0.00046971,
        0.00000056,
        0.00012828,
    )


def jackson_pollock_7_male(df: pd.DataFrame) -> pd.Series:
    return jackson_pollock(
        df["chest"]
        + df["abdominal"]
        + df["tigh"]
        + df["tricep"]
        + df["subscapular"]
        + df["suprailiac"]
        + df["midaxillary"],
        1.112,
        0.00043499,
        0.00000055,
        0.00028826,
    )
