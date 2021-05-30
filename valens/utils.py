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
