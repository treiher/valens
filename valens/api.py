from __future__ import annotations

from datetime import date
from functools import wraps
from http import HTTPStatus
from itertools import chain
from typing import Any, Callable

from flask import Blueprint, jsonify, request, session
from flask.typing import ResponseReturnValue
from sqlalchemy import column, select
from sqlalchemy.exc import IntegrityError, NoResultFound

from valens import database as db, version
from valens.models import (
    BodyFat,
    BodyWeight,
    Exercise,
    Period,
    Routine,
    RoutineExercise,
    Sex,
    User,
    Workout,
    WorkoutSet,
)

bp = Blueprint("api", __name__, url_prefix="/api")


class DeserializationError(Exception):
    pass


def model_to_dict(
    model: object, exclude: list[str] = None, include: list[str] = None
) -> dict[str, object]:
    assert hasattr(model, "__table__")
    exclude = ["user_id"] if exclude is None else exclude
    include = [] if include is None else include
    return {
        name: attr.isoformat() if isinstance(attr, date) else attr
        for col in chain(getattr(model, "__table__").columns, (column(i) for i in include))
        if col.name not in exclude
        for name, attr in [(col.name, getattr(model, col.name))]
    }


def to_routine_exercises(json: list[dict[str, Any]]) -> list[RoutineExercise]:  # type: ignore[misc]
    exercises = [
        RoutineExercise(
            position=exercise["position"],
            exercise_id=exercise["exercise_id"],
            sets=exercise["sets"],
        )
        for exercise in json
    ]

    if sorted(e.position for e in exercises) != list(range(1, len(exercises) + 1)):
        raise DeserializationError(
            "exercise positions must be in ascending order without gaps, starting with 1"
        )

    return exercises


def to_workout_sets(json: list[dict[str, Any]]) -> list[WorkoutSet]:  # type: ignore[misc]
    sets = [
        WorkoutSet(
            position=workout_set["position"],
            exercise_id=workout_set["exercise_id"],
            reps=workout_set["reps"],
            time=workout_set["time"],
            weight=workout_set["weight"],
            rpe=workout_set["rpe"],
        )
        for workout_set in json
    ]

    if sorted(e.position for e in sets) != list(range(1, len(sets) + 1)):
        raise DeserializationError(
            "workout set positions must be in ascending order without gaps, starting with 1"
        )

    return sets


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
    # ISSUE: PyCQA/pylint#3793
    session.permanent = True

    return jsonify(model_to_dict(user))


@bp.route("/session", methods=["DELETE"])
def delete_session() -> ResponseReturnValue:
    session.clear()
    return "", HTTPStatus.NO_CONTENT


@bp.route("/users")
def read_users() -> ResponseReturnValue:
    users = db.session.execute(select(User)).scalars().all()
    return jsonify([model_to_dict(u) for u in users])


@bp.route("/users/<int:user_id>")
@session_required
def read_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    return jsonify(model_to_dict(user))


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
        jsonify(model_to_dict(user)),
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

    return jsonify(model_to_dict(user)), HTTPStatus.OK


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
    return jsonify([model_to_dict(bw) for bw in body_weight])


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
        jsonify(model_to_dict(body_weight)),
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
        jsonify(model_to_dict(body_weight)),
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
    return jsonify([model_to_dict(bf) for bf in body_fat])


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
        jsonify(model_to_dict(body_fat)),
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
        jsonify(model_to_dict(body_fat)),
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
    return jsonify([model_to_dict(p) for p in period])


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
        jsonify(model_to_dict(period)),
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
        jsonify(model_to_dict(period)),
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
    return jsonify([model_to_dict(e) for e in exercises])


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
        )
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(exercise)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(model_to_dict(exercise)),
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
    except (KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(model_to_dict(exercise)),
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
        db.session.execute(select(Routine).where(Routine.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify(
        [
            {
                **model_to_dict(r),
                "exercises": [{**model_to_dict(e, exclude=["routine_id"])} for e in r.exercises],
            }
            for r in routines
        ]
    )


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
            exercises=to_routine_exercises(data["exercises"]),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(routine)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(
            {
                **model_to_dict(routine),
                "exercises": [
                    {**model_to_dict(e, exclude=["routine_id"])} for e in routine.exercises
                ],
            }
        ),
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
        if "exercises" in data or request.method == "PUT":
            routine.exercises = to_routine_exercises(data["exercises"])
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify(
            {
                **model_to_dict(routine),
                "exercises": [model_to_dict(e, exclude=["routine_id"]) for e in routine.exercises],
            }
        ),
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
        db.session.execute(select(Workout).where(Workout.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify(
        [
            {**model_to_dict(w), "sets": [model_to_dict(s, exclude=["workout_id"]) for s in w.sets]}
            for w in workouts
        ]
    )


@bp.route("/workouts", methods=["POST"])
@session_required
@json_expected
def create_workout() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        routine = (
            db.session.execute(
                select(Routine)
                .where(Routine.user_id == session["user_id"])
                .where(Routine.id == data["routine_id"])
            )
            .scalars()
            .one()
        )

        workout = Workout(
            user_id=session["user_id"],
            routine=routine,
            date=date.fromisoformat(data["date"]),
            notes=data["notes"],
            sets=to_workout_sets(data["sets"]),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(workout)

    db.session.commit()

    return (
        jsonify(
            {
                **model_to_dict(workout),
                "sets": [model_to_dict(e, exclude=["workout_id"]) for e in workout.sets],
            }
        ),
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

    try:
        if "date" in data or request.method == "PUT":
            workout.date = date.fromisoformat(data["date"])
        if "notes" in data or request.method == "PUT":
            workout.notes = data["notes"]
        if "sets" in data or request.method == "PUT":
            workout.sets = to_workout_sets(data["sets"])
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.commit()

    return (
        jsonify(
            {
                **model_to_dict(workout),
                "sets": [model_to_dict(e, exclude=["workout_id"]) for e in workout.sets],
            }
        ),
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
