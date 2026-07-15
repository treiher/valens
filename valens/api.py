from __future__ import annotations

import math
from collections.abc import Callable, Iterable
from datetime import date
from functools import cache, singledispatch, wraps
from http import HTTPStatus
from typing import Any

from flask import Blueprint, jsonify, make_response, request, session
from flask.typing import ResponseReturnValue
from sqlalchemy import Table, delete, select
from sqlalchemy.dialects.sqlite import insert as sqlite_insert
from sqlalchemy.exc import IntegrityError, NoResultFound
from sqlalchemy.orm import selectinload

from valens import database as db, version
from valens.models import (
    BodyFat,
    BodyWeight,
    DataVersion,
    Exercise,
    ExerciseMuscle,
    Period,
    Role,
    Routine,
    RoutineActivity,
    RoutinePart,
    RoutineSection,
    ScheduleRotation,
    ScheduleRotationRoutine,
    ScheduleSlot,
    Sex,
    User,
    Workout,
    WorkoutElement,
    WorkoutExerciseNote,
    WorkoutRest,
    WorkoutSet,
)

bp = Blueprint("api", __name__, url_prefix="/api")

# Largest integer storable in an SQLite `INTEGER` column
MAX_ID = 2**63 - 1

# Valid muscle IDs, mirroring `MuscleID` in `crates/domain/src/exercise.rs`
MUSCLE_IDS = frozenset({1, 11, 21, 22, 31, 32, 33, 41, 42, 51, 61, 62, 71, 72, 81, 82, 83, 91})


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
def _(model: ScheduleRotation) -> dict[str, object]:
    return {
        **model_to_dict(model),
        "routines": [r.routine_id for r in sorted(model.routines, key=lambda r: r.position)],
    }


@to_dict.register
def _(model: Workout) -> dict[str, object]:
    return {
        **model_to_dict(model),
        "elements": [to_dict(e) for e in model.elements],
        "exercise_notes": [
            {"exercise_id": n.exercise_id, "notes": n.notes}
            for n in sorted(model.exercise_notes, key=lambda n: n.exercise_id)
        ],
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
    exclude_key = ("user_id",) if exclude is None else tuple(exclude)
    include_key = () if include is None else tuple(include)
    result: dict[str, object] = {}
    for name, is_date in _serialized_columns(model.__table__, exclude_key, include_key):
        value = getattr(model, name)
        if is_date and value is not None:
            value = value.isoformat()
        result[name] = value
    return result


@cache
def _serialized_columns(
    table: Table, exclude: tuple[str, ...], include: tuple[str, ...]
) -> tuple[tuple[str, bool], ...]:
    return (
        *(
            (col.name, issubclass(col.type.python_type, date))
            for col in table.columns
            if col.name not in exclude
        ),
        *((name, False) for name in include if name not in exclude),
    )


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
        rounds=to_int(json["rounds"], "rounds", 1, 999),
        parts=to_routine_parts(json["parts"]),
    )


def to_routine_activity(  # type: ignore[explicit-any]
    json: dict[str, Any], position: int
) -> RoutineActivity:
    return RoutineActivity(
        position=position,
        exercise_id=to_optional_id(json["exercise_id"], "exercise_id"),
        reps=to_int(json["reps"], "reps", 0, 999),
        time=to_int(json["time"], "time", 0, 999),
        weight=to_weight(json["weight"], "weight", 0.0),
        rpe=to_rpe(json["rpe"], "rpe"),
        automatic=to_bool(json["automatic"], "automatic"),
    )


def to_workout_elements(json: list[dict[str, Any]]) -> list[WorkoutElement]:  # type: ignore[explicit-any]
    return [
        (
            WorkoutSet(
                position=position,
                exercise_id=to_id(element["exercise_id"], "exercise_id"),
                reps=to_optional_int(element["reps"], "reps", 1, 999),
                time=to_optional_int(element["time"], "time", 1, 999),
                weight=to_optional_weight(element["weight"], "weight", 0.01),
                rpe=to_optional_rpe(element["rpe"], "rpe"),
                target_reps=to_optional_int(element["target_reps"], "target_reps", 1, 999),
                target_time=to_optional_int(element["target_time"], "target_time", 1, 999),
                target_weight=to_optional_weight(element["target_weight"], "target_weight", 0.01),
                target_rpe=to_optional_rpe(element["target_rpe"], "target_rpe"),
                automatic=to_bool(element["automatic"], "automatic"),
            )
            if "exercise_id" in element
            else WorkoutRest(
                position=position,
                target_time=to_optional_int(element["target_time"], "target_time", 1, 999),
                automatic=to_bool(element["automatic"], "automatic"),
            )
        )
        for position, element in enumerate(json, start=1)
    ]


def to_workout_exercise_notes(json: list[dict[str, Any]]) -> list[WorkoutExerciseNote]:  # type: ignore[explicit-any]
    return [
        WorkoutExerciseNote(
            exercise_id=to_id(note["exercise_id"], "exercise_id"),
            notes=to_string(note["notes"], "notes"),
        )
        for note in json
        if note["notes"]
    ]


def _referenced_exercises_exist(user_id: int, exercise_ids: set[int]) -> bool:
    """Return whether every referenced exercise belongs to the user."""
    if not exercise_ids:
        return True
    owned = set(
        db.session.execute(select(Exercise.id).where(Exercise.user_id == user_id)).scalars()
    )
    return exercise_ids <= owned


def _routine_exercise_ids(routine: Routine) -> set[int]:
    return _exercise_ids_in_parts(routine.sections)


def _exercise_ids_in_parts(parts: Iterable[RoutinePart]) -> set[int]:
    ids: set[int] = set()
    for part in parts:
        if isinstance(part, RoutineSection):
            ids |= _exercise_ids_in_parts(part.parts)
        elif isinstance(part, RoutineActivity) and part.exercise_id is not None:
            ids.add(part.exercise_id)
    return ids


def _workout_exercise_ids(workout: Workout) -> set[int]:
    return {e.exercise_id for e in workout.elements if isinstance(e, WorkoutSet)} | {
        n.exercise_id for n in workout.exercise_notes
    }


def schedule_to_dict(
    rotations: list[ScheduleRotation], slots: list[ScheduleSlot]
) -> dict[str, object]:
    entries: dict[int, list[dict[str, int]]] = {}
    for slot in sorted(slots, key=lambda s: (s.weekday, s.position)):
        entries.setdefault(slot.weekday, []).append(schedule_slot_to_dict(slot))
    return {
        "rotations": [to_dict(rotation) for rotation in sorted(rotations, key=lambda r: r.id)],
        "entries": [
            {"weekday": weekday, "slots": slots} for weekday, slots in sorted(entries.items())
        ],
    }


def schedule_slot_to_dict(slot: ScheduleSlot) -> dict[str, int]:
    if slot.routine_id is not None:
        return {"routine": slot.routine_id}
    assert slot.rotation_id is not None
    return {"rotation": slot.rotation_id}


def to_schedule_rotations(  # type: ignore[explicit-any]
    json: list[dict[str, Any]], user_id: int
) -> list[ScheduleRotation]:
    if len({rotation["id"] for rotation in json}) < len(json):
        raise DeserializationError("schedule must not contain duplicate rotations")
    for rotation in json:
        if len(set(rotation["routines"])) < len(rotation["routines"]):
            raise DeserializationError("rotation must not contain duplicate routines")
    return [
        ScheduleRotation(
            id=to_id(rotation["id"], "rotation id"),
            user_id=user_id,
            name=to_name(rotation["name"]),
            routines=[
                ScheduleRotationRoutine(
                    position=position, routine_id=to_id(routine_id, "routine id")
                )
                for position, routine_id in enumerate(rotation["routines"], start=1)
            ],
        )
        for rotation in json
    ]


def to_schedule_slots(  # type: ignore[explicit-any]
    json: list[dict[str, Any]], user_id: int
) -> list[ScheduleSlot]:
    if len({entry["weekday"] for entry in json}) < len(json):
        raise DeserializationError("schedule must not contain duplicate weekdays")
    return [
        to_schedule_slot(slot, user_id, entry["weekday"], position)
        for entry in json
        for position, slot in enumerate(entry["slots"], start=1)
    ]


def to_schedule_slot(  # type: ignore[explicit-any]
    json: dict[str, Any], user_id: int, weekday: int, position: int
) -> ScheduleSlot:
    weekday = to_int(weekday, "weekday", 1, 7)
    if set(json) == {"routine"}:
        return ScheduleSlot(
            user_id=user_id,
            weekday=weekday,
            position=position,
            routine_id=to_id(json["routine"], "routine id"),
        )
    if set(json) == {"rotation"}:
        return ScheduleSlot(
            user_id=user_id,
            weekday=weekday,
            position=position,
            rotation_id=to_id(json["rotation"], "rotation id"),
        )
    raise DeserializationError("slot must contain either 'routine' or 'rotation'")


def to_name(json: object) -> str:
    if not isinstance(json, str):
        raise DeserializationError("name must be a string")
    name = json.strip()
    if not name:
        raise DeserializationError("name must not be empty")
    # The frontend limits the length in UTF-8 bytes (`str::len` in `Name::new`)
    if len(name.encode()) > 64:
        raise DeserializationError("name must be 64 characters or fewer")
    return name


def to_notes(json: object) -> str | None:
    if json is not None and not isinstance(json, str):
        raise DeserializationError("notes must be a string or null")
    return json


def to_string(json: object, what: str) -> str:
    if not isinstance(json, str):
        raise DeserializationError(f"{what} must be a string")
    return json


def to_bool(json: object, what: str) -> bool:
    if not isinstance(json, bool):
        raise DeserializationError(f"{what} must be a boolean")
    return json


def to_date(json: object) -> date:
    if not isinstance(json, str):
        raise DeserializationError("date must be a string")
    return date.fromisoformat(json)


def to_sex(json: object) -> Sex:
    if isinstance(json, bool) or not isinstance(json, int):
        raise DeserializationError("sex must be an integer")
    return Sex(json)


def to_role(json: object) -> Role:
    if isinstance(json, bool) or not isinstance(json, int):
        raise DeserializationError("role must be an integer")
    return Role(json)


def to_muscle_id(json: object) -> int:
    if isinstance(json, bool) or not isinstance(json, int):
        raise DeserializationError("muscle_id must be an integer")
    if json not in MUSCLE_IDS:
        raise DeserializationError(f"{json} is not a valid muscle id")
    return json


def to_optional_id(json: object, what: str) -> int | None:
    return None if json is None else to_id(json, what)


def to_id(json: object, what: str) -> int:
    return to_int(json, what, 1, MAX_ID)


def to_optional_int(json: object, what: str, minimum: int, maximum: int) -> int | None:
    return None if json is None else to_int(json, what, minimum, maximum)


def to_int(json: object, what: str, minimum: int, maximum: int) -> int:
    if isinstance(json, bool) or not isinstance(json, int):
        raise DeserializationError(f"{what} must be an integer")
    if not minimum <= json <= maximum:
        raise DeserializationError(f"{what} must be in the range {minimum} to {maximum}")
    return json


def to_optional_weight(json: object, what: str, minimum: float) -> float | None:
    return None if json is None else to_weight(json, what, minimum)


def to_weight(json: object, what: str, minimum: float) -> float:
    value = to_number(json, what)
    if not minimum <= value <= 999.99:
        raise DeserializationError(f"{what} must be in the range {minimum} to 999.99")
    if abs(value * 100 - round(value * 100)) > 1e-3:
        raise DeserializationError(f"{what} must be a multiple of 0.01")
    return value


def to_optional_rpe(json: object, what: str) -> float | None:
    return None if json is None else to_rpe(json, what)


def to_rpe(json: object, what: str) -> float:
    value = to_number(json, what)
    if not 0 <= value <= 10:
        raise DeserializationError(f"{what} must be in the range 0 to 10")
    if abs(value * 2 - round(value * 2)) > 1e-3:
        raise DeserializationError(f"{what} must be a multiple of 0.5")
    return value


def to_positive_number(json: object, what: str) -> float:
    value = to_number(json, what)
    if value <= 0:
        raise DeserializationError(f"{what} must be positive")
    return value


def to_number(json: object, what: str) -> float:
    if isinstance(json, bool) or not isinstance(json, (int, float)):
        raise DeserializationError(f"{what} must be a number")
    try:
        value = float(json)
    except OverflowError:
        raise DeserializationError(f"{what} must be finite") from None
    if not math.isfinite(value):
        raise DeserializationError(f"{what} must be finite")
    return value


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
        if "user_id" not in session:
            return "", HTTPStatus.UNAUTHORIZED
        return function(*args, **kwargs)

    return decorated_function


def admin_required(function: Callable) -> Callable:  # type: ignore[type-arg]
    @wraps(function)
    def decorated_function(*args: object, **kwargs: object) -> ResponseReturnValue:
        if not _session_user_is_admin():
            return jsonify({"details": "user is not an administrator"}), HTTPStatus.FORBIDDEN
        return function(*args, **kwargs)

    return decorated_function


def self_or_admin(function: Callable) -> Callable:  # type: ignore[type-arg]
    @wraps(function)
    def decorated_function(*args: object, **kwargs: object) -> ResponseReturnValue:
        if kwargs["user_id"] != session["user_id"] and not _session_user_is_admin():
            return jsonify({"details": "user is not an administrator"}), HTTPStatus.FORBIDDEN
        return function(*args, **kwargs)

    return decorated_function


def conditional(collection: str) -> Callable:  # type: ignore[type-arg]
    def decorator(function: Callable) -> Callable:  # type: ignore[type-arg]
        @wraps(function)
        def decorated_function(*args: object, **kwargs: object) -> ResponseReturnValue:
            etag = collection_etag(session["user_id"], collection)
            if request.if_none_match.contains_weak(etag):
                response = make_response("", HTTPStatus.NOT_MODIFIED)
            else:
                response = make_response(function(*args, **kwargs))
            response.set_etag(etag, weak=True)
            # The response is user-specific, so shared caches must not store it and any
            # cache must revalidate the ETag before reusing it.
            response.cache_control.private = True
            response.cache_control.no_cache = True
            return response

        return decorated_function

    return decorator


def collection_etag(user_id: int, collection: str) -> str:
    stored = db.session.execute(
        select(DataVersion.version).where(
            DataVersion.user_id == user_id, DataVersion.collection == collection
        )
    ).scalar_one_or_none()
    # The user id makes the ETag user-specific to prevent the browser HTTP cache from
    # revalidating one user's cached response with a 304 meant for another user.
    return f"{collection}-{user_id}-{stored or 0}"


def bump_data_version(user_id: int, *collections: str) -> None:
    for collection in collections:
        db.session.execute(
            sqlite_insert(DataVersion)
            .values(user_id=user_id, collection=collection, version=1)
            .on_conflict_do_update(
                index_elements=[DataVersion.user_id, DataVersion.collection],
                set_={"version": DataVersion.version + 1},
            )
        )


def _session_user() -> User | None:
    user_id = session.get("user_id")
    if user_id is None:
        return None
    # The user data is determined from the database instead of the session to ensure that
    # out-of-band changes (e.g. via the CLI) take effect without re-login
    return db.session.execute(select(User).where(User.id == user_id)).scalar_one_or_none()


def _session_user_is_admin() -> bool:
    user = _session_user()
    return user is not None and user.role == Role.ADMIN


def _is_last_admin(user: User) -> bool:
    if user.role != Role.ADMIN:
        return False
    other_admin = (
        db.session.execute(select(User).where(User.role == Role.ADMIN, User.id != user.id))
        .scalars()
        .first()
    )
    return other_admin is None


@bp.route("/version")
def read_version() -> ResponseReturnValue:
    return jsonify(version.get())


@bp.route("/session")
def read_session() -> ResponseReturnValue:
    user = _session_user()

    if user is None:
        session.clear()
        return "", HTTPStatus.NOT_FOUND

    return jsonify(to_dict(user))


@bp.route("/session", methods=["POST"])
@json_expected
def create_session() -> ResponseReturnValue:
    try:
        assert isinstance(request.json, dict)
        name = to_name(request.json["name"])
    except (DeserializationError, KeyError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        user = db.session.execute(select(User).where(User.name == name)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    session["user_id"] = user.id
    session.permanent = True

    return jsonify(to_dict(user))


@bp.route("/session", methods=["DELETE"])
def delete_session() -> ResponseReturnValue:
    session.clear()
    return "", HTTPStatus.NO_CONTENT


@bp.route("/users")
@session_required
@admin_required
def read_users() -> ResponseReturnValue:
    users = db.session.execute(select(User)).scalars().all()
    return jsonify([to_dict(u) for u in users])


@bp.route("/users/<int:user_id>")
@session_required
@self_or_admin
def read_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    return jsonify(to_dict(user))


@bp.route("/users", methods=["POST"])
@session_required
@admin_required
@json_expected
def create_user() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        user = User(
            name=to_name(data["name"]),
            sex=to_sex(data["sex"]),
            height=to_optional_int(data.get("height"), "height", 1, 255),
            role=to_role(data["role"]),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(user)

    try:
        db.session.commit()
    except IntegrityError:
        return jsonify({"details": "name is already used"}), HTTPStatus.CONFLICT

    return (
        jsonify(to_dict(user)),
        HTTPStatus.CREATED,
        {"Location": f"/users/{user.id}"},
    )


@bp.route("/users/<int:user_id>", methods=["PUT"])
@session_required
@self_or_admin
@json_expected
def replace_user(user_id: int) -> ResponseReturnValue:
    is_admin = _session_user_is_admin()

    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        name = to_name(data["name"])
        sex = to_sex(data["sex"])
        height = to_optional_int(data.get("height"), "height", 1, 255)
        role = to_role(data["role"])
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if role != user.role and not is_admin:
        return (
            jsonify({"details": "role can only be changed by an administrator"}),
            HTTPStatus.FORBIDDEN,
        )

    if user.role == Role.ADMIN and role != Role.ADMIN and _is_last_admin(user):
        return (
            jsonify({"details": "last administrator cannot be demoted"}),
            HTTPStatus.CONFLICT,
        )

    user.name = name
    user.sex = sex
    user.height = height
    user.role = role

    try:
        db.session.commit()
    except IntegrityError:
        return jsonify({"details": "name is already used"}), HTTPStatus.CONFLICT

    return jsonify(to_dict(user)), HTTPStatus.OK


@bp.route("/users/<int:user_id>", methods=["PATCH"])
@session_required
@self_or_admin
@json_expected
def update_user(user_id: int) -> ResponseReturnValue:
    is_admin = _session_user_is_admin()

    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        name = to_name(data["name"]) if "name" in data else user.name
        sex = to_sex(data["sex"]) if "sex" in data else user.sex
        height = to_optional_int(data.get("height", user.height), "height", 1, 255)
        role = to_role(data["role"]) if "role" in data else user.role
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if role != user.role and not is_admin:
        return (
            jsonify({"details": "role can only be changed by an administrator"}),
            HTTPStatus.FORBIDDEN,
        )

    if user.role == Role.ADMIN and role != Role.ADMIN and _is_last_admin(user):
        return (
            jsonify({"details": "last administrator cannot be demoted"}),
            HTTPStatus.CONFLICT,
        )

    user.name = name
    user.sex = sex
    user.height = height
    user.role = role

    try:
        db.session.commit()
    except IntegrityError:
        return jsonify({"details": "name is already used"}), HTTPStatus.CONFLICT

    return jsonify(to_dict(user)), HTTPStatus.OK


@bp.route("/users/<int:user_id>", methods=["DELETE"])
@session_required
@admin_required
def delete_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    if _is_last_admin(user):
        return (
            jsonify({"details": "last administrator cannot be deleted"}),
            HTTPStatus.CONFLICT,
        )

    db.session.delete(user)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/body_weight")
@session_required
@conditional("body_weight")
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
            date=to_date(data["date"]),
            weight=to_positive_number(data["weight"], "weight"),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(body_weight)
    bump_data_version(session["user_id"], "body_weight")

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
        body_weight.weight = to_positive_number(data["weight"], "weight")
    except (DeserializationError, KeyError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    bump_data_version(session["user_id"], "body_weight")
    db.session.commit()

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
    bump_data_version(session["user_id"], "body_weight")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/body_fat")
@session_required
@conditional("body_fat")
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
            date=to_date(data["date"]),
            **{
                part: to_optional_int(data[part], part, 1, 255)
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
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(body_fat)
    bump_data_version(session["user_id"], "body_fat")

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
            setattr(body_fat, attr, to_optional_int(data[attr], attr, 1, 255))
    except (DeserializationError, KeyError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    bump_data_version(session["user_id"], "body_fat")
    db.session.commit()

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
    bump_data_version(session["user_id"], "body_fat")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/period")
@session_required
@conditional("period")
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
            date=to_date(data["date"]),
            intensity=to_int(data["intensity"], "intensity", 1, 4),
        )
    except (DeserializationError, KeyError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(period)
    bump_data_version(session["user_id"], "period")

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
        period.intensity = to_int(data["intensity"], "intensity", 1, 4)
    except (DeserializationError, KeyError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    bump_data_version(session["user_id"], "period")
    db.session.commit()

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
    bump_data_version(session["user_id"], "period")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/exercises")
@session_required
@conditional("exercises")
def read_exercises() -> ResponseReturnValue:
    exercises = (
        db.session.execute(
            select(Exercise)
            .where(Exercise.user_id == session["user_id"])
            .options(selectinload(Exercise.muscles))
        )
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
            name=to_name(data["name"]),
            muscles=[
                ExerciseMuscle(
                    user_id=session["user_id"],
                    muscle_id=to_muscle_id(muscle["muscle_id"]),
                    stimulus=to_int(muscle["stimulus"], "stimulus", 1, 100),
                )
                for muscle in data["muscles"]
            ],
        )
    except (DeserializationError, KeyError, TypeError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(exercise)
    bump_data_version(session["user_id"], "exercises")

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
        exercise.name = to_name(data["name"])
        muscle_stimulus = {
            to_muscle_id(m["muscle_id"]): to_int(m["stimulus"], "stimulus", 1, 100)
            for m in data["muscles"]
        }

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
    except (DeserializationError, KeyError, TypeError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    bump_data_version(session["user_id"], "exercises")

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
    # Deleting an exercise cascades to its workout sets and routine activities.
    bump_data_version(session["user_id"], "exercises", "workouts", "routines")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/routines")
@session_required
@conditional("routines")
def read_routines() -> ResponseReturnValue:
    routines = (
        db.session.execute(
            select(Routine)
            .where(Routine.user_id == session["user_id"])
            .options(
                selectinload(Routine.sections).selectinload(
                    RoutineSection.parts, recursion_depth=-1
                )
            )
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
            name=to_name(data["name"]),
            notes=to_notes(data["notes"]),
            archived=to_bool(data["archived"], "archived"),
            sections=to_routine_sections(data["sections"]),
        )
    except (DeserializationError, KeyError, TypeError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if not _referenced_exercises_exist(session["user_id"], _routine_exercise_ids(routine)):
        return (
            jsonify({"details": "routine references an unknown exercise"}),
            HTTPStatus.CONFLICT,
        )

    db.session.add(routine)
    bump_data_version(session["user_id"], "routines")

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
            routine.name = to_name(data["name"])
        if "notes" in data or request.method == "PUT":
            routine.notes = to_notes(data["notes"])
        if "archived" in data or request.method == "PUT":
            routine.archived = to_bool(data["archived"], "archived")
        if "sections" in data or request.method == "PUT":
            routine.sections = to_routine_sections(data["sections"])
    except (DeserializationError, KeyError, TypeError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if ("sections" in data or request.method == "PUT") and not _referenced_exercises_exist(
        session["user_id"], _routine_exercise_ids(routine)
    ):
        return (
            jsonify({"details": "routine references an unknown exercise"}),
            HTTPStatus.CONFLICT,
        )

    bump_data_version(session["user_id"], "routines")

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

    if (
        db.session.execute(
            select(ScheduleSlot)
            .where(ScheduleSlot.user_id == session["user_id"])
            .where(ScheduleSlot.routine_id == id_)
        ).first()
        is not None
        or db.session.execute(
            select(ScheduleRotationRoutine)
            .join(ScheduleRotation)
            .where(ScheduleRotation.user_id == session["user_id"])
            .where(ScheduleRotationRoutine.routine_id == id_)
        ).first()
        is not None
    ):
        return (
            jsonify({"details": "routine is used in the schedule"}),
            HTTPStatus.CONFLICT,
        )

    db.session.delete(routine)
    # Deleting a routine detaches it from the workouts that referenced it.
    bump_data_version(session["user_id"], "routines", "workouts")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT


@bp.route("/schedule")
@session_required
@conditional("schedule")
def read_schedule() -> ResponseReturnValue:
    rotations = (
        db.session.execute(
            select(ScheduleRotation)
            .where(ScheduleRotation.user_id == session["user_id"])
            .options(selectinload(ScheduleRotation.routines))
        )
        .scalars()
        .all()
    )
    slots = (
        db.session.execute(select(ScheduleSlot).where(ScheduleSlot.user_id == session["user_id"]))
        .scalars()
        .all()
    )
    return jsonify(schedule_to_dict(list(rotations), list(slots)))


@bp.route("/schedule", methods=["PUT"])
@session_required
@json_expected
def replace_schedule() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        rotations = to_schedule_rotations(data["rotations"], session["user_id"])
        slots = to_schedule_slots(data["entries"], session["user_id"])
    except (DeserializationError, KeyError, TypeError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    routine_ids = set(
        db.session.execute(
            select(Routine.id).where(Routine.user_id == session["user_id"])
        ).scalars()
    )
    referenced_routine_ids = {r.routine_id for rotation in rotations for r in rotation.routines} | {
        slot.routine_id for slot in slots if slot.routine_id is not None
    }
    if not referenced_routine_ids <= routine_ids:
        return (
            jsonify({"details": "schedule references an unknown routine"}),
            HTTPStatus.CONFLICT,
        )

    rotation_ids = {rotation.id for rotation in rotations}
    if any(slot.rotation_id is not None and slot.rotation_id not in rotation_ids for slot in slots):
        return (
            jsonify({"details": "schedule references an unknown rotation"}),
            HTTPStatus.CONFLICT,
        )

    # Delete slots before rotations so an `ON DELETE CASCADE` from a slot's rotation never fires
    db.session.execute(delete(ScheduleSlot).where(ScheduleSlot.user_id == session["user_id"]))
    db.session.execute(
        delete(ScheduleRotation).where(ScheduleRotation.user_id == session["user_id"])
    )

    db.session.add_all([*rotations, *slots])
    bump_data_version(session["user_id"], "schedule")

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return jsonify(schedule_to_dict(rotations, slots)), HTTPStatus.OK


@bp.route("/workouts")
@session_required
@conditional("workouts")
def read_workouts() -> ResponseReturnValue:
    workouts = (
        db.session.execute(
            select(Workout)
            .where(Workout.user_id == session["user_id"])
            .options(selectinload(Workout.elements), selectinload(Workout.exercise_notes))
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
        routine_id = to_optional_id(data["routine_id"], "routine_id")
        routine = (
            (
                db.session.execute(
                    select(Routine)
                    .where(Routine.user_id == session["user_id"])
                    .where(Routine.id == routine_id)
                )
                .scalars()
                .one()
            )
            if routine_id is not None
            else None
        )

        workout = Workout(
            user_id=session["user_id"],
            routine=routine,
            date=to_date(data["date"]),
            notes=to_notes(data["notes"]),
            elements=to_workout_elements(data["elements"]),
            exercise_notes=to_workout_exercise_notes(data["exercise_notes"]),
        )
    except (DeserializationError, NoResultFound, KeyError, TypeError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if not _referenced_exercises_exist(session["user_id"], _workout_exercise_ids(workout)):
        return (
            jsonify({"details": "workout references an unknown exercise"}),
            HTTPStatus.CONFLICT,
        )

    db.session.add(workout)
    bump_data_version(session["user_id"], "workouts")

    db.session.commit()

    return (
        jsonify(to_dict(workout)),
        HTTPStatus.CREATED,
        {"Location": f"/workouts/{workout.id}"},
    )


def _apply_workout_update(workout: Workout, data: dict[str, Any], *, is_put: bool) -> None:  # type: ignore[explicit-any]
    if "elements" in data or is_put:
        for e in workout.elements:
            db.session.delete(e)
        db.session.flush()
    if "exercise_notes" in data or is_put:
        for n in workout.exercise_notes:
            db.session.delete(n)
        db.session.flush()
    if "date" in data or is_put:
        workout.date = to_date(data["date"])
    if "notes" in data or is_put:
        workout.notes = to_notes(data["notes"])
    if "elements" in data or is_put:
        workout.elements = to_workout_elements(data["elements"])
    if "exercise_notes" in data or is_put:
        workout.exercise_notes = to_workout_exercise_notes(data["exercise_notes"])


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
                .options(selectinload(Workout.elements), selectinload(Workout.exercise_notes))
            )
            .scalars()
            .one()
        )
    except (NoResultFound, ValueError):
        return "", HTTPStatus.NOT_FOUND

    data = request.json

    assert isinstance(data, dict)

    try:
        _apply_workout_update(workout, data, is_put=request.method == "PUT")
    except (DeserializationError, KeyError, TypeError, ValueError) as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    if not _referenced_exercises_exist(session["user_id"], _workout_exercise_ids(workout)):
        return (
            jsonify({"details": "workout references an unknown exercise"}),
            HTTPStatus.CONFLICT,
        )

    bump_data_version(session["user_id"], "workouts")
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
    bump_data_version(session["user_id"], "workouts")
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT
