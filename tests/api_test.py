from __future__ import annotations

from http import HTTPStatus
from pathlib import Path
from typing import Generator

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

    with app.test_client() as client:
        with app.app_context():
            yield client


def add_session(client: Client, user_id: int = 1) -> Response:
    return client.post("/api/session", json={"id": user_id})


def delete_session(client: Client) -> Response:
    return client.delete("/api/session")


@pytest.mark.parametrize(
    "method, route",
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
    ],
)
def test_session_required(client: Client, method: str, route: str) -> None:
    resp = getattr(client, method)(route)

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert not resp.data


@pytest.mark.parametrize(
    "method, route",
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
    ],
)
def test_json_required(client: Client, method: str, route: str) -> None:
    tests.utils.init_db_data()

    assert add_session(client).status_code == HTTPStatus.OK

    resp = getattr(client, method)(route, data={})

    assert resp.status_code == HTTPStatus.UNSUPPORTED_MEDIA_TYPE
    assert not resp.data


@pytest.mark.parametrize(
    "method, route",
    [
        ("post", "/api/session"),
        ("post", "/api/users"),
        ("put", "/api/users/2"),
        ("post", "/api/body_weight"),
        ("put", "/api/body_weight/2002-02-22"),
        ("post", "/api/body_fat"),
        ("put", "/api/body_fat/2002-02-20"),
        ("post", "/api/period"),
        ("put", "/api/period/2002-02-22"),
        ("post", "/api/exercises"),
        ("put", "/api/exercises/1"),
    ],
)
def test_invalid_data(client: Client, method: str, route: str) -> None:
    tests.utils.init_db_data()

    assert add_session(client).status_code == HTTPStatus.OK

    resp = getattr(client, method)(route, json={"invalid": "data"})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.is_json


def test_get_version(client: Client) -> None:
    resp = client.get("/api/version")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json


def test_session(client: Client) -> None:
    tests.utils.init_db_data()

    resp = add_session(client)
    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = client.get("/api/session")
    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = delete_session(client)
    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data


def test_get_session_not_found(client: Client) -> None:
    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_add_session_not_found(client: Client) -> None:
    resp = client.post("/api/session", json={"id": 1})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_get_users(client: Client) -> None:
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


def test_get_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = add_session(client)
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


def test_add_user(client: Client) -> None:
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


def test_add_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.post("/api/users", json={"name": " Alice ", "sex": 0})

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json


def test_edit_user(client: Client) -> None:
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


def test_edit_user_not_found(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/3", json={"name": "Carol", "sex": 0})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_edit_user_conflict(client: Client) -> None:
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
    "user_id, route, data",
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
            2,
            "/api/body_weight?format=statistics",
            [
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-02-20",
                    "weight": 100.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-02-21",
                    "weight": 101.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-02-22",
                    "weight": 102.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-02-24",
                    "weight": 104.0,
                },
                {
                    "avg_weight": 105.0,
                    "avg_weight_change": None,
                    "date": "2002-02-25",
                    "weight": 105.0,
                },
                {
                    "avg_weight": 106.22222222222223,
                    "avg_weight_change": None,
                    "date": "2002-02-26",
                    "weight": 106.0,
                },
                {
                    "avg_weight": 107.55555555555556,
                    "avg_weight_change": None,
                    "date": "2002-02-28",
                    "weight": 108.0,
                },
                {
                    "avg_weight": 108.88888888888889,
                    "avg_weight_change": None,
                    "date": "2002-03-01",
                    "weight": 109.0,
                },
                {
                    "avg_weight": 110.11111111111111,
                    "avg_weight_change": None,
                    "date": "2002-03-02",
                    "weight": 110.0,
                },
                {
                    "avg_weight": 111.33333333333333,
                    "avg_weight_change": None,
                    "date": "2002-03-03",
                    "weight": 111.0,
                },
                {
                    "avg_weight": 112.66666666666667,
                    "avg_weight_change": 6.066945606694563,
                    "date": "2002-03-05",
                    "weight": 113.0,
                },
                {
                    "avg_weight": 113.88888888888889,
                    "avg_weight_change": 6.548856548856552,
                    "date": "2002-03-06",
                    "weight": 114.0,
                },
                {
                    "avg_weight": 115.11111111111111,
                    "avg_weight_change": 7.024793388429762,
                    "date": "2002-03-07",
                    "weight": 115.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-03-08",
                    "weight": 116.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-03-10",
                    "weight": 118.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-03-11",
                    "weight": 119.0,
                },
                {
                    "avg_weight": None,
                    "avg_weight_change": None,
                    "date": "2002-03-12",
                    "weight": 120.0,
                },
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
                    "tigh": 3,
                    "tricep": 4,
                },
                {
                    "abdominal": None,
                    "chest": None,
                    "date": "2002-02-21",
                    "midaxillary": None,
                    "subscapular": None,
                    "suprailiac": 13,
                    "tigh": 10,
                    "tricep": 11,
                },
            ],
        ),
        (
            1,
            "/api/body_fat?format=statistics",
            [
                {
                    "abdominal": 2,
                    "chest": 1,
                    "date": "2002-02-20",
                    "jp3": 7.14935882262705,
                    "jp7": 8.147206788471749,
                    "midaxillary": 7,
                    "subscapular": 5,
                    "suprailiac": 6,
                    "tigh": 3,
                    "tricep": 4,
                },
                {
                    "abdominal": None,
                    "chest": None,
                    "date": "2002-02-21",
                    "jp3": 15.131007672030591,
                    "jp7": None,
                    "midaxillary": None,
                    "subscapular": None,
                    "suprailiac": 13,
                    "tigh": 10,
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
                {"id": 1, "name": "Exercise 1"},
                {"id": 3, "name": "Exercise 2"},
                {"id": 5, "name": "Unused Exercise"},
            ],
        ),
    ],
)
def test_get(client: Client, user_id: int, route: str, data: list[dict[str, object]]) -> None:
    tests.utils.init_db_users()

    assert add_session(client, user_id).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == []

    tests.utils.clear_db()
    tests.utils.init_db_data()

    assert add_session(client, user_id).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == data


@pytest.mark.parametrize(
    "route, data, result",
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
                "tigh": 17,
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
                    "tigh": 3,
                    "tricep": 4,
                    "subscapular": 5,
                    "suprailiac": 6,
                    "midaxillary": 7,
                },
                {
                    "date": "2002-02-21",
                    "chest": None,
                    "abdominal": None,
                    "tigh": 10,
                    "tricep": 11,
                    "subscapular": None,
                    "suprailiac": 13,
                    "midaxillary": None,
                },
                {
                    "date": "2002-02-24",
                    "chest": 15,
                    "abdominal": 16,
                    "tigh": 17,
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
            {"id": 6, "name": "New Exercise"},
            [
                {"id": 1, "name": "Exercise 1"},
                {"id": 3, "name": "Exercise 2"},
                {"id": 6, "name": "New Exercise"},
                {"id": 5, "name": "Unused Exercise"},
            ],
        ),
    ],
)
def test_add(
    client: Client, route: str, data: dict[str, object], result: list[dict[str, object]]
) -> None:
    tests.utils.init_db_data()

    assert add_session(client).status_code == HTTPStatus.OK

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
    "route, data, response, result, conflicting_data",
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
                "tigh": 31,
                "tricep": 32,
                "subscapular": 33,
                "suprailiac": 34,
                "midaxillary": None,
            },
            {
                "date": "2002-02-20",
                "chest": 29,
                "abdominal": 30,
                "tigh": 31,
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
                    "tigh": 31,
                    "tricep": 32,
                    "subscapular": 33,
                    "suprailiac": 34,
                    "midaxillary": None,
                },
                {
                    "date": "2002-02-21",
                    "chest": None,
                    "abdominal": None,
                    "tigh": 10,
                    "tricep": 11,
                    "subscapular": None,
                    "suprailiac": 13,
                    "midaxillary": None,
                },
            ],
            {
                "chest": 0,
                "abdominal": 0,
                "tigh": 0,
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
            {"name": "Changed Exercise"},
            {"id": 1, "name": "Changed Exercise"},
            [
                {"id": 1, "name": "Changed Exercise"},
                {"id": 3, "name": "Exercise 2"},
                {"id": 5, "name": "Unused Exercise"},
            ],
            {"name": "Exercise 2"},
        ),
    ],
)
def test_edit(
    client: Client,
    route: str,
    data: dict[str, object],
    response: dict[str, object],
    result: list[dict[str, object]],
    conflicting_data: dict[str, object],
) -> None:
    tests.utils.init_db_data()

    assert add_session(client).status_code == HTTPStatus.OK

    resp = client.put(route, json=data)

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == response

    resp = client.get(str(Path(route).parent))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.put(str(Path(route).parent / "0"), json=data)

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    resp = client.put(route, json=conflicting_data)

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json


@pytest.mark.parametrize(
    "route, result",
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
                    "tigh": 3,
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
                {"id": 1, "name": "Exercise 1"},
                {"id": 5, "name": "Unused Exercise"},
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

    assert add_session(client).status_code == HTTPStatus.OK

    resp = client.delete(route)

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data

    resp = client.get(str(Path(route).parent))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == result

    resp = client.delete(str(Path(route).parent / "0"))

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


@pytest.mark.parametrize(
    "first, last",
    [
        ("", ""),
        (None, None),
        (None, "2000-12-31"),
        ("2000-01-01", None),
        ("2000-01-01", "2000-12-31"),
        ("2002-02-01", "2002-02-28"),
    ],
)
@pytest.mark.parametrize(
    "user_id",
    [1, 2],
)
@pytest.mark.parametrize(
    "kind",
    ["bodyweight", "bodyfat", "period", "workouts", "exercise"],
)
def test_get_images(client: Client, user_id: int, kind: str, first: str, last: str) -> None:
    args = "&".join(
        [
            *([f"first={first}"] if first is not None else []),
            *([f"last={last}"] if last is not None else []),
        ]
    )
    route = f"/api/images/{kind}?{args}"
    tests.utils.init_db_users()

    assert add_session(client, user_id).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.mimetype == "image/svg+xml"
    assert resp.data

    tests.utils.clear_db()
    tests.utils.init_db_data()

    assert add_session(client, user_id).status_code == HTTPStatus.OK

    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert resp.mimetype == "image/svg+xml"
    assert resp.data


def test_get_images_invalid_kind(client: Client) -> None:
    tests.utils.init_db_users()

    assert add_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/images/invalid")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_get_images_invalid_argument(client: Client) -> None:
    tests.utils.init_db_users()

    assert add_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/images/workouts?first=invalid")

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json
