import datetime
import re
import tempfile
from typing import Any

import pandas as pd
import pytest
from werkzeug.datastructures import MultiDict
from werkzeug.middleware.dispatcher import DispatcherMiddleware
from werkzeug.test import Client
from werkzeug.wrappers import BaseResponse

import tests.data
import tests.utils
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
        "/workout/2002-02-20",
    ],
)
@pytest.mark.parametrize("path", ["", "/test"])
def test_availability(client: Client, path: str, route: str, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

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
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        resp = client.get("/bodyweight?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for d in tests.data.BODYWEIGHT:
            assert str(d) in resp.data.decode("utf-8")


def test_bodyweight_add(client: Client, monkeypatch: Any) -> None:
    args = {}
    monkeypatch.setattr(
        web.storage, "write_bodyweight", lambda x, y: args.update({"date": x, "weight": y})
    )
    resp = client.post("/bodyweight", data={"date": "2002-02-20", "weight": "42"})
    assert resp.status_code == 200
    assert args["date"] == datetime.date.fromisoformat("2002-02-20")
    assert args["weight"] == 42


def test_exercise(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        for date, exercises in tests.data.WORKOUTS.items():
            for exercise in exercises:
                resp = client.get(f"/exercise/{exercise}?first=2002-02-01&last=2002-03-01")
                assert resp.status_code == 200
                assert str(date) in resp.data.decode("utf-8")


def test_workouts(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        resp = client.get("/workouts?first=2002-02-01&last=2002-03-01")
        assert resp.status_code == 200
        for d in tests.data.WORKOUTS:
            assert str(d) in resp.data.decode("utf-8")


def test_workouts_add(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x: args.update({"df": x}))
        resp = client.post("/workouts", data={"date": "2002-02-24", "template": "T1"})
        assert resp.status_code == 302

        templates_df = tests.data.TEMPLATES_DF["T1"].copy()
        templates_df["date"] = [datetime.date(2002, 2, 24)] * len(templates_df)
        assert args["df"].equals(pd.concat([tests.data.WORKOUTS_DF, templates_df]))


def test_workouts_add_existing(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x: args.update({"df": x}))
        resp = client.post("/workouts", data={"date": "2002-02-20", "template": "T1"})
        assert resp.status_code == 200
        assert "df" not in args


def test_workout_delete(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22", data={"delete": ""})
        assert resp.status_code == 302
        assert args["df"].equals(
            tests.data.WORKOUTS_DF[tests.data.WORKOUTS_DF["date"] != datetime.date(2002, 2, 22)]
        )


def test_workout_save(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22")
        assert resp.status_code == 200
        assert args["df"].equals(
            tests.data.WORKOUTS_DF[tests.data.WORKOUTS_DF["date"] != datetime.date(2002, 2, 22)]
        )

        resp = client.post(
            "/workout/2002-02-22",
            data=MultiDict(
                [
                    (k, e)
                    for k, v in tests.data.WORKOUTS[datetime.date(2002, 2, 22)].items()
                    for e in v
                ]
            ),
        )
        assert resp.status_code == 200
        assert args["df"][["date", "exercise", "reps", "time", "weight", "rpe"]].equals(
            tests.data.WORKOUTS_DF[["date", "exercise", "reps", "time", "weight", "rpe"]]
        )


def test_workout_save_error(client: Client, monkeypatch: Any) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(web.storage.utils, "parse_config", lambda: tests.utils.config(tmp_dir))

        args = {}
        monkeypatch.setattr(web.storage, "write_workouts", lambda x: args.update({"df": x}))
        resp = client.post("/workout/2002-02-22", data={"E4": "error"})
        assert resp.status_code == 200
        assert "df" not in args
