from http import HTTPStatus
from pathlib import Path
from typing import Generator

import pytest
from werkzeug.test import Client

from valens import app


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client:
        with app.app_context():
            yield client


@pytest.mark.parametrize(
    "route",
    [
        "/",
        "/manifest.json",
        "/index.css",
        "/index.js",
        "/index.wasm",
        "/service-worker.js",
    ],
)
def test_static_files(client: Client, route: str) -> None:
    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
