import io
from datetime import date, timedelta

import matplotlib
import matplotlib.pyplot as plt
import matplotlib.style
import numpy as np
import pandas as pd
from flask import session
from matplotlib.backends.backend_svg import FigureCanvasSVG
from matplotlib.figure import Figure

from valens import bodyfat, storage
from valens.models import Sex

matplotlib.style.use("seaborn-whitegrid")

matplotlib.rc("font", family="Roboto", size=12)
matplotlib.rc("legend", handletextpad=0.5, columnspacing=0.5, handlelength=1)

STYLE = ".-"
COLOR = {
    "avg. weight": "#FAA43A",
    "intensity": "#F15854",
    "reps": "#5DA5DA",
    "reps+rir": "#FAA43A",
    "rpe": "#F17CB0",
    "time": "#B276B2",
    "tut": "#F15854",
    "volume": "#4D4D4D",
    "weight": "#60BD68",
    "fat3": "#FAA43A",
    "fat7": "#F15854",
}


def plot_svg(fig: Figure) -> bytes:
    output = io.BytesIO()
    FigureCanvasSVG(fig).print_svg(output)
    return output.getvalue()


def plot_workouts(user_id: int, first: date = None, last: date = None) -> Figure:
    df = storage.read_sets(user_id)
    df["reps+rir"] = df["reps"] + df["rir"]
    df["tut"] = df["reps"].replace(np.nan, 1) * df["time"]
    return _workouts_exercise(df, first, last)


def plot_exercise(user_id: int, name: str, first: date = None, last: date = None) -> Figure:
    df = storage.read_sets(user_id)
    df["reps+rir"] = df["reps"] + df["rir"]
    df["tut"] = df["reps"].replace(np.nan, 1) * df["time"]
    df_ex = df.loc[lambda x: x["exercise"] == name]
    return _workouts_exercise(df_ex, first, last)


def _workouts_exercise(df: pd.DataFrame, first: date = None, last: date = None) -> Figure:
    fig, axs = plt.subplots(4)

    margin_first = first - timedelta(days=90) if first else None
    margin_last = last + timedelta(days=90) if last else None

    df_mean = df.loc[:, ["date", "reps", "reps+rir", "weight", "time"]].groupby(["date"]).mean()
    df_mean_interval = df_mean[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159

    for i, cols in enumerate([["reps", "reps+rir"], ["weight"], ["time"]]):
        d = df_mean_interval.loc[:, cols]
        ymax = max(
            10,
            int(max(list(d.max()))) + 1 if not d.empty and not all(pd.isna(list(d.max()))) else 0,
        )
        d.plot(
            ax=axs[i],
            style=STYLE,
            color=COLOR,
            xlim=(first, last),
            ylim=(0, ymax),
            legend=False,
        )

    df_sum = df.loc[:, ["date", "reps", "tut"]].groupby(["date"]).sum()
    df_sum_interval = df_sum[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    df_sum_interval.columns = ["volume", "tut"]
    df_sum_interval.plot(
        ax=axs[3],
        style=STYLE,
        color=COLOR,
        xlim=(first, last),
        ylim=(0, None),
        legend=False,
    ).set(xlabel=None)

    _common_layout(fig)
    fig.set_size_inches(5, 8)
    fig.subplots_adjust(left=0.1, right=0.9, top=0.9, bottom=0.1)
    return fig


def plot_bodyweight(user_id: int, first: date = None, last: date = None) -> Figure:
    df = storage.read_bodyweight(user_id).set_index("date")

    margin_first = first - timedelta(days=90) if first else None
    margin_last = last + timedelta(days=90) if last else None

    df_interval = df.loc[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    ymin = int(df_interval.min()) if not df_interval.empty else None
    ymax = int(df_interval.max()) + 1 if not df_interval.empty else None

    plot = df_interval.plot(
        style=STYLE,
        color=COLOR,
        xlim=(first, last),
        ylim=(ymin, ymax),
        legend=False,
    )
    df.rolling(window=9, center=True).mean()["weight"].plot(
        style="-", color=COLOR, label="avg. weight"
    ).set(xlabel=None)

    fig = plot.get_figure()
    _common_layout(fig)
    fig.set_size_inches(5, 4)
    return fig


def plot_bodyfat(user_id: int, first: date = None, last: date = None) -> Figure:
    fig, ax1 = plt.subplots(1, 1)

    margin_first = first - timedelta(days=90) if first else None
    margin_last = last + timedelta(days=90) if last else None

    df = storage.read_bodyfat(user_id).set_index("date")
    df_interval = df.loc[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    df_diagram = pd.DataFrame(
        {
            "fat3": (
                bodyfat.jackson_pollock_3_female(df_interval)
                if session["sex"] == Sex.FEMALE
                else bodyfat.jackson_pollock_3_male(df_interval)
            ),
            "fat7": (
                bodyfat.jackson_pollock_7_female(df_interval)
                if session["sex"] == Sex.FEMALE
                else bodyfat.jackson_pollock_7_male(df_interval)
            ),
        }
    )

    ymin = int(df_diagram.min().min()) if not df_diagram.empty else None
    ymax = int(df_diagram.max().max()) + 1 if not df_diagram.empty else None

    for col in ["fat3", "fat7"]:
        df_diagram[col].dropna().plot(
            ax=ax1,
            style=STYLE,
            color=COLOR,
            xlim=(first, last),
            ylim=(ymin, ymax),
            legend=False,
        ).set(xlabel=None)

    ax2 = ax1.twinx()
    ax1.set_zorder(1)  # plot ax1 above ax2
    ax1.patch.set_visible(False)  # prevent hiding of ax2

    df = storage.read_bodyweight(user_id).set_index("date")

    df_interval = df.loc[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    ymin = int(df_interval.min()) if not df_interval.empty else None
    ymax = int(df_interval.max()) + 1 if not df_interval.empty else None

    df_interval.plot(
        ax=ax2,
        style=STYLE,
        color=COLOR,
        xlim=(first, last),
        ylim=(ymin, ymax),
        legend=False,
    )

    ax2.grid(None)

    _common_layout(fig)
    fig.set_size_inches(5, 4)
    return fig


def plot_period(user_id: int, first: date = None, last: date = None) -> Figure:
    fig, ax1 = plt.subplots(1, 1)

    df = storage.read_period(user_id).set_index("date")

    margin_first = first - timedelta(days=90) if first else None
    margin_last = last + timedelta(days=90) if last else None

    df_interval = df.loc[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    idx = pd.date_range(margin_first, margin_last)
    df_interval.reindex(idx, fill_value=0).plot(
        ax=ax1,
        style=STYLE,
        color=COLOR,
        xlim=(first, last),
        ylim=(0, 4),
        yticks=[0, 1, 2, 3, 4],
        legend=False,
    ).set(xlabel=None)

    ax2 = ax1.twinx()

    df = storage.read_bodyweight(user_id).set_index("date")

    df_interval = df.loc[margin_first:margin_last]  # type: ignore  # ISSUE: python/typing#159
    ymin = int(df_interval.min()) if not df_interval.empty else None
    ymax = int(df_interval.max()) + 1 if not df_interval.empty else None

    df_interval = df_interval.reindex(idx).dropna()

    if not df_interval.empty:
        df_interval.plot(
            ax=ax2,
            style=STYLE,
            color=COLOR,
            xlim=(first, last),
            ylim=(ymin, ymax),
            legend=False,
        )

    ax2.grid(None)

    _common_layout(fig)
    fig.set_size_inches(5, 4)
    return fig


def _common_layout(fig: Figure) -> None:
    fig.legend(loc="upper center", bbox_to_anchor=(0.5, 0.97), ncol=6)
    fig.autofmt_xdate()
