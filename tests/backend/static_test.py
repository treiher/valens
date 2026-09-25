from collections.abc import Generator
from contextlib import closing
from http import HTTPStatus
from pathlib import Path

import brotli
import pytest
from werkzeug.test import Client

from valens import app, version

INDEX = "<html></html>"
STYLESHEET = "body {}"
NESTED_STYLESHEET = "div {}"
SERVICE_WORKER = "self.addEventListener();"
SCRIPT = "export default function () {}"


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True

    with app.test_client() as client, app.app_context():
        yield client


@pytest.fixture(name="static_client")
def fixture_static_client(
    client: Client, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> Client:
    """
    Serve static files from a directory of the test.

    The files of the package depend on whether the frontend was built, and the backend test targets
    create missing ones as empty placeholders.
    """
    assets = tmp_path / "static/assets"
    generated = tmp_path / "static/generated"
    assets.mkdir(parents=True)
    (assets / "sub").mkdir()
    generated.mkdir(parents=True)

    (assets / "manifest.json").write_text("{}")
    (assets / "sub/main.css").write_text(NESTED_STYLESHEET)
    (generated / "index.html").write_text(INDEX)
    (generated / "main.css").write_text(STYLESHEET)
    (generated / "sw.js").write_text(SERVICE_WORKER)
    (generated / "valens-web-app-dioxus.js").write_text(SCRIPT)
    (generated / "index.html.br").write_bytes(brotli.compress(INDEX.encode()))
    (generated / "main.css.br").write_bytes(brotli.compress(STYLESHEET.encode()))

    monkeypatch.setattr(app, "root_path", str(tmp_path))

    return client


@pytest.mark.parametrize("route", ["/", "/home"])
def test_html_routes(static_client: Client, route: str) -> None:
    with closing(static_client.get(route)) as resp:
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
def test_static_files(static_client: Client, route: str) -> None:
    with closing(static_client.get(route)) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert "</html>" not in resp.get_data().decode("utf-8"), resp.content_encoding


@pytest.mark.parametrize(
    ("route", "content"),
    [
        # The index of the generated directory, reached directly, by name and as fallback for a
        # route of the app.
        ("/", INDEX),
        ("/index.html", INDEX),
        ("/home", INDEX),
        # A file of the generated directory.
        ("/main.css", STYLESHEET),
    ],
)
def test_variant_sent(static_client: Client, route: str, content: str) -> None:
    with closing(static_client.get(route, headers={"Accept-Encoding": "br"})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Content-Encoding"] == "br"
        assert resp.headers["Vary"] == "Accept-Encoding"
        assert brotli.decompress(resp.get_data()).decode("utf-8") == content


@pytest.mark.parametrize("query", [f"v={version.get()}", "v=0.1.0"])
def test_versioned_file(static_client: Client, query: str) -> None:
    with closing(static_client.get(f"/main.css?{query}")) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.get_data().decode("utf-8") == STYLESHEET

    with closing(static_client.get(f"/main.css?{query}", headers={"Accept-Encoding": "br"})) as (
        resp
    ):
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Content-Encoding"] == "br"
        assert brotli.decompress(resp.get_data()).decode("utf-8") == STYLESHEET


@pytest.mark.parametrize(
    ("route", "accept_encoding"),
    [
        ("/", "gzip"),
        ("/home", "gzip"),
        ("/main.css", "gzip"),
        ("/main.css", "br"),
    ],
)
def test_version_header(static_client: Client, route: str, accept_encoding: str) -> None:
    with closing(static_client.get(route, headers={"Accept-Encoding": accept_encoding})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Valens-Version"] == version.get()


def test_no_version_header_for_asset(static_client: Client) -> None:
    with closing(static_client.get("/manifest.json", headers={"Accept-Encoding": "br"})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert "Valens-Version" not in resp.headers


@pytest.mark.parametrize("accept_encoding", ["gzip", "br;q=0"])
def test_variant_not_accepted(static_client: Client, accept_encoding: str) -> None:
    with closing(static_client.get("/main.css", headers={"Accept-Encoding": accept_encoding})) as (
        resp
    ):
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Vary"] == "Accept-Encoding"
        assert resp.headers.get("Content-Encoding") != "br"


def test_no_variant(static_client: Client) -> None:
    with closing(static_client.get("/manifest.json", headers={"Accept-Encoding": "br"})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Vary"] == "Accept-Encoding"
        assert "Content-Encoding" not in resp.headers


def test_no_variant_for_subdirectory(static_client: Client) -> None:
    with closing(static_client.get("/sub/main.css", headers={"Accept-Encoding": "br"})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Vary"] == "Accept-Encoding"
        assert "Content-Encoding" not in resp.headers
        assert resp.get_data().decode("utf-8") == NESTED_STYLESHEET


def test_variant_requested_by_name(static_client: Client) -> None:
    with closing(static_client.get("/main.css.br", headers={"Accept-Encoding": "br"})) as resp:
        assert resp.status_code == HTTPStatus.OK
        assert resp.headers["Content-Encoding"] == "br"
        assert brotli.decompress(resp.get_data()).decode("utf-8") == STYLESHEET


@pytest.mark.parametrize("accept_encoding", ["br", "gzip"])
def test_revalidation(static_client: Client, accept_encoding: str) -> None:
    with closing(static_client.get("/main.css", headers={"Accept-Encoding": accept_encoding})) as (
        resp
    ):
        etag = resp.headers["ETag"]

    with closing(
        static_client.get(
            "/main.css",
            headers={"Accept-Encoding": accept_encoding, "If-None-Match": etag},
        )
    ) as resp:
        assert resp.status_code == HTTPStatus.NOT_MODIFIED
