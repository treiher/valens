from collections.abc import Generator
from http import HTTPStatus
from pathlib import Path

import pytest
from werkzeug.test import Client

from valens import app


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client, app.app_context():
        yield client


def test_root(client: Client) -> None:
    resp = client.get("/")

    assert resp.status_code == HTTPStatus.MOVED_PERMANENTLY
    assert resp.location == "app"


@pytest.mark.parametrize(
    "route",
    [
        "/app",
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
