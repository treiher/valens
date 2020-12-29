import io
from datetime import date

import matplotlib
import matplotlib.style
import pandas as pd
from matplotlib.backends.backend_svg import FigureCanvasSVG
from matplotlib.figure import Figure

from valens import storage

matplotlib.style.use("seaborn-whitegrid")

matplotlib.rc("font", family="Roboto", size=12)
matplotlib.rc("legend", handletextpad=0.5, columnspacing=0.5, handlelength=1)


def plot_svg(fig: Figure) -> bytes:
    fig.set_size_inches(5, 4)
    output = io.BytesIO()
    FigureCanvasSVG(fig).print_svg(output)
    return output.getvalue()


def workouts(user_id: int, first: date = None, last: date = None) -> Figure:
    df = storage.read_sets(user_id)
    df["reps+rir"] = df["reps"] + df["rir"]
    df = df.drop("rir", 1)
    r = df.groupby(["date"]).mean()

    r_interval = r[first:last]  # type: ignore  # ISSUE: python/typing#159
    ymax = max(10, int(max(list(r_interval.max()))) + 1 if not r_interval.empty else 0)

    plot = r_interval.plot(style=".-", xlim=(first, last), ylim=(0, ymax), legend=False)

    ax2 = (
        df.groupby(["date"])
        .sum()["reps"]
        .plot(secondary_y=True, style=".-", label="volume (right)")
    )
    ax2.set(ylim=(0, None))
    ax2.grid(None)

    plot.set(xlabel=None)
    plot.grid()

    fig = plot.get_figure()
    _common_layout(fig)
    return fig


def exercise(user_id: int, name: str, first: date = None, last: date = None) -> Figure:
    df = storage.read_sets(user_id)
    df["reps+rir"] = df["reps"] + df["rir"]
    df_ex = df.loc[lambda x: x["exercise"] == name]
    r = df_ex.loc[:, ["date", "reps", "reps+rir", "weight", "time"]].groupby(["date"]).mean()

    r_interval = r[first:last]  # type: ignore  # ISSUE: python/typing#159
    ymax = int(max([v for v in r_interval.max() if not pd.isna(v)] + [9])) + 1

    plot = r.plot(style=".-", xlim=(first, last), ylim=(0, ymax), legend=False)
    plot.set(xlabel=None)

    fig = plot.get_figure()
    _common_layout(fig)
    return fig


def bodyweight(user_id: int, first: date = None, last: date = None) -> Figure:
    df = storage.read_bodyweight(user_id).set_index("date")

    df_interval = df.loc[first:last]  # type: ignore  # ISSUE: python/typing#159
    ymin = int(df_interval.min()) if not df_interval.empty else None
    ymax = int(df_interval.max()) + 1 if not df_interval.empty else None

    plot = df_interval.plot(style=".-", xlim=(first, last), ylim=(ymin, ymax), legend=False)
    df.rolling(window=9, center=True).mean()["weight"].plot(style="-")

    fig = plot.get_figure()
    _common_layout(fig)
    return fig


def _common_layout(fig: Figure) -> None:
    fig.legend(loc="upper center", bbox_to_anchor=(0.5, 0.97), ncol=6)
    fig.autofmt_xdate()
