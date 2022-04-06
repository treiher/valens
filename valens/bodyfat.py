from dataclasses import dataclass
from datetime import date, timedelta
from typing import Optional

import pandas as pd
from flask import session

from valens.models import Sex


@dataclass
class Bodyfat:
    last: timedelta
    current: float


def analyze(df: pd.DataFrame) -> Optional[Bodyfat]:
    if df.empty:
        return None

    return Bodyfat(
        date.today() - df.iloc[-1, 0],
        (
            jackson_pollock_3_female(df)
            if session["sex"] == Sex.FEMALE
            else jackson_pollock_3_male(df)
        ).iloc[-1],
    )


def jackson_pollock(s: pd.Series, k0: float, k1: float, k2: float, ka: float) -> pd.Series:
    # pylint: disable = invalid-name
    a = 30
    return (495 / (k0 - (k1 * s) + (k2 * s**2) - (ka * a))) - 450


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
