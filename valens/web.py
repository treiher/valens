from collections import deque
from dataclasses import dataclass
from datetime import date, timedelta
from typing import Sequence, Tuple, Union

import numpy as np
import pandas as pd
from flask import (
    Flask,
    Response,
    make_response,
    redirect,
    render_template,
    request,
    session,
    url_for,
)

from valens import bodyfat, bodyweight, diagram, storage, utils

app = Flask(__name__)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True

app.secret_key = b"Q|6s:@}cC{>v:$,#"
app.permanent_session_lifetime = timedelta(weeks=52)


@dataclass
class Interval:
    first: date
    last: date


def is_logged_in() -> bool:
    return "username" in session and "user_id" in session and "sex" in session


@app.route("/")
def index_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

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
                if session["sex"] == utils.Sex.FEMALE
                else []
            ),
        ],
    )


@app.route("/login", methods=["GET", "POST"])
def login_view() -> Union[str, Response]:
    users = storage.read_users().set_index("user_id").itertuples()

    if request.method == "POST":
        for user_id, username, sex in users:
            if username == request.form["username"]:
                session["user_id"] = int(user_id)
                session["username"] = username
                session["sex"] = utils.Sex(sex)
                session.permanent = True
        return redirect(url_for("index_view"), Response=Response)

    return render_template("login.html", usernames=[n for _, n, _ in users])


@app.route("/logout")
def logout_view() -> Response:
    session.pop("username", None)
    return redirect(url_for("login_view"), Response=Response)


@app.route("/users", methods=["GET", "POST"])
def users_view() -> Union[str, Response]:
    df = storage.read_users().set_index("user_id")

    if request.method == "POST":
        form_user_ids = [int(i) for i in request.form.getlist("user_id")]
        assert len(form_user_ids) == len(set(form_user_ids)), "duplicate user id"
        form_usernames = request.form.getlist("username")
        assert len([n for n in form_usernames if n]) == len(
            {n for n in form_usernames if n}
        ), "duplicate username"
        form_sexes = [int(i) for i in request.form.getlist("sex")]
        next_user_id = max(form_user_ids) + 1 if len(form_user_ids) > 0 else 1
        users = [
            (user_id if user_id > 0 else next_user_id, name.strip(), sex)
            for user_id, name, sex in zip(form_user_ids, form_usernames, form_sexes)
            if name
        ]
        user_ids, usernames, sexes = zip(*users) if users else ([], [], [])
        df = pd.DataFrame({"user_id": user_ids, "name": usernames, "sex": sexes}).set_index(
            "user_id"
        )
        storage.write_users(df.reset_index())

    return render_template(
        "users.html",
        users=df.itertuples(),
    )


@app.route("/bodyweight", methods=["GET", "POST"])
def bodyweight_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        weight = float(request.form["weight"])

        df = storage.read_bodyweight(session["user_id"]).set_index("date")
        df.loc[date_] = weight
        if weight <= 0:
            df = df.drop(date_)
        df = df.sort_index()
        storage.write_bodyweight(df.reset_index(), session["user_id"])

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
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        bodyweight=bodyweight_list,
    )


@app.route("/bodyfat", methods=["GET", "POST"])
def bodyfat_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    notification = ""

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        chest = request.form["chest"]
        abdominal = request.form["abdominal"]
        tigh = request.form["tigh"]
        tricep = request.form["tricep"]
        subscapular = request.form["subscapular"]
        suprailiac = request.form["suprailiac"]
        midaxillary = request.form["midaxillary"]
        df = storage.read_bodyfat(session["user_id"]).set_index("date")
        df.loc[date_] = (
            int(chest) if chest else np.nan,
            int(abdominal) if abdominal else np.nan,
            int(tigh) if tigh else np.nan,
            int(tricep) if tricep else np.nan,
            int(subscapular) if subscapular else np.nan,
            int(suprailiac) if suprailiac else np.nan,
            int(midaxillary) if midaxillary else np.nan,
        )
        if not (chest or abdominal or tigh or tricep or subscapular or suprailiac or midaxillary):
            df = df.drop(date_)
        df = df.sort_index()
        storage.write_bodyfat(df.reset_index(), session["user_id"])

    interval = parse_interval_args()
    df = storage.read_bodyfat(session["user_id"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        df = df.set_index("date")
        df = df[interval.first : interval.last]  # type: ignore
        df["fat3"] = (
            bodyfat.jackson_pollock_3_female(df)
            if session["sex"] == utils.Sex.FEMALE
            else bodyfat.jackson_pollock_3_male(df)
        )
        df["fat7"] = (
            bodyfat.jackson_pollock_7_female(df)
            if session["sex"] == utils.Sex.FEMALE
            else bodyfat.jackson_pollock_7_male(df)
        )

    return render_template(
        "bodyfat.html",
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        bodyfat=df.iloc[::-1].itertuples(),
        sites_3=["tricep", "suprailiac", "tigh"]
        if session["sex"] == utils.Sex.FEMALE
        else ["chest", "abdominal", "tigh"],
        additional_sites_7=["chest", "abdominal", "subscapular", "midaxillary"]
        if session["sex"] == utils.Sex.FEMALE
        else ["tricep", "subscapular", "suprailiac", "midaxillary"],
        notification=notification,
    )


@app.route("/period", methods=["GET", "POST"])
def period_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    notification = ""

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        try:
            intensity = int(request.form["intensity"])
            if not 0 <= intensity <= 4:
                raise ValueError()
        except ValueError:
            notification = f"Invalid intensity value {request.form['intensity']}"
        else:
            df = storage.read_period(session["user_id"]).set_index("date")
            df.loc[date_] = intensity
            if intensity == 0:
                df = df.drop(date_)
            df = df.sort_index()
            storage.write_period(df.reset_index(), session["user_id"])

    interval = parse_interval_args()
    df = storage.read_period(session["user_id"])
    if not df.empty:
        df["date"] = pd.to_datetime(df["date"])
        df = df.set_index("date")
        df = df[interval.first : interval.last]  # type: ignore

    period_list: deque = deque()
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
        notification=notification,
    )


@app.route("/exercises")
def exercises_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    df = storage.read_sets(session["user_id"])
    exercise_list = df.sort_index(ascending=False).loc[:, "exercise"].unique()

    return render_template("exercises.html", exercise_list=exercise_list)


@app.route("/exercise/<name>", methods=["GET", "POST"])
def exercise_view(name: str) -> Union[str, Response]:  # pylint: disable=too-many-locals
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    interval = parse_interval_args()
    df = storage.read_sets(session["user_id"])

    if request.method == "POST":
        new_name = request.form["new_name"]
        storage.write_sets(df.replace(to_replace=name, value=new_name), session["user_id"])
        storage.write_routine_sets(
            storage.read_routine_sets(session["user_id"]).replace(to_replace=name, value=new_name),
            session["user_id"],
        )
        return redirect(
            url_for("exercise_view", name=new_name, first=interval.first, last=interval.last),
            Response=Response,
        )

    df["reps+rir"] = df["reps"] + df["rir"]
    df = df.loc[lambda x: x["exercise"] == name]
    df["tut"] = df["reps"].replace(np.nan, 1) * df["time"]
    df_sum = df.groupby(["date"]).sum()
    wo = df.groupby(["date"]).mean()
    wo["tut"] = df_sum["tut"]
    wo["volume"] = df_sum["reps"]
    wo = wo[interval.first : interval.last]  # type: ignore

    workouts_list: deque = deque()
    for wo_date, reps, time, weight, rpe, _, reps_rir, tut, volume in wo.itertuples():
        workouts_list.appendleft(
            (
                wo_date,
                utils.format_number(reps),
                utils.format_number(time),
                utils.format_number(weight),
                utils.format_number(rpe),
                utils.format_number(reps_rir),
                utils.format_number(tut),
                utils.format_number(volume),
            )
        )

    return render_template(
        "exercise.html",
        exercise=name,
        current=interval,
        intervals=intervals(interval),
        workouts=workouts_list,
        today=request.form.get("date", date.today()),
    )


@app.route("/routines", methods=["GET", "POST"])
def routines_view() -> Union[str, Response]:
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    if request.method == "POST":
        return redirect(
            url_for("routine_view", name=request.form["name"].strip()), Response=Response
        )

    df = storage.read_routine_sets(session["user_id"])
    routines = df.sort_index(ascending=False).loc[:, "routine"].unique()

    return render_template("routines.html", routines=routines)


@app.route("/routine/<name>", methods=["GET", "POST"])
def routine_view(name: str) -> Union[str, Response]:
    name = name.strip()

    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    df_s = storage.read_routine_sets(session["user_id"])
    df_r = storage.read_routines(session["user_id"])

    if request.method == "POST":
        df_s = df_s.loc[df_s["routine"] != name]
        df_r = df_r.loc[df_r["routine"] != name]

        if "delete" in request.form:
            storage.write_routine_sets(df_s, session["user_id"])
            storage.write_routines(df_r, session["user_id"])
            return redirect(url_for("routines_view"), Response=Response)

        for ex, set_count in zip(
            request.form.getlist("exercise"), request.form.getlist("set_count")
        ):
            ex = ex.strip()

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

        storage.write_routine_sets(df_s, session["user_id"])
        storage.write_routines(df_r, session["user_id"])

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

    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    notification = ""
    df_rs = storage.read_routine_sets(session["user_id"])
    df_r = storage.read_routines(session["user_id"])
    df_s = storage.read_sets(session["user_id"])
    df_w = storage.read_workouts(session["user_id"])

    if request.method == "POST":
        date_ = date.fromisoformat(request.form["date"])
        routine = request.form["routine"]
        df_routine = df_rs.loc[df_rs["routine"] == routine, df_rs.columns != "routine"]

        if not df_routine.empty:
            df_routine["date"] = date_

            if len(df_s[df_s["date"] == date_]) == 0:
                df_s = pd.concat([df_s[df_s["date"] != date_], df_routine])
                storage.write_sets(df_s, session["user_id"])

                df_r = df_r.loc[df_r["routine"] == routine]
                assert 0 <= len(df_r) <= 1
                if len(df_r) == 1:
                    df_w = df_w.loc[df_w["date"] != date_]
                    df_w = df_w.append({"date": date_, "notes": df_r.iat[0, 1]}, ignore_index=True)
                    storage.write_workouts(df_w, session["user_id"])

                return redirect(
                    url_for("workout_view", workout_date=request.form["date"]), Response=Response
                )

            notification = f"Workout on {date_} already exists"

        else:
            notification = f"Routine {routine} undefined"

    interval = parse_interval_args()
    df_s["reps+rir"] = df_s["reps"] + df_s["rir"]
    df_s = df_s[(df_s["date"] >= interval.first) & (df_s["date"] <= interval.last)].drop("rir", 1)
    wo = df_s.groupby(["date"]).mean()
    df_s["tut"] = df_s["reps"].replace(np.nan, 1) * df_s["time"]
    df_sum = df_s.groupby(["date"]).sum()
    wo["tut"] = df_sum["tut"]
    wo["volume"] = df_sum["reps"]

    workouts_list: deque = deque()
    for wo_date, reps, time, weight, rpe, reps_rir, tut, volume in wo.itertuples():
        workouts_list.appendleft(
            (
                wo_date,
                utils.format_number(reps),
                utils.format_number(time),
                utils.format_number(weight),
                utils.format_number(rpe),
                utils.format_number(reps_rir),
                utils.format_number(tut),
                utils.format_number(volume),
            )
        )

    routines = reversed([r for r, _ in df_rs.groupby("routine", sort=False)])

    return render_template(
        "workouts.html",
        current=interval,
        intervals=intervals(interval),
        today=request.form.get("date", date.today()),
        routines=routines,
        workouts=workouts_list,
        notification=notification,
    )


@app.route("/workout/<workout_date>", methods=["GET", "POST"])
def workout_view(workout_date: str) -> Union[str, Response]:
    # pylint: disable=too-many-locals

    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

    notification = ""
    date_ = date.fromisoformat(workout_date)
    df_s = storage.read_sets(session["user_id"])
    df_w = storage.read_workouts(session["user_id"])

    if request.method == "POST":
        df_s = df_s.loc[df_s["date"] != date_]
        df_w = df_w.loc[df_w["date"] != date_]

        if "delete" in request.form:
            storage.write_sets(df_s, session["user_id"])
            storage.write_workouts(df_w, session["user_id"])
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

            storage.write_sets(df_s, session["user_id"])
            storage.write_workouts(df_w, session["user_id"])
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
    if not is_logged_in():
        return redirect(url_for("login_view"), Response=Response)

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
