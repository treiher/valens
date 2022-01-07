from functools import wraps
from http import HTTPStatus
from typing import Callable

from flask import jsonify, request, session
from flask.typing import ResponseReturnValue
from sqlalchemy import select
from sqlalchemy.exc import IntegrityError, NoResultFound

from valens import __version__, app, database as db
from valens.models import User


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


@app.route("/api/version")
def get_version() -> ResponseReturnValue:
    return jsonify(__version__)


@app.route("/api/session")
def get_session() -> ResponseReturnValue:
    if "username" not in session or "user_id" not in session or "sex" not in session:
        return "", HTTPStatus.NOT_FOUND

    return jsonify({"id": session["user_id"], "name": session["username"], "sex": session["sex"]})


@app.route("/api/session", methods=["POST"])
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


@app.route("/api/session", methods=["DELETE"])
def delete_session() -> ResponseReturnValue:
    session.clear()
    return "", HTTPStatus.NO_CONTENT


@app.route("/api/users")
def get_users() -> ResponseReturnValue:
    users = db.session.execute(select(User)).scalars().all()
    return jsonify([{"id": u.id, "name": u.name, "sex": u.sex} for u in users])


@app.route("/api/users/<int:user_id>")
@session_required
def get_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    return jsonify({"id": user.id, "name": user.name, "sex": user.sex})


@app.route("/api/users", methods=["POST"])
@json_expected
def add_user() -> ResponseReturnValue:
    data = request.json

    assert isinstance(data, dict)

    try:
        user = User(name=data["name"].strip(), sex=data["sex"])
    except KeyError as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    db.session.add(user)

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return (
        jsonify({"id": user.id, "name": user.name, "sex": user.sex}),
        HTTPStatus.CREATED,
        {"Location": f"/api/users/{user.id}"},
    )


@app.route("/api/users/<int:user_id>", methods=["PUT"])
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
        user.sex = data["sex"]
    except KeyError as e:
        return jsonify({"details": str(e)}), HTTPStatus.BAD_REQUEST

    try:
        db.session.commit()
    except IntegrityError as e:
        return jsonify({"details": str(e)}), HTTPStatus.CONFLICT

    return jsonify({"id": user.id, "name": user.name, "sex": user.sex}), HTTPStatus.OK


@app.route("/api/users/<int:user_id>", methods=["DELETE"])
def delete_user(user_id: int) -> ResponseReturnValue:
    try:
        user = db.session.execute(select(User).where(User.id == user_id)).scalars().one()
    except NoResultFound:
        return "", HTTPStatus.NOT_FOUND

    db.session.delete(user)
    db.session.commit()

    return "", HTTPStatus.NO_CONTENT
