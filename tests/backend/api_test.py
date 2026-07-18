from __future__ import annotations

from collections.abc import Callable, Generator
from http import HTTPStatus
from pathlib import Path

import pytest
from sqlalchemy import select
from werkzeug.test import Client, TestResponse as Response

import tests.data
import tests.utils
from valens import app, database as db
from valens.models import Role, User

CONFLICT_DETAILS = {
    "body_weight": "entry with this date already exists",
    "body_fat": "entry with this date already exists",
    "period": "entry with this date already exists",
    "exercises": "name is already used",
    "routines": "name is already used",
}

INVALID_ROUTINE_ACTIVITY_FIELDS: list[dict[str, object]] = [
    {"exercise_id": 0},
    {"reps": 1000},
    {"time": -1},
    {"weight": 1000.0},
    {"weight": 8.001},
    {"rpe": 10.5},
    {"rpe": 7.3},
    {"automatic": 1},
]

INVALID_WORKOUT_FIELDS: list[dict[str, object]] = [
    {"date": 3},
    {"routine_id": "1"},
    {"notes": 1},
    {"exercise_notes": [{"exercise_id": 1, "notes": 5}]},
    {"elements": [{"target_time": 0, "automatic": False}]},
    {"elements": [{"target_time": 60, "automatic": "yes"}]},
]

INVALID_WORKOUT_SET_FIELDS: list[dict[str, object]] = [
    {"exercise_id": None},
    {"reps": 0},
    {"reps": 1000},
    {"weight": 0},
    {"rpe": 8.25},
    {"target_rpe": -1},
]


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client, app.app_context():
        yield client


def create_session(client: Client, name: str = "Alice") -> Response:
    return client.post("/api/session", json={"name": name})


def delete_session(client: Client) -> Response:
    return client.delete("/api/session")


@pytest.mark.parametrize(
    ("method", "route"),
    [
        ("get", "/api/users"),
        ("post", "/api/users"),
        ("get", "/api/users/1"),
        ("put", "/api/users/2"),
        ("patch", "/api/users/2"),
        ("delete", "/api/users/2"),
        ("get", "/api/users/1/passkeys"),
        ("patch", "/api/users/1/passkeys/1"),
        ("delete", "/api/users/1/passkeys/1"),
        ("post", "/api/auth/passkeys/registration/options"),
        ("post", "/api/auth/passkeys/registration"),
        ("post", "/api/auth/login-link"),
        ("get", "/api/body_weight"),
        ("post", "/api/body_weight"),
        ("put", "/api/body_weight/2002-02-22"),
        ("get", "/api/body_fat"),
        ("post", "/api/body_fat"),
        ("put", "/api/body_fat/2002-02-22"),
        ("get", "/api/period"),
        ("post", "/api/period"),
        ("put", "/api/period/2002-02-22"),
        ("get", "/api/exercises"),
        ("post", "/api/exercises"),
        ("put", "/api/exercises/1"),
        ("get", "/api/routines"),
        ("post", "/api/routines"),
        ("put", "/api/routines/1"),
        ("get", "/api/schedule"),
        ("put", "/api/schedule"),
        ("get", "/api/workouts"),
        ("post", "/api/workouts"),
    ],
)
def test_session_required(client: Client, method: str, route: str) -> None:
    resp = getattr(client, method)(route)

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert not resp.data


@pytest.mark.parametrize(
    ("method", "route"),
    [
        ("post", "/api/session"),
        ("post", "/api/auth/session"),
        ("post", "/api/auth/passkeys/registration"),
        ("post", "/api/auth/passkeys/authentication"),
        ("post", "/api/auth/login-link"),
        ("post", "/api/users"),
        ("put", "/api/users/2"),
        ("patch", "/api/users/2"),
        ("patch", "/api/users/1/passkeys/1"),
        ("post", "/api/body_weight"),
        ("put", "/api/body_weight/2002-02-22"),
        ("post", "/api/body_fat"),
        ("put", "/api/body_fat/2002-02-22"),
        ("post", "/api/period"),
        ("put", "/api/period/2002-02-22"),
        ("post", "/api/exercises"),
        ("put", "/api/exercises/1"),
        ("post", "/api/routines"),
        ("put", "/api/routines/1"),
        ("put", "/api/schedule"),
        ("post", "/api/workouts"),
    ],
)
def test_json_required(client: Client, method: str, route: str) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = getattr(client, method)(route, data={})

    assert resp.status_code == HTTPStatus.UNSUPPORTED_MEDIA_TYPE
    assert not resp.data


@pytest.mark.parametrize(
    ("method", "route", "data"),
    [
        ("post", "/api/session", {"invalid": "data"}),
        ("post", "/api/session", {"name": 1}),
        ("post", "/api/users", {"invalid": "data"}),
        ("post", "/api/users", {"name": 1, "sex": 0, "role": 0}),
        ("post", "/api/users", {"name": " ", "sex": 0, "role": 0}),
        ("post", "/api/users", {"name": "A" * 65, "sex": 0, "role": 0}),
        ("post", "/api/users", {"name": "Ä" * 33, "sex": 0, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": "female", "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": True, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 2, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "height": "invalid", "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "height": 175.5, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "height": True, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "height": 0, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "height": 256, "role": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "role": "user"}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "role": True}),
        ("post", "/api/users", {"name": "Carol", "sex": 0, "role": 2}),
        ("put", "/api/users/2", {"invalid": "data"}),
        ("put", "/api/users/2", {"name": "Carol", "sex": 0, "height": -175, "role": 0}),
        ("put", "/api/users/2", {"name": "Carol", "sex": 0}),
        ("put", "/api/users/2", {"name": "Carol", "sex": 0, "role": 2}),
        ("patch", "/api/users/2", {"name": 1}),
        ("patch", "/api/users/2", {"sex": 2}),
        ("patch", "/api/users/2", {"height": 0}),
        ("patch", "/api/users/2", {"role": 2}),
        ("post", "/api/body_weight", {"invalid": "data"}),
        ("post", "/api/body_weight", {"date": 20020224, "weight": 80.0}),
        ("post", "/api/body_weight", {"date": "2002-02-24", "weight": "80"}),
        ("post", "/api/body_weight", {"date": "2002-02-24", "weight": float("nan")}),
        ("post", "/api/body_weight", {"date": "2002-02-24", "weight": 10**400}),
        ("post", "/api/body_weight", {"date": "2002-02-24", "weight": 0}),
        ("put", "/api/body_weight/2002-02-22", {"invalid": "data"}),
        ("put", "/api/body_weight/2002-02-22", {"weight": -1}),
        ("post", "/api/body_fat", {"invalid": "data"}),
        (
            "post",
            "/api/body_fat",
            {
                "date": "2002-02-24",
                "chest": 256,
                "abdominal": None,
                "thigh": None,
                "tricep": None,
                "subscapular": None,
                "suprailiac": None,
                "midaxillary": None,
            },
        ),
        (
            "post",
            "/api/body_fat",
            {
                "date": "2002-02-24",
                "chest": True,
                "abdominal": None,
                "thigh": None,
                "tricep": None,
                "subscapular": None,
                "suprailiac": None,
                "midaxillary": None,
            },
        ),
        ("put", "/api/body_fat/2002-02-20", {"invalid": "data"}),
        (
            "put",
            "/api/body_fat/2002-02-20",
            {
                "chest": 0,
                "abdominal": None,
                "thigh": None,
                "tricep": None,
                "subscapular": None,
                "suprailiac": None,
                "midaxillary": None,
            },
        ),
        ("post", "/api/period", {"invalid": "data"}),
        ("post", "/api/period", {"date": "2002-02-24", "intensity": 5}),
        ("post", "/api/period", {"date": "2002-02-24", "intensity": "1"}),
        ("put", "/api/period/2002-02-22", {"invalid": "data"}),
        ("put", "/api/period/2002-02-22", {"intensity": 0}),
        ("post", "/api/exercises", {"invalid": "data"}),
        ("post", "/api/exercises", {"name": "data", "muscles": [{"invalid": "data"}]}),
        ("post", "/api/exercises", {"name": "", "muscles": []}),
        ("post", "/api/exercises", {"name": "A", "muscles": 1}),
        ("post", "/api/exercises", {"name": "A", "muscles": [{"muscle_id": 12, "stimulus": 100}]}),
        (
            "post",
            "/api/exercises",
            {"name": "A", "muscles": [{"muscle_id": "11", "stimulus": 100}]},
        ),
        ("post", "/api/exercises", {"name": "A", "muscles": [{"muscle_id": 11, "stimulus": 0}]}),
        ("post", "/api/exercises", {"name": "A", "muscles": [{"muscle_id": 11, "stimulus": 101}]}),
        ("put", "/api/exercises/1", {"invalid": "data"}),
        ("put", "/api/exercises/1", {"name": "A", "muscles": [{"muscle_id": 12, "stimulus": 100}]}),
        ("post", "/api/routines", {"invalid": "data"}),
        ("post", "/api/routines", {"name": "", "notes": None, "archived": False, "sections": []}),
        ("post", "/api/routines", {"name": "R", "notes": 1, "archived": False, "sections": []}),
        ("post", "/api/routines", {"name": "R", "notes": None, "archived": 1, "sections": []}),
        ("post", "/api/routines", {"name": "R", "notes": None, "archived": False, "sections": 1}),
        (
            "post",
            "/api/routines",
            {
                "name": "R",
                "notes": None,
                "archived": False,
                "sections": [{"rounds": 0, "parts": []}],
            },
        ),
        (
            "post",
            "/api/routines",
            {
                "name": "R",
                "notes": None,
                "archived": False,
                "sections": [{"rounds": 1000, "parts": []}],
            },
        ),
        *[
            (
                "post",
                "/api/routines",
                {
                    "name": "R",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 10,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                    **invalid_activity_field,
                                }
                            ],
                        }
                    ],
                },
            )
            for invalid_activity_field in INVALID_ROUTINE_ACTIVITY_FIELDS
        ],
        ("put", "/api/routines/1", {"invalid": "data"}),
        ("patch", "/api/routines/1", {"sections": [{"invalid": "data"}]}),
        ("put", "/api/schedule", {"invalid": "data"}),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 1, "slots": [{"invalid": "data"}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {
                "rotations": [],
                "entries": [{"weekday": 1, "slots": [{"routine": 1, "rotation": 1}]}],
            },
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": 1, "name": "A"}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 0, "slots": [{"routine": 1}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 8, "slots": [{"routine": 1}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 1.5, "slots": [{"routine": 1}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": True, "slots": [{"routine": 1}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": 1, "name": "A/B", "routines": [1, 1]}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": 1, "name": "", "routines": [1]}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": 0, "name": "A/B", "routines": [1]}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": "1", "name": "A/B", "routines": [1]}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [{"id": 1, "name": "A/B", "routines": ["1"]}], "entries": []},
        ),
        (
            "put",
            "/api/schedule",
            {
                "rotations": [
                    {"id": 1, "name": "A/B", "routines": [1]},
                    {"id": 1, "name": "C/D", "routines": [2]},
                ],
                "entries": [],
            },
        ),
        (
            "put",
            "/api/schedule",
            {
                "rotations": [],
                "entries": [
                    {"weekday": 1, "slots": [{"routine": 1}]},
                    {"weekday": 1, "slots": [{"routine": 2}]},
                ],
            },
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 1, "slots": [{"routine": 0}]}]},
        ),
        (
            "put",
            "/api/schedule",
            {"rotations": [], "entries": [{"weekday": 1, "slots": [{"rotation": "1"}]}]},
        ),
        ("post", "/api/workouts", {"invalid": "data"}),
        *[
            (
                "post",
                "/api/workouts",
                {
                    "date": "2002-02-24",
                    "routine_id": None,
                    "notes": None,
                    "elements": [],
                    "exercise_notes": [],
                    **invalid_workout_field,
                },
            )
            for invalid_workout_field in INVALID_WORKOUT_FIELDS
        ],
        *[
            (
                "post",
                "/api/workouts",
                {
                    "date": "2002-02-24",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                            **invalid_set_field,
                        }
                    ],
                    "exercise_notes": [],
                },
            )
            for invalid_set_field in INVALID_WORKOUT_SET_FIELDS
        ],
        ("put", "/api/workouts/1", {"invalid": "data"}),
        ("patch", "/api/workouts/1", {"elements": [{"invalid": "data"}]}),
    ],
)
def test_invalid_data(client: Client, method: str, route: str, data: object) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = getattr(client, method)(route, json=data)

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.is_json


def test_read_version(client: Client) -> None:
    resp = client.get("/api/version")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json


def test_session(client: Client) -> None:
    tests.utils.init_db_data()

    resp = create_session(client)
    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}

    resp = client.get("/api/session")
    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}

    resp = delete_session(client)
    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data


def test_read_session_not_found(client: Client) -> None:
    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_create_session_not_found(client: Client) -> None:
    resp = client.post("/api/session", json={"name": "Alice"})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_read_users(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1},
        {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 0},
    ]


def test_read_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = create_session(client)
    assert resp.status_code == HTTPStatus.OK

    resp = client.get("/api/users/0")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    resp = client.get("/api/users/1")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}

    resp = delete_session(client)
    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data


def test_create_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/users", json={"name": "Carol", "sex": 0, "height": 175, "role": 0})

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json == {"id": 3, "name": "Carol", "sex": 0, "height": 175, "role": 0}

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1},
        {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 0},
        {"id": 3, "name": "Carol", "sex": 0, "height": 175, "role": 0},
    ]


def test_create_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/users", json={"name": " Alice ", "sex": 0, "role": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "name is already used"}


def test_replace_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": "Carol", "sex": 0, "role": 0})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Carol", "sex": 0, "height": None, "role": 0}

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1},
        {"id": 2, "name": "Carol", "sex": 0, "height": None, "role": 0},
    ]


def test_read_session_reflects_out_of_band_changes(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    user = db.session.execute(select(User).where(User.id == 1)).scalars().one()
    user.name = "Anna"
    user.role = Role.USER
    db.session.commit()

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Anna", "sex": 0, "height": 168, "role": 0}


def test_read_session_user_deleted(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    user = db.session.execute(select(User).where(User.id == 1)).scalars().one()
    db.session.delete(user)
    db.session.commit()

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    resp = client.get("/api/body_weight")

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert not resp.data


def test_replace_user_not_found(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/3", json={"name": "Carol", "sex": 0, "role": 0})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_replace_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": " Alice ", "sex": 0, "role": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "name is already used"}


def test_delete_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.delete("/api/users/2")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1},
    ]

    resp = client.delete("/api/users/2")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_delete_user_with_routines_and_schedule(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    # A second administrator is required so that deleting user 1 is permitted
    assert client.patch("/api/users/2", json={"role": 1}).status_code == HTTPStatus.OK

    resp = client.delete("/api/users/1")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


@pytest.mark.parametrize(
    ("method", "route"),
    [
        ("get", "/api/users"),
        ("post", "/api/users"),
        ("get", "/api/users/1"),
        ("put", "/api/users/1"),
        ("patch", "/api/users/1"),
        ("delete", "/api/users/1"),
    ],
)
def test_admin_required(client: Client, method: str, route: str) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = getattr(client, method)(route, json={})

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_admin_required_user_deleted(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": "Bob", "sex": 1, "role": 1})
    assert resp.status_code == HTTPStatus.OK

    resp = client.delete("/api/users/1")
    assert resp.status_code == HTTPStatus.NO_CONTENT

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_read_own_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = client.get("/api/users/2")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 0}


def test_replace_own_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": "Ben", "sex": 1, "height": 182, "role": 0})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Ben", "sex": 1, "height": 182, "role": 0}

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Ben", "sex": 1, "height": 182, "role": 0}


def test_replace_own_user_role_change_forbidden(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": "Bob", "sex": 1, "role": 1})

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "role can only be changed by an administrator"}


def test_update_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/2", json={"height": 182})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": 182, "role": 0}

    resp = client.patch("/api/users/2", json={"height": None})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 0}


def test_update_user_not_found(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/3", json={"name": "Carol"})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_update_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/2", json={"name": " Alice "})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "name is already used"}


def test_update_user_role_change(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/2", json={"role": 1})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 1}


def test_update_own_user_keeps_role(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = client.patch("/api/users/2", json={"name": "Ben", "sex": 0, "height": 182})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Ben", "sex": 0, "height": 182, "role": 0}


def test_update_own_user_role_change_forbidden(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, name="Bob").status_code == HTTPStatus.OK

    resp = client.patch("/api/users/2", json={"role": 1})

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "role can only be changed by an administrator"}


def test_replace_user_role_change(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/2", json={"name": "Bob", "sex": 1, "role": 1})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 1}

    resp = client.put("/api/users/2", json={"name": "Bob", "sex": 1, "role": 0})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Bob", "sex": 1, "height": None, "role": 0}


def test_replace_user_last_admin_role_change(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/users/1", json={"name": "Alice", "sex": 0, "height": 168, "role": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "last administrator cannot be demoted"}

    resp = client.get("/api/users/1")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}


def test_update_user_last_admin_role_change(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/1", json={"role": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "last administrator cannot be demoted"}

    resp = client.get("/api/users/1")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}


def test_delete_user_last_admin(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.delete("/api/users/1")

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "last administrator cannot be deleted"}

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0, "height": 168, "role": 1}


@pytest.mark.parametrize(
    ("user_id", "route", "data"),
    [
        (
            1,
            "/api/body_weight",
            [
                {"date": "2002-02-20", "weight": 67.5},
                {"date": "2002-02-21", "weight": 67.7},
                {"date": "2002-02-22", "weight": 67.3},
            ],
        ),
        (
            1,
            "/api/body_fat",
            [
                {
                    "abdominal": 2,
                    "chest": 1,
                    "date": "2002-02-20",
                    "midaxillary": 7,
                    "subscapular": 5,
                    "suprailiac": 6,
                    "thigh": 3,
                    "tricep": 4,
                },
                {
                    "abdominal": None,
                    "chest": None,
                    "date": "2002-02-21",
                    "midaxillary": None,
                    "subscapular": None,
                    "suprailiac": 13,
                    "thigh": 10,
                    "tricep": 11,
                },
            ],
        ),
        (
            1,
            "/api/period",
            [
                {"date": "2002-02-20", "intensity": 2},
                {"date": "2002-02-21", "intensity": 4},
                {"date": "2002-02-22", "intensity": 1},
            ],
        ),
        (
            1,
            "/api/exercises",
            [
                {"id": 1, "name": "Exercise 1", "muscles": [{"muscle_id": 11, "stimulus": 100}]},
                {"id": 3, "name": "Exercise 3", "muscles": []},
                {"id": 6, "name": "Exercise 6", "muscles": []},
                {"id": 5, "name": "Unused Exercise", "muscles": []},
            ],
        ),
        (
            1,
            "/api/routines",
            [
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "First Routine",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
        ),
        (
            1,
            "/api/workouts",
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
        ),
    ],
)
def test_read_all(client: Client, user_id: int, route: str, data: list[dict[str, object]]) -> None:
    tests.utils.init_db_users()

    username = next(u.name for u in tests.data.users_only() if u.id == user_id)
    assert create_session(client, username).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == []

    tests.utils.clear_db()
    tests.utils.init_db_data()

    assert create_session(client, username).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == data


@pytest.mark.parametrize(
    ("route", "data", "result"),
    [
        (
            "/api/body_weight",
            {"date": "2002-02-24", "weight": 68.1},
            [
                {"date": "2002-02-20", "weight": 67.5},
                {"date": "2002-02-21", "weight": 67.7},
                {"date": "2002-02-22", "weight": 67.3},
                {"date": "2002-02-24", "weight": 68.1},
            ],
        ),
        (
            "/api/body_fat",
            {
                "date": "2002-02-24",
                "chest": 15,
                "abdominal": 16,
                "thigh": 17,
                "tricep": 18,
                "subscapular": 19,
                "suprailiac": 20,
                "midaxillary": None,
            },
            [
                {
                    "date": "2002-02-20",
                    "chest": 1,
                    "abdominal": 2,
                    "thigh": 3,
                    "tricep": 4,
                    "subscapular": 5,
                    "suprailiac": 6,
                    "midaxillary": 7,
                },
                {
                    "date": "2002-02-21",
                    "chest": None,
                    "abdominal": None,
                    "thigh": 10,
                    "tricep": 11,
                    "subscapular": None,
                    "suprailiac": 13,
                    "midaxillary": None,
                },
                {
                    "date": "2002-02-24",
                    "chest": 15,
                    "abdominal": 16,
                    "thigh": 17,
                    "tricep": 18,
                    "suprailiac": 20,
                    "subscapular": 19,
                    "midaxillary": None,
                },
            ],
        ),
        (
            "/api/period",
            {"date": "2002-02-24", "intensity": 1},
            [
                {"date": "2002-02-20", "intensity": 2},
                {"date": "2002-02-21", "intensity": 4},
                {"date": "2002-02-22", "intensity": 1},
                {"date": "2002-02-24", "intensity": 1},
            ],
        ),
        (
            "/api/exercises",
            {
                "id": 7,
                "name": "New Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 100}, {"muscle_id": 21, "stimulus": 50}],
            },
            [
                {"id": 1, "name": "Exercise 1", "muscles": [{"muscle_id": 11, "stimulus": 100}]},
                {"id": 3, "name": "Exercise 3", "muscles": []},
                {"id": 6, "name": "Exercise 6", "muscles": []},
                {
                    "id": 7,
                    "name": "New Exercise",
                    "muscles": [
                        {"muscle_id": 11, "stimulus": 100},
                        {"muscle_id": 21, "stimulus": 50},
                    ],
                },
                {"id": 5, "name": "Unused Exercise", "muscles": []},
            ],
        ),
        (
            "/api/routines",
            {
                "id": 5,
                "name": "New Routine",
                "notes": "Something New",
                "archived": True,
                "sections": [
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": None,
                                        "reps": 0,
                                        "time": 30,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": True,
                                    },
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": True,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 5,
                    "name": "New Routine",
                    "notes": "Something New",
                    "archived": True,
                    "sections": [
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": True,
                                        },
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": True,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "First Routine",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
        ),
    ],
)
def test_create(
    client: Client, route: str, data: dict[str, object], result: list[dict[str, object]]
) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post(route, json=data)

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json == data

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.post(route, json=data)

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": CONFLICT_DETAILS[Path(route).name]}


@pytest.mark.parametrize(
    ("data", "created_id"),
    [
        (
            {
                "date": "2002-02-24",
                "routine_id": 1,
                "notes": "",
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": None,
                        "time": None,
                        "weight": None,
                        "rpe": None,
                        "target_reps": 10,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": 8,
                        "automatic": False,
                    },
                    {
                        "target_time": 60,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": None,
                        "weight": None,
                        "rpe": None,
                        "target_reps": None,
                        "target_time": 120,
                        "target_weight": 10,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "target_time": 120,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": None,
                        "weight": None,
                        "rpe": None,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
                "exercise_notes": [],
            },
            {"id": 6},
        ),
        (
            {
                "date": "2002-02-24",
                "routine_id": None,
                "notes": "",
                "elements": [],
                "exercise_notes": [],
            },
            {"id": 6},
        ),
        (
            {
                "date": "2002-03-01",
                "notes": None,
                "routine_id": None,
                "elements": [
                    {
                        "exercise_id": 1,
                        "reps": 10,
                        "time": None,
                        "weight": None,
                        "rpe": None,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    }
                ],
                "exercise_notes": [
                    {"exercise_id": 1, "notes": "Felt pain in right knee"},
                ],
            },
            {"id": 6},
        ),
    ],
)
def test_create_workout(
    client: Client,
    data: dict[str, object],
    created_id: dict[str, int],
) -> None:
    route = "/api/workouts"
    created = {
        **data,
        **created_id,
    }
    result = [
        {
            "date": "2002-01-11",
            "id": 1,
            "notes": "First Workout",
            "routine_id": 1,
            "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
            "elements": [
                {
                    "exercise_id": 3,
                    "reps": 10,
                    "time": 4,
                    "weight": None,
                    "rpe": 8.0,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 1,
                    "reps": 9,
                    "time": 4,
                    "weight": None,
                    "rpe": 8.5,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 1,
                    "reps": None,
                    "time": 60,
                    "weight": None,
                    "rpe": 9.0,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
            ],
        },
        {
            "date": "2002-02-22",
            "id": 3,
            "notes": None,
            "routine_id": None,
            "exercise_notes": [],
            "elements": [
                {
                    "exercise_id": 3,
                    "reps": 9,
                    "time": None,
                    "weight": None,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 3,
                    "reps": 8,
                    "time": None,
                    "weight": None,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 3,
                    "reps": 7,
                    "time": None,
                    "weight": None,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 6,
                    "reps": 6,
                    "time": None,
                    "weight": None,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 6,
                    "reps": 5,
                    "time": None,
                    "weight": None,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
            ],
        },
        {
            "id": 4,
            "date": "2002-02-24",
            "notes": None,
            "routine_id": 1,
            "exercise_notes": [],
            "elements": [
                {
                    "exercise_id": 3,
                    "reps": 11,
                    "time": 4,
                    "weight": None,
                    "rpe": 8.5,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
            ],
        },
        {
            "id": 5,
            "date": "2002-03-02",
            "notes": None,
            "routine_id": None,
            "exercise_notes": [],
            "elements": [
                {
                    "exercise_id": 6,
                    "reps": 10,
                    "time": None,
                    "weight": 100.0,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
                {
                    "exercise_id": 6,
                    "reps": 8,
                    "time": None,
                    "weight": 105.0,
                    "rpe": None,
                    "target_reps": None,
                    "target_time": None,
                    "target_weight": None,
                    "target_rpe": None,
                    "automatic": False,
                },
            ],
        },
        created,
    ]

    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post(route, json=data)

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json == created

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result


def _workout_with_exercise(exercise_id: int) -> dict[str, object]:
    return {
        "date": "2002-04-01",
        "routine_id": None,
        "notes": None,
        "elements": [
            {
                "exercise_id": exercise_id,
                "reps": 10,
                "time": None,
                "weight": None,
                "rpe": None,
                "target_reps": None,
                "target_time": None,
                "target_weight": None,
                "target_rpe": None,
                "automatic": False,
            }
        ],
        "exercise_notes": [],
    }


def _routine_with_exercise(exercise_id: int) -> dict[str, object]:
    return {
        "name": "New Routine",
        "notes": None,
        "archived": False,
        "sections": [
            {
                "rounds": 1,
                "parts": [
                    {
                        "exercise_id": exercise_id,
                        "reps": 10,
                        "time": 0,
                        "weight": 0.0,
                        "rpe": 0.0,
                        "automatic": False,
                    }
                ],
            }
        ],
    }


# Exercise 2 belongs to Bob, exercise 999 does not exist; Alice must be able to reference neither
@pytest.mark.parametrize("exercise_id", [2, 999])
def test_create_workout_rejects_unknown_exercise(client: Client, exercise_id: int) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/workouts", json=_workout_with_exercise(exercise_id))

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "workout references an unknown exercise"}


@pytest.mark.parametrize("exercise_id", [2, 999])
def test_create_workout_rejects_unknown_exercise_in_notes(client: Client, exercise_id: int) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    data = {
        "date": "2002-04-01",
        "routine_id": None,
        "notes": None,
        "elements": [],
        "exercise_notes": [{"exercise_id": exercise_id, "notes": "note"}],
    }
    resp = client.post("/api/workouts", json=data)

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "workout references an unknown exercise"}


@pytest.mark.parametrize("exercise_id", [2, 999])
def test_update_workout_rejects_unknown_exercise(client: Client, exercise_id: int) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch(
        "/api/workouts/1", json={"elements": _workout_with_exercise(exercise_id)["elements"]}
    )

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "workout references an unknown exercise"}


@pytest.mark.parametrize("exercise_id", [2, 999])
def test_create_routine_rejects_unknown_exercise(client: Client, exercise_id: int) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/routines", json=_routine_with_exercise(exercise_id))

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "routine references an unknown exercise"}


@pytest.mark.parametrize("exercise_id", [2, 999])
def test_update_routine_rejects_unknown_exercise(client: Client, exercise_id: int) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch(
        "/api/routines/1", json={"sections": _routine_with_exercise(exercise_id)["sections"]}
    )

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "routine references an unknown exercise"}


def test_create_workout_allows_own_unused_exercise(client: Client) -> None:
    tests.utils.init_db_data()
    assert create_session(client).status_code == HTTPStatus.OK

    # Exercise 5 belongs to Alice but is referenced by none of her workouts
    resp = client.post("/api/workouts", json=_workout_with_exercise(5))

    assert resp.status_code == HTTPStatus.CREATED


@pytest.mark.parametrize(
    ("route", "data", "response", "result", "conflicting_data"),
    [
        (
            "/api/body_weight/2002-02-20",
            {"weight": 68.1},
            {"date": "2002-02-20", "weight": 68.1},
            [
                {"date": "2002-02-20", "weight": 68.1},
                {"date": "2002-02-21", "weight": 67.7},
                {"date": "2002-02-22", "weight": 67.3},
            ],
            None,
        ),
        (
            "/api/body_fat/2002-02-20",
            {
                "chest": 29,
                "abdominal": 30,
                "thigh": 31,
                "tricep": 32,
                "subscapular": 33,
                "suprailiac": 34,
                "midaxillary": None,
            },
            {
                "date": "2002-02-20",
                "chest": 29,
                "abdominal": 30,
                "thigh": 31,
                "tricep": 32,
                "subscapular": 33,
                "suprailiac": 34,
                "midaxillary": None,
            },
            [
                {
                    "date": "2002-02-20",
                    "chest": 29,
                    "abdominal": 30,
                    "thigh": 31,
                    "tricep": 32,
                    "subscapular": 33,
                    "suprailiac": 34,
                    "midaxillary": None,
                },
                {
                    "date": "2002-02-21",
                    "chest": None,
                    "abdominal": None,
                    "thigh": 10,
                    "tricep": 11,
                    "subscapular": None,
                    "suprailiac": 13,
                    "midaxillary": None,
                },
            ],
            None,
        ),
        (
            "/api/period/2002-02-20",
            {"intensity": 3},
            {"date": "2002-02-20", "intensity": 3},
            [
                {"date": "2002-02-20", "intensity": 3},
                {"date": "2002-02-21", "intensity": 4},
                {"date": "2002-02-22", "intensity": 1},
            ],
            None,
        ),
        (
            "/api/exercises/1",
            {
                "name": "Changed Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 50}, {"muscle_id": 21, "stimulus": 100}],
            },
            {
                "id": 1,
                "name": "Changed Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 50}, {"muscle_id": 21, "stimulus": 100}],
            },
            [
                {
                    "id": 1,
                    "name": "Changed Exercise",
                    "muscles": [
                        {"muscle_id": 11, "stimulus": 50},
                        {"muscle_id": 21, "stimulus": 100},
                    ],
                },
                {"id": 3, "name": "Exercise 3", "muscles": []},
                {"id": 6, "name": "Exercise 6", "muscles": []},
                {"id": 5, "name": "Unused Exercise", "muscles": []},
            ],
            {"name": "Exercise 3", "muscles": []},
        ),
        (
            "/api/routines/1",
            {
                "name": "Changed Routine",
                "notes": "First Changed Routine",
                "archived": True,
                "sections": [
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            {
                "id": 1,
                "name": "Changed Routine",
                "notes": "First Changed Routine",
                "archived": True,
                "sections": [
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "name": "Changed Routine",
                    "notes": "First Changed Routine",
                    "archived": True,
                    "sections": [
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
            {
                "id": 3,
                "name": "R2",
                "notes": "",
                "archived": False,
                "sections": [],
            },
        ),
        (
            "/api/workouts/1",
            {
                "date": "2002-02-23",
                "notes": "",
                "elements": [
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": 10,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": 8,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": 120,
                        "target_weight": 10,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
                "exercise_notes": [],
            },
            {
                "id": 1,
                "routine_id": 1,
                "date": "2002-02-23",
                "notes": "",
                "exercise_notes": [],
                "elements": [
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": 10,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": 8,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": 120,
                        "target_weight": 10,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "routine_id": 1,
                    "date": "2002-02-23",
                    "notes": "",
                    "elements": [
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": 10,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": 8,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": 120,
                            "target_weight": 10,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "notes": None,
                    "routine_id": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
        (
            "/api/workouts/1",
            {
                "date": "2002-01-11",
                "notes": "First Workout",
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
                "exercise_notes": [{"exercise_id": 1, "notes": "Good range of motion"}],
            },
            {
                "id": 1,
                "routine_id": 1,
                "date": "2002-01-11",
                "notes": "First Workout",
                "exercise_notes": [{"exercise_id": 1, "notes": "Good range of motion"}],
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "routine_id": 1,
                    "date": "2002-01-11",
                    "notes": "First Workout",
                    "exercise_notes": [{"exercise_id": 1, "notes": "Good range of motion"}],
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "notes": None,
                    "routine_id": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
    ],
)
def test_replace(
    client: Client,
    route: str,
    data: dict[str, object],
    response: dict[str, object],
    result: list[dict[str, object]],
    conflicting_data: dict[str, object],
) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put(route, json=data)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == response

    resp = client.get(str(Path(route).parent))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.put(str(Path(route).parent / "0"), json=data)

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    if conflicting_data is not None:
        resp = client.put(route, json=conflicting_data)

        assert resp.status_code == HTTPStatus.CONFLICT
        assert resp.json == {"details": CONFLICT_DETAILS[Path(route).parent.name]}


@pytest.mark.parametrize(
    ("route", "data", "response", "result", "conflicting_data"),
    [
        (
            "/api/routines/1",
            {
                "name": "Changed Routine",
            },
            {
                "id": 1,
                "name": "Changed Routine",
                "notes": "First Routine",
                "archived": False,
                "sections": [
                    {
                        "rounds": 1,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 30,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 60,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                    {
                                        "exercise_id": None,
                                        "reps": 0,
                                        "time": 30,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "name": "Changed Routine",
                    "notes": "First Routine",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
            {
                "name": "R2",
            },
        ),
        (
            "/api/routines/1",
            {
                "notes": "Changed Notes",
            },
            {
                "id": 1,
                "name": "R1",
                "notes": "Changed Notes",
                "archived": False,
                "sections": [
                    {
                        "rounds": 1,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 30,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 60,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                    {
                                        "exercise_id": None,
                                        "reps": 0,
                                        "time": 30,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "Changed Notes",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
            {
                "name": "R2",
                "notes": "",
            },
        ),
        (
            "/api/routines/1",
            {
                "archived": True,
            },
            {
                "id": 1,
                "name": "R1",
                "notes": "First Routine",
                "archived": True,
                "sections": [
                    {
                        "rounds": 1,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 30,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 60,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                    {
                                        "exercise_id": None,
                                        "reps": 0,
                                        "time": 30,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "First Routine",
                    "archived": True,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
            {
                "name": "R2",
            },
        ),
        (
            "/api/routines/1",
            {
                "sections": [
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            {
                "id": 1,
                "name": "R1",
                "notes": "First Routine",
                "archived": False,
                "sections": [
                    {
                        "rounds": 3,
                        "parts": [
                            {
                                "exercise_id": 1,
                                "reps": 0,
                                "time": 0,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": False,
                            },
                            {
                                "rounds": 2,
                                "parts": [
                                    {
                                        "exercise_id": 1,
                                        "reps": 0,
                                        "time": 0,
                                        "weight": 0.0,
                                        "rpe": 0.0,
                                        "automatic": False,
                                    },
                                ],
                            },
                        ],
                    },
                    {
                        "rounds": 2,
                        "parts": [
                            {
                                "exercise_id": 3,
                                "reps": 0,
                                "time": 20,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                            {
                                "exercise_id": None,
                                "reps": 0,
                                "time": 10,
                                "weight": 0.0,
                                "rpe": 0.0,
                                "automatic": True,
                            },
                        ],
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "First Routine",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
                {
                    "id": 3,
                    "name": "R2",
                    "notes": None,
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 5,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        }
                    ],
                },
            ],
            {"name": "R2", "notes": "", "archived": False, "sections": []},
        ),
        (
            "/api/workouts/1",
            {
                "date": "2002-02-23",
            },
            {
                "id": 1,
                "date": "2002-02-23",
                "routine_id": 1,
                "notes": "First Workout",
                "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "date": "2002-02-23",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
        (
            "/api/workouts/1",
            {
                "notes": "",
            },
            {
                "id": 1,
                "date": "2002-01-11",
                "routine_id": 1,
                "notes": "",
                "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "",
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
        (
            "/api/workouts/1",
            {
                "elements": [
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": 10,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": 8,
                        "automatic": False,
                    },
                    {
                        "target_time": 120,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": 120,
                        "target_weight": 10,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            {
                "id": 1,
                "routine_id": 1,
                "date": "2002-01-11",
                "notes": "First Workout",
                "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                "elements": [
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": 10,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": 8,
                        "automatic": False,
                    },
                    {
                        "target_time": 120,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": 120,
                        "target_weight": 10,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "elements": [
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": 10,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": 8,
                            "automatic": False,
                        },
                        {
                            "target_time": 120,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": 120,
                            "target_weight": 10,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
        (
            "/api/workouts/1",
            {
                "exercise_notes": [{"exercise_id": 3, "notes": "Knee pain"}],
            },
            {
                "id": 1,
                "date": "2002-01-11",
                "routine_id": 1,
                "notes": "First Workout",
                "exercise_notes": [{"exercise_id": 3, "notes": "Knee pain"}],
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "exercise_notes": [{"exercise_id": 3, "notes": "Knee pain"}],
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
        (
            "/api/workouts/1",
            {
                "exercise_notes": [{"exercise_id": 1, "notes": ""}],
            },
            {
                "id": 1,
                "date": "2002-01-11",
                "routine_id": 1,
                "notes": "First Workout",
                "exercise_notes": [],
                "elements": [
                    {
                        "exercise_id": 3,
                        "reps": 10,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": 9,
                        "time": 4,
                        "weight": None,
                        "rpe": 8.5,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                    {
                        "exercise_id": 1,
                        "reps": None,
                        "time": 60,
                        "weight": None,
                        "rpe": 9.0,
                        "target_reps": None,
                        "target_time": None,
                        "target_weight": None,
                        "target_rpe": None,
                        "automatic": False,
                    },
                ],
            },
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
                {
                    "id": 3,
                    "date": "2002-02-22",
                    "routine_id": None,
                    "notes": None,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 9,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 8,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 3,
                            "reps": 7,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 6,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 5,
                            "time": None,
                            "weight": None,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
            None,
        ),
    ],
)
def test_modify(
    client: Client,
    route: str,
    data: dict[str, object],
    response: dict[str, object],
    result: list[dict[str, object]],
    conflicting_data: dict[str, object],
) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch(route, json=data)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == response

    resp = client.get(str(Path(route).parent))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.patch(str(Path(route).parent / "0"), json=data)

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    if conflicting_data is not None:
        resp = client.patch(route, json=conflicting_data)

        assert resp.status_code == HTTPStatus.CONFLICT
        assert resp.json == {"details": CONFLICT_DETAILS[Path(route).parent.name]}


@pytest.mark.parametrize(
    ("route", "result"),
    [
        (
            "/api/body_weight/2002-02-21",
            [
                {"date": "2002-02-20", "weight": 67.5},
                {"date": "2002-02-22", "weight": 67.3},
            ],
        ),
        (
            "/api/body_fat/2002-02-21",
            [
                {
                    "date": "2002-02-20",
                    "chest": 1,
                    "abdominal": 2,
                    "thigh": 3,
                    "tricep": 4,
                    "subscapular": 5,
                    "suprailiac": 6,
                    "midaxillary": 7,
                },
            ],
        ),
        (
            "/api/period/2002-02-21",
            [
                {"date": "2002-02-20", "intensity": 2},
                {"date": "2002-02-22", "intensity": 1},
            ],
        ),
        (
            "/api/exercises/3",
            [
                {"id": 1, "name": "Exercise 1", "muscles": [{"muscle_id": 11, "stimulus": 100}]},
                {"id": 6, "name": "Exercise 6", "muscles": []},
                {"id": 5, "name": "Unused Exercise", "muscles": []},
            ],
        ),
        (
            "/api/routines/3",
            [
                {
                    "id": 1,
                    "name": "R1",
                    "notes": "First Routine",
                    "archived": False,
                    "sections": [
                        {
                            "rounds": 1,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 30,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                            ],
                        },
                        {
                            "rounds": 2,
                            "parts": [
                                {
                                    "exercise_id": 1,
                                    "reps": 0,
                                    "time": 0,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 60,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": False,
                                },
                                {
                                    "rounds": 2,
                                    "parts": [
                                        {
                                            "exercise_id": 1,
                                            "reps": 0,
                                            "time": 0,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                        {
                                            "exercise_id": None,
                                            "reps": 0,
                                            "time": 30,
                                            "weight": 0.0,
                                            "rpe": 0.0,
                                            "automatic": False,
                                        },
                                    ],
                                },
                            ],
                        },
                        {
                            "rounds": 3,
                            "parts": [
                                {
                                    "exercise_id": 3,
                                    "reps": 0,
                                    "time": 20,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                                {
                                    "exercise_id": None,
                                    "reps": 0,
                                    "time": 10,
                                    "weight": 0.0,
                                    "rpe": 0.0,
                                    "automatic": True,
                                },
                            ],
                        },
                    ],
                },
            ],
        ),
        (
            "/api/workouts/3",
            [
                {
                    "id": 1,
                    "date": "2002-01-11",
                    "routine_id": 1,
                    "notes": "First Workout",
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 10,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": 9,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 1,
                            "reps": None,
                            "time": 60,
                            "weight": None,
                            "rpe": 9.0,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [{"exercise_id": 3, "notes": "Pain"}],
                },
                {
                    "id": 4,
                    "date": "2002-02-24",
                    "notes": None,
                    "routine_id": 1,
                    "elements": [
                        {
                            "exercise_id": 3,
                            "reps": 11,
                            "time": 4,
                            "weight": None,
                            "rpe": 8.5,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                    "exercise_notes": [],
                },
                {
                    "id": 5,
                    "date": "2002-03-02",
                    "notes": None,
                    "routine_id": None,
                    "exercise_notes": [],
                    "elements": [
                        {
                            "exercise_id": 6,
                            "reps": 10,
                            "time": None,
                            "weight": 100.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                        {
                            "exercise_id": 6,
                            "reps": 8,
                            "time": None,
                            "weight": 105.0,
                            "rpe": None,
                            "target_reps": None,
                            "target_time": None,
                            "target_weight": None,
                            "target_rpe": None,
                            "automatic": False,
                        },
                    ],
                },
            ],
        ),
    ],
)
def test_delete(
    client: Client,
    route: str,
    result: list[dict[str, object]],
) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.delete(route)

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data

    resp = client.get(str(Path(route).parent))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.delete(str(Path(route).parent / "0"))

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


SCHEDULE = {
    "rotations": [{"id": 1, "name": "A/B", "routines": [1, 3]}],
    "entries": [
        {"weekday": 1, "slots": [{"rotation": 1}, {"routine": 1}]},
        {"weekday": 3, "slots": [{"rotation": 1}]},
    ],
}


def test_read_schedule_empty(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"rotations": [], "entries": []}


def test_replace_schedule(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/schedule", json=SCHEDULE)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == SCHEDULE

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == SCHEDULE


def test_replace_schedule_replaces_existing(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK
    assert client.put("/api/schedule", json=SCHEDULE).status_code == HTTPStatus.OK

    schedule = {
        "rotations": [],
        "entries": [{"weekday": 5, "slots": [{"routine": 3}]}],
    }

    resp = client.put("/api/schedule", json=schedule)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == schedule

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == schedule

    resp = client.put("/api/schedule", json={"rotations": [], "entries": []})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"rotations": [], "entries": []}


def test_replace_schedule_only_affects_session_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK
    assert client.put("/api/schedule", json=SCHEDULE).status_code == HTTPStatus.OK
    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {
        "rotations": [{"id": 1, "name": "A/B", "routines": [2, 4]}],
        "entries": [{"weekday": 1, "slots": [{"rotation": 1}, {"routine": 2}]}],
    }

    resp = client.put("/api/schedule", json={"rotations": [], "entries": []})

    assert resp.status_code == HTTPStatus.OK

    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == SCHEDULE


def test_replace_schedule_same_rotation_id_for_different_users(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK
    assert client.put("/api/schedule", json=SCHEDULE).status_code == HTTPStatus.OK
    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    schedule = {
        "rotations": [{"id": 1, "name": "C/D", "routines": [2, 4]}],
        "entries": [{"weekday": 2, "slots": [{"rotation": 1}]}],
    }

    resp = client.put("/api/schedule", json=schedule)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == schedule

    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/schedule")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == SCHEDULE


@pytest.mark.parametrize(
    ("schedule", "details"),
    [
        (
            {
                "rotations": [],
                "entries": [{"weekday": 1, "slots": [{"rotation": 1}]}],
            },
            "schedule references an unknown rotation",
        ),
        (
            {
                "rotations": [{"id": 1, "name": "A/B", "routines": [1, 2]}],
                "entries": [],
            },
            "schedule references an unknown routine",
        ),
        (
            {
                "rotations": [],
                "entries": [{"weekday": 1, "slots": [{"routine": 2}]}],
            },
            "schedule references an unknown routine",
        ),
        (
            {
                "rotations": [
                    {"id": 1, "name": "A/B", "routines": [1]},
                    {"id": 2, "name": "A/B", "routines": [3]},
                ],
                "entries": [],
            },
            "rotation name is already used",
        ),
    ],
)
def test_replace_schedule_conflict(
    client: Client, schedule: dict[str, object], details: str
) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.put("/api/schedule", json=schedule)

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": details}


@pytest.mark.parametrize("routine_id", [1, 3])
def test_delete_routine_blocked_by_schedule(client: Client, routine_id: int) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK
    assert client.put("/api/schedule", json=SCHEDULE).status_code == HTTPStatus.OK

    resp = client.delete(f"/api/routines/{routine_id}")

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "routine is used in the schedule"}

    assert (
        client.put("/api/schedule", json={"rotations": [], "entries": []}).status_code
        == HTTPStatus.OK
    )

    resp = client.delete(f"/api/routines/{routine_id}")

    assert resp.status_code == HTTPStatus.NO_CONTENT


@pytest.mark.parametrize(
    ("route", "mutate"),
    [
        ("/api/body_weight", lambda c: c.delete("/api/body_weight/2002-02-20")),
        ("/api/body_fat", lambda c: c.delete("/api/body_fat/2002-02-20")),
        ("/api/period", lambda c: c.delete("/api/period/2002-02-20")),
        ("/api/exercises", lambda c: c.delete("/api/exercises/5")),
        ("/api/routines", lambda c: c.patch("/api/routines/3", json={"name": "Renamed"})),
        ("/api/schedule", lambda c: c.put("/api/schedule", json={"rotations": [], "entries": []})),
        ("/api/workouts", lambda c: c.patch("/api/workouts/1", json={"notes": "changed"})),
    ],
)
def test_etag_conditional(client: Client, route: str, mutate: Callable[[Client], Response]) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get(route)
    assert resp.status_code == HTTPStatus.OK
    etag = resp.headers["ETag"]
    assert etag

    resp = client.get(route, headers={"If-None-Match": etag})
    assert resp.status_code == HTTPStatus.NOT_MODIFIED
    assert resp.data == b""

    assert mutate(client).status_code in {
        HTTPStatus.OK,
        HTTPStatus.CREATED,
        HTTPStatus.NO_CONTENT,
    }

    resp = client.get(route, headers={"If-None-Match": etag})
    assert resp.status_code == HTTPStatus.OK
    assert resp.headers["ETag"] != etag


def test_etag_increments_on_repeated_mutation(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    etag_0 = client.get("/api/body_weight").headers["ETag"]

    assert (
        client.post("/api/body_weight", json={"date": "2002-02-24", "weight": 68.1}).status_code
        == HTTPStatus.CREATED
    )
    etag_1 = client.get("/api/body_weight").headers["ETag"]

    assert (
        client.post("/api/body_weight", json={"date": "2002-02-25", "weight": 68.2}).status_code
        == HTTPStatus.CREATED
    )
    etag_2 = client.get("/api/body_weight").headers["ETag"]

    assert etag_0 != etag_1
    assert etag_1 != etag_2
    assert etag_0 != etag_2


def test_etag_delete_exercise_bumps_related_collections(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    related = ["/api/exercises", "/api/workouts", "/api/routines"]
    before = {route: client.get(route).headers["ETag"] for route in [*related, "/api/body_weight"]}

    # Exercise 3 is referenced by workouts and routines, so its deletion cascades
    assert client.delete("/api/exercises/3").status_code == HTTPStatus.NO_CONTENT

    for route in related:
        assert client.get(route).headers["ETag"] != before[route]

    # An unrelated collection keeps its ETag
    assert client.get("/api/body_weight").headers["ETag"] == before["/api/body_weight"]


def test_etag_delete_routine_bumps_workouts(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    related = ["/api/routines", "/api/workouts"]
    before = {route: client.get(route).headers["ETag"] for route in [*related, "/api/exercises"]}

    # Routine 1 is referenced by workouts, so its deletion detaches them
    assert client.delete("/api/routines/1").status_code == HTTPStatus.NO_CONTENT

    for route in related:
        assert client.get(route).headers["ETag"] != before[route]

    # An unrelated collection keeps its ETag
    assert client.get("/api/exercises").headers["ETag"] == before["/api/exercises"]


def test_conditional_response_is_privately_cached(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/body_weight")
    assert resp.cache_control.private
    assert resp.cache_control.no_cache
    etag = resp.headers["ETag"]

    resp = client.get("/api/body_weight", headers={"If-None-Match": etag})
    assert resp.status_code == HTTPStatus.NOT_MODIFIED
    assert resp.headers["ETag"] == etag
    assert resp.cache_control.private
    assert resp.cache_control.no_cache


def test_etag_is_per_user(client: Client) -> None:
    tests.utils.init_db_data()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK
    bob_etag = client.get("/api/body_weight").headers["ETag"]

    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client, "Alice").status_code == HTTPStatus.OK
    # Both users are at the same data version, but a browser HTTP cache shared between
    # them must never get a 304 for the other user's cached response.
    assert client.get("/api/body_weight").headers["ETag"] != bob_etag
    assert (
        client.get("/api/body_weight", headers={"If-None-Match": bob_etag}).status_code
        == HTTPStatus.OK
    )
    assert client.delete("/api/body_weight/2002-02-20").status_code == HTTPStatus.NO_CONTENT

    assert delete_session(client).status_code == HTTPStatus.NO_CONTENT
    assert create_session(client, "Bob").status_code == HTTPStatus.OK
    assert client.get("/api/body_weight").headers["ETag"] == bob_etag
