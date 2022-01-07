from pathlib import Path
from typing import Generator

import pytest
from werkzeug.test import Client, TestResponse as Response

import tests.data
import tests.utils
from valens import api, app  # pylint: disable = unused-import


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


def test_get_version(client: Client) -> None:
    resp = client.get("/api/version")

    assert resp.status_code == 200
    assert resp.json


def test_session(client: Client) -> None:
    tests.utils.init_db_data()

    resp = add_session(client)
    assert resp.status_code == 200
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = client.get("/api/session")
    assert resp.status_code == 200
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = delete_session(client)
    assert resp.status_code == 204
    assert not resp.data


def test_get_session_not_found(client: Client) -> None:
    resp = client.get("/api/session")

    assert resp.status_code == 404
    assert not resp.data


def test_add_session_bad_request(client: Client) -> None:
    resp = client.post("/api/session", json={"invalid": "data"})

    assert resp.status_code == 400
    assert resp.is_json


def test_add_session_not_found(client: Client) -> None:
    resp = client.post("/api/session", json={"id": 1})

    assert resp.status_code == 404
    assert not resp.data


def test_add_session_invalid_content_type(client: Client) -> None:
    resp = client.post("/api/session", data={"id": 1})

    assert resp.status_code == 415
    assert not resp.data


def test_get_users(client: Client) -> None:
    resp = client.get("/api/users")

    assert resp.status_code == 200
    assert resp.json == []

    tests.utils.init_db_data()
    resp = client.get("/api/users")

    assert resp.status_code == 200
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Bob", "sex": 1},
    ]


def test_get_user(client: Client) -> None:
    resp = client.get("/api/users/1")

    assert resp.status_code == 401
    assert not resp.data

    tests.utils.init_db_data()

    resp = add_session(client)
    assert resp.status_code == 200

    resp = client.get("/api/users/0")

    assert resp.status_code == 404
    assert not resp.data

    resp = client.get("/api/users/1")

    assert resp.status_code == 200
    assert resp.json == {"id": 1, "name": "Alice", "sex": 0}

    resp = delete_session(client)
    assert resp.status_code == 204
    assert not resp.data


def test_add_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.post("/api/users", json={"name": "Carol", "sex": 0})

    assert resp.status_code == 201
    assert resp.json == {"id": 3, "name": "Carol", "sex": 0}

    resp = client.get("/api/users")

    assert resp.status_code == 200
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Bob", "sex": 1},
        {"id": 3, "name": "Carol", "sex": 0},
    ]


def test_add_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.post("/api/users", json={"name": " Alice ", "sex": 0})

    assert resp.status_code == 409
    assert resp.json


def test_add_user_invalid_content_type(client: Client) -> None:
    resp = client.post("/api/users", data={"name": "Carol", "sex": 0})

    assert resp.status_code == 415
    assert not resp.data


def test_add_user_bad_request(client: Client) -> None:
    resp = client.post("/api/users", json={"invalid": "data"})

    assert resp.status_code == 400
    assert resp.is_json


def test_edit_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/2", json={"name": "Carol", "sex": 0})

    assert resp.status_code == 200
    assert resp.json == {"id": 2, "name": "Carol", "sex": 0}

    resp = client.get("/api/users")

    assert resp.status_code == 200
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
        {"id": 2, "name": "Carol", "sex": 0},
    ]


def test_edit_user_not_found(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/3", json={"name": "Carol", "sex": 0})

    assert resp.status_code == 404
    assert not resp.data


def test_edit_user_conflict(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/2", json={"name": " Alice ", "sex": 0})

    assert resp.status_code == 409
    assert resp.json


def test_edit_user_invalid_content_type(client: Client) -> None:
    resp = client.put("/api/users/2", data={"name": "Carol", "sex": 0})

    assert resp.status_code == 415
    assert not resp.data


def test_edit_user_bad_request(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.put("/api/users/2", json={"invalid": "data"})

    assert resp.status_code == 400
    assert resp.is_json


def test_delete_user(client: Client) -> None:
    tests.utils.init_db_data()

    resp = client.delete("/api/users/2")

    assert resp.status_code == 204
    assert not resp.data

    resp = client.get("/api/users")

    assert resp.status_code == 200
    assert resp.json == [
        {"id": 1, "name": "Alice", "sex": 0},
    ]

    resp = client.delete("/api/users/2")

    assert resp.status_code == 404
    assert not resp.data