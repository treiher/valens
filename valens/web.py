from dataclasses import dataclass
from datetime import date, timedelta
from typing import Sequence, Tuple

from flask import Flask, Response, make_response, render_template, request, url_for

from valens import diagram, storage

app = Flask(__name__)


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

    return render_template(
        "bodyweight.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
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

    return render_template(
        "exercise.html",
        exercise=name,
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
    )


@app.route("/workouts")
def workouts() -> str:
    period = parse_period_args()

    return render_template(
        "workouts.html",
        current=period,
        periods=periods(),
        previous=prev_period(period),
        next=next_period(period),
        today=date.today(),
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
