from __future__ import annotations

from collections.abc import Callable
from datetime import date
from functools import singledispatch, wraps
from http import HTTPStatus
from itertools import chain
from typing import Any

from flask import Blueprint, jsonify, request, session
from flask.typing import ResponseReturnValue
from sqlalchemy import column, select
from sqlalchemy.exc import IntegrityError, NoResultFound
from sqlalchemy.orm import selectinload

from valens import database as db, version
from valens.models import (
    BodyFat,
    BodyWeight,
    Exercise,
    ExerciseMuscle,
    Period,
    Routine,
    RoutineActivity,
    RoutinePart,
    RoutineSection,
    Sex,
    User,
    Workout,
    WorkoutElement,
    WorkoutRest,
    WorkoutSet,
)

bp = Blueprint("api", __name__, url_prefix="/api")


class DeserializationError(Exception):
    pass


@singledispatch
def to_dict(
    model: object, exclude: list[str] | None = None, include: list[str] | None = None
) -> dict[str, object]:
    return model_to_dict(model, exclude, include)


@to_dict.register
def _(model: Exercise) -> dict[str, object]:
    return {
        **model_to_dict(model),
        "muscles": [
            to_dict(m, exclude=["user_id", "exercise_id"])
            for m in sorted(model.muscles, key=lambda x: x.muscle_id)
        ],
    }


@to_dict.register
def _(model: Routine) -> dict[str, object]:
    return {
        **model_to_dict(model),
        "sections": [to_dict(s) for s in sorted(model.sections, key=lambda x: x.position)],
    }


@to_dict.register
def _(model: RoutineSection) -> dict[str, object]:
    return {
        **model_to_dict(model, exclude=["id", "routine_id"]),
        "parts": [to_dict(p) for p in sorted(model.parts, key=lambda x: x.position)],
    }


@to_dict.register
def _(model: RoutineActivity) -> dict[str, object]:
    return {
        **model_to_dict(model, exclude=["id"]),
    }


@to_dict.register
def _(model: Workout) -> dict[str, object]:
    return {
        **model_to_dict(model),
        "elements": [to_dict(e) for e in model.elements],
    }


@to_dict.register
def _(model: WorkoutElement) -> dict[str, object]:
    return {
        **model_to_dict(model, exclude=["workout_id", "position"], include=["automatic"]),
    }


def model_to_dict(
    model: object, exclude: list[str] | None = None, include: list[str] | None = None
) -> dict[str, object]:
    assert hasattr(model, "__table__")
    exclude = ["user_id"] if exclude is None else exclude
    include = [] if include is None else include
    return {
        name: attr.isoformat() if isinstance(attr, date) else attr
        for col in chain(model.__table__.columns, (column(i) for i in include))
        if col.name not in exclude
        for name, attr in [(col.name, getattr(model, col.name))]
    }


def to_routine_parts(json: list[dict[str, Any]]) -> list[RoutinePart]:  # type: ignore[explicit-any]
    return [
        (
            to_routine_section(part, position)
            if "rounds" in part
            else to_routine_activity(part, position)
        )
        for position, part in enumerate(json, start=1)
    ]


def to_routine_sections(json: list[dict[str, Any]]) -> list[RoutineSection]:  # type: ignore[explicit-any]
    return [to_routine_section(section, position) for position, section in enumerate(json, start=1)]


def to_routine_section(json: dict[str, Any], position: int) -> RoutineSection:  # type: ignore[explicit-any]
    return RoutineSection(
        position=position,
        rounds=json["rounds"],
        parts=to_routine_parts(json["parts"]),
    )


def to_routine_activity(  # type: ignore[explicit-any]
    json: dict[str, Any], position: int
) -> RoutineActivity:
    return RoutineActivity(
        position=position,
        exercise_id=json["exercise_id"],
        reps=json["reps"],
        time=json["time"],
        weight=json["weight"],
        rpe=json["rpe"],
        automatic=json["automatic"],
    )


def to_workout_elements(json: list[dict[str, Any]]) -> list[WorkoutElement]:  # type: ignore[explicit-any]
    return [
        (
            WorkoutSet(
                position=position,
                exercise_id=element["exercise_id"],
                reps=element["reps"],
                time=element["time"],
                weight=element["weight"],
                rpe=element["rpe"],
                target_reps=element["target_reps"],
                target_time=element["target_time"],
                target_weight=element["target_weight"],
                target_rpe=element["target_rpe"],
                automatic=element["automatic"],
            )
            if "exercise_id" in element
            else WorkoutRest(
                position=position,
                target_time=element["target_time"],
                automatic=element["automatic"],
            )
        )
        for position, element in enumerate(json, start=1)
    ]


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
def read_version() -> ResponseReturnValue:
    return jsonify(version.get())


@bp.route("/session")
def read_session() -> ResponseReturnValue:
    if "username" not in session or "user_id" not in session or "sex" not in session:
        return "", HTTPStatus.NOT_FOUND

    return jsonify({"id": session["user_id"], "name": session["username"], "sex": session["sex"]})


@bp.route("/session", methods=["POST"])
@json_expected
def create_session() -> ResponseReturnValue:
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
    session.permanent = True

    return jsonify(to_dict(user))


@bp.route("/session", methods=["DELETE"])
def delete_session() -> ResponseReturnValue:
    session.clear()
    return "", HTTPStatus.NO_CONTENT


@bp.route("/users")
def read_users() -> ResponseReturnValue:
    users = db.session.execute(select(User)).scalars().all()
    return jsonify([to_dict(u) for u in users])


@bp.route("/users/<int:user_id>")
@session_required
def read_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    return jsonify(to_dict(user))


@bp.route("/users", methods=["POST"])
@json_expected
def create_user() -> ResponseReturnValue:
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
        jsonify(to_dict(user)),
        HTTPStatus.CREATED,
        {"Location": f"/users/{user.id}"},
    )


@bp.route("/users/<int:user_id>", methods=["PUT"])
@json_expected
def replace_user(user_id: int) -> ResponseReturnValue:
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

    return jsonify(to_dict(user)), HTTPStatus.OK


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
def read_body_weight() -> ResponseReturnValue:
    body_weight = (
        db.session.execute(select(BodyWeight).where(BodyWeight.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify([to_dict(bw) for bw in body_weight])


@bp.route("/body_weight", methods=["POST"])
@session_required
@json_expected
def create_body_weight() -> ResponseReturnValue:
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
        jsonify(to_dict(body_weight)),
        HTTPStatus.CREATED,
        {"Location": f"/body_weight/{body_weight.date}"},
    )


@bp.route("/body_weight/<date_>", methods=["PUT"])
@session_required
@json_expected
def replace_body_weight(date_: str) -> ResponseReturnValue:
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
        jsonify(to_dict(body_weight)),
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
def read_body_fat() -> ResponseReturnValue:
    body_fat = (
        db.session.execute(select(BodyFat).where(BodyFat.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify([to_dict(bf) for bf in body_fat])


@bp.route("/body_fat", methods=["POST"])
@session_required
@json_expected
def create_body_fat() -> ResponseReturnValue:
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
                    "thigh",
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
        jsonify(to_dict(body_fat)),
        HTTPStatus.CREATED,
        {"Location": f"/body_fat/{body_fat.date}"},
    )


@bp.route("/body_fat/<date_>", methods=["PUT"])
@session_required
@json_expected
def replace_body_fat(date_: str) -> ResponseReturnValue:
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
            "thigh",
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
        jsonify(to_dict(body_fat)),
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


@bp.route("/period")
@session_required
def read_period() -> ResponseReturnValue:
    period = (
        db.session.execute(select(Period).where(Period.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify([to_dict(p) for p in period])


@bp.route("/period", methods=["POST"])
@session_required
@json_expected
def create_period() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        period = Period(
            user_id=session["user_id"],
            date=date.fromisoformat(data["date"]),
            intensity=int(data["intensity"]),
        )
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(period)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(period)),
        HTTPStatus.CREATED,
        {"Location": f"/period/{period.date}"},
    )


@bp.route("/period/<date_>", methods=["PUT"])
@session_required
@json_expected
def replace_period(date_: str) -> ResponseReturnValue:
    try:
        period = (
            db.session.execute(
                select(Period)
                .where(Period.user_id == session["user_id"])
                .where(Period.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        period.intensity = int(data["intensity"])
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(period)),
        HTTPStatus.OK,
    )


@bp.route("/period/<date_>", methods=["DELETE"])
@session_required
def delete_period(date_: str) -> ResponseReturnValue:
    try:
        period = (
            db.session.execute(
                select(Period)
                .where(Period.user_id == session["user_id"])
                .where(Period.date == date.fromisoformat(date_))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(period)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/exercises")
@session_required
def read_exercises() -> ResponseReturnValue:
    exercises = (
        db.session.execute(select(Exercise).where(Exercise.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify([to_dict(e) for e in exercises])


@bp.route("/exercises", methods=["POST"])
@session_required
@json_expected
def create_exercise() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        exercise = Exercise(
            user_id=session["user_id"],
            name=data["name"],
            muscles=[
                ExerciseMuscle(
                    user_id=session["user_id"],
                    muscle_id=muscle["muscle_id"],
                    stimulus=muscle["stimulus"],
                )
                for muscle in data["muscles"]
            ],
        )
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(exercise)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(exercise)),
        HTTPStatus.CREATED,
        {"Location": f"/exercises/{exercise.id}"},
    )


@bp.route("/exercises/<int:id_>", methods=["PUT"])
@session_required
@json_expected
def replace_exercise(id_: int) -> ResponseReturnValue:
    try:
        exercise = (
            db.session.execute(
                select(Exercise)
                .where(Exercise.id == id_)
                .where(Exercise.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        exercise.name = data["name"]
        muscle_stimulus = {m["muscle_id"]: m["stimulus"] for m in data["muscles"]}

        for m in exercise.muscles:
            if m.muscle_id in muscle_stimulus:
                m.stimulus = muscle_stimulus[m.muscle_id]
            else:
                db.session.delete(m)

        for muscle_id, stimulus in muscle_stimulus.items():
            if any(m.muscle_id == muscle_id for m in exercise.muscles):
                continue
            exercise.muscles.append(
                ExerciseMuscle(user_id=session["user_id"], muscle_id=muscle_id, stimulus=stimulus)
            )
    except (KeyError, ValueError, TypeError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(exercise)),
        HTTPStatus.OK,
    )


@bp.route("/exercises/<int:id_>", methods=["DELETE"])
@session_required
def delete_exercise(id_: int) -> ResponseReturnValue:
    try:
        exercise = (
            db.session.execute(
                select(Exercise)
                .where(Exercise.id == id_)
                .where(Exercise.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(exercise)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/routines")
@session_required
def read_routines() -> ResponseReturnValue:
    routines = (
        db.session.execute(
            select(Routine)
            .where(Routine.user_id == session["user_id"])
            .options(selectinload(Routine.sections))
        )
        .scalars()
        .all()
    )
    return jsonify([to_dict(r) for r in routines])


@bp.route("/routines", methods=["POST"])
@session_required
@json_expected
def create_routine() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        routine = Routine(
            user_id=session["user_id"],
            name=data["name"],
            notes=data["notes"],
            archived=data["archived"],
            sections=to_routine_sections(data["sections"]),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(routine)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(routine)),
        HTTPStatus.CREATED,
        {"Location": f"/routines/{routine.id}"},
    )


@bp.route("/routines/<int:id_>", methods=["PUT", "PATCH"])
@session_required
@json_expected
def update_routine(id_: int) -> ResponseReturnValue:
    try:
        routine = (
            db.session.execute(
                select(Routine)
                .where(Routine.id == id_)
                .where(Routine.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        if "name" in data or request.method == "PUT":
            routine.name = data["name"]
        if "notes" in data or request.method == "PUT":
            routine.notes = data["notes"]
        if "archived" in data or request.method == "PUT":
            routine.archived = data["archived"]
        if "sections" in data or request.method == "PUT":
            routine.sections = to_routine_sections(data["sections"])
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(routine)),
        HTTPStatus.OK,
    )


@bp.route("/routines/<int:id_>", methods=["DELETE"])
@session_required
def delete_routine(id_: int) -> ResponseReturnValue:
    try:
        routine = (
            db.session.execute(
                select(Routine)
                .where(Routine.id == id_)
                .where(Routine.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(routine)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/workouts")
@session_required
def read_workouts() -> ResponseReturnValue:
    workouts = (
        db.session.execute(
            select(Workout)
            .where(Workout.user_id == session["user_id"])
            .options(selectinload(Workout.elements))
        )
        .scalars()
        .all()
    )
    return jsonify([to_dict(w) for w in workouts])


@bp.route("/workouts", methods=["POST"])
@session_required
@json_expected
def create_workout() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        routine = (
            (
                db.session.execute(
                    select(Routine)
                    .where(Routine.user_id == session["user_id"])
                    .where(Routine.id == data["routine_id"])
                )
                .scalars()
                .one()
            )
            if isinstance(data["routine_id"], int)
            else None
        )

        workout = Workout(
            user_id=session["user_id"],
            routine=routine,
            date=date.fromisoformat(data["date"]),
            notes=data["notes"],
            elements=to_workout_elements(data["elements"]),
        )
    except (DeserializationError, NoResultFound, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(workout)

    db.session.commit()

    return (
        jsonify(to_dict(workout)),
        HTTPStatus.CREATED,
        {"Location": f"/workouts/{workout.id}"},
    )


@bp.route("/workouts/<int:id_>", methods=["PUT", "PATCH"])
@session_required
@json_expected
def update_workout(id_: int) -> ResponseReturnValue:
    try:
        workout = (
            db.session.execute(
                select(Workout)
                .where(Workout.id == id_)
                .where(Workout.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    if "elements" in data or request.method == "PUT":
        for e in workout.elements:
            db.session.delete(e)

        db.session.flush()

    try:
        if "date" in data or request.method == "PUT":
            workout.date = date.fromisoformat(data["date"])
        if "notes" in data or request.method == "PUT":
            workout.notes = data["notes"]
        if "elements" in data or request.method == "PUT":
            workout.elements = to_workout_elements(data["elements"])
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.commit()

    return (
        jsonify(to_dict(workout)),
        HTTPStatus.OK,
    )


@bp.route("/workouts/<int:id_>", methods=["DELETE"])
@session_required
def delete_workout(id_: int) -> ResponseReturnValue:
    try:
        workout = (
            db.session.execute(
                select(Workout)
                .where(Workout.id == id_)
                .where(Workout.user_id == session["user_id"])
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(workout)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT
