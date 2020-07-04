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
def index() -> str:
    return render_template(
        "index.html",
        navigation=[
            (url_for("workouts"), "Workouts"),
            (url_for("exercises"), "Exercises"),
            (url_for("bodyweight"), "Bodyweight"),
        ],
    )


@app.route("/bodyweight", methods=["GET", "POST"])
def bodyweight() -> str:
    notification = ""

    if request.method == "POST":
        d = date.fromisoformat(request.form["date"])
        w = float(request.form["weight"])
        storage.write_bodyweight(d, w)
        notification = f"Added weight of {w} kg on {d}"

    period = parse_period_args()
    bw = storage.read_bodyweight()
    df = pd.DataFrame({"weight": list(bw.values())}, index=list(bw.keys()))
    df["avg_weight"] = df.rolling(window=9, center=True).mean()["weight"]
    df = df[period.first : period.last]  # type: ignore

    bodyweight_list: deque = deque()
    for bw_date, weight, avg_weight in df.itertuples():
        bodyweight_list.appendleft(
            (bw_date, utils.format_number(weight), utils.format_number(avg_weight))
        )

    return render_template(
        "bodyweight.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
        bodyweight=bodyweight_list,
        notification=notification,
    )


@app.route("/exercises")
def exercises() -> str:
    df = storage.read_workouts()
    exercise_list = df.sort_index(ascending=False).loc[:, "exercise"].unique()

    return render_template("exercises.html", exercise_list=exercise_list)


@app.route("/exercise/<name>")
def exercise(name: str) -> str:
    period = parse_period_args()
    df = storage.read_workouts()
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


@app.route("/workouts", methods=["GET", "POST"])
def workouts() -> Union[str, Response]:
    notification = ""
    templates = storage.read_templates()
    df = storage.read_workouts()

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        template = templates[request.form["template"]]
        template["date"] = date_

        if len(df[df["date"] == date_]) == 0:
            df = pd.concat([df[df["date"] != date_], template])
            storage.write_workouts(df)
            return redirect(
                url_for("workout", workout_date=request.form["date"]), Response=Response
            )

        notification = f"Workout on {date_} already exists"

    period = parse_period_args()
    df["reps+rir"] = df["reps"] + df["rir"]
    df = df[(df["date"] >= period.first) & (df["date"] <= period.last)].drop("rir", 1)
    wo = df.groupby(["date"]).mean()
    wo["volume"] = df.groupby(["date"]).sum()["reps"]

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

    return render_template(
        "workouts.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
        templates=templates.keys(),
        workouts=workouts_list,
        notification=notification,
    )


@app.route("/workout/<workout_date>", methods=["GET", "POST"])
def workout(workout_date: str) -> Union[str, Response]:
    notification = ""
    date_ = date.fromisoformat(workout_date)
    df = storage.read_workouts()

    if request.method == "POST":
        df_new = df[df["date"] != date_]

        if "delete" in request.form:
            df = df[df["date"] != date_]
            storage.write_workouts(df)
            return redirect(url_for("workouts"), Response=Response)

        try:
            for ex, sets in request.form.lists():
                for set_str in sets:
                    df_new = df_new.append(
                        {"date": date_, "exercise": ex, **utils.parse_set(set_str)},
                        ignore_index=True,
                    )
            df = df_new
            storage.write_workouts(df)
        except ValueError as e:
            notification = str(e)

    df_cur = df[df["date"] == date_].groupby("exercise", sort=False)
    workout_data = []
    for ex, sets in df_cur:
        current = [
            utils.format_set(set_tuple[1:])
            for set_tuple in sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous_date = df[(df["date"] < date_) & (df["exercise"] == ex)]["date"].max()
        previous_sets = df.loc[(df["date"] == previous_date) & (df["exercise"] == ex)]
        previous = [
            utils.format_set(set_tuple[1:])
            for set_tuple in previous_sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous = previous + [""] * (len(current) - len(previous))
        workout_data.append((ex, zip(current, previous)))

    return render_template(
        "workout.html", date=date_, workout=workout_data, notification=notification
    )


@app.route("/image/<image_type>")
def image(image_type: str) -> Response:
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
