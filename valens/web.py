from collections import deque
from dataclasses import dataclass
from datetime import date, timedelta
from typing import Sequence, Tuple, Union

import pandas as pd
from flask import Flask, Response, make_response, redirect, render_template, request, url_for

from valens import diagram, storage, utils

app = Flask(__name__)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True


@dataclass
class Period:
    first: date
    last: date


@app.route("/")
def index_view() -> str:
    return render_template(
        "index.html",
        navigation=[
            (url_for("workouts_view"), "Workouts"),
            (url_for("routines_view"), "Routines"),
            (url_for("exercises_view"), "Exercises"),
            (url_for("bodyweight_view"), "Bodyweight"),
        ],
    )


@app.route("/bodyweight", methods=["GET", "POST"])
def bodyweight_view() -> str:
    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        weight = float(request.form["weight"])

        df = storage.read_bodyweight().set_index("date")
        if weight > 0:
            df.loc[date_] = weight
        else:
            df = df.drop(date_)
        storage.write_bodyweight(df.reset_index())

    period = parse_period_args()
    df = storage.read_bodyweight()
    df["date"] = pd.to_datetime(df["date"])
    ts_df = pd.DataFrame({"date": pd.date_range(df.iloc[0, 0], df.iloc[-1, 0])}).set_index("date")
    df = df.set_index("date")

    df["avg_weight"] = df.rolling(window=9, center=True).mean()["weight"]

    df = df.join(
        ts_df.join(df["avg_weight"]).interpolate().pct_change(periods=7, fill_method=None).mul(100),
        rsuffix="_change",
    )
    df["avg_weight_change"] = df["avg_weight_change"].iloc[0:-4]

    df = df[period.first : period.last]  # type: ignore

    bodyweight_list: deque = deque()
    for bw_date, weight, avg_weight, avg_weight_change in df.itertuples():
        bodyweight_list.appendleft(
            (
                bw_date,
                utils.format_number(weight),
                utils.format_number(avg_weight),
                utils.format_number(avg_weight_change),
            )
        )

    return render_template(
        "bodyweight.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
        bodyweight=bodyweight_list,
    )


@app.route("/exercises")
def exercises_view() -> str:
    df = storage.read_sets()
    exercise_list = df.sort_index(ascending=False).loc[:, "exercise"].unique()

    return render_template("exercises.html", exercise_list=exercise_list)


@app.route("/exercise/<name>")
def exercise_view(name: str) -> str:
    period = parse_period_args()
    df = storage.read_sets()
    df["reps+rir"] = df["reps"] + df["rir"]
    df = df.loc[lambda x: x["exercise"] == name].groupby(["date"]).mean()
    df = df[period.first : period.last]  # type: ignore

    workouts_list: deque = deque()
    for wo_date, reps, time, weight, rpe, _, reps_rir in df.itertuples():
        workouts_list.appendleft(
            (
                wo_date,
                utils.format_number(reps),
                utils.format_number(time),
                utils.format_number(weight),
                utils.format_number(rpe),
                utils.format_number(reps_rir),
            )
        )

    return render_template(
        "exercise.html",
        exercise=name,
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        workouts=workouts_list,
        today=date.today(),
    )


@app.route("/routines", methods=["GET", "POST"])
def routines_view() -> Union[str, Response]:
    if request.method == "POST":
        return redirect(url_for("routine_view", name=request.form["name"]), Response=Response)

    df = storage.read_routine_sets()
    routines = df.sort_index(ascending=False).loc[:, "routine"].unique()

    return render_template("routines.html", routines=routines)


@app.route("/routine/<name>", methods=["GET", "POST"])
def routine_view(name: str) -> Union[str, Response]:
    df_s = storage.read_routine_sets()
    df_r = storage.read_routines()

    if request.method == "POST":
        df_s = df_s.loc[df_s["routine"] != name]
        df_r = df_r.loc[df_r["routine"] != name]

        if "delete" in request.form:
            storage.write_routine_sets(df_s)
            storage.write_routines(df_r)
            return redirect(url_for("routines_view"), Response=Response)

        for ex, set_count in zip(
            request.form.getlist("exercise"), request.form.getlist("set_count")
        ):
            if ex and set_count:
                set_count = int(set_count)
                df_s = df_s.append(
                    pd.DataFrame({"routine": [name] * set_count, "exercise": [ex] * set_count}),
                    ignore_index=True,
                )

        if "notes" in request.form:
            notes = request.form["notes"]
            if notes:
                df_r = df_r.append(
                    {"routine": name, "notes": notes},
                    ignore_index=True,
                )

        storage.write_routine_sets(df_s)
        storage.write_routines(df_r)

    df_s = df_s.loc[df_s["routine"] == name, df_s.columns != "routine"]
    routine = [
        (i + 1, exercise, sets["exercise"].count())
        for i, (exercise, sets) in enumerate(df_s.groupby("exercise", sort=False))
    ]

    df_r = df_r[df_r["routine"] == name]
    assert 0 <= len(df_r) <= 1
    notes = df_r.iat[0, 1] if len(df_r) == 1 else ""

    return render_template("routine.html", name=name, routine=routine, notes=notes)


@app.route("/workouts", methods=["GET", "POST"])
def workouts_view() -> Union[str, Response]:
    # pylint: disable=too-many-locals

    notification = ""
    df_rs = storage.read_routine_sets()
    df_r = storage.read_routines()
    df_s = storage.read_sets()
    df_w = storage.read_workouts()

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        routine = request.form["routine"]
        df_routine = df_rs.loc[df_rs["routine"] == routine, df_rs.columns != "routine"]

        if not df_routine.empty:
            df_routine["date"] = date_

            if len(df_s[df_s["date"] == date_]) == 0:
                df_s = pd.concat([df_s[df_s["date"] != date_], df_routine])
                storage.write_sets(df_s)

                df_r = df_r.loc[df_r["routine"] == routine]
                assert 0 <= len(df_r) <= 1
                if len(df_r) == 1:
                    df_w = df_w.loc[df_w["date"] != date_]
                    df_w = df_w.append({"date": date_, "notes": df_r.iat[0, 1]}, ignore_index=True)
                    storage.write_workouts(df_w)

                return redirect(
                    url_for("workout_view", workout_date=request.form["date"]), Response=Response
                )

            notification = f"Workout on {date_} already exists"

        else:
            notification = f"Routine {routine} undefined"

    period = parse_period_args()
    df_s["reps+rir"] = df_s["reps"] + df_s["rir"]
    df_s = df_s[(df_s["date"] >= period.first) & (df_s["date"] <= period.last)].drop("rir", 1)
    wo = df_s.groupby(["date"]).mean()
    wo["volume"] = df_s.groupby(["date"]).sum()["reps"]

    workouts_list: deque = deque()
    for wo_date, reps, time, weight, rpe, reps_rir, volume in wo.itertuples():
        workouts_list.appendleft(
            (
                wo_date,
                utils.format_number(reps),
                utils.format_number(time),
                utils.format_number(weight),
                utils.format_number(rpe),
                utils.format_number(reps_rir),
                utils.format_number(volume),
            )
        )

    routines = reversed([r for r, _ in df_rs.groupby("routine", sort=False)])

    return render_template(
        "workouts.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
        routines=routines,
        workouts=workouts_list,
        notification=notification,
    )


@app.route("/workout/<workout_date>", methods=["GET", "POST"])
def workout_view(workout_date: str) -> Union[str, Response]:
    # pylint: disable=too-many-locals

    notification = ""
    date_ = date.fromisoformat(workout_date)
    df_s = storage.read_sets()
    df_w = storage.read_workouts()

    if request.method == "POST":
        df_s = df_s.loc[df_s["date"] != date_]
        df_w = df_w.loc[df_w["date"] != date_]

        if "delete" in request.form:
            storage.write_sets(df_s)
            storage.write_workouts(df_w)
            return redirect(url_for("workouts_view"), Response=Response)

        try:
            for name, values in request.form.lists():
                if name.startswith("exercise:"):
                    for set_str in values:
                        df_s = df_s.append(
                            {
                                "date": date_,
                                "exercise": name.split(":")[1],
                                **utils.parse_set(set_str),
                            },
                            ignore_index=True,
                        )
                elif name == "notes" and values[0]:
                    df_w = df_w.append(
                        {"date": date_, "notes": values[0]},
                        ignore_index=True,
                    )

            storage.write_sets(df_s)
            storage.write_workouts(df_w)
        except ValueError as e:
            notification = str(e)

    df_cur = df_s[df_s["date"] == date_].groupby("exercise", sort=False)
    workout_data = []
    for ex, sets in df_cur:
        current = [
            utils.format_set(set_tuple[1:])
            for set_tuple in sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous_date = df_s[(df_s["date"] < date_) & (df_s["exercise"] == ex)]["date"].max()
        previous_sets = df_s.loc[(df_s["date"] == previous_date) & (df_s["exercise"] == ex)]
        previous = [
            utils.format_set(set_tuple[1:])
            for set_tuple in previous_sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous = previous + [""] * (len(current) - len(previous))
        workout_data.append((ex, zip(current, previous)))

    df_w = df_w[df_w["date"] == date_]
    assert 0 <= len(df_w) <= 1
    notes = df_w.iat[0, 1] if len(df_w) == 1 else ""

    return render_template(
        "workout.html",
        date=date_,
        workout=workout_data,
        notes=notes,
        notification=notification,
    )


@app.route("/image/<image_type>")
def image_view(image_type: str) -> Response:
    period = parse_period_args()
    if image_type == "bodyweight":
        fig = diagram.bodyweight(period.first, period.last)
    elif image_type == "workouts":
        fig = diagram.workouts(period.first, period.last)
    elif image_type.startswith("exercise"):
        name = request.args.get("name", "")
        fig = diagram.exercise(name, period.first, period.last)
    else:
        return make_response("", 404)
    return Response(diagram.plot_svg(fig), mimetype="image/svg+xml")


def parse_period_args() -> Period:
    args_first = request.args.get("first", "")
    args_last = request.args.get("last", "")
    first = date.fromisoformat(args_first) if args_first else date.today() - timedelta(days=30)
    last = date.fromisoformat(args_last) if args_last else date.today()
    return Period(first, last)


def periods() -> Sequence[Tuple[str, date, date]]:
    today = date.today()
    return [
        ("12M", today - timedelta(weeks=52), today),
        ("6M", today - timedelta(weeks=26), today),
        ("3M", today - timedelta(weeks=13), today),
        ("1M", today - timedelta(days=30), today),
    ]


def prev_period(current: Period) -> Period:
    prev_last = current.first - timedelta(days=1)
    prev_first = prev_last - (current.last - current.first)
    return Period(prev_first, prev_last)


def next_period(current: Period) -> Period:
    next_first = current.last + timedelta(days=1)
    next_last = next_first + (current.last - current.first)
    return Period(next_first, next_last)
