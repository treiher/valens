from collections.abc import Generator
from http import HTTPStatus
from pathlib import Path

import pytest
from werkzeug.test import Client

from valens import app


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    test_db = tmp_path / "test.db"
    app.config["DATABASE"] = f"sqlite:///{test_db}"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client, app.app_context():
        yield client


@pytest.mark.parametrize(
    "route",
    [
        "/",
        "/manifest.json",
        "/index.css",
        "/valens-frontend.js",
        "/valens-frontend_bg.wasm",
        "/service-worker.js",
    ],
)
def test_static_files(client: Client, route: str) -> None:
    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
