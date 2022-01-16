from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from datetime import date, timedelta
from functools import wraps
from itertools import chain, zip_longest
from typing import Any, Callable, Sequence, Tuple, Union

import numpy as np
import pandas as pd
from flask import Flask, flash, make_response, redirect, render_template, request, session, url_for
from sqlalchemy import delete, select
from werkzeug.wrappers import Response

from valens import bodyfat, bodyweight, database as db, diagram, query, storage, utils, version
from valens.models import (
    BodyFat,
    BodyWeight,
    Period,
    Routine,
    RoutineExercise,
    Sex,
    User,
    Workout,
    WorkoutSet,
)

app = Flask(__name__)

app.config.from_object("valens.default_config")
app.config.from_envvar("VALENS_CONFIG", silent=True)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True


@dataclass
class Interval:
    first: date
    last: date


def login_required(function: Callable) -> Callable:  # type: ignore[type-arg]
    @wraps(function)
    def decorated_function(*args: Any, **kwargs: Any) -> Union[str, Response]:  # type: ignore[misc]
        if "username" not in session or "user_id" not in session or "sex" not in session:
            return redirect(url_for("login_view", next=f"{request.root_path}{request.path}"))
        return function(*args, **kwargs)

    return decorated_function


@app.teardown_appcontext
def teardown(_: BaseException = None) -> None:
    db.remove_session()


@app.route("/service-worker.js")
def service_worker() -> Union[str, Response]:
    return app.send_static_file("js/service-worker.js")


@app.route("/offline")
def offline() -> Union[str, Response]:
    return render_template("offline.html")


@app.route("/")
@app.route("/index")
@login_required
def index_view() -> Union[str, Response]:
    bw = bodyweight.analyze(storage.read_bodyweight(session["user_id"]))
    bf = bodyfat.analyze(storage.read_bodyfat(session["user_id"]))

    return render_template(
        "index.html",
        navigation=[
            (
                url_for("workouts_view"),
                "Workouts",
                "",
                "",
            ),
            (
                url_for("routines_view"),
                "Routines",
                "",
                "",
            ),
            (
                url_for("exercises_view"),
                "Exercises",
                "",
                "",
            ),
            (
                url_for("bodyweight_view"),
                "Bodyweight",
                f"{bw.current:.1f} kg" if bw else "",
                f"Last update {days(bw.last)}." if bw else "",
            ),
            (
                url_for("bodyfat_view"),
                "Body fat",
                f"{bf.current:.1f} %" if bf else "",
                f"Last update {days(bf.last)}." if bf else "",
            ),
            *(
                [
                    (
                        url_for("period_view"),
                        "Period",
                        "",
                        "",
                    )
                ]
                if session["sex"] == Sex.FEMALE
                else []
            ),
        ],
    )


@app.route("/login", methods=["GET", "POST"])
def login_view() -> Union[str, Response]:
    users = db.session.execute(select(User)).scalars().all()

    if request.method == "POST":
        for user in users:
            if user.name == request.form["username"]:
                session["user_id"] = user.id
                session["username"] = user.name
                session["sex"] = user.sex
                session.permanent = True
        return redirect(request.form["next"])

    return render_template("login.html", usernames=[u.name for u in users])


@app.route("/logout")
def logout_view() -> Response:
    session.pop("username", None)
    return redirect(url_for("login_view"))


@app.route("/users", methods=["GET", "POST"])
def users_view() -> Union[str, Response]:
    if request.method == "POST":
        users = db.session.execute(select(User)).scalars().all()

        form_user_ids = [int(i) for i in request.form.getlist("user_id")]
        assert len(form_user_ids) == len(set(form_user_ids)), "duplicate user id"
        form_usernames = request.form.getlist("username")
        assert len([n for n in form_usernames if n]) == len(
            {n for n in form_usernames if n}
        ), "duplicate username"
        form_sexes = [Sex(int(i)) for i in request.form.getlist("sex")]

        delete_users = []

        for user, user_id, name, sex in zip_longest(
            users, form_user_ids, form_usernames, form_sexes
        ):
            if user:
                assert user.id == user_id
            if user and name:
                user.name = name
                user.sex = sex
            if user is None and name:
                db.session.add(User(name=name, sex=sex))
            elif user and not name:
                delete_users.append(user.id)
            elif user is None and not name:
                continue

        db.session.execute(delete(User).where(User.id.in_(delete_users)))
        db.session.commit()

    users = db.session.execute(select(User)).scalars().all()

    return render_template(
        "users.html",
        users=[(u.id, u.name, u.sex) for u in users],
        version=version.get(),
    )


@app.route("/bodyweight", methods=["GET", "POST"])
@login_required
def bodyweight_view() -> Union[str, Response]:
    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        weight = float(request.form["weight"])

        db.session.execute(
            delete(BodyWeight)
            .where(BodyWeight.user_id == session["user_id"])
            .where(BodyWeight.date == date_)
        )

        if weight > 0:
            db.session.add(BodyWeight(user_id=session["user_id"], date=date_, weight=weight))

        db.session.commit()

    interval = parse_interval_args()
    df = storage.read_bodyweight(session["user_id"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        ts_df = pd.DataFrame({"date": pd.date_range(df.iloc[0, 0], df.iloc[-1, 0])}).set_index(
            "date"
        )
        df = df.set_index("date")

        df["avg_weight"] = df.rolling(window=9, center=True).mean()["weight"]

        df = df.join(
            ts_df.join(df["avg_weight"])
            .interpolate()
            .pct_change(periods=7, fill_method=None)
            .mul(100),
            rsuffix="_change",
        )
        df["avg_weight_change"] = df["avg_weight_change"].iloc[0:-4]

        df = df[interval.first : interval.last]  # type: ignore

    bodyweight_list: deque[tuple[date, str, str, str]] = deque()
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
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        bodyweight=bodyweight_list,
    )


@app.route("/bodyfat", methods=["GET", "POST"])
@login_required
def bodyfat_view() -> Union[str, Response]:
    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        chest = request.form["chest"]
        abdominal = request.form["abdominal"]
        tigh = request.form["tigh"]
        tricep = request.form["tricep"]
        subscapular = request.form["subscapular"]
        suprailiac = request.form["suprailiac"]
        midaxillary = request.form["midaxillary"]

        db.session.execute(
            delete(BodyFat)
            .where(BodyFat.user_id == session["user_id"])
            .where(BodyFat.date == date_)
        )

        values = {
            k: int(v) if v and int(v) else None
            for k, v in [
                ("chest", chest),
                ("abdominal", abdominal),
                ("tigh", tigh),
                ("tricep", tricep),
                ("subscapular", subscapular),
                ("suprailiac", suprailiac),
                ("midaxillary", midaxillary),
            ]
        }

        if any(values.values()):
            db.session.add(BodyFat(user_id=session["user_id"], date=date_, **values))

        db.session.commit()

    interval = parse_interval_args()
    df = storage.read_bodyfat(session["user_id"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        df = df.set_index("date")
        df = df[interval.first : interval.last]  # type: ignore
        df["fat3"] = (
            bodyfat.jackson_pollock_3_female(df)
            if session["sex"] == Sex.FEMALE
            else bodyfat.jackson_pollock_3_male(df)
        )
        df["fat7"] = (
            bodyfat.jackson_pollock_7_female(df)
            if session["sex"] == Sex.FEMALE
            else bodyfat.jackson_pollock_7_male(df)
        )

    return render_template(
        "bodyfat.html",
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        bodyfat=df.iloc[::-1].itertuples(),
        sites_3=["tricep", "suprailiac", "tigh"]
        if session["sex"] == Sex.FEMALE
        else ["chest", "abdominal", "tigh"],
        additional_sites_7=["chest", "abdominal", "subscapular", "midaxillary"]
        if session["sex"] == Sex.FEMALE
        else ["tricep", "subscapular", "suprailiac", "midaxillary"],
    )


@app.route("/period", methods=["GET", "POST"])
@login_required
def period_view() -> Union[str, Response]:
    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        try:
            intensity = int(request.form["intensity"])
            if not 0 <= intensity <= 4:
                raise ValueError()
        except ValueError:
            flash(f"Invalid intensity value {request.form['intensity']}")
        else:
            db.session.execute(
                delete(Period)
                .where(Period.user_id == session["user_id"])
                .where(Period.date == date_)
            )

            if intensity > 0:
                db.session.add(Period(user_id=session["user_id"], date=date_, intensity=intensity))

            db.session.commit()

    interval = parse_interval_args()
    df = storage.read_period(session["user_id"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        df = df.set_index("date")
        df = df[interval.first : interval.last]  # type: ignore

    period_list: deque[tuple[date, int]] = deque()
    for date_, intensity in df.itertuples():
        period_list.appendleft(
            (
                date_,
                int(intensity),
            )
        )

    return render_template(
        "period.html",
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        period=period_list,
    )


@app.route("/exercises", methods=["GET", "POST"])
@login_required
def exercises_view() -> Union[str, Response]:
    if request.method == "POST":
        return redirect(url_for("exercise_view", name=request.form["exercise"]))

    exercises = query.get_exercises()

    return render_template(
        "exercises.html",
        exercise_list=[e.name for e in sorted(exercises, key=lambda x: x.id, reverse=True)],
    )


@app.route("/exercise/<name>", methods=["GET", "POST"])
@login_required
def exercise_view(name: str) -> Union[str, Response]:
    interval = parse_interval_args()

    return render_template(
        "exercise.html",
        navbar_items=[
            ("Rename", url_for("exercise_rename_view", name=name)),
            ("Delete", url_for("exercise_delete_view", name=name)),
        ],
        exercise=name,
        current=interval,
        intervals=intervals(interval),
        workouts=create_workouts_list(interval, exercise_name=name),
        today=request.form.get("date", date.today()),
    )


@app.route("/exercise/<name>/rename", methods=["GET", "POST"])
@login_required
def exercise_rename_view(name: str) -> Union[str, Response]:
    exercise = query.get_exercise(name)

    if request.method == "POST":
        new_name = request.form["new_name"].strip()
        if new_name:
            exercise.name = new_name
            db.session.commit()
            return redirect(url_for("exercise_view", name=new_name))

    return render_template(
        "new_name.html",
        name=name,
        element="exercise",
        target=url_for("exercise_rename_view", name=name),
        button_text="Rename",
    )


@app.route("/exercise/<name>/delete", methods=["GET", "POST"])
@login_required
def exercise_delete_view(name: str) -> Union[str, Response]:
    exercise = query.get_exercise(name)

    if exercise.sets or exercise.routine_exercises:
        flash("Only exercises that are not used in any routine or workout can be deleted.")
        return redirect(url_for("exercise_view", name=name), Response=Response)

    if request.method == "POST":
        db.session.delete(exercise)
        db.session.commit()
        return redirect(url_for("exercises_view"))

    return render_template(
        "delete.html",
        name=name,
        element="exercise",
        delete_target=url_for("exercise_delete_view", name=name),
        cancel_target=url_for("exercise_view", name=name),
    )


@app.route("/routines", methods=["GET", "POST"])
@login_required
def routines_view() -> Union[str, Response]:
    if request.method == "POST":
        return redirect(
            url_for("routine_edit_view", name=request.form["name"].strip()),
            Response=Response,
            code=307,
        )

    routines = query.get_routines()

    return render_template(
        "routines.html",
        routines=[r.name for r in sorted(routines, key=lambda x: x.id, reverse=True)],
    )


@app.route("/routine/<name>", methods=["GET", "POST"])
@login_required
def routine_view(name: str) -> Union[str, Response]:
    name = name.strip()
    routine = query.get_routine(name)
    current_exercises = {e.exercise.name for e in routine.exercises}
    all_exercises = {s.exercise.name for w in routine.workouts for s in w.sets}

    return render_template(
        "routine.html",
        navbar_items=[
            ("Edit", url_for("routine_edit_view", name=name)),
        ],
        name=name,
        routine=[
            (e.position, e.exercise.name, e.sets)
            for e in sorted(routine.exercises, key=lambda x: x.position)
        ]
        if routine
        else [],
        notes=routine.notes if routine and routine.notes else "",
        previous_exercises=sorted(all_exercises - current_exercises),
        diagram=diagram.plot_svg(diagram.plot_workouts(session["user_id"], routine=routine)).decode(
            "utf-8"
        ),
        workouts=create_workouts_list(routine=routine),
    )


@app.route("/routine/<name>/edit", methods=["GET", "POST"])
@login_required
def routine_edit_view(name: str) -> Union[str, Response]:
    name = name.strip()

    if request.method == "POST":  # pylint: disable = too-many-nested-blocks
        routine = query.get_or_create_routine(name)

        if "exercise" in request.form:
            offset = 1

            for position, (routine_exercise, exercise_name, sets) in enumerate(
                zip_longest(
                    sorted(routine.exercises, key=lambda x: x.position),
                    request.form.getlist("exercise"),
                    [int(s) if s else 0 for s in request.form.getlist("set_count")],
                )
            ):
                if not routine_exercise and exercise_name and sets:
                    exercise = query.get_or_create_exercise(exercise_name)
                    routine.exercises.append(
                        RoutineExercise(position=position + offset, exercise=exercise, sets=sets)
                    )
                if routine_exercise:
                    if (
                        routine_exercise.position == position + offset
                        and routine_exercise.exercise.name == exercise_name
                        and routine_exercise.sets == sets
                    ):
                        continue
                    if not exercise_name or not sets:
                        db.session.delete(routine_exercise)
                        db.session.commit()
                        offset -= 1
                    else:
                        if routine_exercise.exercise.name != exercise_name:
                            db.session.delete(routine_exercise)
                            exercise = query.get_or_create_exercise(exercise_name)
                            routine.exercises.append(
                                RoutineExercise(
                                    position=position + offset, exercise=exercise, sets=sets
                                )
                            )
                        else:
                            routine_exercise.position = position + offset
                            routine_exercise.sets = sets

        if "notes" in request.form:
            routine.notes = request.form["notes"]

        db.session.commit()

    routine = query.get_routine(name)
    exercises = query.get_exercises()

    return render_template(
        "routine_edit.html",
        navbar_items=[
            ("Rename", url_for("routine_rename_view", name=name)),
            ("Copy", url_for("routine_copy_view", name=name)),
            ("Delete", url_for("routine_delete_view", name=name)),
        ],
        name=name,
        routine=[
            (e.position, e.exercise.name, e.sets)
            for e in sorted(routine.exercises, key=lambda x: x.position)
        ]
        if routine
        else [],
        notes=routine.notes if routine and routine.notes else "",
        exercises=[e.name for e in sorted(exercises, key=lambda x: x.id, reverse=True)],
    )


@app.route("/routine/<name>/rename", methods=["GET", "POST"])
@login_required
def routine_rename_view(name: str) -> Union[str, Response]:
    routine = query.get_routine(name)

    if request.method == "POST":
        new_name = request.form["new_name"].strip()
        if new_name:
            routine.name = new_name
            db.session.commit()
            return redirect(url_for("routine_view", name=new_name))

    return render_template(
        "new_name.html",
        name=name,
        element="routine",
        target=url_for("routine_rename_view", name=name),
        button_text="Rename",
    )


@app.route("/routine/<name>/copy", methods=["GET", "POST"])
@login_required
def routine_copy_view(name: str) -> Union[str, Response]:
    routine = query.get_routine(name)

    if request.method == "POST":
        new_name = request.form["new_name"].strip()
        if new_name and new_name != routine.name:
            db.session.add(
                Routine(
                    user_id=session["user_id"],
                    name=new_name,
                    notes=routine.notes,
                    exercises=[
                        RoutineExercise(
                            position=routine_exercise.position,
                            exercise_id=routine_exercise.exercise_id,
                            sets=routine_exercise.sets,
                        )
                        for routine_exercise in routine.exercises
                    ],
                )
            )
            db.session.commit()
            return redirect(url_for("routine_view", name=new_name))

    return render_template(
        "new_name.html",
        name=name,
        element="routine",
        target=url_for("routine_copy_view", name=name),
        button_text="Copy",
    )


@app.route("/routine/<name>/delete", methods=["GET", "POST"])
@login_required
def routine_delete_view(name: str) -> Union[str, Response]:
    if request.method == "POST":
        routine = query.get_routine(name)
        db.session.delete(routine)
        db.session.commit()
        return redirect(url_for("routines_view"))

    return render_template(
        "delete.html",
        name=name,
        element="routine",
        delete_target=url_for("routine_delete_view", name=name),
        cancel_target=url_for("routine_view", name=name),
    )


@app.route("/workouts", methods=["GET", "POST"])
@login_required
def workouts_view() -> Union[str, Response]:
    if request.method == "POST":
        if "routine" not in request.form:
            flash("A routine has to be defined before creating a workout.")
        else:
            date_ = date.fromisoformat(request.form["date"])
            routine_name = request.form["routine"]
            routine = query.get_routine(routine_name)

            workout = Workout(
                user_id=session["user_id"],
                routine=routine,
                date=date_,
                sets=[
                    WorkoutSet(position=position, exercise_id=routine_exercise.exercise_id)
                    for position, routine_exercise in enumerate(
                        (
                            routine_exercise
                            for routine_exercise in routine.exercises
                            for _ in range(routine_exercise.sets)
                        ),
                        start=1,
                    )
                ],
            )
            db.session.add(workout)
            db.session.commit()

            return redirect(
                url_for("workout_view", workout_id=workout.id),
                Response=Response,
            )

    interval = parse_interval_args()
    routines = query.get_routines()

    return render_template(
        "workouts.html",
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        routines=[r.name for r in sorted(routines, key=lambda x: x.id, reverse=True)],
        workouts=create_workouts_list(interval),
    )


@app.route("/workout/<int:workout_id>", methods=["GET", "POST"])
@login_required
def workout_view(workout_id: int) -> Union[str, Response]:
    # pylint: disable=too-many-locals

    if request.method == "POST":
        workout = query.get_workout(workout_id)

        for workout_set, (reps, time, weight, rpe) in zip_longest(
            sorted(workout.sets, key=lambda x: x.position),
            [values for input_name, values in request.form.lists() if input_name.startswith("set")],
        ):
            workout_set.reps = int(reps) if reps else None
            workout_set.time = int(time) if time else None
            workout_set.weight = float(weight) if weight else None
            workout_set.rpe = float(rpe) if rpe else None

        workout.notes = request.form.get("notes", "")

        db.session.commit()

    workout = query.get_workout(workout_id)
    df_s = storage.read_sets(session["user_id"])
    df_cur = df_s[df_s["workout_id"] == workout.id].groupby("exercise", sort=False)
    workout_data = []
    for ex, sets in df_cur:
        current = [
            set_tuple[1:]
            for set_tuple in sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous_date = df_s[(df_s["date"] < workout.date) & (df_s["exercise"] == ex)]["date"].max()
        previous_sets = df_s.loc[(df_s["date"] == previous_date) & (df_s["exercise"] == ex)]
        previous = [
            set_tuple[1:]
            for set_tuple in previous_sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
        ]
        previous = previous + [(np.nan, np.nan, np.nan, np.nan)] * (len(current) - len(previous))
        workout_data.append((ex, zip(current, previous)))

    return render_template(
        "workout.html",
        navbar_items=[("Delete", url_for("workout_delete_view", workout_id=workout_id))],
        workout_id=workout.id,
        routine=workout.routine,
        date=workout.date,
        workout=workout_data,
        notes=workout.notes if workout.notes else "",
    )


@app.route("/workout/<int:workout_id>/delete", methods=["GET", "POST"])
@login_required
def workout_delete_view(workout_id: int) -> Union[str, Response]:
    workout = query.get_workout(workout_id)

    if request.method == "POST":
        db.session.delete(workout)
        db.session.commit()
        return redirect(url_for("workouts_view"))

    return render_template(
        "delete.html",
        name=f"Workout on {workout.date}",
        element="workout",
        delete_target=url_for("workout_delete_view", workout_id=workout_id),
        cancel_target=url_for("workout_view", workout_id=workout_id),
    )


@app.route("/image/<image_type>")
@login_required
def image_view(image_type: str) -> Response:
    interval = parse_interval_args()
    if image_type == "bodyweight":
        fig = diagram.plot_bodyweight(session["user_id"], interval.first, interval.last)
    elif image_type == "bodyfat":
        fig = diagram.plot_bodyfat(session["user_id"], interval.first, interval.last)
    elif image_type == "period":
        fig = diagram.plot_period(session["user_id"], interval.first, interval.last)
    elif image_type == "workouts":
        fig = diagram.plot_workouts(session["user_id"], interval.first, interval.last)
    elif image_type.startswith("exercise"):
        name = request.args.get("name", "")
        fig = diagram.plot_exercise(session["user_id"], name, interval.first, interval.last)
    else:
        return make_response("", 404)
    return Response(diagram.plot_svg(fig), mimetype="image/svg+xml")


def parse_interval_args() -> Interval:
    args_first = request.args.get("first", "")
    args_last = request.args.get("last", "")
    first = date.fromisoformat(args_first) if args_first else date.today() - timedelta(days=30)
    last = date.fromisoformat(args_last) if args_last else date.today()
    if last <= first:
        first = last - timedelta(days=1)
    return Interval(first, last)


def intervals(current: Interval) -> Sequence[Tuple[str, date, date]]:
    today = date.today()
    interval = (current.last - current.first) + timedelta(days=2)
    return [
        ("1Y", today - timedelta(weeks=52), today),
        ("6M", today - timedelta(weeks=26), today),
        ("3M", today - timedelta(weeks=13), today),
        ("1M", today - timedelta(days=30), today),
        ("+", current.first + interval / 4, current.last - interval / 4),
        ("−", current.first - interval / 2, current.last + interval / 2),
        ("<", current.first - interval / 4, current.last - interval / 4),
        (">", current.first + interval / 4, current.last + interval / 4),
    ]


def days(td: timedelta) -> str:
    if td == timedelta(days=0):
        return "<strong>today</strong>"

    if td == timedelta(days=1):
        return "<strong>yesterday</strong>"

    return f"<strong>{td.days} days</strong> ago"


def create_workouts_list(  # pylint: disable = too-many-locals
    interval: Interval = None, routine: Routine = None, exercise_name: str = None
) -> Sequence[Tuple[int, date, str, str, str, str, str, str, str, str]]:
    df = storage.read_sets(session["user_id"])
    if interval:
        df = df[(df["date"] >= interval.first) & (df["date"] <= interval.last)]
    if routine:
        df = df[df["workout_id"].isin({w.id for w in routine.workouts})]
    if exercise_name:
        df = df.loc[lambda x: x["exercise"] == exercise_name]
    df["reps+rir"] = df["reps"] + df["rir"]
    df = df.drop("rir", 1)
    df["tut"] = df["reps"].replace(np.nan, 1) * df["time"]
    df_grouped = df.groupby([df.workout_id, df.date, df.routine.fillna("")])
    df_sum = df_grouped.sum()
    wo = df_grouped.mean()
    wo["tut"] = df_sum["tut"]
    wo["volume"] = df_sum["reps"]

    workouts_list = []
    for (
        (workout_id, wo_date, routine_name),
        reps,
        time,
        weight,
        rpe,
        reps_rir,
        tut,
        volume,
    ) in chain(
        wo.itertuples(),
        (
            (
                (workout.id, workout.date, workout.routine.name),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            for workout in query.get_workouts()
            if not workout.sets
        ),
    ):
        workouts_list.append(
            (
                workout_id,
                wo_date,
                routine_name,
                utils.format_number(reps),
                utils.format_number(time),
                utils.format_number(weight),
                utils.format_number(rpe),
                utils.format_number(reps_rir),
                utils.format_number(tut),
                utils.format_number(volume),
            )
        )

    return sorted(workouts_list, key=lambda x: (x[1], x[0]), reverse=True)
