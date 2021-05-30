from dataclasses import dataclass
from datetime import date, timedelta
from typing import Optional

import pandas as pd


@dataclass
class Bodyweight:
    last: timedelta
    current: int


def analyze(df: pd.DataFrame) -> Optional[Bodyweight]:
    if df.empty:
        return None

    return Bodyweight(date.today() - df.iloc[-1, 0], df.iloc[-1, 1])
