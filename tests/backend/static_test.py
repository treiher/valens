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


@pytest.mark.parametrize("route", ["/", "/home"])
def test_html_routes(client: Client, route: str) -> None:
    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert "</html>" in resp.get_data().decode("utf-8"), resp.content_encoding


@pytest.mark.parametrize(
    "route",
    [
        "/main.css",
        "/manifest.json",
        "/sw.js",
        "/valens-web-app-dioxus.js",
    ],
)
def test_static_files(client: Client, route: str) -> None:
    resp = client.get(route)

    assert resp.status_code == HTTPStatus.OK
    assert "</html>" not in resp.get_data().decode("utf-8"), resp.content_encoding
