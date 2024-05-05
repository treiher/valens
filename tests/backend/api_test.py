from __future__ import annotations

from collections.abc import Generator
from http import HTTPStatus
from pathlib import Path

import pytest
from werkzeug.test import Client, TestResponse as Response

import tests.data
import tests.utils
from valens import app


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client, app.app_context():
        yield client


def create_session(client: Client, user_id: int = 1) -> Response:
    return client.post("/api/session", json={"id": user_id})


def delete_session(client: Client) -> Response:
    return client.delete("/api/session")


@pytest.mark.parametrize(
    ("method", "route"),
    [
        ("get", "/api/users/1"),
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
        ("post", "/api/users"),
        ("put", "/api/users/2"),
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
        ("post", "/api/users", {"invalid": "data"}),
        ("put", "/api/users/2", {"invalid": "data"}),
        ("post", "/api/body_weight", {"invalid": "data"}),
        ("put", "/api/body_weight/2002-02-22", {"invalid": "data"}),
        ("post", "/api/body_fat", {"invalid": "data"}),
        ("put", "/api/body_fat/2002-02-20", {"invalid": "data"}),
        ("post", "/api/period", {"invalid": "data"}),
        ("put", "/api/period/2002-02-22", {"invalid": "data"}),
        ("post", "/api/exercises", {"invalid": "data"}),
        ("post", "/api/exercises", {"name": "data", "muscles": [{"invalid": "data"}]}),
        ("put", "/api/exercises/1", {"invalid": "data"}),
        ("post", "/api/routines", {"invalid": "data"}),
        ("put", "/api/routines/1", {"invalid": "data"}),
        ("patch", "/api/routines/1", {"sections": [{"invalid": "data"}]}),
        ("post", "/api/workouts", {"invalid": "data"}),
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
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = client.get("/api/session")
    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = delete_session(client)
    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data


def test_read_session_not_found(client: Client) -> None:
    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_create_session_not_found(client: Client) -> None:
    resp = client.post("/api/session", json={"id": 1})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_read_users(client: Client) -> None:
    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == []

    tests.utils.init_db_data()
    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Bob", "sex": 1},
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
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = delete_session(client)
    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data


def test_create_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.post("/api/users", json={"name": "Carol", "sex": 0})

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json == {"id": 3, "name": "Carol", "sex": 0}

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Bob", "sex": 1},
        {"id": 3, "name": "Carol", "sex": 0},
    ]


def test_create_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.post("/api/users", json={"name": " Alice ", "sex": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json


def test_replace_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/2", json={"name": "Carol", "sex": 0})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 2, "name": "Carol", "sex": 0}

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Carol", "sex": 0},
    ]


def test_replace_user_not_found(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/3", json={"name": "Carol", "sex": 0})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_replace_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/2", json={"name": " Alice ", "sex": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json


def test_delete_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.delete("/api/users/2")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data

    resp = client.get("/api/users")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
    ]

    resp = client.delete("/api/users/2")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


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
                {"id": 3, "name": "Exercise 2", "muscles": []},
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
                    "date": "2002-02-20",
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
                            "exercise_id": 3,
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
                            "exercise_id": 3,
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
            ],
        ),
    ],
)
def test_read_all(client: Client, user_id: int, route: str, data: list[dict[str, object]]) -> None:
    tests.utils.init_db_users()

    assert create_session(client, user_id).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == []

    tests.utils.clear_db()
    tests.utils.init_db_data()

    assert create_session(client, user_id).status_code == HTTPStatus.OK

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
                "id": 6,
                "name": "New Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 100}, {"muscle_id": 12, "stimulus": 50}],
            },
            [
                {"id": 1, "name": "Exercise 1", "muscles": [{"muscle_id": 11, "stimulus": 100}]},
                {"id": 3, "name": "Exercise 2", "muscles": []},
                {
                    "id": 6,
                    "name": "New Exercise",
                    "muscles": [
                        {"muscle_id": 11, "stimulus": 100},
                        {"muscle_id": 12, "stimulus": 50},
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
    assert resp.json


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
            },
            {"id": 5},
        ),
        (
            {
                "date": "2002-02-24",
                "routine_id": None,
                "notes": "",
                "elements": [],
            },
            {"id": 5},
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
            "date": "2002-02-20",
            "id": 1,
            "notes": "First Workout",
            "routine_id": 1,
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
                    "exercise_id": 3,
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
                    "exercise_id": 3,
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
                {
                    "exercise_id": 1,
                    "reps": 9,
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
                    "reps": None,
                    "time": 60,
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
            {"weight": 0},
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
            {
                "chest": 0,
                "abdominal": 0,
                "thigh": 0,
                "tricep": 0,
                "subscapular": 0,
                "suprailiac": 0,
                "midaxillary": 0,
            },
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
            {"intensity": 0},
        ),
        (
            "/api/exercises/1",
            {
                "name": "Changed Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 50}, {"muscle_id": 12, "stimulus": 100}],
            },
            {
                "id": 1,
                "name": "Changed Exercise",
                "muscles": [{"muscle_id": 11, "stimulus": 50}, {"muscle_id": 12, "stimulus": 100}],
            },
            [
                {
                    "id": 1,
                    "name": "Changed Exercise",
                    "muscles": [
                        {"muscle_id": 11, "stimulus": 50},
                        {"muscle_id": 12, "stimulus": 100},
                    ],
                },
                {"id": 3, "name": "Exercise 2", "muscles": []},
                {"id": 5, "name": "Unused Exercise", "muscles": []},
            ],
            {"name": "Exercise 2", "muscles": []},
        ),
        (
            "/api/routines/1",
            {
                "name": "Changed Routine",
                "notes": "First Changed Routine",
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
            },
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
                            "exercise_id": 3,
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
                            "exercise_id": 3,
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
        assert resp.json


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
            {"name": "R2", "notes": "", "sections": []},
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
                            "exercise_id": 3,
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
                            "exercise_id": 3,
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
                "date": "2002-02-20",
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
            },
            [
                {
                    "id": 1,
                    "date": "2002-02-20",
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
                            "exercise_id": 3,
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
                            "exercise_id": 3,
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
                "date": "2002-02-20",
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
            },
            [
                {
                    "id": 1,
                    "date": "2002-02-20",
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
                            "exercise_id": 3,
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
                            "exercise_id": 3,
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
        assert resp.json


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
                    "date": "2002-02-20",
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
                        {
                            "exercise_id": 1,
                            "reps": 9,
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
                            "reps": None,
                            "time": 60,
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
