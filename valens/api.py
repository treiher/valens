from dataclasses import dataclass
from datetime import date, timedelta
from functools import wraps
from http import HTTPStatus
from typing import Callable

import numpy as np
from flask import Blueprint, Response, jsonify, request, session
from flask.typing import ResponseReturnValue
from sqlalchemy import select
from sqlalchemy.exc import IntegrityError, NoResultFound

from valens import bodyfat, bodyweight, database as db, diagram, storage, version
from valens.models import BodyFat, BodyWeight, Sex, User

bp = Blueprint("api", __name__, url_prefix="/api")


def json_expected(function: Callable) -> Callable:  # type: ignore[type-arg]
    @wraps(function)
    def decorated_function(*args: object, **kwargs: object) -> ResponseReturnValue:
        if not request.is_json:
            return "", HTTPStatus.UNSUPPORTED_MEDIA_TYPE
        return function(*args, **kwargs)

    return decorated_function


def session_required(function: Callable) -> Callable:  # type: ignore[type-arg]
    @wraps(function)
    def decorated_function(*args: object, **kwargs: object) -> ResponseReturnValue:
        if "username" not in session or "user_id" not in session or "sex" not in session:
            return "", HTTPStatus.UNAUTHORIZED
        return function(*args, **kwargs)

    return decorated_function


@bp.route("/version")
def get_version() -> ResponseReturnValue:
    return jsonify(version.get())


@bp.route("/session")
def get_session() -> ResponseReturnValue:
    if "username" not in session or "user_id" not in session or "sex" not in session:
        return "", HTTPStatus.NOT_FOUND

    return jsonify({"id": session["user_id"], "name": session["username"], "sex": session["sex"]})


@bp.route("/session", methods=["POST"])
@json_expected
def add_session() -> ResponseReturnValue:
    try:
        assert isinstance(request.json, dict)
        user_id = request.json["id"]
    except KeyError as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    session["user_id"] = user.id
    session["username"] = user.name
    session["sex"] = user.sex
    # ISSUE: PyCQA/pylint#3793
    session.permanent = True

    return jsonify({"id": user.id, "name": user.name, "sex": user.sex})


@bp.route("/session", methods=["DELETE"])
def delete_session() -> ResponseReturnValue:
    session.clear()
    return "", HTTPStatus.NO_CONTENT


@bp.route("/users")
def get_users() -> ResponseReturnValue:
    users = db.session.execute(select(User)).scalars().all()
    return jsonify([{"id": u.id, "name": u.name, "sex": u.sex} for u in users])


@bp.route("/users/<int:user_id>")
@session_required
def get_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    return jsonify({"id": user.id, "name": user.name, "sex": user.sex})


@bp.route("/users", methods=["POST"])
@json_expected
def add_user() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        user = User(name=data["name"].strip(), sex=Sex(data["sex"]))
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(user)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify({"id": user.id, "name": user.name, "sex": user.sex}),
        HTTPStatus.CREATED,
        {"Location": f"/users/{user.id}"},
    )


@bp.route("/users/<int:user_id>", methods=["PUT"])
@json_expected
def edit_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        user.name = data["name"].strip()
        user.sex = Sex(data["sex"])
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return jsonify({"id": user.id, "name": user.name, "sex": user.sex}), HTTPStatus.OK


@bp.route("/users/<int:user_id>", methods=["DELETE"])
def delete_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(user)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/body_weight")
@session_required
def get_body_weight() -> ResponseReturnValue:
    if request.args.get("format", None) == "statistics":
        df = storage.read_bodyweight(session["user_id"])

        if df.empty:
            return jsonify([])

        df = df.set_index("date")
        df["avg_weight"] = bodyweight.avg_weight(df)
        df["avg_weight_change"] = bodyweight.avg_weight_change(df)
        df.reset_index(inplace=True)
        df["date"] = df["date"].apply(lambda x: x.isoformat())

        return jsonify(df.replace([np.nan], [None]).to_dict(orient="records"))

    body_weight = (
        db.session.execute(select(BodyWeight).where(BodyWeight.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify([{"date": bw.date.isoformat(), "weight": bw.weight} for bw in body_weight])


@bp.route("/body_weight", methods=["POST"])
@session_required
@json_expected
def add_body_weight() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        body_weight = BodyWeight(
            user_id=session["user_id"],
            date=date.fromisoformat(data["date"]),
            weight=float(data["weight"]),
        )
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(body_weight)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify({"date": body_weight.date.isoformat(), "weight": body_weight.weight}),
        HTTPStatus.CREATED,
        {"Location": f"/body_weight/{body_weight.date}"},
    )


@bp.route("/body_weight/<date_>", methods=["PUT"])
@session_required
@json_expected
def edit_body_weight(date_: str) -> ResponseReturnValue:
    try:
        body_weight = (
            db.session.execute(
                select(BodyWeight)
                .where(BodyWeight.user_id == session["user_id"])
                .where(BodyWeight.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        body_weight.weight = float(data["weight"])
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify({"date": body_weight.date.isoformat(), "weight": body_weight.weight}),
        HTTPStatus.OK,
    )


@bp.route("/body_weight/<date_>", methods=["DELETE"])
@session_required
def delete_body_weight(date_: str) -> ResponseReturnValue:
    try:
        body_weight = (
            db.session.execute(
                select(BodyWeight)
                .where(BodyWeight.user_id == session["user_id"])
                .where(BodyWeight.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(body_weight)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/body_fat")
@session_required
def get_body_fat() -> ResponseReturnValue:
    if request.args.get("format", None) == "statistics":
        df = storage.read_bodyfat(session["user_id"])

        if df.empty:
            return jsonify([])

        df["date"] = df["date"].apply(lambda x: x.isoformat())
        df["jp3"] = (
            bodyfat.jackson_pollock_3_female(df)
            if session["sex"] == Sex.FEMALE
            else bodyfat.jackson_pollock_3_male(df)
        )
        df["jp7"] = (
            bodyfat.jackson_pollock_7_female(df)
            if session["sex"] == Sex.FEMALE
            else bodyfat.jackson_pollock_7_male(df)
        )

        return jsonify(df.replace([np.nan], [None]).to_dict(orient="records"))

    body_fat = (
        db.session.execute(select(BodyFat).where(BodyFat.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify(
        [
            {
                "date": bf.date.isoformat(),
                "chest": bf.chest,
                "abdominal": bf.abdominal,
                "tigh": bf.tigh,
                "tricep": bf.tricep,
                "subscapular": bf.subscapular,
                "suprailiac": bf.suprailiac,
                "midaxillary": bf.midaxillary,
            }
            for bf in body_fat
        ]
    )


@bp.route("/body_fat", methods=["POST"])
@session_required
@json_expected
def add_body_fat() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        body_fat = BodyFat(
            user_id=int(session["user_id"]),
            date=date.fromisoformat(data["date"]),
            **{
                part: int(data[part]) if data[part] is not None else None
                for part in [
                    "chest",
                    "abdominal",
                    "tigh",
                    "tricep",
                    "subscapular",
                    "suprailiac",
                    "midaxillary",
                ]
            },
        )
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(body_fat)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(
            {
                "date": body_fat.date.isoformat(),
                "chest": body_fat.chest,
                "abdominal": body_fat.abdominal,
                "tigh": body_fat.tigh,
                "tricep": body_fat.tricep,
                "subscapular": body_fat.subscapular,
                "suprailiac": body_fat.suprailiac,
                "midaxillary": body_fat.midaxillary,
            }
        ),
        HTTPStatus.CREATED,
        {"Location": f"/body_fat/{body_fat.date}"},
    )


@bp.route("/body_fat/<date_>", methods=["PUT"])
@session_required
@json_expected
def edit_body_fat(date_: str) -> ResponseReturnValue:
    try:
        body_fat = (
            db.session.execute(
                select(BodyFat)
                .where(BodyFat.user_id == session["user_id"])
                .where(BodyFat.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        for attr in [
            "chest",
            "abdominal",
            "tigh",
            "tricep",
            "subscapular",
            "suprailiac",
            "midaxillary",
        ]:
            setattr(body_fat, attr, int(data[attr]) if data[attr] is not None else None)
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(
            {
                "date": body_fat.date.isoformat(),
                "chest": body_fat.chest,
                "abdominal": body_fat.abdominal,
                "tigh": body_fat.tigh,
                "tricep": body_fat.tricep,
                "subscapular": body_fat.subscapular,
                "suprailiac": body_fat.suprailiac,
                "midaxillary": body_fat.midaxillary,
            }
        ),
        HTTPStatus.OK,
    )


@bp.route("/body_fat/<date_>", methods=["DELETE"])
@session_required
def delete_body_fat(date_: str) -> ResponseReturnValue:
    try:
        body_fat = (
            db.session.execute(
                select(BodyFat)
                .where(BodyFat.user_id == session["user_id"])
                .where(BodyFat.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(body_fat)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/images/<kind>")
@session_required
def get_images(kind: str) -> ResponseReturnValue:
    try:
        interval = _parse_interval_args()
    except ValueError as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if kind == "bodyweight":
        fig = diagram.plot_bodyweight(session["user_id"], interval.first, interval.last)
    elif kind == "bodyfat":
        fig = diagram.plot_bodyfat(session["user_id"], interval.first, interval.last)
    elif kind == "period":
        fig = diagram.plot_period(session["user_id"], interval.first, interval.last)
    elif kind == "workouts":
        fig = diagram.plot_workouts(session["user_id"], interval.first, interval.last)
    elif kind.startswith("exercise"):
        name = request.args.get("name", "")
        fig = diagram.plot_exercise(session["user_id"], name, interval.first, interval.last)
    else:
        return "", HTTPStatus.NOT_FOUND

    return Response(diagram.plot_svg(fig), mimetype="image/svg+xml")


@dataclass
class _Interval:
    first: date
    last: date


def _parse_interval_args() -> _Interval:
    args_first = request.args.get("first", "")
    args_last = request.args.get("last", "")
    first = date.fromisoformat(args_first) if args_first else date.today() - timedelta(days=30)
    last = date.fromisoformat(args_last) if args_last else date.today()

    if last <= first:
        first = last - timedelta(days=1)

    return _Interval(first, last)
