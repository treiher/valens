import datetime
import re
import tempfile
from typing import Any

import pytest  # type: ignore
from werkzeug.middleware.dispatcher import DispatcherMiddleware
from werkzeug.test import Client
from werkzeug.wrappers import BaseResponse

from tests import utils
from valens import web


@pytest.fixture(name="client", scope="module")
def fixture_client() -> Client:
    web.app.config["TESTING"] = True
    app = DispatcherMiddleware(web.app, {"/test": web.app})
    return Client(app, BaseResponse)


def assert_resources_available(client: Client, data: bytes) -> None:
    for r in re.findall(r' (?:href|src)="([^"]*)"', data.decode("utf-8")):
        assert client.get(r).status_code == 200, f"{r} not found"


@pytest.mark.parametrize(
    "route",
    [
        "/",
        "/bodyweight",
        "/exercise/foo",
        "/exercises",
        "/image/bodyweight",
        "/image/exercise",
        "/image/workouts",
        "/workouts",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability(client: Client, path: str, route: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: utils.config(tmp_dir))

        url = path + route
        resp = client.get(url)
        assert resp.status_code == 200, f"{url} not found"
        assert_resources_available(client, resp.data)


@pytest.mark.parametrize(
    "url", ["/image/foo"],
)
def test_non_availability(client: Client, url: str) -> None:
    resp = client.get(url)
    assert resp.status_code == 404, f"{url} found"


def test_bodyweight(client: Client, monkeypatch: Any) -> None:
    args = {}
    monkeypatch.setattr(
        web.storage, "write_bodyweight", lambda x, y: args.update({"date": x, "weight": y})
    )
    resp = client.post("/bodyweight", data={"date": "2002-02-20", "weight": "42"})
    assert resp.status_code == 200
    assert args["date"] == datetime.date.fromisoformat("2002-02-20")
    assert args["weight"] == 42
